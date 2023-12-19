use exchange::{bybit::Bybit, order::Request, bitmex::Bitmex};
use futures::{select, StreamExt};
use utils::sigmoid;

mod utils;

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt().init();

  let bybit = Bybit::new();
  let mut bybit_order_book = bybit.watch_order_book("MATICUSDT").await;
  let mut bybit_active_orders = bybit.watch_active_orders().await;

  let bitmex = Bitmex::new();
  // let mut bitmex_order_book = bitmex.watch_order_book("MATICUSDT").await;
  // let mut bitmex_active_orders = bitmex.watch_active_orders().await;

  let balance = bitmex.get_balances("all").await.text().await.unwrap();
  tracing::info!("balance: {:?}", balance);

  let request = exchange::bitmex::SubmitRequest::new("MATICUSDT", "Buy", "1000", "0.6");
  let response = bitmex.submit_request(request).await.text().await.unwrap();
  tracing::info!("response: {:?}", response);

  let response = bitmex.cancel_all_active_orders().await.text().await.unwrap();
  tracing::info!("response: {:?}", response);

  let mut bitmex_order_book = bitmex.watch_order_book("MATICUSDT").await;
  let mut bitmex_active_orders = bitmex.watch_active_orders().await;

  loop {
    let order = bitmex_active_orders.next().await;
    tracing::info!("order: {:?}", order);
  }

  // let mut price =
  //   (order_book.asks.first().unwrap().price + order_book.bids.first().unwrap().price) / 2_f64;
  // let mut ask = order_book.asks.first().unwrap().price;
  // let mut bid = order_book.bids.first().unwrap().price;

  // loop {
  //   select! {
  //     order_book = bybit_matic_usdt_order_book.next() => {
  //       bitmex_matic_usdt.cancel_all_active_orders();
  //       let order_book = order_book.unwrap();
  //       ask = order_book.asks.first().unwrap().price;
  //       bid = order_book.bids.first().unwrap().price;
  //       price = 1_f64 * (ask + bid) / 4_f64 + 3_f64 * price / 4_f64;
  //       let base_quantity = bitmex.get_balance("MATIC").unwrap();
  //       let quote_quantity = bitmex.get_balance("USDT").unwrap();
  //       let x = base_quantity * price - quote_quantity;
  //       let buy_quantity = base_quantity * sigmoid(x) / 4_f64;
  //       let sell_quantity = quote_quantity * sigmoid(-x) / 4_f64;
  //       let buy_request = Request {
  //         price: price * 0.99_f64,
  //         quantity: buy_quantity,
  //       };
  //       let sell_request = Request {
  //         price: price * 1.01_f64,
  //         quantity: -sell_quantity,
  //       };
  //       bitmex_matic_usdt.submit_requests(vec![buy_request, sell_request]);
  //     },
  //     active_orders = bitmex_matic_usdt_active_orders.next() => {
  //       let active_orders = active_orders.unwrap();
  //       let request = if active_orders.base_delta > 0_f64 {
  //         Request {
  //           price: ask,
  //           quantity: active_orders.base_delta,
  //         }
  //       } else {
  //         Request {
  //           price: bid,
  //           quantity: active_orders.quote_delta / bid,
  //         }
  //       };
  //       bybit_matic_usdt.submit_requests(vec![request]);
  //     },
  //   }
  // }
}
