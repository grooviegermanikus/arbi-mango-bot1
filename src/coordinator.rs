use std::time::Duration;
use log::info;
use std::time;
use tokio::task::JoinHandle;
use tokio::time::interval;
use crate::services;

pub async fn run_coordinator_service() {
    info!("Starting coordination service...");

    let mut interval = interval(Duration::from_secs(2));

    let poll_buy_price = tokio::spawn({
        async move {
            println!("trying to get price");
            let price = services::asset_price_swap_buy::get_price_for_buy().await;
            println!("price {:?}", price); // 0.0536755


            interval.tick().await;

            price
        }
    });

    let out = poll_buy_price.await.unwrap();


}