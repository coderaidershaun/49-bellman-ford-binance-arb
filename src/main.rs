mod arbitrage;
mod bellmanford;
mod constants;
mod exchanges;
mod helpers;
mod models;
mod traits;

use traits::{ApiCalls, BellmanFordEx};
use exchanges::binance;
use bellmanford::Edge;

use std::time::Duration;

#[tokio::main]
async fn main() {

   loop {
      std::thread::sleep(Duration::from_millis(100));
      println!("running analysis...");

      let exch_binance = binance::Binance::new().await;
      let cycles = exch_binance.run_bellman_ford_multi();
      
      /// Calculate Total Arbitrage Percentage of a Cycle
      fn calculate_arbitrage_percentage(cycle: &Vec<Edge>) -> f64 {
         cycle.iter().fold(1.0, |acc, edge| acc * f64::exp(-edge.weight)) - 1.0
      }
   
      for cycle in cycles {
         let arb_surface = calculate_arbitrage_percentage(&cycle) + 1.0;
         let arb_opt = arbitrage::validate_arbitrage_cycle(&cycle, &exch_binance).await;
         if let Some(arb_rate) = arb_opt {
            let _: () = arbitrage::store_arb_cycle(&cycle, arb_rate, arb_surface).unwrap();
         }
      }
   }
}
