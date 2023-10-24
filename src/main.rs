mod services;
mod coordinator;
mod numerics;

use std::future::Future;
use std::rc::Rc;
use clap::{Args, Parser, Subcommand};
use mango_v4_client::{keypair_from_cli, pubkey_from_cli, Client, JupiterSwapMode, MangoClient, TransactionBuilderConfig, AnyhowWrap};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};
use chrono::Utc;
use futures::TryFutureExt;
// use jsonrpc_core_client::transports::ws;
// use jsonrpc_core_client::TypedSubscriptionStream;
use solana_client::rpc_config::RpcSignatureSubscribeConfig;
use solana_client::rpc_response::{Response, RpcSignatureResult};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::keypair;
use anchor_client::Cluster;
use env_logger::Env;
use solana_sdk::signature::Signer;
use fixed::FixedI128;
use fixed::types::extra::U48;
use fixed::types::I80F48;
use mango_v4::state::{PerpMarket, PerpMarketIndex, PlaceOrderType, QUOTE_DECIMALS, Side};
use crate::numerics::{native_amount, native_amount_to_lot, quote_amount_to_lot};
use crate::services::blockhash::start_blockhash_service;
use crate::services::perp_orders::{perp_bid_asset, perp_ask_asset, calc_perp_position_allowance};
use crate::services::swap_orders::swap_buy_asset;
use crate::services::{trading_config, transactions};

use solana_client::rpc_response::SlotUpdate;
// use jsonrpc_core::futures::StreamExt;
use log::info;
use serde_json::json;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use tokio::time::Interval;
use url::Url;
use websocket_tungstenite_retry::websocket_stable::StableWebSocket;
// use jsonrpc_core_client::transports::ws;
// use jsonrpc_core_client::TypedSubscriptionStream;

#[derive(Parser, Debug, Clone)]
#[clap()]
struct Cli {

    #[clap(short, long)]
    dry_run: bool,

    // e.g. https://mango.devnet.rpcpool.com
    #[clap(short, long, env)]
    rpc_url: String,

    // from app mango -> "Accounts"
    #[clap(short, long, env)]
    mango_account: Pubkey,

    // path to json array with private key
    #[clap(short, long, env)]
    owner: String,

}


// command args for testnet see /Users/stefan/mango/notes/BOT1
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV,
                                             "info,arbi_bot=trace,websocket_tungstenite_retry::websocket_stable=trace"),
    );


    let cli = Cli::parse_from(std::env::args_os());

    let dry_run = cli.dry_run;
    let rpc_url = cli.rpc_url;
    let ws_url = rpc_url.replace("https", "wss").replace("http", "ws");

    // use private key (solana-keygen)
    let owner: Arc<Keypair> = Arc::new(keypair_from_cli(cli.owner.as_str()));

    let cluster = Cluster::Custom(rpc_url.clone(), ws_url.clone());

    info!("Starting arbi-bot{} to {} trading '{}' vs '{}' ...", if dry_run { "(DRYRUN)" } else { "" },
        rpc_url, trading_config::PERP_MARKET_NAME, trading_config::TOKEN_NAME);

    let mango_client = Arc::new(
        MangoClient::new_for_existing_account(
            Client::new(
                cluster,
                // TODO need two (ask Max)
                CommitmentConfig::processed(),
                owner.clone(),
                Some(Duration::from_secs(12)),
                TransactionBuilderConfig {
                    prioritization_micro_lamports: Some(1),
                },
            ),
            cli.mango_account,
            owner.clone(),
        ).await?);


    let coordinator_thread = tokio::spawn(coordinator::run_coordinator_service(mango_client.clone(), dry_run));
    coordinator_thread.await?;

    Ok(())
}

