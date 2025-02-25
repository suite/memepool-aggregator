use anchor_client::{
    solana_sdk::signature::Keypair,
    Program
};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::Mint;
use std::rc::Rc;

use crate::{memepool, raydium::get_pool_state};
use super::{
    utils::{VAULT_PDA, MEME_MINT_PDA, POOL_ADDRESS},
    instructions::vault_fill_withdraw,
};

pub async fn process_withdraw_request(
    program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    request_pubkey: Pubkey,
    withdraw_request: memepool::accounts::WithdrawRequest,
) -> Result<(), String> {
    // Get the vault account
    let vault = program.account::<memepool::accounts::Vault>(*VAULT_PDA)
        .await
        .map_err(|e| format!("Failed to fetch vault account: {}", e))?;
    
    // Get meme token supply
    let mint = spl_program.account::<Mint>(*MEME_MINT_PDA)
        .await
        .map_err(|e| format!("Failed to fetch mint account: {}", e))?;
    let meme_token_supply = mint.supply;

    // Calculate required SOL (  withdraw_request.meme_amt * (vault.lamports / meme_token_supply) )
    let required_sol = (withdraw_request.meme_amt as u64)
        .checked_mul(vault.lamports)
        .and_then(|product| product.checked_div(meme_token_supply))
        .ok_or("Failed to calculate required SOL: overflow or division by zero")?;

    if required_sol <= vault.lamports {
        println!("Processing withdraw request {} with {} SOL", request_pubkey, required_sol);
        
        // Call fill_withdraw_request with the calculated amount
        let tx = vault_fill_withdraw(
            program,
            aggregator_keypair,
            request_pubkey,
            &withdraw_request,
            required_sol
        ).await?;
        
        println!("Fill withdraw request transaction: {}", tx);
        Ok(())
    } else {
        Err(format!(
            "Insufficient SOL in vault. Need {} SOL but vault only has {} SOL",
            required_sol, vault.lamports
        ))
    }
}

pub async fn process_withdraw_requests_batch(
    program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    withdraw_requests: Vec<(Pubkey, memepool::accounts::WithdrawRequest)>
) -> Vec<Result<(), String>> {
    let mut results = Vec::with_capacity(withdraw_requests.len());
    
    for (request_pubkey, withdraw_request) in withdraw_requests {
        println!("Starting to process request {}", request_pubkey);
        
        let result = process_withdraw_request(
            program,
            spl_program,
            aggregator_keypair,
            request_pubkey,
            withdraw_request
        ).await;
        
        match &result {
            Ok(_) => println!("Successfully processed request {}", request_pubkey),
            Err(e) => println!("Failed to process request {}: {}", request_pubkey, e),
        }
        
        results.push(result);
    }
    
    results
}

pub async fn process_lp_swap(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    swap_amount: u64,
    base_token: bool, // true: swap WSOL into other token, false: swap other token into WSOL
    slippage: u64, // slippage tolerance (e.g., 99 for 99%)
) -> Result<(String, u64), String> {
    // Get pool state and amounts
    let pool_state = get_pool_state(raydium_program, POOL_ADDRESS)
        .await
        .map_err(|e| format!("Failed to get pool state: {}", e))?;
    
    let pool_amounts = pool_state.get_vault_amounts(spl_program)
        .await
        .map_err(|e| format!("Failed to get pool amounts: {}", e))?;
    
    // Calculate expected output based on the current pool ratio and direction
    let amount_out = if base_token {
        // Swap WSOL to other token: amount_out = swap_amount * (pool_amounts.1 / pool_amounts.0)
        (swap_amount as u128)
            .checked_mul(pool_amounts.1 as u128)
            .and_then(|product| product.checked_div(pool_amounts.0 as u128))
            .ok_or("Failed to calculate amount out: overflow or division by zero")?
    } else {
        // Swap other token to WSOL: amount_out = swap_amount * (pool_amounts.0 / pool_amounts.1)
        (swap_amount as u128)
            .checked_mul(pool_amounts.0 as u128)
            .and_then(|product| product.checked_div(pool_amounts.1 as u128))
            .ok_or("Failed to calculate amount out: overflow or division by zero")?
    };

    let minimum_amount_out = amount_out
        .checked_mul(slippage as u128)
        .and_then(|with_slippage| with_slippage.checked_div(100))
        .and_then(|final_result| u64::try_from(final_result).ok())
        .ok_or("Failed to apply slippage: overflow or conversion error")?;

    // Execute the swap
    let swap_tx = super::instructions::lp_swap(
        program,
        raydium_program,
        aggregator_keypair,
        swap_amount,
        minimum_amount_out,
        base_token, // Pass base_token to determine swap direction
    ).await?;

    // Return the transaction signature and the expected output amount
    Ok((swap_tx, minimum_amount_out))
}

/// Calculate the expected LP token amount based on the token amounts and pool state
/// 
/// This follows Raydium's specified_tokens_to_lp_tokens logic:
/// lp_amount = min(
///    token0_amount * (lp_supply / token0_pool_amount),
///    token1_amount * (lp_supply / token1_pool_amount)
/// )
/// 
/// * `token0_amount` - Amount of token0 (usually WSOL)
/// * `token1_amount` - Amount of token1 (the other token)
/// * `pool_state` - The current pool state
/// * `pool_amounts` - The current pool amounts (token0, token1)
pub fn calculate_lp_amount(
    token0_amount: u64,
    token1_amount: u64,
    lp_supply: u64,
    pool_amount0: u64,
    pool_amount1: u64,
) -> Result<u64, String> {
    let lp_amount = std::cmp::min(
        ((token0_amount as u128)
            .checked_mul(lp_supply as u128)
            .and_then(|product| product.checked_div(pool_amount0 as u128))
            .and_then(|result| u64::try_from(result).ok()))
            .ok_or("Failed to calculate LP amount from token0")?,
        ((token1_amount as u128)
            .checked_mul(lp_supply as u128)
            .and_then(|product| product.checked_div(pool_amount1 as u128))
            .and_then(|result| u64::try_from(result).ok()))
            .ok_or("Failed to calculate LP amount from token1")?
    );
    
    Ok(lp_amount)
}

/// Calculate the expected LP token amount based on a swap followed by LP deposit
/// 
/// * `swap_amount` - Amount to swap
/// * `amount_out` - Expected amount received from swap
/// * `base_token` - Direction of swap (true: WSOL to other, false: other to WSOL)
/// * `pool_state` - The current pool state
/// * `pool_amounts` - The current pool amounts (token0, token1)
pub fn calculate_lp_amount_after_swap(
    swap_amount: u64,
    amount_out: u64,
    base_token: bool,
    lp_supply: u64,
    pool_amount0: u64,
    pool_amount1: u64,
) -> Result<u64, String> {
    // Calculate expected LP token amounts based on swap direction
    let (token0_amount, token1_amount) = if base_token {
        // Token0 is WSOL, Token1 is other token
        (swap_amount, amount_out)
    } else {
        // Token0 is WSOL, Token1 is other token
        (amount_out, swap_amount)
    };
    
    calculate_lp_amount(token0_amount, token1_amount, lp_supply, pool_amount0, pool_amount1)
} 