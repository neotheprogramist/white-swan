use std::{collections::HashMap, env, pin::Pin};

use chrono::Utc;
use futures::{stream, SinkExt, Stream, StreamExt};
use hex::encode;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use uuid::Uuid;

use crate::pair::Pair;

pub struct Bitmex {
  api_url: String,
  wss_url: String,
  api_key: String,
  secret_key: String,
  to_internal_ids: HashMap<String, String>,
  from_internal_ids: HashMap<String, String>,
  pairs: HashMap<(String, String), Pair>,
  balances: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct WsRequest {
  pub req_id: String,
  pub op: String,
  pub args: Vec<String>,
}

impl Bitmex {
  pub fn new() -> Self {
    Self {
      api_url: "https://api.bybit.com".to_string(),
      wss_url: "wss://ws.bitmex.com/realtime".to_string(),
      api_key: env::var("BITMEX_API_KEY").unwrap(),
      secret_key: env::var("BITMEX_SECRET_KEY").unwrap(),
      to_internal_ids: HashMap::new(),
      from_internal_ids: HashMap::new(),
      pairs: HashMap::new(),
      balances: HashMap::new(),
    }
  }

  pub async fn watch_order_book(&self, symbol: &str) -> Pin<Box<dyn Stream<Item = String>>> {
    let (mut ws, _) = connect_async(Url::parse(self.wss_url.as_str()).unwrap())
      .await
      .unwrap();

    let request = WsRequest {
      req_id: Uuid::new_v4().to_string(),
      op: "subscribe".to_string(),
      args: vec![format!("orderbook.1.{}", symbol)],
    };

    ws.send(Message::Text(serde_json::to_string(&request).unwrap()))
      .await
      .unwrap();

    stream::unfold(ws, |mut ws| async {
      match ws.next().await {
        Some(Ok(Message::Text(text))) => Some((text, ws)),
        Some(Ok(Message::Ping(x))) => {
          ws.send(Message::Pong(x)).await.unwrap();
          None
        }
        Some(Err(e)) => Some((e.to_string(), ws)),
        Some(_) => None, // Ignore other messages
        None => None,    // Stream ended
      }
    })
    .boxed()
  }

  pub async fn watch_active_orders(&self) -> Pin<Box<dyn Stream<Item = String>>> {
    let (mut ws, _) = connect_async(Url::parse(self.wss_url.as_str()).unwrap())
      .await
      .unwrap();

    let expires = Utc::now().timestamp_millis() + 1000 * 60 * 60 * 24;

    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(format!("GET/realtime{}", expires).as_bytes());

    let signature = mac.finalize().into_bytes();
    let signature_hex = encode(&*signature);

    let auth_request = WsRequest {
      req_id: Uuid::new_v4().to_string(),
      op: "auth".to_string(),
      args: vec![self.api_key.clone(), expires.to_string(), signature_hex],
    };

    ws.send(Message::Text(serde_json::to_string(&auth_request).unwrap()))
      .await
      .unwrap();

    let request = WsRequest {
      req_id: Uuid::new_v4().to_string(),
      op: "subscribe".to_string(),
      args: vec!["order.spot".to_string()],
    };

    ws.send(Message::Text(serde_json::to_string(&request).unwrap()))
      .await
      .unwrap();

    stream::unfold(ws, |mut ws| async {
      match ws.next().await {
        Some(Ok(Message::Text(text))) => {
          tracing::info!("text: {}", text);
          Some((text, ws))
        }
        Some(Ok(Message::Ping(x))) => {
          tracing::info!("ping: {:?}", x);
          ws.send(Message::Pong(x)).await.unwrap();
          None
        }
        Some(Err(e)) => {
          tracing::error!("err: {}", e);
          Some((e.to_string(), ws))
        }
        Some(x) => {
          tracing::info!("other: {:?}", x);
          None
        }
        None => {
          tracing::info!("none");
          None
        }
      }
    })
    .boxed()
  }
}
