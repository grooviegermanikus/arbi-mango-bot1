use std::cmp::Ordering;
use std::fmt::format;
use std::iter;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, to_writer, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite};
use tokio_tungstenite::tungstenite::{client, connect, Message, WebSocket};
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::tungstenite::http::Uri;
use tokio_tungstenite::tungstenite::stream::MaybeTlsStream;
use url::Url;

mod services;
mod mango;
mod coordinator;

#[tokio::main]
async fn main() {

    let price = services::asset_price_swap_buy::get_price_for_buy().await;
    println!("price {:?}", price); // 0.0536755

    services::orderbook_stream_sell::listen_orderbook_feed(mango::MARKET_ETH_PERP);

    coordinator::run_coordinator_service();

}
