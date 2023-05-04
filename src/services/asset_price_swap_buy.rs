use std::cmp::Ordering;
use std::iter;
use std::time::Instant;
use anyhow::Context;
use reqwest::{Client, Error, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_json::to_writer;

#[derive(Debug, Copy, Clone)]
pub struct BuyPrice {
    // USDC in ETH - 0,00052587
    pub price: f64,
    pub approx_timestamp: Instant,
}


pub async fn get_price_for_buy_eth() -> BuyPrice {
    // TODO add retry
    let result = call_buy_usd().await;

    match result {
        Ok(res) =>
            BuyPrice {
                price: calc_price1(res),
                approx_timestamp: Instant::now(),
            },
        Err(err) => {
            panic!("Error getting price from mango swap: {:?}", err);
        }
    }
}

pub async fn get_price_for_buy_usd() -> BuyPrice {
    // TODO add retry
    let result = call_buy_eth().await;

    match result {
        Ok(res) =>
            BuyPrice {
                price: calc_price2(res),
                approx_timestamp: Instant::now(),
            },
        Err(err) => {
            panic!("Error getting price from mango swap: {:?}", err);
        }
    }
}

// e.g. 0.000536755 ETH for 1 USDC
fn calc_price1(response: Vec<SwapQueryResult>) -> f64 {
    let route_with_highest_buy_price = response.into_iter()
        .max_by(|x, y|
            x.out_amount.parse::<u64>().unwrap().cmp(&y.out_amount.parse::<u64>().unwrap())
        )
        .expect("no outAmounts found");

    // TODO findMax(out) für buy
    // https://github.com/blockworks-foundation/mango-v4/blob/dev/ts/client/src/router.ts
    // prepareMangoRouterInstructions

    // should be same as requested amount (100000000)
    let in_amount = route_with_highest_buy_price.in_amount.parse::<u64>().unwrap();
    let out_amount = route_with_highest_buy_price.out_amount.parse::<u64>().unwrap();

    let usd_decimals = 6;
    let eth_decimals = 8;
    let decimals = usd_decimals - eth_decimals;
    let multiplier = 10f64.powf(decimals.into()) as f64;
    out_amount as f64 / in_amount as f64 * multiplier
}


// e.g. price(USD) for 1 ETH asking for 0.001 ETH
fn calc_price2(response: Vec<SwapQueryResult>) -> f64 {
    let route_with_highest_buy_price = response.into_iter()
        .max_by(|x, y|
            x.out_amount.parse::<u64>().unwrap().cmp(&y.out_amount.parse::<u64>().unwrap())
        )
        .expect("no outAmounts found");

    // TODO findMax(out) für buy
    // https://github.com/blockworks-foundation/mango-v4/blob/dev/ts/client/src/router.ts
    // prepareMangoRouterInstructions

    let in_amount = route_with_highest_buy_price.in_amount.parse::<u64>().unwrap();
    let out_amount = route_with_highest_buy_price.out_amount.parse::<u64>().unwrap();

    println!("in_amount: {}", in_amount);
    println!("out_amount: {}", out_amount);
    println!("out_amount: {}", out_amount as f64/10f64.powf(8.into()) as f64);

    let usd_decimals = 6;
    let eth_decimals = 8;
    let decimals = eth_decimals - usd_decimals;
    let multiplier = 10f64.powf(decimals.into()) as f64;
    in_amount as f64 / out_amount as f64 * multiplier
}

// see mango-v4 lib/client/src/jupiter.rs
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct SwapQueryResult {

    in_amount: String,
    out_amount: String,

}

async fn call_buy_usd() -> anyhow::Result<Vec<SwapQueryResult>> {

    // USDC
    const input_mint: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    // ETH
    const output_mint: &str = "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs";
    const amount: u64 = 1_000_000; // 1 USD
    const wallet_address: &str = "11111111111111111111111111111111";
    const slippage: &str = "0.005";

    // see mango-v4 -> router.ts
    let quote =
        reqwest::Client::new()
            .get("https://api.mngo.cloud/router/v1/swap")
            .query(&[
                ("inputMint", input_mint.to_string()),
                ("outputMint", output_mint.to_string()),
                ("amount", format!("{}", amount)),
                ("slippage", format!("{}", slippage)),
                ("feeBps", 0.to_string()),
                ("mode", "ExactIn".to_string()),
                ("wallet", wallet_address.to_string()),
                ("otherAmountThreshold", 0.to_string()), // 'ExactIn' ? 0 : MAX_INTEGER
            ])
            .send()
            .await
            .context("swap price request to jupiter")?
            .json::<Vec<SwapQueryResult>>()
            .await
            .context("receiving json response from jupiter swap price")?;

    return Ok(quote);
}

async fn call_buy_eth() -> anyhow::Result<Vec<SwapQueryResult>> {

    // USDC
    const input_mint: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    // ETH
    const output_mint: &str = "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs";
    const amount: u64 = 100000;
    const wallet_address: &str = "11111111111111111111111111111111";
    const slippage: &str = "0.005";

    // see mango-v4 -> router.ts
    let quote =
        reqwest::Client::new()
            .get("https://api.mngo.cloud/router/v1/swap")
            .query(&[
                ("inputMint", input_mint.to_string()),
                ("outputMint", output_mint.to_string()),
                ("amount", format!("{}", amount)),
                ("slippage", format!("{}", slippage)),
                ("feeBps", 0.to_string()),
                ("mode", "ExactOut".to_string()),
                ("wallet", wallet_address.to_string()),
            ])
            .send()
            .await
            .context("swap price request to jupiter")?
            .json::<Vec<SwapQueryResult>>()
            .await
            .context("receiving json response from jupiter swap price")?;

    return Ok(quote);
}

mod test {
    use crate::services::asset_price_swap_buy::{calc_price1, SwapQueryResult};

    #[test]
    fn test_best_route_single() {
        let routes = vec![SwapQueryResult{ in_amount: "10000".to_string(), out_amount: "6000".to_string() }];
        assert_eq!(0.006, calc_price1(routes));
    }

    #[test]
    fn test_best_route_buy_highest() {
        let routes1 = vec![
            SwapQueryResult{ in_amount: "10000".to_string(), out_amount: "4000".to_string() },
            SwapQueryResult{ in_amount: "10000".to_string(), out_amount: "8000".to_string() },
        ];
        assert_eq!(0.008, calc_price1(routes1));

        let routes2 = vec![
            SwapQueryResult{ in_amount: "10000".to_string(), out_amount: "4000".to_string() },
            SwapQueryResult{ in_amount: "10000".to_string(), out_amount: "8000".to_string() },
        ];
        assert_eq!(0.008, calc_price1(routes2));

    }

}
