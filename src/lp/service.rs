use std::rc::Rc;

use anchor_client::{solana_sdk::signature::Keypair, Program};
use anchor_lang::prelude::Pubkey;

use crate::{
    raydium::get_pool_state,
    utils::{get_token_account_balance, POOL_ADDRESS, VAULT_PDA},
};

use super::utils::calculate_lp_amount;

pub async fn process_lp_swap(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    swap_amount: u64,
    base_token: bool, // true: swap WSOL into other token, false: swap other token into WSOL
    slippage: u64,    // slippage tolerance (e.g., 99 for 99%)
) -> Result<(String, u64), String> {
    // TODO: take in pool_state as param, maybe not
    // Get pool state and amounts
    let pool_state = get_pool_state(raydium_program, POOL_ADDRESS)
        .await
        .map_err(|e| format!("Failed to get pool state: {}", e))?;

    let pool_amounts = pool_state
        .get_vault_amounts(spl_program)
        .await
        .map_err(|e| format!("Failed to get pool amounts: {}", e))?;

    println!(
        "Pool amounts - WSOL: {}, Other token: {}",
        pool_amounts.0, pool_amounts.1
    );

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

    println!(
        "Swapping {} token0 for {} token1 (expected: {})",
        swap_amount, minimum_amount_out, amount_out
    );

    if minimum_amount_out == 0 {
        return Err("Minimum output amount cannot be zero".to_string());
    }

    // Execute the swap
    let swap_tx = super::instructions::lp_swap(
        program,
        raydium_program,
        aggregator_keypair,
        swap_amount,
        minimum_amount_out,
        base_token, // Pass base_token to determine swap direction
    )
    .await?;

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
) -> Result<(String, String, u64), String> {
    // Return (swap_tx, deposit_tx, lp_token_amount)
    // Swap half
    let wsol_to_swap = deposit_amount
        .checked_div(2)
        .ok_or("Failed to calculate WSOL swap amount: division error")?;

    let wsol_leftover = deposit_amount
        .checked_sub(wsol_to_swap)
        .ok_or("Failed to calculate WSOL leftover amount: subtraction error")?;

    let slippage = 95;
    let (swap_tx, actual_amount_in) = process_lp_swap(
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

    println!(
        "Swapped {} WSOL for {} tokens",
        wsol_to_swap, actual_amount_in
    );
    println!("Tx: {}", swap_tx);

    // NOTE: Pull in new pool_state after swap
    let pool_state = get_pool_state(raydium_program, pool_address)
        .await
        .map_err(|e| format!("Failed to get pool state: {}", e))?;

    let lp_supply = pool_state.lp_supply;
    let (pool_amount0, pool_amount1) = pool_state
        .get_vault_amounts(spl_program)
        .await
        .map_err(|e| format!("Failed to get vault amounts: {}", e))?;

    let lp_token_amount = calculate_lp_amount(
        wsol_leftover,
        actual_amount_in,
        lp_supply,
        pool_amount0,
        pool_amount1,
    )?;

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
        actual_amount_in,
    )
    .await?;

    println!(
        "Deposited {} WSOL and {} tokens for {} LP tokens",
        wsol_leftover, actual_amount_in, lp_token_amount
    );
    println!("Tx: {}", deposit_tx);

    Ok((swap_tx, deposit_tx, lp_token_amount))
}

pub async fn process_lp_withdraw(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    pool_address: Pubkey,
) -> Result<String, String> {
    // Get pool state and amounts
    let pool_state = get_pool_state(raydium_program, pool_address)
        .await
        .map_err(|e| format!("Failed to get pool state: {}", e))?;

    let (pool_amount0, pool_amount1) = pool_state
        .get_vault_amounts(spl_program)
        .await
        .map_err(|e| format!("Failed to get vault amounts: {}", e))?;

    let lp_supply = pool_state.lp_supply;

    println!("lp mint: {}", pool_state.lp_mint);
    println!(
        "Current pool amounts - WSOL: {}, Token1: {}, lp supply: {}",
        pool_amount0, pool_amount1, lp_supply
    );

    // Get the LP token balance of VAULT_PDA
    let lp_balance = get_token_account_balance(spl_program, &*VAULT_PDA, &pool_state.lp_mint)
        .await
        .map_err(|e| format!("Failed to get LP token balance: {}", e))?;

    println!("LP token balance owned by vault: {}", lp_balance);

    // Check if we have any LP tokens to burn
    if lp_balance == 0 {
        return Err("No LP tokens available to withdraw".to_string());
    }

    // Use all available LP tokens for withdrawal
    let lp_to_burn = lp_balance;

    // Calculate expected token amounts based on LP amount
    let wsol_received = (lp_to_burn as u128)
        .checked_mul(pool_amount0 as u128)
        .and_then(|product| product.checked_div(lp_supply as u128))
        .and_then(|result| u64::try_from(result).ok())
        .ok_or("Failed to calculate WSOL received: overflow or conversion error")?;

    let token1_received = (lp_to_burn as u128)
        .checked_mul(pool_amount1 as u128)
        .and_then(|product| product.checked_div(lp_supply as u128))
        .and_then(|result| u64::try_from(result).ok())
        .ok_or("Failed to calculate Token1 received: overflow or conversion error")?;

    let slippage = 95; // 95% slippage tolerance
    let minimum_wsol_received = (wsol_received as u128)
        .checked_mul(slippage as u128)
        .and_then(|product| product.checked_div(100))
        .and_then(|result| u64::try_from(result).ok())
        .ok_or("Failed to calculate minimum WSOL received")?;

    let minimum_token1_received = (token1_received as u128)
        .checked_mul(slippage as u128)
        .and_then(|product| product.checked_div(100))
        .and_then(|result| u64::try_from(result).ok())
        .ok_or("Failed to calculate minimum Token1 received")?;

    println!(
        "Withdrawing {} LP tokens, expecting at least {} WSOL and {} Token1",
        lp_to_burn, minimum_wsol_received, minimum_token1_received
    );

    let withdraw_tx = super::instructions::lp_withdraw(
        program,
        raydium_program,
        aggregator_keypair,
        lp_to_burn,
        minimum_wsol_received,
        minimum_token1_received,
    )
    .await
    .map_err(|e| format!("Failed to execute LP withdrawal: {}", e))?;

    // Get total token1 balance after withdrawal
    let token1_balance =
        get_token_account_balance(spl_program, &*VAULT_PDA, &pool_state.token_1_mint).await?;

    // Swap ALL token1 owned by VAULT_PDA into WSOL
    let slippage = 95; // 95% slippage tolerance
    let (swap_tx, wsol_from_swap) = process_lp_swap(
        program,
        raydium_program,
        spl_program,
        aggregator_keypair,
        token1_balance, // Swap ALL token1 balance
        false,          // Swap Token1 to WSOL
        slippage,
    )
    .await
    .map_err(|e| format!("Failed to swap Token1 to WSOL: {}", e))?;

    println!(
        "Swapped {} Token1 for at least {} WSOL. Swap tx: {}",
        token1_balance, wsol_from_swap, swap_tx
    );

    // Return the withdrawal transaction signature
    Ok(withdraw_tx)
}
