mod client;
mod vault;
mod raydium;
mod lp;
mod utils;

use tokio::time::{interval, Duration};
use anchor_lang::prelude::declare_program;
use utils::POOL_ADDRESS;

/*

TODO:
withdraw lp

*/

// NOTE: declare_program! does not handle constants in IDL properly, just remove and define elsewhere
declare_program!(memepool);

#[tokio::main]
async fn main() {
    let aggregator_keypair = client::load_aggregator_keypair();
    let (program, spl_program, raydium_program) = client::get_programs(&aggregator_keypair);
    
    let debug_lp_deposit = true;

    if debug_lp_deposit {
        println!("Enter LP deposit amount (or 'q' to quit):");
        let mut input = String::new();
        
        loop {
            input.clear();
            if std::io::stdin().read_line(&mut input).is_err() {
                println!("Failed to read input");
                continue;
            }

            let trimmed = input.trim();
            if trimmed == "q" {
                break;
            }

            match trimmed.parse::<u64>() {
                Ok(amount) => {
                    println!("Initiating LP deposit of {} tokens...", amount);
                    match lp::process_lp_deposit(
                        &program,
                        &raydium_program,
                        &spl_program,
                        &aggregator_keypair,
                        POOL_ADDRESS,
                        amount,
                    ).await {
                        Ok(_) => println!("Deposit successful"),
                        Err(e) => println!("Deposit failed: {}", e)
                    }
                },
                Err(_) => println!("Please enter a valid number or 'q' to quit")
            }
        }
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
