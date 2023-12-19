use std::pin::Pin;

use futures::stream::FusedStream;

use crate::{order::{OrderBook, Request, OrderDelta}, pair::Pair};

pub trait PairGenerator {
  fn get_pair(&self, symbol: &str) -> Pair;
}

pub trait StreamFactory {
  fn watch_order_book(&self) -> Pin<Box<dyn FusedStream<Item = OrderBook>>>;
  fn watch_active_orders(&self) -> Pin<Box<dyn FusedStream<Item = OrderDelta>>>;
}

pub trait OrderManager {
  fn submit_requests(&self, requests: Vec<Request>);
  fn cancel_all_active_orders(&self);
}

pub trait FromApi {
  fn from_api(api_url: &str, api_wss_url: &str, api_key: &str, secret_key: &str) -> Self;
}

pub trait BalancesManager {
  fn load_all_balances(&self);
  fn get_balance(&self, ticker: &str) -> Option<f64>;
}
