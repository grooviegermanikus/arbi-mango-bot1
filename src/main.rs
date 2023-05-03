use std::iter;
use anyhow::Context;
use jsonrpsee::{rpc_params, tracing};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::tracing::instrument::WithSubscriber;
use serde::{Deserialize, Serialize};
use serde_json::to_writer;

mod services;

#[tokio::main]
async fn main() {

    // make_jsonrpc_sample_call().await;


    let response = make_http_call().await;
    println!("resO {:?}", response);
    let price = calc_price(response.expect("http call failed"));
    println!("price {:?}", price); // 0.0536755

}

// e.g. 0.0536755 for ETH/USDC
fn calc_price(response: Vec<SwapQueryResult>) -> f64 {
    assert_eq!(response.len(), 1);
    let result = &response[0];

    // TODO findMax(out) für buy
    // https://github.com/blockworks-foundation/mango-v4/blob/dev/ts/client/src/router.ts
    // prepareMangoRouterInstructions

    // should be same as amount (100000000)
    let in_amount = result.inAmount.parse::<u64>().unwrap();
    let out_amount = result.outAmount.parse::<u64>().unwrap();

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
    
    const input_mint: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
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

// https://api.mngo.cloud/router/v1/swap
//  ?inputMint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
//  &outputMint=7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs
//  &amount=100000000
//  &slippage=0.005
//  &feeBps=0
//  &mode=ExactIn
//  &wallet=11111111111111111111111111111111
//  &otherAmountThresholdotherAmountThreshold=0

// https://github.com/paritytech/jsonrpsee/tree/master/examples/examples
async fn make_jsonrpc_sample_call() {
    let client: HttpClient<_> = jsonrpsee::http_client::HttpClientBuilder::default()
        .build("https://api.mngo.cloud:443/router").unwrap();

    let params = rpc_params![1_u64, 2, 3];
    let response: Result<String, _> = client.request("say_hello", params).await;
    tracing::info!("r: {:?}", response);

}

