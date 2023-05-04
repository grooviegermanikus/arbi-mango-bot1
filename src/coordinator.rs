use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use log::{debug, info, trace};
use std::time;
use mpsc::unbounded_channel;
use ordered_float::OrderedFloat;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep};
use crate::{mango, services};
use crate::services::asset_price_swap_buy::BuyPrice;
use crate::services::orderbook_stream_sell::SellPrice;

const STARTUP_DELAY: Duration = Duration::from_secs(2);

struct Coordinator {
    buy_price_stream: UnboundedReceiver<BuyPrice>,
    sell_price_stream: UnboundedReceiver<SellPrice>,
}

pub async fn run_coordinator_service() {
    let (buy_price_xwrite, mut buy_price_xread) = unbounded_channel();
    let (sell_price_xwrite, mut sell_price_xread) = unbounded_channel();

    let mut coo = Coordinator {
        buy_price_stream: buy_price_xread,
        sell_price_stream: sell_price_xread,
    };

    let poll_buy_price = tokio::spawn({
        async move {
            // startup delay
            sleep(STARTUP_DELAY).await;
            let mut interval = interval(Duration::from_secs(2));
            loop {
                let price = services::asset_price_swap_buy::get_price_for_buy_eth().await;
                println!("buy price for eth {:?}", price);
                // TODO use it!
                println!("buy price for usd {:?}", services::asset_price_swap_buy::get_price_for_buy_usd().await);

                buy_price_xwrite.send(price).unwrap();

                interval.tick().await;
            }
        }
    });

    let poll_sell_price = tokio::spawn({
        async move {
            // startup delay
            sleep(STARTUP_DELAY).await;
            services::orderbook_stream_sell::listen_orderbook_feed(mango::MARKET_ETH_PERP, &sell_price_xwrite).await;
        }
    });

    let main_poller = tokio::spawn({
        async move {
            let mut orderbook_asks: BTreeMap<OrderedFloat<f64>, f64> = BTreeMap::new();
            
            let mut interval = interval(Duration::from_secs(2));
            info!("Entering coordinator loop (interval={:?}) ...", interval.period());
            loop {

                let latest_buy = drain_buy_feed(&mut coo);
                info!("latest buy price {:?}", latest_buy);

                // from orderbook
                update_orderbook_from_feed(&mut coo, &mut orderbook_asks);
                debug!("orderbook {:?}", orderbook_asks.iter().map(|(k, v)| (k.0, v)).collect::<Vec<_>>());
                info!("min ask price in orderbook {:?} (size={})", orderbook_asks.first_key_value().map(|p| p.0.0), orderbook_asks.len());

                if let (Some(x), Some(y)) = (latest_buy, orderbook_asks.first_key_value()) {
                    info!("sell vs buy: {:.2?}%", (y.0.0 * x.price - 1.0) * 100.0 );
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

// drain feeds and get latest value
fn update_orderbook_from_feed(coo: &mut Coordinator, orderbook: &mut BTreeMap<OrderedFloat<f64>, f64>) {
    while let Ok(price) = coo.sell_price_stream.try_recv() {
        trace!("drain sell price from feed {:?}", price);
        if price.quantity != 0.0 {
            orderbook.insert(OrderedFloat(price.price), price.quantity);
        } else {
            orderbook.remove(&OrderedFloat(price.price));
        }
    }
}
