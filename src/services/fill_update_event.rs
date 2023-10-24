use serde::{Deserialize, Serialize};

/*
     {
       "event": {
         "eventType": "perp",
         "maker": "9XJt2tvSZghsMAhWto1VuPBrwXsiimPtsTR8XwGgDxK2",
         "taker": "G3bTQi9vjC1ggTMm89Guus9a2BpsxizPg6gkiK6RiVkC",
         "takerSide": "bid",
         "timestamp": "2023-05-08T09:16:13+00:00",
         "seqNum": 62970,
         "makerClientOrderId": 1683537372240897,
         "takerClientOrderId": 1683537372242188,
         "makerFee": -0.0003,
         "takerFee": 0.0006,
         "price": 1854.06,
         "quantity": 0.0001
       },
       "marketKey": "Fgh9JSZ2qfSjCw9RPJ85W2xbihsp2muLvfRztzoVR7f1",
       "marketName": "ETH-PERP",
       "status": "new",
       "slot": 192769877,
       "writeVersion": 700671312884
     }
  */

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_type: String,
    pub maker: String,
    pub taker: String,
    pub taker_side: String,
    pub timestamp: String,
    pub seq_num: i64,
    pub maker_client_order_id: i64,
    pub taker_client_order_id: i64,
    pub maker_fee: f64,
    pub taker_fee: f64,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillUpdateEvent {
    pub event: Event,
    pub market_key: String,
    pub market_name: String,
    pub status: String,
    pub slot: i64,
    pub write_version: i64,
}
