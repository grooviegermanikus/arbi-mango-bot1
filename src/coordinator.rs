use std::time::Duration;
use log::info;
use std::time;
use mpsc::unbounded_channel;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio::time::interval;
use crate::services;


struct Coordinator {
    buy_price_stream: UnboundedReceiver<f64>
}

pub async fn run_coordinator_service() {
    info!("Starting coordination service...");

    let (tx, mut rx) = mpsc::unbounded_channel();

    let mut coo = Coordinator {
        buy_price_stream: rx
    };


    let poll_buy_price = tokio::spawn({
        async move {
            let mut interval = interval(Duration::from_secs(2));
            loop {
                // println!("trying to get price");
                let price = services::asset_price_swap_buy::get_price_for_buy().await;
                // println!("price {:?}", price); // 0.0536755

                tx.send(price);

                interval.tick().await;
            }
        }
    });


    let main_poller = tokio::spawn({
        async move {
            let mut interval = interval(Duration::from_secs(2));
            loop {
                println!("main loop");

                let latest = drain_feed(&mut coo);
                println!("latest price {:?}", latest);

                interval.tick().await;
            }
        }
    });


    main_poller.await.unwrap();

}

// drain feeds and get latest value
fn drain_feed(coo: &mut Coordinator) -> Option<f64> {
    let mut latest = None;
    while let Ok(foo) = coo.buy_price_stream.try_recv() {
        println!("drain price from feed {:?}", foo);
        latest = Some(foo);
    }
    latest
}