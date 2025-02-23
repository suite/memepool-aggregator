use std::{fs, rc::Rc};

use anchor_client::{solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair}, Client, Cluster, Program};

use crate::memepool;

pub fn load_aggregator_keypair() -> Keypair {
    let keypair_str = fs::read_to_string("./target/deploy/aggregator-keypair.json")
        .expect("Failed to read aggregator-keypair.json");
    
    let keypair_bytes: Vec<u8> = serde_json::from_str(&keypair_str)
        .expect("Failed to parse JSON keypair data");
   
    assert_eq!(keypair_bytes.len(), 64, "Keypair must be 64 bytes (32 secret + 32 public)");
   
    Keypair::from_bytes(&keypair_bytes).expect("Failed to create Keypair from bytes")
}

pub fn get_programs(aggregator_keypair: &Keypair) -> (Program<Rc<Keypair>>, Program<Rc<Keypair>>) {
    let provider = Client::new_with_options(
        Cluster::Devnet,
        Rc::new(aggregator_keypair.insecure_clone()),
        CommitmentConfig::confirmed(),
    );
    let memepool_program = provider.program(memepool::ID).unwrap();
    let spl_program = provider.program(anchor_spl::token::ID).unwrap();
    (memepool_program, spl_program)
}