use anchor_client::{
    solana_sdk::{signature::Keypair, system_program, signer::Signer},
    Program
};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::spl_token;
use std::rc::Rc;

use crate::memepool;
use super::utils::{get_oracle_pda, CP_SWAP_PROGRAM, MEME_MINT_PDA, POOL_ADDRESS, SWAP_AUTHORITY_PDA, TEST_TOKEN_MINT, VAULT_PDA, WSOL_MINT};

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
    aggregator_keypair: &Keypair,
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<String, String> {
    let vault_address = *VAULT_PDA;
    let cp_swap_program = CP_SWAP_PROGRAM;
    let wsol_mint = WSOL_MINT;
    let test_token = TEST_TOKEN_MINT;
    let pool_address = POOL_ADDRESS;

    // TODO: These should be fetched from pool info in the future
    let config_id = Pubkey::try_from("11111111111111111111111111111111")
        .map_err(|e| format!("Invalid config ID: {}", e))?;
    let vault_a = Pubkey::try_from("11111111111111111111111111111111")
        .map_err(|e| format!("Invalid vault A: {}", e))?;
    let vault_b = Pubkey::try_from("11111111111111111111111111111111")
        .map_err(|e| format!("Invalid vault B: {}", e))?;

    // Get authority PDA for the swap program
    let authority = *SWAP_AUTHORITY_PDA;
    
    // Get token accounts
    let input_token_account = anchor_spl::associated_token::get_associated_token_address(
        &vault_address,
        &wsol_mint,
    );
    let output_token_account = anchor_spl::associated_token::get_associated_token_address(
        &vault_address,
        &test_token,
    );

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
        input_vault: vault_a,
        output_vault: vault_b,
        input_token_program: spl_token::ID,
        output_token_program: spl_token::ID,
        input_token_mint: wsol_mint,
        output_token_mint: test_token,
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

    let tx = tx_builder
        .send()
        .await
        .map_err(|e| format!("Failed to send swap transaction: {}", e))?;

    Ok(tx.to_string())
}