use anchor_lang::prelude::Pubkey;
use crate::memepool;
use once_cell::sync::Lazy;

pub static MEME_MINT_PDA: Lazy<Pubkey> = Lazy::new(|| {
    let seeds = b"meme";
    let (pda, _) = Pubkey::find_program_address(&[seeds], &memepool::ID);
    pda
});

pub static VAULT_PDA: Lazy<Pubkey> = Lazy::new(|| {
    let seeds = b"vault";
    let (pda, _) = Pubkey::find_program_address(&[seeds], &memepool::ID);
    pda
});