use std::str::FromStr;
use std::time::Instant;
use anchor_lang::prelude::Pubkey;

use anyhow::Context;
use mango_v4_client::jupiter::v4::{JupiterV4, QueryRoute};
use mango_v4_client::JupiterSwapMode;
use serde::{Deserialize, Serialize};
use crate::services::trading_config::{BASE_DECIMALS, MINT_ADDRESS_INPUT, MINT_ADDRESS_OUTPUT};

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
async fn calc_price_exactin<'a>(jupiter: &JupiterV4<'a>) -> f64 {
    let usd_decimals = 6;
    let decimals = BASE_DECIMALS - usd_decimals;
    let multiplier = 10f64.powf(decimals.into()) as f64;

    const slippage_bps: u64 = 5;
    const amount: u64 = 100000;

    let route: QueryRoute = jupiter
        .quote(
            Pubkey::from_str(MINT_ADDRESS_INPUT).unwrap(),
            Pubkey::from_str(MINT_ADDRESS_OUTPUT).unwrap(),
            amount, slippage_bps, JupiterSwapMode::ExactOut, true)
        .await.unwrap();

    let price = route.in_amount.parse::<u64>().unwrap() as f64 / route.out_amount.parse::<u64>().unwrap() as f64 * multiplier;

    price

}

// e.g. price(USD) for 1 ETH asking for 0.001 ETH
// e.g. 43.11 USD for 1 SOL
// min(buy)
async fn calc_price_exactout<'a>(jupiter: &JupiterV4<'a>) -> f64 {

    let usd_decimals = 6;
    let decimals = BASE_DECIMALS - usd_decimals;
    let multiplier = 10f64.powf(decimals.into()) as f64;

    const slippage_bps: u64 = 5;
    const amount: u64 = 100000;
    let route: QueryRoute = jupiter
        .quote(
            Pubkey::from_str(MINT_ADDRESS_INPUT).unwrap(),
            Pubkey::from_str(MINT_ADDRESS_OUTPUT).unwrap(),
            amount, slippage_bps, JupiterSwapMode::ExactOut, true)
        .await.unwrap();

    let price = route.in_amount.parse::<u64>().unwrap() as f64 / route.out_amount.parse::<u64>().unwrap() as f64 * multiplier;

    // route: QueryRoute { in_amount: "4311", out_amount: "100000",
    // price_impact_pct: 0.002647819591813372, market_infos:
    // [QueryMarketInfo { id: "C7AD8EHcbKvFL3zw2z4YKKn1ZMTCYqkThZGtY3hPajsd",
    // label: "Orca (Whirlpools)", input_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
    // , output_mint: "So11111111111111111111111111111111111111112", not_enough_liquidity: false,
    // in_amount: "4311", out_amount: "100000", min_in_amount: None, min_out_amount: None,
    // price_impact_pct: Some(0.002647819591813326),
    // lp_fee: QueryFee { amount: "11", mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", pct: Some(0.0025) }, platform_fee: QueryFee {

    price

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

pub async fn call_buy<'a>(jupiter: &JupiterV4<'a>) -> SwapBuyPrice {

    let price = calc_price_exactin(jupiter).await;

    SwapBuyPrice {
        price: price,
        approx_timestamp: Instant::now(),
    }
}

pub async fn call_sell<'a>(jupiter: &JupiterV4<'a>) -> SwapSellPrice {

    let res = calc_price_exactout(jupiter).await;

    SwapSellPrice {
        price: res,
        approx_timestamp: Instant::now(),
    }
}
