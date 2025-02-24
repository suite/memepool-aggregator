mod client;
mod vault;
mod raydium;

use anchor_client::solana_sdk::native_token::LAMPORTS_PER_SOL;
use raydium::get_pool_state;
use tokio::time::{interval, Duration};
use anchor_lang::prelude::declare_program;
use vault::{service::process_lp_deposit, utils::POOL_ADDRESS};

/*

TODO:
swap 
deposit lp
withdraw lp

*/

// NOTE: declare_program! does not handle constants in IDL properly, just remove and define elsewhere
declare_program!(memepool);

#[tokio::main]
async fn main() {
    let aggregator_keypair = client::load_aggregator_keypair();
    let (program, spl_program, raydium_program) = client::get_programs(&aggregator_keypair);

    let test = get_pool_state(&raydium_program, POOL_ADDRESS).await.unwrap();
    let amts = test.get_vault_amounts(&spl_program).await.unwrap();
    println!("test pool: {:?} pool amounts: {:?}", test, amts);

    println!("Initiated test deposit...");
    let result = process_lp_deposit(&program, &raydium_program, &spl_program, &aggregator_keypair, 10).await;
    match result {
        Ok(_) => println!("LP deposit successful"),
        Err(e) => println!("LP deposit failed: {}", e),
    }


 
    let mut interval = interval(Duration::from_secs(5));
    loop {
        interval.tick().await;    
        
        // Get pending withdraw requests (status = 0)
        let withdraw_requests = vault::get_withdraw_requests(
            &program,
            Some(0),
            None
        ).await;
        
        if !withdraw_requests.is_empty() {
            println!("Processing {} withdraw requests...", withdraw_requests.len());
            let results = vault::process_withdraw_requests_batch(
                &program,
                &spl_program,
                &aggregator_keypair,
                withdraw_requests
            ).await;
            
            // Count successes and failures
            let (successes, failures): (Vec<_>, Vec<_>) = results
                .into_iter()
                .partition(Result::is_ok);
                
            println!(
                "Batch processing complete. Successful: {}, Failed: {}",
                successes.len(),
                failures.len()
            );
        } else {
            println!("No pending withdraw requests found, sleeping");
        }
    }
}
