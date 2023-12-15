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

#[tokio::main]
async fn main() {
   println!("initializing program...");   
   match constants::MODE {
      Mode::Searcher(_, _) => {
         arb_detection::arb_scanner().await.unwrap();
      },
      Mode::Listener(_, _) => {
         binance_ws::websocket_binance().await.unwrap();
      }
   }
}
