use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use log::{debug, error, info, trace};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use tokio::sync::RwLock;
use url::Url;
use websocket_tungstenite_retry::websocket_stable::StableWebSocket;
use websocket_tungstenite_retry::websocket_stable::WsMessage::Text;

#[derive(Debug, Copy, Clone)]
pub struct SellPrice {
    // ETH in USDC - 1901,59495311
    pub price: f64,
    pub quantity: f64,
    pub approx_timestamp: Instant,
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

#[derive(Default)]
struct Orderbook {
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
}

impl Orderbook {
    fn update_bid_price(&mut self, price: f64, quantity: f64) {
        let price = OrderedFloat(price);
        if quantity != 0.0 {
            self.bids.insert(price, quantity);
        } else {
            self.bids.remove(&price);
        }
    }
    fn update_ask_price(&mut self, price: f64, quantity: f64) {
        let price = OrderedFloat(price);
        if quantity != 0.0 {
            self.asks.insert(price, quantity);
        } else {
            self.asks.remove(&price);
        }
    }
    fn dump(&self) {
        info!("orderbook asks {:?}", self.asks.iter().map(|(k, v)| (k.0, v)).collect::<Vec<_>>());
        info!("orderbook bids {:?}", self.bids.iter().map(|(k, v)| (k.0, v)).collect::<Vec<_>>());


    }

}

// requires running "service-mango-orderbook" - see README
pub async fn listen_orderbook_feed(market_id: &str,
                                   lowest_ask_price: Arc<RwLock<Option<f64>>>,
                                   lowest_bid_price: Arc<RwLock<Option<f64>>>) {

    let mut ws = StableWebSocket::new_with_timeout(
        Url::parse("wss://api.mngo.cloud/orderbook/v1/").unwrap(),
        json!({
            "command": "subscribe",
            "marketId": market_id.to_string(),
        }),
        Duration::from_secs(3),
    )
        .await
        .unwrap();

    let mut orderbook: Orderbook = Orderbook::default();

    let mut orderbook_updates_channel = ws.subscribe_message_channel();

    while let Ok(Text(msg)) = orderbook_updates_channel.recv().await {

        let plain = from_str::<Value>(&msg).expect("Can't parse to JSON");

        // detect checkpoint messages via property bid+ask
        let is_checkpoint_message = plain.get("bids").is_some() && plain.get("asks").is_some();
        // detect update messages
        let is_update_message = plain.get("update").is_some();


        if is_checkpoint_message {
            let checkpoint: OrderbookCheckpoint = serde_json::from_value(plain.clone()).expect("");
            for ask in checkpoint.asks {
                let price = SellPrice {
                    price: ask[0],
                    quantity: ask[1],
                    // TODO derive from slot
                    approx_timestamp: Instant::now(),
                };
                orderbook.update_ask_price(price.price, price.quantity);
                let mut lock = lowest_ask_price.write().await;
                *lock = Some(price.price);

                // sell_price_xwrite.send(price).unwrap();
            }
            for bid in checkpoint.bids {
                let price = SellPrice {
                    price: bid[0],
                    quantity: bid[1],
                    // TODO derive from slot
                    approx_timestamp: Instant::now(),
                };
                orderbook.update_bid_price(price.price, price.quantity);
                let mut lock = lowest_bid_price.write().await;
                *lock = Some(price.price);
                // sell_price_xwrite.send(price).unwrap();
            }
        }

        if is_update_message {
            let update: OrderbookUpdate = serde_json::from_value(plain.clone()).expect(format!("Can't convert json <{}>", msg).as_str());

            debug!("update({:?}): {:?}", update.slot, update.update);
            for data in update.update {
                let price = SellPrice {
                    price: data[0],
                    quantity: data[1],
                    approx_timestamp: Instant::now(),
                };
                if update.side == OrderbookSide::Ask {
                    orderbook.update_ask_price(price.price, price.quantity);
                    let mut lock = lowest_ask_price.write().await;
                    *lock = Some(price.price);
                }
                if update.side == OrderbookSide::Bid {
                    orderbook.update_bid_price(price.price, price.quantity);
                    let mut lock = lowest_bid_price.write().await;
                    *lock = Some(price.price);
                }

                // TODO remove
                orderbook.dump();
                // sell_price_xwrite.send(price).unwrap();
            }

        }



        // Ok(Text("{\"market\":\"ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2\",\"side\":\"ask\",\"update\":[[21.62,0.0],[21.63,1.32]],\"slot\":191923195,\"write_version\":695276659185}"))d
        // OrderbookLevel {
        //     price: 21.62,
        //     quantity: 0.0,
        // }
    }


}


async fn _rpc_slot() {

    // Build client
    let client: HttpClient = HttpClientBuilder::default()
        // TODO move
        .build("http://api.mainnet-beta.solana.com:80")
        .unwrap();

    // e.g. 1683205217
    let response: Result<u32, _> = client.request("getBlockTime",  rpc_params!(192071700)).await;
    println!("response: {:?}", response);

}

