use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;

pub fn get_solana_client() -> RpcClient {
    RpcClient::new("https://api.devnet.solana.com")
}

pub fn load_aggregator_keypair() -> Keypair {
    let keypair_bytes = std::fs::read("bot_keypair.json").unwrap();
    Keypair::from_bytes(&keypair_bytes).unwrap()
}