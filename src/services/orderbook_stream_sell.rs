use std::cmp::Ordering;
use std::fmt::format;
use std::iter;
use std::time::Instant;
use anyhow::Context;
use log::{debug, error, trace};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, to_writer, Value};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::{connect_async, tungstenite};
use tokio_tungstenite::tungstenite::{client, connect, Message, WebSocket};
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::tungstenite::http::Uri;
use tokio_tungstenite::tungstenite::stream::MaybeTlsStream;
use url::Url;

#[derive(Debug)]
pub struct SellPrice {
    price: f64,
    approx_timestamp: Instant,
}

// mango-feeds
type OrderbookLevel = [f64; 2];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum OrderbookSide {
    Bid = 0,
    Ask = 1,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct OrderbookUpdate {
    pub market: String,
    pub side: OrderbookSide,
    pub update: Vec<OrderbookLevel>,
    pub slot: u64,
    pub write_version: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct OrderbookCheckpoint {
    pub market: String,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
    pub slot: u64,
    pub write_version: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WsSubscription {
    pub command: String,
    pub market_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Subscriptions {
    pub market_id: String,
}

// requires running "service-mango-orderbook" - see README
pub async fn listen_orderbook_feed(market_id: &str, sell_price_xwrite: &UnboundedSender<f64>) {

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
        market_id: market_id.to_string(),
    };
    // Ok(Text("{\"success\":false,\"message\":\"market not found\"}"))
    // Ok(Text("{\"success\":true,\"message\":\"subscribed\"}"))

    socket.write_message(Message::text(json!(sub).to_string())).unwrap();

    loop {
        match socket.read_message() {
            Ok(msg) => {
                trace!("Received: {}", msg);
            }
            Err(e) => {
                match e {
                    tungstenite::Error::ConnectionClosed => {
                        error!("Connection closed");
                        break;
                    }
                    _ => {}
                }
                error!("Error reading message: {:?}", e);
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


        if is_checkpoint_message {
            let checkpoint: OrderbookCheckpoint = serde_json::from_value(plain.clone()).expect("");
            debug!("chkpt asks: {:?}", checkpoint.asks);
            for ask in checkpoint.asks {
                sell_price_xwrite.send(ask[0]).unwrap();
            }
        }

        if is_update_message {
            let update: OrderbookUpdate = serde_json::from_value(plain.clone()).expect(format!("Can't convert json <{}>", msg).as_str());
            if update.side == OrderbookSide::Ask {
                debug!("update({:?}): {:?}", update.slot, update.update);
                for ask in update.update {
                    sell_price_xwrite.send(ask[0]).unwrap();
                }
            }
        }



        // Ok(Text("{\"market\":\"ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2\",\"side\":\"ask\",\"update\":[[21.62,0.0],[21.63,1.32]],\"slot\":191923195,\"write_version\":695276659185}"))d
        // OrderbookLevel {
        //     price: 21.62,
        //     quantity: 0.0,
        // }
    }


}

