use std::{collections::HashMap, env, pin::Pin, str::FromStr};

use chrono::Utc;
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

pub struct Bybit {
  api_url: String,
  public_wss_url: String,
  private_wss_url: String,
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
  category: String,
  symbol: String,
  side: String,
  order_type: String,
  qty: String,
  price: String,
}
impl SubmitRequest {
  pub fn new(
    symbol: impl Into<String>,
    side: impl Into<String>,
    qty: impl Into<String>,
    price: impl Into<String>,
  ) -> Self {
    Self {
      category: "spot".to_string(),
      symbol: symbol.into(),
      side: side.into(),
      order_type: "Limit".to_string(),
      qty: qty.into(),
      price: price.into(),
    }
  }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesRequest {
  account_type: String,
  coin: String,
}
impl GetBalancesRequest {
  pub fn new(coin: impl Into<String>) -> Self {
    Self {
      account_type: "UNIFIED".to_string(),
      coin: coin.into(),
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

#[derive(Debug, Deserialize, Serialize)]
pub struct OrderBookResponse {
  pub topic: String,
  pub ts: u64,
  #[serde(rename = "type")]
  pub t: String,
  pub data: OrderBookDataResponse,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OrderBookDataResponse {
  pub s: String,
  pub b: Vec<PriceVolumePair>,
  pub a: Vec<PriceVolumePair>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PriceVolumePair(pub String, pub String);

#[derive(Debug, Deserialize, Serialize)]
struct WsRequest {
  pub req_id: String,
  pub op: String,
  pub args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveOrdersResponse {
  pub topic: String,
  pub id: String,
  pub creation_time: u64,
  pub data: Vec<OrderData>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderData {
  pub category: String,
  pub symbol: String,
  pub order_link_id: String,
  pub block_trade_id: String,
  pub side: String,
  pub position_idx: u32,
  pub order_status: String,
  pub cancel_type: String,
  pub reject_reason: String,
  pub time_in_force: String,
  pub is_leverage: String,
  pub price: String,
  pub qty: String,
  pub avg_price: String,
  pub leaves_qty: String,
  pub leaves_value: String,
  pub cum_exec_qty: String,
  pub cum_exec_value: String,
  pub cum_exec_fee: String,
  pub order_type: String,
  pub stop_order_type: String,
  pub order_iv: String,
  pub trigger_price: String,
  pub take_profit: String,
  pub stop_loss: String,
  pub trigger_by: String,
  pub tp_trigger_by: String,
  pub sl_trigger_by: String,
  pub trigger_direction: u32,
  pub place_type: String,
  pub last_price_on_created: String,
  pub close_on_trigger: bool,
  pub reduce_only: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesResponse {
  pub ret_code: i32,
  pub ret_msg: String,
  pub result: GetBalancesResult,
  pub ret_ext_info: serde_json::Value,
  pub time: i64,
}

impl FromStr for GetBalancesResponse {
  type Err = serde_json::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    serde_json::from_str(s)
  }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesResult {
  pub list: Vec<GetBalancesAccount>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesAccount {
  pub total_equity: String,
  #[serde(rename = "accountIMRate")]
  pub account_im_rate: String,
  pub total_margin_balance: String,
  pub total_initial_margin: String,
  pub account_type: String,
  pub total_available_balance: String,
  #[serde(rename = "accountMMRate")]
  pub account_mm_rate: String,
  #[serde(rename = "totalPerpUPL")]
  pub total_perp_upl: String,
  pub total_wallet_balance: String,
  #[serde(rename = "accountLTV")]
  pub account_ltv: String,
  pub total_maintenance_margin: String,
  pub coin: Vec<GetBalancesCoin>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesCoin {
  pub available_to_borrow: String,
  pub bonus: String,
  pub accrued_interest: String,
  pub available_to_withdraw: String,
  #[serde(rename = "totalOrderIM")]
  pub total_order_im: String,
  pub equity: String,
  #[serde(rename = "totalPositionMM")]
  pub total_position_mm: String,
  pub usd_value: String,
  pub unrealised_pnl: String,
  pub collateral_switch: bool,
  pub spot_hedging_qty: String,
  pub borrow_amount: String,
  #[serde(rename = "totalPositionIM")]
  pub total_position_im: String,
  pub wallet_balance: String,
  pub cum_realised_pnl: String,
  pub locked: String,
  pub margin_collateral: bool,
  pub coin: String,
}

impl Bybit {
  pub fn new() -> Self {
    Self {
      api_url: "https://api.bybit.com".to_string(),
      public_wss_url: "wss://stream.bybit.com/v5/public/spot".to_string(),
      private_wss_url: "wss://stream.bybit.com/v5/private".to_string(),
      api_key: env::var("BYBIT_API_KEY").unwrap(),
      secret_key: env::var("BYBIT_SECRET_KEY").unwrap(),
      to_internal_ids: HashMap::new(),
      from_internal_ids: HashMap::new(),
      pairs: HashMap::new(),
      balances: HashMap::new(),
    }
  }

  pub async fn get_balances(&self, coin: &str) -> Result<GetBalancesResponse, serde_json::Error> {
    let timestamp = Utc::now().timestamp_millis();
    let request = GetBalancesRequest::new(coin);
    let qs = serde_qs::to_string(&request).unwrap();

    let param_str = format!("{}{}{}", timestamp, self.api_key, qs);
    tracing::info!("param_str: {}", param_str);

    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(param_str.as_bytes());
    let signature = encode(mac.finalize().into_bytes());

    let mut headers = HeaderMap::new();
    headers.append("X-BAPI-API-KEY", self.api_key.parse().unwrap());
    headers.append("X-BAPI-TIMESTAMP", timestamp.to_string().parse().unwrap());
    headers.append("X-BAPI-SIGN", signature.parse().unwrap());
    headers.append("Content-Type", "application/json".parse().unwrap());

    reqwest::Client::new()
      .get(format!("{}/v5/account/wallet-balance", self.api_url))
      .headers(headers)
      .query(&request)
      .send()
      .await
      .unwrap()
      .text()
      .await
      .unwrap()
      .parse()
  }

  pub async fn submit_request(&self, request: SubmitRequest) -> Response {
    let timestamp = Utc::now().timestamp_millis();
    let payload = serde_json::to_string(&request).unwrap();

    let param_str = format!("{}{}{}", timestamp, self.api_key, payload);
    tracing::info!("param_str: {}", param_str);

    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(param_str.as_bytes());
    let signature = encode(mac.finalize().into_bytes());

    let mut headers = HeaderMap::new();
    headers.append("X-BAPI-API-KEY", self.api_key.parse().unwrap());
    headers.append("X-BAPI-TIMESTAMP", timestamp.to_string().parse().unwrap());
    headers.append("X-BAPI-SIGN", signature.parse().unwrap());
    headers.append("Content-Type", "application/json".parse().unwrap());

    reqwest::Client::new()
      .post(format!("{}/v5/order/create", self.api_url))
      .headers(headers)
      .body(payload)
      .send()
      .await
      .unwrap()
  }

  pub async fn cancel_all_active_orders(&self) -> Response {
    let timestamp = Utc::now().timestamp_millis();
    let request = CancelAllRequest::new();
    let payload = serde_json::to_string(&request).unwrap();

    let param_str = format!("{}{}{}", timestamp, self.api_key, payload);
    tracing::info!("param_str: {}", param_str);

    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(param_str.as_bytes());
    let signature = encode(mac.finalize().into_bytes());

    let mut headers = HeaderMap::new();
    headers.append("X-BAPI-API-KEY", self.api_key.parse().unwrap());
    headers.append("X-BAPI-TIMESTAMP", timestamp.to_string().parse().unwrap());
    headers.append("X-BAPI-SIGN", signature.parse().unwrap());
    headers.append("Content-Type", "application/json".parse().unwrap());

    reqwest::Client::new()
      .post(format!("{}/v5/order/cancel-all", self.api_url))
      .headers(headers)
      .body(payload)
      .send()
      .await
      .unwrap()
  }

  pub async fn watch_order_book(
    &self,
    symbol: &str,
  ) -> Pin<Box<dyn Stream<Item = Result<OrderBookResponse, String>>>> {
    let (mut ws, _) = connect_async(Url::parse(self.public_wss_url.as_str()).unwrap())
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
    let (mut ws, _) = connect_async(Url::parse(self.private_wss_url.as_str()).unwrap())
      .await
      .unwrap();

    let expires = Utc::now().timestamp_millis() + 1000 * 60 * 60 * 24;

    let mut mac = Hmac::<Sha256>::new_from_slice(self.secret_key.as_bytes()).unwrap();
    mac.update(format!("GET/realtime{}", expires).as_bytes());

    let signature = mac.finalize().into_bytes();
    let signature_hex = encode(&*signature);

    tracing::info!("signature_hex: {}", signature_hex);

    let auth_request = WsRequest {
      req_id: Uuid::new_v4().to_string(),
      op: "authKeyExpires".to_string(),
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
