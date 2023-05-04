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

#[tokio::main]
async fn main() {

    // let price = services::asset_price_swap_buy::get_price_for_buy().await;
    // println!("price {:?}", price); // 0.0536755

    listen_orderbook_feed();


}

// mango-feeds
pub type OrderbookLevel = [f64; 2];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OrderbookSide {
    Bid = 0,
    Ask = 1,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderbookUpdate {
    pub market: String,
    pub side: OrderbookSide,
    pub update: Vec<OrderbookLevel>,
    pub slot: u64,
    pub write_version: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderbookCheckpoint {
    pub market: String,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
    pub slot: u64,
    pub write_version: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WsSubscription {
    pub command: String,
    pub marketId: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Subscriptions {
    pub marketId: String,
    // pub marketIds: String,
}

// requires running "service-mango-orderbook"
//
// cargo run --bin service-mango-orderbook service-mango-orderbook/conf/test-config.toml
fn listen_orderbook_feed() {

    let (mut socket, response) =
        connect(Url::parse("ws://127.0.0.1:8080").unwrap()).expect("Can't connect");

    if response.status() != 101 {
        // TODO implement reconnects
        panic!("Error connecting to the server: {:?}", response);
    }
    // Response { status: 101, version: HTTP/1.1, headers: {"connection": "Upgrade", "upgrade": "websocket", "sec-websocket-accept": "ppgfXDDxtQBmL0eczLMf5VGbFIo="}, body: () }

    // subscriptions= {"command":"subscribe","marketId":"ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2"}
    let sub = &WsSubscription {
        command: "subscribe".to_string(),
        marketId: mango::MARKET_ETH_PERP.to_string(),
    };
    // Ok(Text("{\"success\":false,\"message\":\"market not found\"}"))
    // Ok(Text("{\"success\":true,\"message\":\"subscribed\"}"))

    socket.write_message(Message::text(json!(sub).to_string()));

    loop {
        match socket.read_message() {
            Ok(msg) => {
                println!("Received: {}", msg);
            }
            Err(e) => {
                match e {
                    tungstenite::Error::ConnectionClosed => {
                        println!("Connection closed");
                        break;
                    }
                    _ => {}
                }
                println!("Error reading message: {:?}", e);
                break;
            }
        }
        let msg = socket.read_message().unwrap();

        let msg = match msg {
            tungstenite::Message::Text(s) => { s }
            _ => continue
        };

        let plain = from_str::<Value>(&msg).expect("Can't parse to JSON");

        // detect checkpoint messages via property bid+ask
        let is_checkpoint_message = plain.get("bids").is_some() && plain.get("asks").is_some();
        // detect update messages
        let is_update_message = plain.get("update").is_some();


        if (is_checkpoint_message) {
            let checkpoint: OrderbookCheckpoint = serde_json::from_value(plain.clone()).expect("");
            // println!("chkpt bids: {:?}", checkpoint.bids);
            println!("chkpt asks: {:?}", checkpoint.asks);
        }

        if is_update_message {
            let update: OrderbookUpdate = serde_json::from_value(plain.clone()).expect(format!("Can't convert json <{}>", msg).as_str());
            // println!("{:?}", msg);
            // let update: OrderbookUpdate = from_str::<OrderbookUpdate>(&msg).expect("Can't parse to JSON");
            if update.side == OrderbookSide::Ask {
                println!("update({:?}): {:?}", update.slot, update.update);
            }
        }



        // Ok(Text("{\"market\":\"ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2\",\"side\":\"ask\",\"update\":[[21.62,0.0],[21.63,1.32]],\"slot\":191923195,\"write_version\":695276659185}"))d
        // OrderbookLevel {
        //     price: 21.62,
        //     quantity: 0.0,
        // }
    }


}
