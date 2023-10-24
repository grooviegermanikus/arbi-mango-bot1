use std::time::Instant;

use anyhow::Context;
use ordered_float::OrderedFloat;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::services::orderbook_stream::OrderstreamPrice;
use crate::services::trading_config;
use crate::services::trading_config::BASE_DECIMALS;

#[derive(Debug, Copy, Clone)]
pub struct SwapBuyPrice {
    // ETH in USD - e.g 1900
    pub price: f64,
    pub approx_timestamp: Instant,
}

#[derive(Debug, Copy, Clone)]
pub struct SwapSellPrice {
    // ETH in USD - e.g 1900
    pub price: f64,
    pub approx_timestamp: Instant,
}

// e.g. 0.18USD for 0.0001 ETH
// max(sell)
fn calc_price_exactin(response: Vec<SwapQueryResult>) -> f64 {
    let usd_decimals = 6;
    let decimals = BASE_DECIMALS - usd_decimals;
    let multiplier = 10f64.powf(decimals.into()) as f64;

    response.into_iter()
        .map(|route| {
            route.in_amount as f64 / route.out_amount as f64 * multiplier
        })
        .max_by_key(|price| OrderedFloat(*price))
        .expect("no outAmounts found")

}

// e.g. price(USD) for 1 ETH asking for 0.001 ETH
// min(buy)
fn calc_price_exactout(response: Vec<SwapQueryResult>) -> f64 {
    let usd_decimals = 6;
    let decimals = BASE_DECIMALS - usd_decimals;
    let multiplier = 10f64.powf(decimals.into()) as f64;

    response.into_iter()
        .map(|route| {
            route.in_amount as f64 / route.out_amount as f64 * multiplier
        })
        .min_by_key(|price| OrderedFloat(*price))
        .expect("no inAmounts found")

}

// see mango-v4 lib/client/src/jupiter.rs
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct SwapQueryResultRaw {
    in_amount: String,
    out_amount: String,
}

struct SwapQueryResult {
    in_amount: f64,
    out_amount: f64,
}

impl From<SwapQueryResultRaw> for SwapQueryResult {
    fn from(value: SwapQueryResultRaw) -> Self {
        SwapQueryResult {
            in_amount: value.in_amount.parse::<f64>().unwrap(),
            out_amount: value.out_amount.parse::<f64>().unwrap(),
        }
    }
}

async fn call_exactin() -> anyhow::Result<Vec<SwapQueryResult>> {
    const amount: u64 = 1_000_000; // 1 USD
    const wallet_address: &str = "11111111111111111111111111111111";
    const slippage: &str = "0.005";

    // see mango-v4 -> router.ts
    let quote = Client::new()
        .get("https://api.mngo.cloud/router/v1/swap")
        .query(&[
            ("inputMint", trading_config::MINT_ADDRESS_INPUT.to_string()),
            ("outputMint", trading_config::MINT_ADDRESS_OUTPUT.to_string()),
            ("amount", format!("{}", amount)),
            ("slippage", format!("{}", slippage)),
            ("feeBps", 0.to_string()),
            ("mode", "ExactIn".to_string()),
            ("wallet", wallet_address.to_string()),
            ("otherAmountThreshold", 0.to_string()), // 'ExactIn' ? 0 : MAX_INTEGER
        ])
        .send()
        .await
        .context("swap price request to jupiter")
        .unwrap()
        .json::<Vec<SwapQueryResultRaw>>()
        .await
        .map(|x| x.into_iter().map(|x| x.into()).collect())
        .context("receiving json response from jupiter swap price")?;
    Ok(quote)
}

pub async fn call_buy() -> SwapBuyPrice {

    match call_exactin().await {
        Ok(res) =>
            SwapBuyPrice {
                price: calc_price_exactin(res),
                approx_timestamp: Instant::now(),
            },
        Err(err) => {
            panic!("Error getting price from mango swap: {:?}", err);
        }
    }
}

async fn call_exactout() -> anyhow::Result<Vec<SwapQueryResult>> {
    const amount: u64 = 100000;
    const wallet_address: &str = "11111111111111111111111111111111";
    const slippage: &str = "0.005";

    // see mango-v4 -> router.ts
    let quote =
        reqwest::Client::new()
            .get("https://api.mngo.cloud/router/v1/swap")
            .query(&[
                ("inputMint", trading_config::MINT_ADDRESS_INPUT.to_string()),
                ("outputMint", trading_config::MINT_ADDRESS_OUTPUT.to_string()),
                ("amount", format!("{}", amount)),
                ("slippage", format!("{}", slippage)),
                ("feeBps", 0.to_string()),
                ("mode", "ExactOut".to_string()),
                ("wallet", wallet_address.to_string()),
            ])
            .send()
            .await
            .context("swap price request to jupiter")?
            .json::<Vec<SwapQueryResultRaw>>()
            .await
            .map(|x| x.into_iter().map(|x| x.into()).collect())
            .context("receiving json response from jupiter swap price")?;

    Ok(quote)
}

pub async fn call_sell() -> SwapSellPrice {

    match call_exactout().await {
        Ok(res) =>
            SwapSellPrice {
                price: calc_price_exactout(res),
                approx_timestamp: Instant::now(),
            },
        Err(err) => {
            panic!("Error getting price from mango swap: {:?}", err);
        }
    }
}

mod test {
    use crate::services::asset_price_swap::{calc_price_exactin, call_buy, SwapQueryResult};

    #[test]
    fn test_best_route_single() {
        let routes = vec![SwapQueryResult { in_amount: 10000_f64, out_amount: 6000_f64 }];
        assert_eq!(0.006, calc_price_exactin(routes));
    }

    #[test]
    fn test_best_route_buy_highest() {
        let routes1 = vec![
            SwapQueryResult { in_amount: 10000_f64, out_amount: 4000_f64 },
            SwapQueryResult { in_amount: 10000_f64, out_amount: 8000_f64 },
        ];
        assert_eq!(0.008, calc_price_exactin(routes1));

        let routes2 = vec![
            SwapQueryResult { in_amount: 10000_f64, out_amount: 4000_f64 },
            SwapQueryResult { in_amount: 10000_f64, out_amount: 8000_f64 },
        ];
        assert_eq!(0.008, calc_price_exactin(routes2));

    }

}
