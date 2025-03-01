mod client;
mod vault;
mod raydium;
mod lp;
mod utils;
mod debug;

use std::env;
use tokio::time::{interval, Duration};
use anchor_lang::prelude::declare_program;
use utils::{POOL_ADDRESS, VAULT_PDA};
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
    
    // Check for command line arguments
    let args: Vec<String> = env::args().collect();
    let debug_mode = args.len() > 1 && args[1] == "--debug";
    
    if debug_mode {
        // Run interactive debug loop
        debug::run_interactive_test_loop(
            &program,
            &raydium_program,
            &spl_program,
            &aggregator_keypair
        ).await;
        return;
    }

    // TODO: If no withdraw requests, and we have sol, deposit into lp
    // need to keep track of available_lamports
 
    let mut interval = interval(Duration::from_secs(15));
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
                &raydium_program,
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
            let vault = program.account::<memepool::accounts::Vault>(*VAULT_PDA)
                .await.unwrap();

            if vault.available_lamports >= 1000 {
                println!("No pending withdraw requests found, but we have avail SOL, depositing into LP");

                match lp::process_lp_deposit(
                    &program,
                    &raydium_program,
                    &spl_program,
                    &aggregator_keypair,
                    POOL_ADDRESS,
                    1000,
                ).await {
                    Ok(_) => println!("Deposit successful"),
                    Err(e) => println!("Deposit failed: {}", e)
                };
            } else {
                println!("No pending withdraw requests found and no avail SOL, sleeping")
            }
        }
    }
}
