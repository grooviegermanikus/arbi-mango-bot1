use std::cmp::Ordering;
use std::fmt::format;
use std::iter;
use anyhow::Context;
use env_logger::Env;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, to_writer, Value};
use serde_json::value::RawValue;
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
    // env_logger::Builder::from_env(Env::default().default_filter_or("debug,reqwest=info")).init();
    env_logger::Builder::from_env(Env::default().default_filter_or("arbi_mango_bot1=info")).init();

    coordinator::run_coordinator_service().await;

    // rpc_slot().await;
}
