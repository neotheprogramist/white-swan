use std::{collections::HashMap, pin::Pin};

use futures::stream::FusedStream;

use crate::{
  order::{Order, OrderBook, OrderDelta, Request},
  traits::{OrderManager, StreamFactory},
};

#[derive(Debug, Clone)]
pub struct Pair {
  pub base: String,
  pub quote: String,
  pub orders: HashMap<String, Order>,
}

impl StreamFactory for Pair {
  fn watch_order_book(&self) -> Pin<Box<dyn FusedStream<Item = OrderBook>>> {
    unimplemented!("watch_order_book")
  }
  fn watch_active_orders(&self) -> Pin<Box<dyn FusedStream<Item = OrderDelta>>> {
    unimplemented!("watch_active_orders")
  }
}

impl OrderManager for Pair {
  fn submit_requests(&self, requests: Vec<Request>) {
    unimplemented!("submit_order")
  }
  fn cancel_all_active_orders(&self) {
    unimplemented!("cancel_all_active_orders")
  }
}
