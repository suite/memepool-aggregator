use anchor_client::{
    solana_sdk::signature::Keypair,
    Program
};
use anchor_lang::prelude::Pubkey;
use anchor_spl::token::Mint;
use std::rc::Rc;

use crate::{
    lp, memepool, utils::{MEME_MINT_PDA, POOL_ADDRESS, VAULT_PDA}, vault::instructions::vault_fill_withdraw
};

pub async fn process_withdraw_request(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
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

    if required_sol <= vault.available_lamports {
        println!("Processing withdraw request {} with {} SOL", request_pubkey, required_sol);
        
        // Call fill_withdraw_request with the calculated amount
        let tx = vault_fill_withdraw(
            program,
            aggregator_keypair,
            request_pubkey,
            &withdraw_request,
            required_sol
        ).await?;
        
        println!("Fill withdraw request transaction: {}", tx);
        Ok(())
    } else {
        // process_lp_withdraw
        println!("Initiating LP withdraw of {} tokens...", required_sol-vault.available_lamports);
        lp::process_lp_withdraw(
            &program,
            &raydium_program,
            &spl_program,
            &aggregator_keypair,
            POOL_ADDRESS,
            required_sol-vault.available_lamports,
        ).await?;

        Ok(())
    }
}

pub async fn process_withdraw_requests_batch(
    program: &Program<Rc<Keypair>>,
    raydium_program: &Program<Rc<Keypair>>,
    spl_program: &Program<Rc<Keypair>>,
    aggregator_keypair: &Keypair,
    withdraw_requests: Vec<(Pubkey, memepool::accounts::WithdrawRequest)>
) -> Vec<Result<(), String>> {
    let mut results = Vec::with_capacity(withdraw_requests.len());
    
    for (request_pubkey, withdraw_request) in withdraw_requests {
        println!("Starting to process request {}", request_pubkey);
        
        let result = process_withdraw_request(
            program,
            raydium_program,
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