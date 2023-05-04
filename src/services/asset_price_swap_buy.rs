use std::time::Instant;

use anyhow::Context;
use ordered_float::OrderedFloat;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::mango::{MINT_ADDRESS_ETH, MINT_ADDRESS_USDC};

#[derive(Debug, Copy, Clone)]
pub struct BuyPrice {
    // USDC in ETH - 0,00052587
    pub price: f64,
    pub approx_timestamp: Instant,
}


// e.g. 0.000536755 ETH for 1 USDC
fn calc_price_exactin(response: Vec<SwapQueryResult>) -> f64 {
    let route_with_highest_buy_price = response.into_iter()
        .max_by(|x, y|
            OrderedFloat(x.out_amount).cmp(&OrderedFloat(y.out_amount))
        )
        .expect("no outAmounts found");

    // should be same as requested amount (100000000)
    let in_amount = route_with_highest_buy_price.in_amount;
    let out_amount = route_with_highest_buy_price.out_amount;

    let usd_decimals = 6;
    let eth_decimals = 8;
    let decimals = usd_decimals - eth_decimals;
    let multiplier = 10f64.powf(decimals.into()) as f64;
    out_amount as f64 / in_amount as f64 * multiplier
}


// e.g. price(USD) for 1 ETH asking for 0.001 ETH
fn calc_price_exactout(response: Vec<SwapQueryResult>) -> f64 {
    let route_with_highest_buy_price = response.into_iter()
        .min_by(|x, y|
            OrderedFloat(x.in_amount).cmp(&OrderedFloat(y.in_amount))
        )
        .expect("no inAmounts found");

    let in_amount = route_with_highest_buy_price.in_amount;
    let out_amount = route_with_highest_buy_price.out_amount;

    let usd_decimals = 6;
    let eth_decimals = 8;
    let decimals = eth_decimals - usd_decimals;
    let multiplier = 10f64.powf(decimals.into()) as f64;
    in_amount as f64 / out_amount as f64 * multiplier
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

async fn call1() -> anyhow::Result<Vec<SwapQueryResult>> {
    const amount: u64 = 1_000_000; // 1 USD
    const wallet_address: &str = "11111111111111111111111111111111";
    const slippage: &str = "0.005";

    // see mango-v4 -> router.ts
    let quote = Client::new()
        .get("https://api.mngo.cloud/router/v1/swap")
        .query(&[
            ("inputMint", MINT_ADDRESS_USDC.to_string()),
            ("outputMint", MINT_ADDRESS_ETH.to_string()),
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

pub async fn call_buy_usd() -> BuyPrice {

    match call1().await {
        Ok(res) =>
            BuyPrice {
                price: calc_price_exactin(res),
                approx_timestamp: Instant::now(),
            },
        Err(err) => {
            panic!("Error getting price from mango swap: {:?}", err);
        }
    }
}

async fn call2() -> anyhow::Result<Vec<SwapQueryResult>> {
    const amount: u64 = 100000;
    const wallet_address: &str = "11111111111111111111111111111111";
    const slippage: &str = "0.005";

    // see mango-v4 -> router.ts
    let quote =
        reqwest::Client::new()
            .get("https://api.mngo.cloud/router/v1/swap")
            .query(&[
                ("inputMint", MINT_ADDRESS_USDC.to_string()),
                ("outputMint", MINT_ADDRESS_ETH.to_string()),
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

pub async fn call_buy_eth() -> BuyPrice {

    match call2().await {
        Ok(res) =>
            BuyPrice {
                price: calc_price_exactout(res),
                approx_timestamp: Instant::now(),
            },
        Err(err) => {
            panic!("Error getting price from mango swap: {:?}", err);
        }
    }
}

mod test {
    use crate::services::asset_price_swap_buy::{calc_price_exactin, SwapQueryResult};

    #[test]
    fn test_best_route_single() {
        let routes = vec![SwapQueryResult { in_amount: 10000f64, out_amount: 6000f64 }];
        assert_eq!(0.006, calc_price_exactin(routes));
    }

    #[test]
    fn test_best_route_buy_highest() {
        let routes1 = vec![
            SwapQueryResult { in_amount: 10000f64, out_amount: 4000f64 },
            SwapQueryResult { in_amount: 10000f64, out_amount: 8000f64 },
        ];
        assert_eq!(0.008, calc_price_exactin(routes1));

        let routes2 = vec![
            SwapQueryResult { in_amount: 10000f64, out_amount: 4000f64 },
            SwapQueryResult { in_amount: 10000f64, out_amount: 8000f64 },
        ];
        assert_eq!(0.008, calc_price_exactin(routes2));

    }

}
