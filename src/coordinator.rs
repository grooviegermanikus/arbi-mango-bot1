use std::time::Duration;
use log::{debug, info, trace};
use std::time;
use mpsc::unbounded_channel;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep};
use crate::{mango, services};

const STARTUP_DELAY: Duration = Duration::from_secs(2);

struct Coordinator {
    buy_price_stream: UnboundedReceiver<f64>,
    sell_price_stream: UnboundedReceiver<f64>,
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
                let price = services::asset_price_swap_buy::get_price_for_buy().await;

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
            let mut interval = interval(Duration::from_secs(2));
            info!("Entering coordinator loop (interval={:?}) ...", interval.period());
            loop {

                let latest_buy = drain_buy_feed(&mut coo);
                info!("latest buy price {:?}", latest_buy);

                let latest_sell = drain_sell_feed(&mut coo);
                info!("latest sell price {:?}", latest_sell);

                interval.tick().await;
            }
        }
    });

    main_poller.await.unwrap();

}

// drain feeds and get latest value
fn drain_buy_feed(coo: &mut Coordinator) -> Option<f64> {
    let mut latest = None;
    while let Ok(foo) = coo.buy_price_stream.try_recv() {
        trace!("drain buy price from feed {:?}", foo);
        latest = Some(foo);
    }
    latest
}

// drain feeds and get latest value
fn drain_sell_feed(coo: &mut Coordinator) -> Option<f64> {
    let mut latest = None;
    while let Ok(foo) = coo.sell_price_stream.try_recv() {
        trace!("drain sell price from feed {:?}", foo);
        latest = Some(foo);
    }
    latest
}