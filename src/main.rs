mod arbitrage;
mod bellmanford;
mod constants;
mod exchanges;
mod helpers;
mod models;
mod traits;

use exchanges::binance_ws;

use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
   let init_symbols: Vec<&str> = vec!["BTCUSDT", "ETHUSDT", "LINKUSDT", "DOTUSDT", "ETHBTC", "LINKBTC", "DOTBTC", "TRYBTC", "PEPEBTC", "DOGEUSDT"].into();
   let init_symbols: Vec<String> = init_symbols.iter().map(|s| s.to_string()).collect();
   let best_symbols = Arc::new(Mutex::new(init_symbols));

   let best_symbols_shared = best_symbols.clone();
   let handle1 = tokio::spawn(async move {
      arbitrage::best_symbols_thread(best_symbols_shared).await.unwrap();
   });

   let best_symbols_shared = best_symbols.clone();
   let handle2 = tokio::spawn(async move {
      binance_ws::websocket_binance(best_symbols_shared).await.unwrap();
   });

   let _ = handle1.await;
   let _ = handle2.await;
}
