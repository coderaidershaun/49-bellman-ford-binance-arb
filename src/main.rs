mod arb_detection;
mod arb_execution;
mod bellmanford;
mod constants;
mod exchanges;
mod helpers;
mod models;
mod traits;

use exchanges::binance_ws;
use models::Mode;

use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
   println!("initializing program...");
   let init_symbols: Vec<&str> = vec!["BTCUSDT", "ETHUSDT", "LINKUSDT", "SOLUSDT", "ADABTC", "LINKBTC", "XMRBTC", "ADAUSDT", "PEPEBTC", "DOGEUSDT", "DOGEBTC"];
   let init_symbols: Vec<String> = init_symbols.iter().map(|s| s.to_string()).collect();
   let best_symbols: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(init_symbols));

   match constants::MODE {
      Mode::TradeSearch(_) | Mode::NoTradeSearch(_) => {
         arb_detection::arb_scanner().await.unwrap();
      },
      Mode::TradeWss(_) | Mode::NoTradeWss(_) => {
         binance_ws::websocket_binance(best_symbols).await.unwrap();
      }
      Mode::TradeWssWithSearch(_) | Mode::NoTradeBoth(_) => {
         let best_symbols_shared = best_symbols.clone();
         let handle1 = tokio::spawn(async move {
            arb_detection::best_symbols_thread(best_symbols_shared).await.unwrap();
         });
      
         let best_symbols_shared = best_symbols.clone();
         let handle2 = tokio::spawn(async move {
            binance_ws::websocket_binance(best_symbols_shared).await.unwrap();
         });
      
         let _ = handle1.await;
         let _ = handle2.await;
      }
   }
}
