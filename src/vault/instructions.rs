use anchor_client::{
    solana_sdk::{signature::Keypair, system_program, signer::Signer},
    Program
};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::spl_token;
use std::rc::Rc;

use crate::memepool;
use super::utils::{VAULT_PDA, MEME_MINT_PDA};

pub async fn fill_withdraw_request(
    program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    request_pubkey: Pubkey,
    withdraw_request: &memepool::accounts::WithdrawRequest,
    fill_lamports: u64,
) -> Result<String, String> {
    let vault_address = *VAULT_PDA;
    let meme_mint = *MEME_MINT_PDA;
    
    let withdraw_request_meme_ata = anchor_spl::associated_token::get_associated_token_address(
        &request_pubkey,
        &meme_mint,
    );

    let accounts = memepool::client::accounts::VaultFillWithdraw {
        aggregator: aggregator_keypair.pubkey(),
        withdrawer: withdraw_request.user,
        withdraw_request: request_pubkey,
        vault: vault_address,
        meme_mint: meme_mint,
        withdraw_request_meme_ata,
        system_program: system_program::ID,
        token_program: spl_token::ID,
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