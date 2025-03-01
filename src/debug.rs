use std::rc::Rc;

use crate::lp;
use crate::utils::POOL_ADDRESS;
use anchor_client::Program;
use anchor_client::solana_sdk::signature::Keypair;

pub async fn run_interactive_test_loop<'a>(
    program: &'a Program<Rc<Keypair>>,
    raydium_program: &'a Program<Rc<Keypair>>,
    spl_program: &'a Program<Rc<Keypair>>,
    aggregator_keypair: &'a Keypair,
) {
    println!("Enter operation (d for deposit, w for withdraw, q to quit):");
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

        match trimmed {
            "d" => {
                println!("Enter LP deposit amount:");
                let mut amount_input = String::new();
                if std::io::stdin().read_line(&mut amount_input).is_err() {
                    println!("Failed to read amount");
                    continue;
                }

                match amount_input.trim().parse::<u64>() {
                    Ok(amount) => {
                        println!("Initiating LP deposit of {} tokens...", amount);
                        match lp::process_lp_deposit(
                            program,
                            raydium_program,
                            spl_program,
                            aggregator_keypair,
                            POOL_ADDRESS,
                            amount,
                        ).await {
                            Ok(_) => println!("Deposit successful"),
                            Err(e) => println!("Deposit failed: {}", e)
                        }
                    },
                    Err(_) => println!("Please enter a valid number")
                }
            },
            "w" => {
                println!("Enter LP withdraw amount:");
                let mut amount_input = String::new();
                if std::io::stdin().read_line(&mut amount_input).is_err() {
                    println!("Failed to read amount");
                    continue;
                }

                match amount_input.trim().parse::<u64>() {
                    Ok(amount) => {
                        println!("Initiating LP withdraw of {} tokens...", amount);
                        match lp::process_lp_withdraw(
                            program,
                            raydium_program,
                            spl_program,
                            aggregator_keypair,
                            POOL_ADDRESS,
                            amount,
                        ).await {
                            Ok(_) => println!("Withdraw successful"),
                            Err(e) => println!("Withdraw failed: {}", e)
                        }
                    },
                    Err(_) => println!("Please enter a valid number")
                }
            },
            _ => println!("Invalid operation. Use 'd' for deposit, 'w' for withdraw, or 'q' to quit")
        }

        println!("\nEnter operation (d for deposit, w for withdraw, q to quit):");
    }
} 