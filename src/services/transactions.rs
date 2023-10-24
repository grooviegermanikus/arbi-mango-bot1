use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::stream::MaybeTlsStream;
use tokio_tungstenite::tungstenite::{connect, Message, WebSocket};
use url::Url;
use mango_v4_client::MangoClient;
use crate::MangoClientRef;

// see https://github.com/blockworks-foundation/mangolana/blob/main/src/transactions.ts

pub async fn await_transaction_signature_confirmation(mango_client: Arc<MangoClientRef>) {

    // TODO wrap in solana_sdk::signature::Signature if possible
    //
    let txid = "3EtVaf1Go41W1dTkG8PtfrRDrrcBsiXzzWCmtmRr4Ce7YRDuPRJ4mXYhqK7zYsCrVAaCJqsPChCd8yUnPPki4WW1";
    // TODO type it
    let confirm_level = "confirmed"; // or processed

    let mut confirmation_levels = Vec::new();
    // TODO implement
    confirmation_levels.push("confirmed");
    confirmation_levels.push("processed");

    let foo = on_signature().await;

    /*
        export type BlockhashWithExpiryBlockHeight = Readonly<{
        blockhash: Blockhash;
        lastValidBlockHeight: number;
        }>;
     */

    // mango_client.client.

    todo!()
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WsSubscription {
    pub method_name: String,
}

/*
 * Register a callback to be invoked upon signature updates
 *
 * @param signature Transaction signature string in base 58
 * @param callback Function to invoke on signature notifications
 * @param commitment Specify the commitment level signature must reach before notification
 * @return subscription id
 */
pub async fn on_signature() -> anyhow::Result<()> {

    let (mut socket, response) =
        connect(Url::parse("wss://api.mainnet-beta.solana.com").unwrap()).expect("Can't connect");
    println!("Connected to the server: {:?}", response);

    if response.status() != 101 {
        // TODO implement reconnects
        panic!("Error connecting to the server: {:?}", response);
    }
    // see SubscriptionConfig in connection.ts
    let sub = &WsSubscription {
        method_name: "signatureSubscribe".to_string(),
    };

    socket.write_message(Message::text(json!(sub).to_string())).unwrap();

    println!("subscribed");


    loop {
        let msg = socket.read_message();
        if let Message::Text(s) = msg.unwrap() {
            let plain = from_str::<Value>(&s).expect("Can't parse to JSON");
            println!("Received: {}", plain);

            if !plain.get("event").is_some() {
                continue;
            }

            // let fill_update_event = from_str::<FillUpdateEvent>(&s).expect("Can't parse to JSON");
            //
            // // TODO add assertions from https://github.com/blockworks-foundation/mango-v4/blob/max/mm/ts/client/scripts/mm/market-maker.ts#L185
            //
            // if fill_update_event.event.taker_client_order_id as u64 == search_order_client_id {
            //     debug!("Recorded fill event for client order id: {}", search_order_client_id);
            //     trace!("Fill Event: {:?}", fill_update_event);
            //     return Ok(());
            // }
        }
    } // -- loop


    panic!("endless loop");
}
