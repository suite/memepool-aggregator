use anchor_client::{
    solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType}, solana_sdk::signature::Keypair, Program
};
use anchor_lang::prelude::Pubkey;
use std::rc::Rc;
use anchor_spl::token::Mint;

use crate::memepool;

pub fn get_meme_token_pda() -> Pubkey {
    let seeds = b"meme";
    let (pda, _) = Pubkey::find_program_address(&[seeds], &memepool::ID);
    pda
}

pub fn get_vault_pda() -> Pubkey {
    let seeds = b"vault";
    let (pda, _) = Pubkey::find_program_address(&[seeds], &memepool::ID);
    pda
}

pub async fn process_withdraw_request(
    program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    request_pubkey: Pubkey,
    withdraw_request: memepool::accounts::WithdrawRequest,
) -> Result<(), String> {
    // Get the vault account
    let vault_address = get_vault_pda();
    let vault = program.account::<memepool::accounts::Vault>(vault_address)
        .await
        .map_err(|e| format!("Failed to fetch vault account: {}", e))?;
    
    let meme_token = get_meme_token_pda();
    
    // Get meme token supply
    let mint = spl_program.account::<Mint>(meme_token)
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
        // TODO: Call fill_withdraw_request instruction
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
    withdraw_requests: Vec<(Pubkey, memepool::accounts::WithdrawRequest)>
) -> Vec<Result<(), String>> {
    let mut results = Vec::with_capacity(withdraw_requests.len());
    
    for (request_pubkey, withdraw_request) in withdraw_requests {
        println!("Starting to process request {}", request_pubkey);
        
        let result = process_withdraw_request(program, spl_program, request_pubkey, withdraw_request).await;
        
        match &result {
            Ok(_) => println!("Successfully processed request {}", request_pubkey),
            Err(e) => println!("Failed to process request {}: {}", request_pubkey, e),
        }
        
        results.push(result);
    }
    
    results
}

pub async fn get_withdraw_requests(
    program: &Program<Rc<Keypair>>,
    status_filter: Option<u8>,
    pubkey_filter: Option<Pubkey>,
) -> Vec<(Pubkey, memepool::accounts::WithdrawRequest)> {
    // Discriminator (8) + user Pubkey (32) + bump (1) + status (1) + meme_amt (8) + count (8) = 58 bytes
    const DATA_SIZE: usize = 8 + 32 + 1 + 1 + 8 + 8;

    let mut filters = vec![
        RpcFilterType::DataSize(DATA_SIZE as u64),
    ];

    if let Some(key) = pubkey_filter {
        filters.push(RpcFilterType::Memcmp(Memcmp::new(
            8, // Skip discriminator (8 bytes)
            MemcmpEncodedBytes::Bytes(key.to_bytes().to_vec()),
        )));
    }

    if let Some(status) = status_filter {
        filters.push(RpcFilterType::Memcmp(Memcmp::new(
            41, // Skip discriminator (8) + pubkey (32) + bump (1)
            MemcmpEncodedBytes::Bytes(vec![status]),
        )));
    }

    program.accounts(filters).await.unwrap()
}