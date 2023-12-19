use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Order {
  pub base_quantity: f64,
  pub quote_quantity: f64,
}

#[derive(Debug, Clone)]
pub struct OrderDelta {
  pub base_delta: f64,
  pub quote_delta: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
  pub time: DateTime<Utc>,
  pub bids: Vec<OrderBookEntry>,
  pub asks: Vec<OrderBookEntry>,
}

#[derive(Debug, Clone)]
pub struct OrderBookEntry {
  pub price: f64,
  pub quantity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
  pub price: f64,
  pub quantity: f64,
}
