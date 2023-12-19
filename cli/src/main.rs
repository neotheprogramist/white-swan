use std::env;

use exchange::{
  bybit::Bybit,
  order::Request,
  traits::{BalancesManager, FromApi, OrderManager, PairGenerator, StreamFactory},
};
use futures::{select, StreamExt};
use utils::sigmoid;

mod utils;

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt().init();

  let bybit = Bybit::new();
  let mut order_book_stream = bybit.watch_order_book("BTCUSDT").await;
  let mut active_orders_stream = bybit.watch_active_orders().await;

  // loop {
  //   let order_book = order_book_stream.next().await.unwrap();
  //   println!("{:?}", order_book);
  // }
  loop {
    let active_orders = active_orders_stream.next().await.unwrap();
    println!("{:?}", active_orders);
  }

  let bitmex = exchange::Exchange::from_api(
    "https://www.bitmex.com/api/v1",
    " wss://ws.bitmex.com/realtime",
    env::var("BITMEX_API_KEY").unwrap().as_str(),
    env::var("BITMEX_SECRET_KEY").unwrap().as_str(),
  );
  let bybit = exchange::Exchange::from_api(
    "https://api.bybit.com",
    "wss://stream.bybit.com/v5/public/spot",
    env::var("BYBIT_API_KEY").unwrap().as_str(),
    env::var("BYBIT_SECRET_KEY").unwrap().as_str(),
  );

  bitmex.load_all_balances();
  bybit.load_all_balances();

  let bitmex_matic_usdt = bitmex.get_pair("MATIC/USDT");
  let bybit_matic_usdt = bybit.get_pair("MATIC/USDT");

  let mut bybit_matic_usdt_order_book = bybit_matic_usdt.watch_order_book();
  let mut bitmex_matic_usdt_active_orders = bitmex_matic_usdt.watch_active_orders();

  let order_book = bybit_matic_usdt_order_book.next().await.unwrap();

  let mut price =
    (order_book.asks.first().unwrap().price + order_book.bids.first().unwrap().price) / 2_f64;
  let mut ask = order_book.asks.first().unwrap().price;
  let mut bid = order_book.bids.first().unwrap().price;

  loop {
    select! {
      order_book = bybit_matic_usdt_order_book.next() => {
        bitmex_matic_usdt.cancel_all_active_orders();
        let order_book = order_book.unwrap();
        ask = order_book.asks.first().unwrap().price;
        bid = order_book.bids.first().unwrap().price;
        price = 1_f64 * (ask + bid) / 4_f64 + 3_f64 * price / 4_f64;
        let base_quantity = bitmex.get_balance("MATIC").unwrap();
        let quote_quantity = bitmex.get_balance("USDT").unwrap();
        let x = base_quantity * price - quote_quantity;
        let buy_quantity = base_quantity * sigmoid(x) / 4_f64;
        let sell_quantity = quote_quantity * sigmoid(-x) / 4_f64;
        let buy_request = Request {
          price: price * 0.99_f64,
          quantity: buy_quantity,
        };
        let sell_request = Request {
          price: price * 1.01_f64,
          quantity: -sell_quantity,
        };
        bitmex_matic_usdt.submit_requests(vec![buy_request, sell_request]);
      },
      active_orders = bitmex_matic_usdt_active_orders.next() => {
        let active_orders = active_orders.unwrap();
        let request = if active_orders.base_delta > 0_f64 {
          Request {
            price: ask,
            quantity: active_orders.base_delta,
          }
        } else {
          Request {
            price: bid,
            quantity: active_orders.quote_delta / bid,
          }
        };
        bybit_matic_usdt.submit_requests(vec![request]);
      },
    }
  }
}
