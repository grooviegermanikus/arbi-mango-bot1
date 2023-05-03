use std::cmp::Ordering;
use std::iter;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::to_writer;

mod services;

#[tokio::main]
async fn main() {

    let price = services::asset_price_swap_buy::get_price_for_buy().await;
    println!("price {:?}", price); // 0.0536755

}
