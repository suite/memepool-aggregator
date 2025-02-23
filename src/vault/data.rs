use anchor_client::{
    solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
    solana_sdk::signature::Keypair,
    Program
};
use anchor_lang::prelude::Pubkey;
use std::rc::Rc;
use crate::memepool;

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