use std::rc::Rc;

use anchor_client::{
    solana_sdk::signature::Keypair,
    Program
};
use anchor_lang::prelude::Pubkey;

use crate::{raydium::get_pool_state, utils::POOL_ADDRESS};

use super::utils::calculate_lp_amount;

pub async fn process_lp_swap(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    swap_amount: u64,
    base_token: bool, // true: swap WSOL into other token, false: swap other token into WSOL
    slippage: u64, // slippage tolerance (e.g., 99 for 99%)
) -> Result<(String, u64), String> {
    // TODO: take in pool_state as param, maybe not
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

    if minimum_amount_out == 0 {
        return Err("Minimum output amount cannot be zero".to_string());
    }

    println!(
        "Swapping {} {} for minimum {} {}",
        swap_amount,
        if base_token { "WSOL" } else { "tokens" },
        minimum_amount_out,
        if base_token { "tokens" } else { "WSOL" }
    );

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

pub async fn process_lp_deposit(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    pool_address: Pubkey,
    deposit_amount: u64, // Amount of WSOL you want to deposit, will split and swap into lp
) -> Result<(String, String, u64), String> {  // Return (swap_tx, deposit_tx, lp_token_amount)
    // Swap half
    let wsol_to_swap = deposit_amount.checked_div(2)
        .ok_or("Failed to calculate WSOL swap amount: division error")?;

    let wsol_leftover = deposit_amount.checked_sub(wsol_to_swap)
        .ok_or("Failed to calculate WSOL leftover amount: subtraction error")?;

    let slippage = 95;
    let (swap_tx, minimum_amount_out) = process_lp_swap(
        program,
        raydium_program,
        spl_program,
        aggregator_keypair,
        wsol_to_swap,
        true,
        slippage,
    )
    .await
    .map_err(|e| format!("Failed to process LP swap: {}", e))?;

    println!("Swapped {} WSOL for {} tokens", wsol_to_swap, minimum_amount_out);
    println!("Tx: {}", swap_tx);

    // NOTE: Pull in new pool_state after swap
    let pool_state = get_pool_state(raydium_program, pool_address)
        .await
        .map_err(|e| format!("Failed to get pool state: {}", e))?;

    let lp_supply = pool_state.lp_supply;
    let (pool_amount0, pool_amount1) = pool_state.get_vault_amounts(spl_program).await
        .map_err(|e| format!("Failed to get vault amounts: {}", e))?;

    let lp_token_amount = calculate_lp_amount(
        wsol_leftover,
        minimum_amount_out,
        lp_supply,
        pool_amount0,
        pool_amount1
    )?;

    // TODO: add slippage?

    // Make sure lp_token_amount != 0
    if lp_token_amount == 0 {
        return Err("LP token amount cannot be zero".to_string());
    }

    let deposit_tx = super::instructions::lp_deposit(
        program,
        raydium_program,
        aggregator_keypair,
        lp_token_amount,
        wsol_leftover,
        minimum_amount_out
    ).await?;

    println!("Deposited {} WSOL and {} tokens for {} LP tokens", wsol_leftover, minimum_amount_out, lp_token_amount);
    println!("Tx: {}", deposit_tx);

    Ok((swap_tx, deposit_tx, lp_token_amount))
}

pub async fn process_lp_withdraw(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    pool_address: Pubkey,
    withdraw_amount: u64, // Total WSOL amount you want to receive
) -> Result<String, String> {
    // Get pool state and amounts
    let pool_state = get_pool_state(raydium_program, pool_address)
        .await
        .map_err(|e| format!("Failed to get pool state: {}", e))?;
    
    let (pool_amount0, pool_amount1) = pool_state.get_vault_amounts(spl_program)
        .await
        .map_err(|e| format!("Failed to get vault amounts: {}", e))?;

    println!("Current pool amounts - WSOL: {}, Token1: {}", pool_amount0, pool_amount1);
    
    let lp_supply = pool_state.lp_supply;
    
    let wsol_from_swap = withdraw_amount.checked_div(2)
        .ok_or("Failed to calculate direct WSOL amount")?;

    let wsol_left_over = withdraw_amount.checked_sub(wsol_from_swap)
        .ok_or("Failed to calculate swap WSOL amount: subtraction error")?;

    // Calculate how much of token1 we need (equivalent to wsol_from_swap in value)
    // Formula: token1_needed = wsol_from_swap * (pool_amount1 / pool_amount0)
    let token1_needed = (wsol_from_swap as u128)
        .checked_mul(pool_amount1 as u128)
        .and_then(|product| product.checked_div(pool_amount0 as u128))
        .and_then(|result| u64::try_from(result).ok())
        .ok_or("Failed to calculate token1 needed")?;
    
    // Use the utility function to calculate LP tokens to withdraw
    let lp_token_amount = calculate_lp_amount(
        wsol_left_over,
        token1_needed,
        lp_supply,
        pool_amount0,
        pool_amount1
    )?;
    
    // TODO: move chck into calculate_lp_amount
    if lp_token_amount == 0 {
        return Err("Calculated LP token amount is too small".to_string());
    }

    // Calculate minimum amounts with slippage tolerance
    let slippage = 90; //
    let minimum_token_0_amount = wsol_left_over
        .checked_mul(slippage)
        .and_then(|product| product.checked_div(100))
        .ok_or("Failed to calculate minimum token 0 amount")?;

    let minimum_token_1_amount = token1_needed
        .checked_mul(slippage)
        .and_then(|product| product.checked_div(100))
        .ok_or("Failed to calculate minimum token 1 amount")?;

    println!("Withdrawing {} LP tokens for at least {} token0 and {} token1", 
        lp_token_amount, minimum_token_0_amount, minimum_token_1_amount);
    
    // Execute the withdrawal
    let withdraw_tx = super::instructions::lp_withdraw(
        program,
        raydium_program,
        aggregator_keypair,
        lp_token_amount,
        minimum_token_0_amount,
        minimum_token_1_amount
    ).await?;
    
    println!("Withdrew successfully. Tx: {}", withdraw_tx);
    
    // Sswap token1 into WSOL if needed
    let slippage = 95; // 95% slippage tolerance
    let swap_tx = process_lp_swap(
        program,
        raydium_program,
        spl_program,
        aggregator_keypair,
        minimum_token_1_amount,
        false, // swap token1 to WSOL
        slippage,
    )
    .await
    .map_err(|e| format!("Failed to swap token1 to WSOL: {}", e))?;
    
    println!("Swapped {} token1 for WSOL", token1_needed);
    println!("Swap tx: {}", swap_tx.0);
    
    // Return the withdrawal transaction signature
    Ok(withdraw_tx)
}