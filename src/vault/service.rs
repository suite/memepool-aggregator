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
    instructions::fill_withdraw_request,
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
        let tx = fill_withdraw_request(
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

pub async fn process_lp_deposit(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    total_wsol_deposit: u64,
) -> Result<(), String> {
    // Get pool state and amounts
    let pool_state = get_pool_state(raydium_program, POOL_ADDRESS)
        .await
        .map_err(|e| format!("Failed to get pool state: {}", e))?;
    
    let pool_amounts = pool_state.get_vault_amounts(spl_program)
        .await
        .map_err(|e| format!("Failed to get pool amounts: {}", e))?;

    // Calculate swap amount (roughly half of deposit)
    let wsol_to_swap = total_wsol_deposit / 2;
    
    // Define slippage tolerance (1%)
    let slippage = 99; // 99% means 1% slippage tolerance
    
    // Calculate minimum amount out using constant product (x * y = k) formula
    // amount_out = (dx * y) / (x + dx)
    // where:
    //   dx = wsol_to_swap (amount in)
    //   x = pool_amounts.0 (WSOL pool balance)
    //   y = pool_amounts.1 (MEME pool balance)
    let amount_out = (wsol_to_swap as u128)
        .checked_mul(pool_amounts.1 as u128)
        .and_then(|numerator| {
            (pool_amounts.0 as u128)
                .checked_add(wsol_to_swap as u128)
                .and_then(|denominator| numerator.checked_div(denominator))
        })
        .ok_or("Failed to calculate amount out: overflow or division by zero")?;

    // Apply slippage tolerance (e.g., 99 for 99%)
    let minimum_amount_out = amount_out
        .checked_mul(slippage as u128)
        .and_then(|with_slippage| with_slippage.checked_div(100))
        .and_then(|final_result| u64::try_from(final_result).ok())
        .ok_or("Failed to apply slippage: overflow or conversion error")?;

    println!("SOL to swap: {}, Minimum amount out: {}", wsol_to_swap, minimum_amount_out);

    // Execute the swap
    let swap_tx = super::instructions::lp_swap(
        program,
        raydium_program,
        aggregator_keypair,
        wsol_to_swap,
        minimum_amount_out,
    ).await?;

    println!("Swap transaction completed: {}", swap_tx); // (266, 141)

    // Calculate expected LP token amounts
    let maximum_token0_amount = total_wsol_deposit - wsol_to_swap; // Remaining WSOL
    let maximum_token1_amount = minimum_amount_out; // Amount received from swap

    // Calculate expected LP token amount based on the smaller ratio
    // Following Raydium's specified_tokens_to_lp_tokens logic
    let lp_amount = std::cmp::min(
        ((maximum_token0_amount as u128)
            .checked_mul(pool_state.lp_supply as u128)
            .and_then(|product| product.checked_div(pool_amounts.0 as u128))
            .and_then(|result| u64::try_from(result).ok()))
            .ok_or("Failed to calculate LP amount from token0")?,
        ((maximum_token1_amount as u128)
            .checked_mul(pool_state.lp_supply as u128)
            .and_then(|product| product.checked_div(pool_amounts.1 as u128))
            .and_then(|result| u64::try_from(result).ok()))
            .ok_or("Failed to calculate LP amount from token1")?
    );

    println!("Ready for LP deposit with values:");
    println!("lpTokenAmount: {}", lp_amount);
    println!("maximum_token0_amount (WSOL): {}", maximum_token0_amount);
    println!("maximum_token1_amount (Other): {}", maximum_token1_amount);

    Ok(())
} 