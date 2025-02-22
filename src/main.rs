mod client;

use tokio::time::{interval, Duration};
use client::{get_solana_client, load_aggregator_keypair};

#[tokio::main]
async fn main() {
    let mut interval = interval(Duration::from_secs(10));

    loop {
        interval.tick().await;

        println!("Hello, world!");
    }
}
