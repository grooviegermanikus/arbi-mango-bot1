use std::sync::Arc;
use std::time::Duration;

use log::{debug, info, trace};
use mpsc::unbounded_channel;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{interval, sleep};

use services::orderbook_stream_sell::listen_orderbook_feed;

use crate::{mango, services};
use crate::services::asset_price_swap_buy::BuyPrice;
use crate::services::orderbook_stream_sell::SellPrice;

const STARTUP_DELAY: Duration = Duration::from_secs(2);

struct Coordinator {
    // swap price from router service
    buy_price_stream: UnboundedReceiver<BuyPrice>,
    // orderbook
    last_ask_price_shared: Arc<RwLock<Option<f64>>>,
    last_bid_price_shared: Arc<RwLock<Option<f64>>>,
}

pub async fn run_coordinator_service() {
    let (buy_price_xwrite, mut buy_price_xread) = unbounded_channel();

    let mut coo = Coordinator {
        buy_price_stream: buy_price_xread,
        last_ask_price_shared: Arc::new(RwLock::new(None)),
        last_bid_price_shared: Arc::new(RwLock::new(None)),
    };

    let poll_buy_price = tokio::spawn({
        async move {
            sleep(STARTUP_DELAY).await;
            let mut interval = interval(Duration::from_secs(2));
            loop {
                let price = services::asset_price_swap_buy::call_buy_usd().await;
                println!("buy price for eth {:?}", price);
                // TODO use it!
                println!("buy price for usd {:?}", services::asset_price_swap_buy::call_buy_eth().await);

                buy_price_xwrite.send(price).unwrap();

                interval.tick().await;
            }
        }
    });

    let poll_sell_price = tokio::spawn({
        let last_ask_price = coo.last_ask_price_shared.clone();
        let last_bid_price = coo.last_bid_price_shared.clone();
        async move {
            sleep(STARTUP_DELAY).await;
            listen_orderbook_feed(mango::MARKET_ETH_PERP, last_ask_price, last_bid_price).await;
        }
    });

    let main_poller = tokio::spawn({
        let last_ask_price = coo.last_ask_price_shared.clone();
        let last_bid_price = coo.last_bid_price_shared.clone();
        async move {

            let mut interval = interval(Duration::from_secs(2));
            info!("Entering coordinator loop (interval={:?}) ...", interval.period());
            loop {

                let latest_buy = drain_buy_feed(&mut coo);
                info!("latest buy price {:?}", latest_buy);

                let orderbook_ask = last_ask_price.read().await;
                info!("orderbook best ask {:?}", *orderbook_ask);

                let orderbook_bid = last_bid_price.read().await;
                info!("orderbook best bid {:?}", *orderbook_bid);

                // from orderbook
                // debug!("orderbook {:?}", orderbook_asks.iter().map(|(k, v)| (k.0, v)).collect::<Vec<_>>());
                // info!("min ask price in orderbook {:?} (size={})", orderbook_asks.first_key_value().map(|p| p.0.0), orderbook_asks.len());

                if let (Some(bid), Some(ask)) = (latest_buy, *orderbook_ask) {
                    info!("sell vs buy: {:.2?}%", (ask * bid.price - 1.0) * 100.0 );
                }

                interval.tick().await;
            }
        }
    });

    main_poller.await.unwrap();

}

// drain feeds and get latest value
fn drain_buy_feed(coo: &mut Coordinator) -> Option<BuyPrice> {
    let mut latest = None;
    while let Ok(price) = coo.buy_price_stream.try_recv() {
        trace!("drain buy price from feed {:?}", price);
        latest = Some(price);
    }
    latest
}

