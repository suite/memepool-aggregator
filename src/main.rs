mod client;
mod vault;
mod raydium;
mod lp;
mod utils;

use lp::process_lp_swap;
use raydium::get_pool_state;
use tokio::time::{interval, Duration};
use anchor_lang::prelude::declare_program;
use utils::POOL_ADDRESS;

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

    println!("Initiated test swap...");
    let result = process_lp_swap(
        &program, 
        &raydium_program, 
        &spl_program, 
        &aggregator_keypair, 
        5, 
        true,
        95,
    ).await;
    match result {
        Ok((tx_signature, amount_out)) => {
            println!("LP swap successful:");
            println!("Transaction signature: {}", tx_signature);
            println!("Amount received: {}", amount_out);
        },
        Err(e) => println!("LP swap failed: {}", e),
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
