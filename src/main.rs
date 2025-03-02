mod client;
mod debug;
mod lp;
mod raydium;
mod utils;
mod vault;

use anchor_lang::prelude::declare_program;
use std::env;
use tokio::time::{interval, Duration};
use utils::{POOL_ADDRESS, VAULT_PDA};

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
            &aggregator_keypair,
        )
        .await;
        return;
    }

    let mut interval = interval(Duration::from_secs(15));
    loop {
        interval.tick().await;

        // Get pending withdraw requests (status = 0)
        let withdraw_requests = vault::get_withdraw_requests(&program, Some(0), None).await;

        if !withdraw_requests.is_empty() {
            println!(
                "Processing {} withdraw requests...",
                withdraw_requests.len()
            );
            let results = vault::process_withdraw_requests_batch(
                &program,
                &raydium_program,
                &spl_program,
                &aggregator_keypair,
                withdraw_requests,
            )
            .await;

            // Count successes and failures
            let (successes, failures): (Vec<_>, Vec<_>) =
                results.into_iter().partition(Result::is_ok);

            println!(
                "Batch processing complete. Successful: {}, Failed: {}",
                successes.len(),
                failures.len()
            );
        } else {
            let vault = program
                .account::<memepool::accounts::Vault>(*VAULT_PDA)
                .await
                .unwrap();

            println!(
                "lamports {} avail {}",
                vault.lamports, vault.available_lamports
            );

            let deposit_amount = 1_000_000; // 0.001 WSOL
            if vault.available_lamports >= deposit_amount {
                println!(
                    "No pending withdraw requests found, but we have avail SOL, depositing into LP"
                );

                match lp::process_lp_deposit(
                    &program,
                    &raydium_program,
                    &spl_program,
                    &aggregator_keypair,
                    POOL_ADDRESS,
                    deposit_amount,
                )
                .await
                {
                    Ok(_) => println!("Deposit successful"),
                    Err(e) => println!("Deposit failed: {}", e),
                };
            } else {
                println!("No pending withdraw requests found and no avail SOL, sleeping")
            }
        }
    }
}
