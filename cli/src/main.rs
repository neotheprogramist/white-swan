use exchange::{
  bitmex::Bitmex,
  bybit::Bybit,
};
use futures::{select, StreamExt};
use utils::sigmoid;

mod utils;

#[tokio::main]
async fn main() {
  tracing_subscriber::fmt().init();

  let bybit = Bybit::new();
  let mut bybit_order_book = bybit.watch_order_book("MATICUSDT").await.fuse();
  let mut bybit_active_orders = bybit.watch_active_orders().await.fuse();

  let bitmex = Bitmex::new();
  let mut bitmex_order_book = bitmex.watch_order_book("MATICUSDT").await.fuse();
  let mut bitmex_active_orders = bitmex.watch_active_orders().await.fuse();

  let order_book = loop {
    match bybit_order_book.next().await.unwrap() {
      Ok(ob) => {
        tracing::info!("order_book: {:?}", ob);
        break ob;
      }
      Err(e) => {
        tracing::error!("error: {:?}", e);
      }
    }
  };

  let mut price = (order_book.data.a.first().unwrap().0.parse::<f64>().unwrap()
    + order_book.data.b.first().unwrap().0.parse::<f64>().unwrap())
    / 2_f64;
  let mut ask = order_book.data.a.first().unwrap().0.parse::<f64>().unwrap();
  let mut bid = order_book.data.b.first().unwrap().0.parse::<f64>().unwrap();

  loop {
    select! {
      order_book = bybit_order_book.next() => {
        bitmex.cancel_all_active_orders().await;
        let order_book = order_book.unwrap().unwrap();
        ask = match order_book.data.a.first() {
          Some(ask) => ask.0.parse().unwrap(),
          None => ask,
        };
        bid = match order_book.data.b.first() {
          Some(bid) => bid.0.parse().unwrap(),
          None => bid,
        };
        price = 1_f64 * (ask + bid) / 8_f64 + 3_f64 * price / 4_f64;
        tracing::info!("price: {:?}", price);
        tracing::info!("ask: {:?}", ask);
        tracing::info!("bid: {:?}", bid);
        let base_quantity = bybit.get_balances("MATIC").await
          .unwrap().result.list.first().unwrap()
          .coin.first().unwrap()
          .wallet_balance.parse::<f64>().unwrap();
        tracing::info!("base_quantity: {:?}", base_quantity);
        let quote_quantity = bybit.get_balances("USDT").await
          .unwrap().result.list.first().unwrap()
          .coin.first().unwrap()
          .wallet_balance.parse::<f64>().unwrap();
        tracing::info!("quote_quantity: {:?}", quote_quantity);
        let x = base_quantity * price - quote_quantity;
        let buy_quantity = base_quantity / price * sigmoid(x) / 4_f64;
        let sell_quantity = quote_quantity * sigmoid(-x) / 4_f64;
        tracing::info!("buy_quantity: {:?}", buy_quantity);
        tracing::info!("sell_quantity: {:?}", sell_quantity);
        let buy_request = exchange::bitmex::SubmitRequest {
          symbol: "MATICUSDT".to_string(),
          side: "Buy".to_string(),
          order_qty: format!("{:.0}", buy_quantity.floor() * 10_f64.powi(3)).to_string(),
          price: format!("{:.4}", bid * 0.98_f64).to_string(),
        };
        let sell_request = exchange::bitmex::SubmitRequest {
          symbol: "MATICUSDT".to_string(),
          side: "Sell".to_string(),
          order_qty: format!("{:.0}", sell_quantity.floor() * 10_f64.powi(3)).to_string(),
          price: format!("{:.4}", ask * 1.02_f64).to_string(),
        };
        tracing::info!("buy_request: {:?}", buy_request);
        tracing::info!("sell_request: {:?}", sell_request);
        let buy_request_result = bitmex.submit_request(buy_request).await;
        let sell_request_result = bitmex.submit_request(sell_request).await;
        tracing::info!("buy_request_result: {:?}", buy_request_result.text().await);
        tracing::info!("sell_request_result: {:?}", sell_request_result.text().await);
      },
      active_orders = bitmex_active_orders.next() => {
        let active_orders = active_orders.unwrap();
        tracing::info!("active_orders: {:?}", active_orders);
      },
    }
  }
}
