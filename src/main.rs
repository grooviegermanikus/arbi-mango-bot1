use jsonrpsee::{rpc_params, tracing};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClient;
use jsonrpsee::tracing::instrument::WithSubscriber;

#[tokio::main]
async fn main() {

    make_jsonrpc_sample_call().await;
}

// https://api.mngo.cloud/router/v1/swap?inputMint=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v&outputMint=7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs&amount=100000000&slippage=0.005&feeBps=0&mode=ExactIn&wallet=11111111111111111111111111111111&otherAmountThreshold=0

// https://github.com/paritytech/jsonrpsee/tree/master/examples/examples
async fn make_jsonrpc_sample_call() {
    let client: HttpClient<_> = jsonrpsee::http_client::HttpClientBuilder::default()
        .build("https://api.mngo.cloud:443/router").unwrap();

    let params = rpc_params![1_u64, 2, 3];
    let response: Result<String, _> = client.request("say_hello", params).await;
    tracing::info!("r: {:?}", response);

}

