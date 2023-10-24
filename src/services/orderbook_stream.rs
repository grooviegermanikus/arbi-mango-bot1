use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::net::TcpStream;
use std::pin::Pin;
use std::sync::{Arc, Condvar};
use std::time::{Duration, Instant};
use anyhow::anyhow;

use log::{debug, error, info, trace, warn};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use tokio::io;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::{connect_async, tungstenite, WebSocketStream};
use tokio_tungstenite::tungstenite::{connect, Message, WebSocket, error::Error as WsError};
use tokio_tungstenite::tungstenite::client::connect_with_config;
use tokio_tungstenite::tungstenite::http::Response;
use tokio_tungstenite::tungstenite::Message::Text;
use tokio_tungstenite::tungstenite::stream::MaybeTlsStream;
use url::Url;
use websocket_tungstenite_retry::websocket_stable::{StableWebSocket, WsMessage};
use crate::services::fill_update_event::FillUpdateEvent;

#[derive(Debug, Copy, Clone)]
pub struct OrderstreamPrice {
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
struct WsSubscriptionOrderbook {
    pub command: String,
    pub market_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Subscriptions {
    pub market_id: String,
}

#[derive(Default)]
struct PerpOrderbook {
    pub bids: BTreeMap<OrderedFloat<f64>, f64>,
    pub asks: BTreeMap<OrderedFloat<f64>, f64>,
}

impl PerpOrderbook {

    fn update_bid_price(&mut self, price: f64, quantity: f64) {
        assert!(quantity.is_sign_positive(), "bid quantity must be non-negative but was <{}>", price);
        let price = OrderedFloat(price);
        if quantity != 0.0 {
            if !price.is_sign_positive() {
                // TODO check: orderbook asks [(-9.223372036854774e16, 0.1), (1946.14, 0.0235), (1947.5, 0.2353), ...
                warn!("bid price must be non-negative but was <{}>", price);
                return;
            }
            self.bids.insert(price, quantity);
        } else {
            self.bids.remove(&price);
        }
    }

    fn get_highest_bid_price(&self) -> Option<f64> {
        self.bids.last_key_value().map(|(k, _)| k.0)
    }

    fn update_ask_price(&mut self, price: f64, quantity: f64) {
        assert!(quantity.is_sign_positive(), "ask quantity must be non-negative but was <{}>", price);
        let price = OrderedFloat(price);
        if quantity != 0.0 {
            if !price.is_sign_positive() {
                warn!("ask price must be non-negative but was <{}>", price);
                return;
            }
            self.asks.insert(price, quantity);
        } else {
            self.asks.remove(&price);
        }
    }

    fn get_lowest_ask_price(&self) -> Option<f64> {
        self.asks.first_key_value().map(|(k, _)| k.0)
    }

    fn dump(&self) {
        debug!("orderbook bids {:?}", self.bids.iter().map(|(k, v)| (k.0, v)).collect::<Vec<_>>());
        debug!("orderbook asks {:?}", self.asks.iter().map(|(k, v)| (k.0, v)).collect::<Vec<_>>());
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct PriceInfo {
    pub price: f64,
    pub write_version: u64,
}

// requires running "service-mango-orderbook" - see README
pub async fn listen_perp_market_feed(market_id: &str,
                                     highest_bid_price: Arc<RwLock<Option<PriceInfo>>>,
                                     lowest_ask_price: Arc<RwLock<Option<PriceInfo>>>) {

    let mut orderbook: PerpOrderbook = PerpOrderbook::default();

    let subscription_request = json!({
            "command": "subscribe",
            "marketId": market_id.to_string(),
        });

    let mut socket = StableWebSocket::new_with_timeout(
        Url::parse("wss://api.mngo.cloud/orderbook/v1/").unwrap(),
subscription_request, Duration::from_secs(5)).await.unwrap();

    while let Ok(ws_message) = socket.subscribe_message_channel().recv().await {
        let WsMessage::Text(plain) = ws_message else { continue; };

        let plain = from_str::<Value>(&plain).expect("Can't parse to JSON");

        // detect checkpoint messages via property bid+ask
        let is_checkpoint_message = plain.get("bids").is_some() && plain.get("asks").is_some();
        // detect update messages
        let is_update_message = plain.get("update").is_some();

        if is_checkpoint_message {
            let checkpoint: OrderbookCheckpoint = serde_json::from_value(plain.clone()).expect("");

            for bid in checkpoint.bids {
                let price = OrderstreamPrice {
                    price: bid[0],
                    quantity: bid[1],
                    // TODO derive from slot
                    approx_timestamp: Instant::now(),
                };
                orderbook.update_bid_price(price.price, price.quantity);
                let mut lock = highest_bid_price.write().await;
                if let Some(old_highest) = *lock {
                    if old_highest.write_version > checkpoint.write_version {
                        // TODO reduce log level
                        warn!("skip orderbook update due to old timestamp");
                        continue;
                    }
                }
                *lock = orderbook.get_highest_bid_price().map(|price| PriceInfo {
                    price: price,
                    write_version: checkpoint.write_version,
                });
            }

            for ask in checkpoint.asks {
                let price = OrderstreamPrice {
                    price: ask[0],
                    quantity: ask[1],
                    // TODO derive from slot
                    approx_timestamp: Instant::now(),
                };
                orderbook.update_ask_price(price.price, price.quantity);
                let mut lock = lowest_ask_price.write().await;
                if let Some(old_highest) = *lock {
                    if old_highest.write_version > checkpoint.write_version {
                        // TODO reduce log level
                        warn!("skip orderbook update due to old timestamp");
                        continue;
                    }
                }
                *lock = orderbook.get_lowest_ask_price().map(|price| PriceInfo {
                    price: price,
                    write_version: checkpoint.write_version,
                });
            }
        }

        if is_update_message {
            let update: OrderbookUpdate = serde_json::from_value(plain.clone()).expect(format!("Can't convert json <{}>", plain).as_str());

            debug!("update({:?}): {:?}", update.slot, update.update);
            for data in update.update {
                let price = OrderstreamPrice {
                    price: data[0],
                    quantity: data[1],
                    approx_timestamp: Instant::now(),
                };
                if update.side == OrderbookSide::Bid {
                    orderbook.update_bid_price(price.price, price.quantity);
                    let mut lock = highest_bid_price.write().await;
                    if let Some(old_highest) = *lock {
                        if old_highest.write_version > update.write_version {
                            // TODO reduce log level
                            warn!("skip orderbook update due to old timestamp");
                            continue;
                        }
                    }
                    *lock = Some(PriceInfo {
                        price: price.price,
                        write_version: update.write_version,
                    });
                }
                if update.side == OrderbookSide::Ask {
                    orderbook.update_ask_price(price.price, price.quantity);
                    let mut lock = lowest_ask_price.write().await;
                    if let Some(old_highest) = *lock {
                        if old_highest.write_version > update.write_version {
                            // TODO reduce log level
                            warn!("skip orderbook update due to old timestamp");
                            continue;
                        }
                    }
                    *lock = Some(PriceInfo {
                        price: price.price,
                        write_version: update.write_version,
                    });
                }

                // TODO remove
                orderbook.dump();
                // sell_price_xwrite.send(price).unwrap();
            }

        }

    }

    socket.join().await;
}

