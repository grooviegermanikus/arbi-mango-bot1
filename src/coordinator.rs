use std::collections::HashMap;
use std::sync::{Arc, Condvar};
use std::thread;
use std::time::Duration;
use chrono::Utc;
use futures::future::join_all;
use futures::join;
use itertools::join;

use log::{debug, info, trace, warn};
use mpsc::unbounded_channel;
use tokio::sync::{Barrier, mpsc, Mutex, RwLock};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{interval, sleep};
use anchor_lang::solana_program::example_mocks::solana_sdk::signature::Signature;
use mango_v4_client::MangoClient;
use crate::MangoClientRef;
use crate::services::asset_price_swap;

use crate::services::asset_price_swap::{SwapBuyPrice, SwapSellPrice};
use crate::services::orderbook_stream::{listen_perp_market_feed, PriceInfo};
use crate::services::perp_orders::{calc_perp_position_allowance, perp_ask_asset, perp_bid_asset, perp_bid_blocking_until_fill, PerpAllowance};
use crate::services::swap_orders::{swap_buy_asset, swap_sell_asset};
use crate::services::trading_config::*;

const DRY_RUN: bool = true;

const STARTUP_DELAY: Duration = Duration::from_secs(2);

const MARKET_SCAN_INTERVAL: Duration =Duration::from_millis(500);

// time to wait after trade (per direction)
const TRADING_COOLDOWN: Duration = Duration::from_secs(5);

struct Coordinator {
    // swap price from router service
    buy_price_stream: UnboundedReceiver<SwapBuyPrice>,
    sell_price_stream: UnboundedReceiver<SwapSellPrice>,
    // orderbook
    last_bid_price_shared: Arc<RwLock<Option<PriceInfo>>>,
    last_ask_price_shared: Arc<RwLock<Option<PriceInfo>>>,
}


pub async fn run_coordinator_service(mango_client: Arc<MangoClientRef>, dry_run: bool) {

    let (buy_price_xwrite, mut buy_price_xread) = unbounded_channel();
    let (sell_price_xwrite, mut sell_price_xread) = unbounded_channel();

    let mut coo = Coordinator {
        buy_price_stream: buy_price_xread,
        sell_price_stream: sell_price_xread,
        last_bid_price_shared: Arc::new(RwLock::new(None)),
        last_ask_price_shared: Arc::new(RwLock::new(None)),
    };

    let poll_buy_price = tokio::spawn({
        async move {
            sleep(STARTUP_DELAY).await;
            let mut interval = interval(Duration::from_secs(2));
            loop {
                let price = asset_price_swap::call_buy().await;
                debug!("swap buy price: {:?}", price);

                buy_price_xwrite.send(price).unwrap();

                interval.tick().await;
            }
        }
    });

    let poll_sell_price = tokio::spawn({
        async move {
            sleep(STARTUP_DELAY).await;
            let mut interval = interval(Duration::from_secs(2));
            loop {
                let price = asset_price_swap::call_sell().await;
                debug!("swap sell price: {:?}", price);

                sell_price_xwrite.send(price).unwrap();

                interval.tick().await;
            }
        }
    });

    // TODO crashing thread should stop the whole program
    let poll_orderbook = tokio::spawn({
        let last_bid_price = coo.last_bid_price_shared.clone();
        let last_ask_price = coo.last_ask_price_shared.clone();
        async move {
            sleep(STARTUP_DELAY).await;
            listen_perp_market_feed(MARKET, last_bid_price, last_ask_price).await;
            warn!("Orderbook WebSocket stream thread exited!");
        }
    });

    // buy on jupiter, short on eth-perp
    let main_swap2perp_poller = tokio::spawn({
        let mc = mango_client.clone();
        let last_bid_price = coo.last_bid_price_shared.clone();
        async move {
            let mut poll_interval = interval(MARKET_SCAN_INTERVAL);
            let mut throttle = interval(TRADING_COOLDOWN);
            info!("Entering coordinator JUPITERSWAP->PERP loop (interval={:?}) ...", poll_interval.period());
            loop {

                if matches!(calc_perp_position_allowance(mc.clone()).await, PerpAllowance::NoShort) {
                    debug!("no perp short position allowance, skipping ...");
                    poll_interval.tick().await;
                    continue;
                }

                let latest_swap_buy = drain_swap_buy_feed(&mut coo.buy_price_stream);
                debug!("swap latest buy price {:?}", latest_swap_buy);

                let orderbook_bid = last_bid_price.read().await;
                debug!("orderbook(perp) best bid {:?}", *orderbook_bid);

                if let (Some(perp_bid), Some(swap_buy)) = (*orderbook_bid, latest_swap_buy) {
                    let profit = (perp_bid.price - swap_buy.price) / swap_buy.price;
                    let should_trade = should_trade(profit);
                    info!("{} perp-bid {:.2?} vs swap-buy {:.2?}, expected profit {:.2?}%",
                        if should_trade { "*" } else { "." },
                        perp_bid.price, swap_buy.price, 100.0 * profit);

                    if should_trade && !dry_run {
                        info!("profitable trade swap2perp detected, starting trade sequence ...");
                        trade_sequence_swap2perp(mc.clone()).await;
                        throttle.tick().await;
                    }
                }

                poll_interval.tick().await;
            }
        }
    });

    // buy on eth-perp, sell on jupiter
    let main_perp2swap_poller = tokio::spawn({
        let mc = mango_client.clone();
        let last_ask_price = coo.last_ask_price_shared.clone();
        async move {
            let mut poll_interval = interval(MARKET_SCAN_INTERVAL);
            let mut throttle = interval(TRADING_COOLDOWN);
            info!("Entering coordinator PERP->JUPITERSWAP loop (interval={:?}) ...", poll_interval.period());
            loop {
                if matches!(calc_perp_position_allowance(mc.clone()).await, PerpAllowance::NoLong) {
                    debug!("no perp long position allowance, skipping ...");
                    poll_interval.tick().await;
                    continue;
                }

                let orderbook_ask = last_ask_price.read().await;
                debug!("orderbook(perp) best ask {:?}", *orderbook_ask);

                let latest_swap_sell = drain_swap_sell_feed(&mut coo.sell_price_stream);
                debug!("swap latest sell price {:?}", latest_swap_sell);

                if let (Some(perp_ask), Some(swap_sell)) = (*orderbook_ask, latest_swap_sell) {
                    let profit = (swap_sell.price - perp_ask.price) / perp_ask.price;
                    let should_trade = should_trade(profit);
                    info!("{} swap-sell {:.2?} vs perp-ask {:.2?}, expected profit {:.2?}%",
                        if should_trade { "*" } else { "." },
                        swap_sell.price, perp_ask.price, 100.0 * profit);

                    if should_trade && !dry_run {
                        info!("profitable trade perp2swap detected, starting trade sequence ...");
                        trade_sequence_perp2swap(mc.clone()).await;
                        throttle.tick().await;
                    }
                }

                poll_interval.tick().await;
            }
        }
    });

    // make sure the fillter thread is up

    // buy_asset(mango_client.clone()).await;
    // sell_asset(mango_client.clone()).await;

    // mango_client.mango_account().await.unwrap().

    tokio::join!(poll_buy_price, poll_sell_price, poll_orderbook);

}

