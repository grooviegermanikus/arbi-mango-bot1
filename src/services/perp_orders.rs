use std::sync::Arc;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;
use mango_v4::state::{PerpMarket, PerpPosition, PlaceOrderType, SelfTradeBehavior, Side};
use mango_v4_client::{JupiterSwapMode, MangoClient};
use crate::numerics::*;
use std::future::Future;
use std::iter::Filter;
use std::ops::Deref;
use std::str::FromStr;
use anyhow::anyhow;
use clap::{Args, Subcommand};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use std::net::TcpStream;
use anchor_lang::error;
use itertools::{ExactlyOneError, Itertools};
use solana_sdk::signature::Signature;
use tokio_tungstenite::tungstenite::{connect, Message, WebSocket};
use tokio_tungstenite::tungstenite::client::connect_with_config;
use tokio_tungstenite::tungstenite::stream::MaybeTlsStream;
use url::Url;
use mango_v4_client::{
    keypair_from_cli, pubkey_from_cli, Client,
    TransactionBuilderConfig,
};
use crate::{CacheControl, MangoClientRef};
use crate::services::fill_update_event::FillUpdateEvent;
use crate::services::trading_config;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WsSubscriptionFills {
    pub command: String,
    pub market_id: String,
    pub head_updates: bool,
}

pub fn init_ws_subscription(market_id: &&str) -> WebSocket<MaybeTlsStream<TcpStream>> {
    // TODO TLS is slow - will be replaced
    let (mut socket, response) =
        connect(Url::parse("wss://api.mngo.cloud/fills/v1/").unwrap()).expect("Can't connect");
    println!("Connected to the server: {:?}", response);

    if response.status() != 101 {
        // TODO implement reconnects
        panic!("Error connecting to the server: {:?}", response);
    }
    let sub = &WsSubscriptionFills {
        command: "subscribe".to_string(),
        market_id: market_id.to_string(),
        head_updates: true
    };

    socket.write_message(Message::text(json!(sub).to_string())).unwrap();

    socket
}


pub async fn block_fills_until_client_id(
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    market_id: &str, search_order_client_id: u64) -> anyhow::Result<()> {

    while let msg = socket.read_message() {

        if let Message::Text(s) = msg.unwrap() {
            let plain = from_str::<Value>(&s).expect("Can't parse to JSON");
            if !plain.get("event").is_some() {
                continue;
            }

            println!("Received: {}", s);
            let fill_update_event = from_str::<FillUpdateEvent>(&s).expect("Can't parse to JSON");

            // TODO add assertions from https://github.com/blockworks-foundation/mango-v4/blob/max/mm/ts/client/scripts/mm/market-maker.ts#L185

            if fill_update_event.event.taker_client_order_id as u64 == search_order_client_id {
                debug!("Recorded fill event for client order id: {}", search_order_client_id);
                trace!("Fill Event: {:?}", fill_update_event);
                return Ok(());
            }
        }

    } // -- while

    return Err(anyhow!("Can't find fill event for client order id: {}", search_order_client_id));

}

// not used ATM
pub async fn perp_bid_blocking_until_fill(mango_client: &Arc<MangoClientRef>, client_order_id: u64) {
    let mut web_socket = init_ws_subscription(&trading_config::MARKET);

    perp_bid_asset(mango_client.clone(), client_order_id, 0.001).await;

    block_fills_until_client_id(
        &mut web_socket, trading_config::MARKET, client_order_id).await.unwrap();
}



pub async fn perp_bid_asset(mango_client: Arc<MangoClientRef>, client_order_id: u64, amount: f64) -> Signature {

    let market_index = mango_client.context.perp_market_indexes_by_name.get(trading_config::PERP_MARKET_NAME).unwrap(); // TODO
    let perp_market: PerpMarket = mango_client.context.perp_markets.get(market_index).unwrap().market.clone();

    let order_size_lots = native_amount_to_lot(perp_market.into(), amount);
    debug!("perp order bid with size (client id {}): {}, {} lots", client_order_id, amount, order_size_lots);

    let sig = mango_client.perp_place_order(
        market_index.clone(),
        Side::Bid, 0 /* ignore price */,
        order_size_lots,
        quote_amount_to_lot(perp_market.into(), 100.00),
        client_order_id as u64,
        PlaceOrderType::Market,
        false,
        0,
        64, // max num orders to be skipped based on expiry information in the orderbook
        SelfTradeBehavior::DecrementTake,
    ).await;

    debug!("tx-sig perp-bid: {:?}", sig);

    sig.unwrap()
}

pub enum PerpAllowance {
    Both,
    NoShort,
    NoLong,
}

// note: invalidates mango account cache
pub async fn calc_perp_position_allowance(mango_client: Arc<MangoClientRef>) -> PerpAllowance {
    // reload
    mango_client.clear_account_cache();

    let market_index = mango_client.context.perp_market_indexes_by_name.get(trading_config::PERP_MARKET_NAME).unwrap();
    let perp_market = mango_client.context.perp_markets.get(market_index).unwrap().market.clone();
    let mango_account = mango_client.mango_account().await.unwrap();

    let single_position = mango_account.active_perp_positions()
        .filter(|position| position.market_index == *market_index)
        .at_most_one();


    if let Ok(Some(position)) =
        single_position {

        // 0.0275 * 1e6 = 27500
        let base_native: i64 = position.base_position_native(&perp_market).to_num();
        let base_ui = base_native as f64 / 10f64.powi(perp_market.base_decimals as i32);

        debug!("perp position size: {}", base_ui);

        if base_ui > trading_config::PERP_ALLOWANCE_THRESHOLD_BASE_UI {
            PerpAllowance::NoLong
        } else if base_ui < -trading_config::PERP_ALLOWANCE_THRESHOLD_BASE_UI {
            PerpAllowance::NoShort
        } else {
            PerpAllowance::Both
        }

    } else {
        PerpAllowance::Both
    }
}

// PERP ask
// only return sig, caller must check for progress/confirmation
pub async fn perp_ask_asset(mango_client: Arc<MangoClientRef>, amount: f64) -> Signature {
    let client_order_id = Utc::now().timestamp_micros() as u64;

    let market_index = mango_client.context.perp_market_indexes_by_name.get(trading_config::PERP_MARKET_NAME).unwrap();
    let perp_market = mango_client.context.perp_markets.get(market_index).unwrap().market.clone();

    let order_size_lots = native_amount_to_lot(perp_market.into(), amount);
    debug!("perp order ask with size (client id {}): {}, {} lots", client_order_id, amount, order_size_lots);


    let sig = mango_client.perp_place_order(
        market_index.clone(),
        Side::Ask, 0 /* ignore price */,
        order_size_lots,
        quote_amount_to_lot(perp_market.into(), 100.00),
        client_order_id as u64,
        PlaceOrderType::Market,
        false,
        0,
        64, // max num orders to be skipped based on expiry information in the orderbook
        SelfTradeBehavior::DecrementTake,
    ).await;

    debug!("tx-sig perp-ask: {:?}", sig);

    sig.unwrap()
}
