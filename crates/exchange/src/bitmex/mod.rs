use std::{collections::HashMap, env, pin::Pin};

use chrono::{DateTime, Utc};
use futures::{stream, SinkExt, Stream, StreamExt};
use hex::encode;
use hmac::{Hmac, Mac};
use reqwest::{header::HeaderMap, Response};
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
#[serde(rename_all = "camelCase")]
pub struct SubmitRequest {
  pub symbol: String,
  pub side: String,
  pub order_qty: String,
  pub price: String,
}
impl SubmitRequest {
  pub fn new(
    symbol: impl Into<String>,
    side: impl Into<String>,
    order_qty: impl Into<String>,
    price: impl Into<String>,
  ) -> Self {
    Self {
      symbol: symbol.into(),
      side: side.into(),
      order_qty: order_qty.into(),
      price: price.into(),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesRequest {
  currency: String,
}
impl GetBalancesRequest {
  pub fn new(currency: impl Into<String>) -> Self {
    Self {
      currency: currency.into(),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelAllRequest {
  category: String,
}
impl CancelAllRequest {
  pub fn new() -> Self {
    Self {
      category: "spot".to_string(),
    }
  }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookData {
  pub timestamp: DateTime<Utc>,
  pub symbol: String,
  pub bid_size: i32,
  pub bid_price: f64,
  pub ask_price: f64,
  pub ask_size: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookResponse {
  pub table: String,
  pub action: String,
  pub data: Vec<OrderBookData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PriceVolumePair(String, String);

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum WsRequestArg {
  Str(String),
  Int(i64),
}

#[derive(Debug, Deserialize, Serialize)]
struct WsRequest {
  pub op: String,
  pub args: Vec<WsRequestArg>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveOrdersResponse {
    pub table: String,
    pub action: String,
    pub data: Vec<OrderData>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderData {
    #[serde(rename = "orderID")]
    pub order_id: String,
    pub account: i64,
    pub symbol: String,
    pub side: String,
    pub order_qty: f64,
    pub price: f64,
    pub display_qty: Option<f64>,
    pub stop_px: Option<f64>,
    pub peg_offset_value: Option<f64>,
    pub currency: String,
    pub ord_type: String,
    pub time_in_force: String,
    pub ord_status: String,
    pub working_indicator: bool,
    pub leaves_qty: f64,
    pub cum_qty: f64,
    pub avg_px: Option<f64>,
    pub text: Option<String>,
    pub transact_time: String,
    pub timestamp: String,
}

impl Bitmex {
  pub fn new() -> Self {
    Self {
      api_url: "https://www.bitmex.com".to_string(),
      wss_url: "wss://ws.bitmex.com/realtime".to_string(),
      api_key: env::var("BITMEX_API_KEY").unwrap(),
      secret_key: env::var("BITMEX_SECRET_KEY").unwrap(),
      to_internal_ids: HashMap::new(),
      from_internal_ids: HashMap::new(),
      pairs: HashMap::new(),
      balances: HashMap::new(),
    }
  }

  pub async fn get_balances(&self, coin: &str) -> Response {
    let query = GetBalancesRequest::new(coin);

    let mut headers = HeaderMap::new();
    let timestamp = Utc::now().timestamp();
    let expires = timestamp + 60 * 60 * 24;
    headers.append("api-expires", expires.to_string().parse().unwrap());
    headers.append("api-key", self.api_key.parse().unwrap());
    let qs = serde_qs::to_string(&query).unwrap();
    let path = format!("/api/v1/user/wallet?{}", qs);
    let param_str = format!("GET{}{}", path, expires);
    tracing::info!("param_str: {}", param_str);
    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(param_str.as_bytes());
    let signature = encode(mac.finalize().into_bytes());
    headers.append("api-signature", signature.parse().unwrap());
    headers.append("Content-Type", "application/json".parse().unwrap());

    reqwest::Client::new()
      .get(format!("{}{}", self.api_url, path))
      .headers(headers)
      .send()
      .await
      .unwrap()
  }

  pub async fn submit_request(&self, request: SubmitRequest) -> Response {
    let mut headers = HeaderMap::new();
    let timestamp = Utc::now().timestamp();
    let expires = timestamp + 60 * 60 * 24;
    headers.append("api-expires", expires.to_string().parse().unwrap());
    headers.append("api-key", self.api_key.parse().unwrap());
    let body = serde_json::to_string(&request).unwrap();
    let path = format!("/api/v1/order");
    let param_str = format!("POST{}{}{}", path, expires, body);
    tracing::info!("param_str: {}", param_str);
    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(param_str.as_bytes());
    let signature = encode(mac.finalize().into_bytes());
    headers.append("api-signature", signature.parse().unwrap());
    headers.append("Content-Type", "application/json".parse().unwrap());

    reqwest::Client::new()
      .post(format!("{}{}", self.api_url, path))
      .headers(headers)
      .body(body)
      .send()
      .await
      .unwrap()
  }

  pub async fn cancel_all_active_orders(&self) -> Response {
    let mut headers = HeaderMap::new();
    let timestamp = Utc::now().timestamp();
    let expires = timestamp + 60 * 60 * 24;
    headers.append("api-expires", expires.to_string().parse().unwrap());
    headers.append("api-key", self.api_key.parse().unwrap());
    let path = format!("/api/v1/order/all");
    let param_str = format!("DELETE{}{}", path, expires);
    tracing::info!("param_str: {}", param_str);
    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(param_str.as_bytes());
    let signature = encode(mac.finalize().into_bytes());
    headers.append("api-signature", signature.parse().unwrap());
    headers.append("Content-Type", "application/json".parse().unwrap());

    reqwest::Client::new()
      .delete(format!("{}{}", self.api_url, path))
      .headers(headers)
      .send()
      .await
      .unwrap()
  }

  pub async fn watch_order_book(
    &self,
    symbol: &str,
  ) -> Pin<Box<dyn Stream<Item = Result<OrderBookResponse, String>>>> {
    let (mut ws, _) = connect_async(Url::parse(self.wss_url.as_str()).unwrap())
      .await
      .unwrap();

    let request = WsRequest {
      op: "subscribe".to_string(),
      args: vec![WsRequestArg::Str(format!("quote:{}", symbol))],
    };

    ws.send(Message::Text(serde_json::to_string(&request).unwrap()))
      .await
      .unwrap();

    stream::unfold(ws, |mut ws| async {
      match ws.next().await {
        Some(Ok(Message::Text(text))) => {
          tracing::info!("text: {}", text);
          let x: OrderBookResponse = match serde_json::from_str(&text) {
            Ok(x) => x,
            Err(e) => {
              tracing::error!("err: {}", e);
              return Some((Err(e.to_string()), ws));
            }
          };
          Some((Ok(x), ws))
        }
        Some(Ok(Message::Ping(x))) => {
          tracing::info!("ping: {:?}", x);
          ws.send(Message::Pong(x)).await.unwrap();
          None
        }
        Some(Err(e)) => {
          tracing::error!("err: {}", e);
          Some((Err(e.to_string()), ws))
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

  pub async fn watch_active_orders(
    &self,
  ) -> Pin<Box<dyn Stream<Item = Result<ActiveOrdersResponse, String>>>> {
    let (mut ws, _) = connect_async(Url::parse(self.wss_url.as_str()).unwrap())
      .await
      .unwrap();

    let expires = Utc::now().timestamp() + 60 * 60 * 24;

    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(format!("GET/realtime{}", expires).as_bytes());
    let signature = encode(mac.finalize().into_bytes());

    let auth_request = WsRequest {
      op: "authKeyExpires".to_string(),
      args: vec![
        WsRequestArg::Str(self.api_key.clone()),
        WsRequestArg::Int(expires),
        WsRequestArg::Str(signature),
      ],
    };

    ws.send(Message::Text(serde_json::to_string(&auth_request).unwrap()))
      .await
      .unwrap();

    let request = WsRequest {
      op: "subscribe".to_string(),
      args: vec![WsRequestArg::Str("order".to_string())],
    };

    ws.send(Message::Text(serde_json::to_string(&request).unwrap()))
      .await
      .unwrap();

    stream::unfold(ws, |mut ws| async {
      match ws.next().await {
        Some(Ok(Message::Text(text))) => {
          tracing::info!("text: {}", text);
          let x: ActiveOrdersResponse = match serde_json::from_str(&text) {
            Ok(x) => x,
            Err(e) => {
              tracing::error!("err: {}", e);
              return Some((Err("err".to_string()), ws));
            }
          };
          Some((Ok(x), ws))
        }
        Some(Ok(Message::Ping(x))) => {
          tracing::info!("ping: {:?}", x);
          ws.send(Message::Pong(x)).await.unwrap();
          Some((Err("pong".to_string()), ws))
        }
        Some(Err(e)) => {
          tracing::error!("err: {}", e);
          Some((Err("err".to_string()), ws))
        }
        Some(x) => {
          tracing::info!("other: {:?}", x);
          Some((Err("other".to_string()), ws))
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
