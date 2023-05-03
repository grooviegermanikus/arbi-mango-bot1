use std::cmp::Ordering;
use std::iter;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_writer};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite};
use tokio_tungstenite::tungstenite::{client, connect, Message, WebSocket};
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::tungstenite::http::Uri;
use tokio_tungstenite::tungstenite::stream::MaybeTlsStream;
use url::Url;

mod services;

#[tokio::main]
async fn main() {

    // let price = services::asset_price_swap_buy::get_price_for_buy().await;
    // println!("price {:?}", price); // 0.0536755

    listen_orderbook_feed();


}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UiAccount {
    pub command: String,
    pub marketId: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Subscriptions {
    pub marketId: String,
    // pub marketIds: String,
}

fn listen_orderbook_feed() {

    let (mut socket, response) =
        connect(Url::parse("ws://127.0.0.1:8080").unwrap()).expect("Can't connect");

    // subscriptions= {"command":"subscribe","marketId":"ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2"}
    let json = serde_json::to_string(
        &UiAccount {
            command: "subscribe".to_string(),
            marketId: "ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2".to_string(),
        }).unwrap();
    println!("json {:?}", json);
    // Ok(Text("{\"success\":false,\"message\":\"market not found\"}"))
    // Ok(Text("{\"success\":true,\"message\":\"subscribed\"}"))


    socket.write_message(Message::text(json));

    loop {
        let xx = socket.read_message();
        println!("xx {:?}", xx);
    }


}
