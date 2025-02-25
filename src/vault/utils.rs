use anchor_lang::prelude::{Pubkey, pubkey};
use crate::memepool;
use once_cell::sync::Lazy;

// Constants for swap functionality
pub const CP_SWAP_PROGRAM: Pubkey = pubkey!("CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW"); // DEVNET CPMM ADDRESS
pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
pub const _TEST_TOKEN_MINT: Pubkey = pubkey!("DcPRHwtoWCtzt8WwtD7VdMHvMLtHya7WPknH6kmUsUbw");
pub const POOL_ADDRESS: Pubkey = pubkey!("2zQi1M8QrJpXxLWNyBuec3N7hNG1x7DmChctYYeE5HLT"); // TODO: scan for pool ids

pub static MEME_MINT_PDA: Lazy<Pubkey> = Lazy::new(|| {
    let seeds: [&[u8]; 1]  = [b"meme"];
    let (pda, _) = Pubkey::find_program_address(&seeds, &memepool::ID);
    pda
});

pub static VAULT_PDA: Lazy<Pubkey> = Lazy::new(|| {
    let seeds: [&[u8]; 1]  = [b"vault"];
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
