use crate::memepool;
use anchor_client::solana_sdk::signature::Keypair;
use anchor_client::Program;
use anchor_lang::prelude::{pubkey, Pubkey};
use anchor_spl::{associated_token::get_associated_token_address, token::TokenAccount};
use once_cell::sync::Lazy;
use std::rc::Rc;

pub const CP_SWAP_PROGRAM: Pubkey = pubkey!("CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW"); // DEVNET CPMM ADDRESS
pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
pub const _TEST_TOKEN_MINT: Pubkey = pubkey!("DcPRHwtoWCtzt8WwtD7VdMHvMLtHya7WPknH6kmUsUbw");
pub const POOL_ADDRESS: Pubkey = pubkey!("88hgYfHGZcDfzdqMcG5cEbo82vd2SYkMEhYwAgZcL73C"); // TODO: scan for pool ids
pub const MEMO_PROGRAM: Pubkey = pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"); // TODO: given from logs

pub static MEME_MINT_PDA: Lazy<Pubkey> = Lazy::new(|| {
    let seeds: [&[u8]; 1] = [b"meme"];
    let (pda, _) = Pubkey::find_program_address(&seeds, &memepool::ID);
    pda
});

pub static VAULT_PDA: Lazy<Pubkey> = Lazy::new(|| {
    let seeds: [&[u8]; 1] = [b"vault"];
    let (pda, _) = Pubkey::find_program_address(&seeds, &memepool::ID);
    pda
});

pub static SWAP_AUTHORITY_PDA: Lazy<Pubkey> = Lazy::new(|| {
    let seeds: [&[u8]; 1] = [b"vault_and_lp_mint_auth_seed"];
    let (pda, _) = Pubkey::find_program_address(&seeds, &CP_SWAP_PROGRAM);
    pda
});

pub fn get_oracle_pda(pool: &Pubkey) -> Pubkey {
    let seeds: [&[u8]; 2] = [b"observation", pool.as_ref()];
    let (pda, _) = Pubkey::find_program_address(&seeds, &CP_SWAP_PROGRAM);
    pda
}

pub fn get_vault_pool_pda(pool: &Pubkey) -> Pubkey {
    let seeds: [&[u8]; 2] = [b"vault_pool", pool.as_ref()];
    let (pda, _) = Pubkey::find_program_address(&seeds, &memepool::ID);
    pda
}

pub async fn get_token_account_balance(
    spl_program: &Program<Rc<Keypair>>,
    owner: &Pubkey,
    token_mint: &Pubkey,
) -> Result<u64, String> {
    let token_account = get_associated_token_address(owner, token_mint);
    spl_program
        .account::<TokenAccount>(token_account)
        .await
        .map_err(|e| format!("Failed to get token account details: {}", e))
        .map(|account| account.amount)
}
