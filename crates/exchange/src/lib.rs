pub mod bitmex;
pub mod bybit;
pub mod order;
pub mod pair;
pub mod traits;

use std::collections::HashMap;

use crate::{
  pair::Pair,
  traits::{BalancesManager, FromApi, PairGenerator},
};

#[derive(Debug)]
pub enum EventType {
  OrderBookChange,
  OrderChange,
}

pub struct Exchange {
  api_url: String,
  api_ws_url: String,
  api_key: String,
  secret_key: String,
  to_internal_ids: HashMap<String, String>,
  from_internal_ids: HashMap<String, String>,
  pairs: HashMap<(String, String), Pair>,
  balances: HashMap<String, f64>,
}

impl FromApi for Exchange {
  fn from_api(api_url: &str, api_wss_url: &str, api_key: &str, secret_key: &str) -> Self {
    Self {
      api_url: api_url.to_string(),
      api_ws_url: api_wss_url.to_string(),
      api_key: api_key.to_string(),
      secret_key: secret_key.to_string(),
      to_internal_ids: HashMap::new(),
      from_internal_ids: HashMap::new(),
      pairs: HashMap::new(),
      balances: HashMap::new(),
    }
  }
}

impl BalancesManager for Exchange {
  fn load_all_balances(&self) {
    unimplemented!("load_all_balances")
  }
  fn get_balance(&self, ticker: &str) -> Option<f64> {
    unimplemented!("get_balance")
  }
}

impl PairGenerator for Exchange {
  fn get_pair(&self, symbol: &str) -> Pair {
    unimplemented!("get_pair")
  }
}

#[cfg(test)]
mod test {

}