async fn trade_sequence_swap2perp(mango_client: Arc<MangoClientRef>) {

    // must be unique

    info!("starting swap->perp trade sequence ...");

    let swap_buy = swap_buy_asset(mango_client.clone(), BASE_QTY_UI).await;
    // TODO check for confirmed state (ask max)

    if let Err(err) = swap_buy {
        info!("Swap buy failed, aborting trade sequence: {}", err);
        return;
    }

    let async_ask = perp_ask_asset(mango_client.clone(), BASE_QTY_UI);

    let (sig_ask) = join!(async_ask);

    info!("dispatched trading pair with signatures {} and {:?}", swap_buy.unwrap(), sig_ask);

    info!("trade sequence completed.");
}

async fn trade_sequence_perp2swap(mango_client: Arc<MangoClientRef>) {
    // must be unique
    let client_order_id = Utc::now().timestamp_micros() as u64;
    info!("starting perp->swap trade sequence (client_order_id {}) ...", client_order_id);

    let async_bid = perp_bid_asset(mango_client.clone(), client_order_id, BASE_QTY_UI);
    // TODO check for confirmed state (ask max)

    let swap_sell = swap_sell_asset(mango_client.clone(), BASE_QTY_UI).await;

    if let Err(err) = swap_sell {
        info!("Swap sell failed, aborting trade sequence (!!! perp positions will remain open): {}", err);
        return;
    }

    let (sig_bid) = join!(async_bid);

    info!("dispatched trading pair with signatures {:?} and {}", sig_bid, swap_sell.unwrap());

    info!("trade sequence completed.");
}


// drain feeds and get latest value
fn drain_swap_buy_feed(feed: &mut UnboundedReceiver<SwapBuyPrice>) -> Option<SwapBuyPrice> {
    let mut latest = None;
    while let Ok(price) = feed.try_recv() {
        trace!("drain swap buy price from feed {:?}", price);
        latest = Some(price);
    }
    latest
}

fn drain_swap_sell_feed(feed: &mut UnboundedReceiver<SwapSellPrice>) -> Option<SwapSellPrice> {
    let mut latest = None;
    while let Ok(price) = feed.try_recv() {
        trace!("drain swap sell price from feed {:?}", price);
        latest = Some(price);
    }
    latest
}

fn should_trade(profit: f64) -> bool {
    // 1 bps = 0.0001 = 0.01%
    profit > PROFIT_THRESHOLD // 0.2%
}

