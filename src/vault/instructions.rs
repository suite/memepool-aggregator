use anchor_client::{
    solana_sdk::{signature::Keypair, system_program, signer::Signer},
    Program
};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::spl_token;
use std::rc::Rc;

use crate::{memepool, raydium::get_pool_state};
use super::utils::{get_oracle_pda, CP_SWAP_PROGRAM, MEME_MINT_PDA, POOL_ADDRESS, SWAP_AUTHORITY_PDA, VAULT_PDA, WSOL_MINT};

pub async fn fill_withdraw_request(
    program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    request_pubkey: Pubkey,
    withdraw_request: &memepool::accounts::WithdrawRequest,
    fill_lamports: u64,
) -> Result<String, String> {
    let vault_address = *VAULT_PDA;
    let meme_mint = *MEME_MINT_PDA;
    let wsol_mint = WSOL_MINT;
    
    let withdraw_request_meme_ata = anchor_spl::associated_token::get_associated_token_address(
        &request_pubkey,
        &meme_mint,
    );

    let vault_wsol_ata = anchor_spl::associated_token::get_associated_token_address(
        &vault_address,
        &wsol_mint,
    );

    let temp_vault_wsol_ata = anchor_spl::associated_token::get_associated_token_address(
        &request_pubkey,
        &wsol_mint,
    );

    let accounts = memepool::client::accounts::VaultFillWithdraw {
        aggregator: aggregator_keypair.pubkey(),
        withdrawer: withdraw_request.user,
        withdraw_request: request_pubkey,
        vault: vault_address,
        meme_mint,
        withdraw_request_meme_ata,
        wsol_mint,
        vault_wsol_ata,
        temp_vault_wsol_ata,
        system_program: system_program::ID,
        token_program: spl_token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
    };

    let args = memepool::client::args::VaultFillWithdraw { fill_lamports };
    
    let tx_builder = program
        .request()
        .args(args)
        .accounts(accounts);

    let tx = tx_builder
        .send()
        .await
        .map_err(|e| format!("Failed to send fill withdraw transaction: {}", e))?;

    Ok(tx.to_string())
}

pub async fn lp_swap(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    amount_in: u64,
    minimum_amount_out: u64,
    is_base_token: bool, // true: swap WSOL to test token, false: swap test token to WSOL
) -> Result<String, String> {
    let vault_address = *VAULT_PDA;
    let cp_swap_program = CP_SWAP_PROGRAM;
    let pool_address = POOL_ADDRESS;

    let pool_state = get_pool_state(raydium_program, pool_address)
        .await
        .map_err(|e| format!("Failed to get pool state: {}", e))?;

    let config_id = pool_state.amm_config;
    let vault_a = pool_state.token_0_vault;
    let vault_b = pool_state.token_1_vault;
    let mint_a = pool_state.token_0_mint;
    let mint_b = pool_state.token_1_mint;

    // Get authority PDA for the swap program
    let authority = *SWAP_AUTHORITY_PDA;
    
    // Determine input and output based on swap direction
    let (input_token_account, output_token_account, input_vault, output_vault, input_token_mint, output_token_mint) = 
    if is_base_token {
        (
            anchor_spl::associated_token::get_associated_token_address(&vault_address, &mint_a),
            anchor_spl::associated_token::get_associated_token_address(&vault_address, &mint_b),
            vault_a,
            vault_b,
            mint_a,
            mint_b,
        )
    } else {
        (
            anchor_spl::associated_token::get_associated_token_address(&vault_address, &mint_b),
            anchor_spl::associated_token::get_associated_token_address(&vault_address, &mint_a),
            vault_b,
            vault_a,
            mint_b,
            mint_a
        )
    };

    // Get oracle observation address
    let observation_address = get_oracle_pda(&pool_address);

    let accounts = memepool::client::accounts::LpSwap {
        aggregator: aggregator_keypair.pubkey(),
        vault: vault_address,
        cp_swap_program,
        authority,
        amm_config: config_id,
        pool_state: pool_address,
        input_token_account,
        output_token_account,
        input_vault,
        output_vault,
        input_token_program: spl_token::ID,
        output_token_program: spl_token::ID,
        input_token_mint,
        output_token_mint,
        observation_state: observation_address,
    };

    let args = memepool::client::args::LpSwap {
        amount_in,
        minimum_amount_out,
    };
    
    let tx_builder = program
        .request()
        .args(args)
        .accounts(accounts);

    let tx = match tx_builder
        .send()
        .await 
    {
        Ok(sig) => Ok(sig.to_string()),
        Err(e) => {
            println!("\nTransaction failed with error:");
            // println!("{:#?}", e);
            
            // TODO: TEMP TO GET PROGRAM LOGS
            if let anchor_client::ClientError::ProgramError(program_err) = &e {
                println!("\nProgram error details:");
                println!("Error code: {}", program_err.to_string());
            } else if let anchor_client::ClientError::SolanaClientError(rpc_err) = &e {
                println!("\nRPC error details:");
                println!("{:#?}", rpc_err);
            }
            
            Err(format!("Failed to send swap transaction: {}", e))
        }
    }?;

    Ok(tx.to_string())
}