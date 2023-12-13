mod arbitrage;
mod bellmanford;
mod constants;
mod exchanges;
mod helpers;
mod models;
mod traits;

use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
   let best_symbols = Arc::new(Mutex::new(["BTCUSDT", "ETHUSDT", "LINKUSDT", "DOTUSDT", "ETHBTC", "LINKBTC", "DOTBTC", "TRYBTC", "PEPEBTC", "DOGEUSDT"]));

   let best_symbols_shared = best_symbols.clone();
   let handle1 = tokio::spawn(async move {
      arbitrage::find_best_assets(best_symbols_shared).await.unwrap();
  });

   // let best_symbols_shared = best_symbols.clone();
   // let handle2 = tokio::spawn(async move {
   //    arbitrage::find_best_assets_2(best_symbols_shared).await.unwrap();
   // });

   let _ = handle1.await;
   // let _ = handle2.await;
}
