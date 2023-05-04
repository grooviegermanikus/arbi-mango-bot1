use std::cmp::Ordering;
use std::iter;
use anyhow::Context;
use reqwest::{Client, Error, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_json::to_writer;

// you buy x ETH for y USDC

pub async fn get_price_for_buy() -> f64 {
    // TODO add retry
    let result = make_http_call().await;

    match result {
        Ok(res) => calc_price(res),
        Err(err) => {
            panic!("Error getting price from mango swap: {:?}", err);
        }
    }
}

// e.g. 0.0536755 for ETH/USDC
fn calc_price(response: Vec<SwapQueryResult>) -> f64 {
    let route_with_highest_buy_price = response.into_iter()
        .max_by(|x, y|
            x.outAmount.parse::<u64>().unwrap().cmp(&y.outAmount.parse::<u64>().unwrap())
        )
        .expect("no outAmounts found");

    // TODO findMax(out) für buy
    // https://github.com/blockworks-foundation/mango-v4/blob/dev/ts/client/src/router.ts
    // prepareMangoRouterInstructions

    // should be same as amount (100000000)
    let in_amount = route_with_highest_buy_price.inAmount.parse::<u64>().unwrap();
    let out_amount = route_with_highest_buy_price.outAmount.parse::<u64>().unwrap();

    out_amount as f64 / in_amount as f64
}

// see mango-v4 lib/client/src/jupiter.rs
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct SwapQueryResult {

    inAmount: String,
    outAmount: String,

}

async fn make_http_call() -> anyhow::Result<Vec<SwapQueryResult>> {

    // USDC
    const input_mint: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    // ETH
    const output_mint: &str = "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs";
    const amount: u64 = 100000000;
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

mod test {
    use crate::services::asset_price_swap_buy::{calc_price, SwapQueryResult};

    #[test]
    fn test_best_route_single() {
        let routes = vec![SwapQueryResult{ inAmount: "10000".to_string(), outAmount: "6000".to_string() }];
        assert_eq!(0.6, calc_price(routes));
    }

    #[test]
    fn test_best_route_buy_highest() {
        let routes1 = vec![
            SwapQueryResult{ inAmount: "10000".to_string(), outAmount: "6000".to_string() },
            SwapQueryResult{ inAmount: "10000".to_string(), outAmount: "7000".to_string() },
        ];
        assert_eq!(0.7, calc_price(routes1));

        let routes2 = vec![
            SwapQueryResult{ inAmount: "10000".to_string(), outAmount: "6000".to_string() },
            SwapQueryResult{ inAmount: "10000".to_string(), outAmount: "7000".to_string() },
        ];
        assert_eq!(0.7, calc_price(routes2));

    }

}
