// https://github.com/coderaidershaun/multithread-rust-arbitrage
use crate::arb_detection::{validate_arbitrage_cycle, store_arb_cycle, calculate_arbitrage_surface_rate};
use crate::arb_execution::execute_arbitrage_cycle;
use crate::bellmanford::BellmanFord;
use crate::constants::{MIN_ARB_THRESH, ASSET_HOLDINGS, MODE};
use crate::helpers::create_exchange_rates;
use crate::models::{Mode, SmartError};
use crate::traits::ApiCalls;
use super::binance::Binance;

use tungstenite::{connect, Message};
use std::collections::HashMap;
use serde_json::Value;
use url::Url;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::io::Write;

const BINANCE_WS_API: &str = "wss://stream.binance.com:9443";

/// Extract Tickers
/// Extracts tickers from shared_best symbols
fn extract_tickers(shared_best_symbols: Arc<Mutex<Vec<String>>>) -> Vec<String> {
  let best_symbols_arr = shared_best_symbols.lock().unwrap();
  best_symbols_arr.iter().map(|s| s.to_owned().to_lowercase()).collect()
}

/// Websocket Binance
/// Listens to latest bid and ask prices for a set of assets
pub async fn websocket_binance(shared_best_symbols: Arc<Mutex<Vec<String>>>) -> Result<(), SmartError> {

  let is_calculating = Arc::new(AtomicBool::new(false));
  let is_calculating_for_thread = is_calculating.clone();

  '_outer: loop {

    // Initialize Exchange
    let exchange: Binance = Binance::new().await;
    let mut prices: HashMap<String, f64> = HashMap::new();

    // Extract tickers from best
    let tickers: Vec<String> = extract_tickers(shared_best_symbols.clone());

    // Construct Stream
    let ext_url: Vec<String> = tickers.iter().map(|t| format!("{}@bookTicker/", t)).collect();
    let ext_url_str = ext_url.concat();
    let mut binance_url = format!("{}/stream?streams={}", BINANCE_WS_API, ext_url_str);
    binance_url.pop();

    // Connect to websocket
    let (mut socket, _) = connect(Url::parse(&binance_url).unwrap()).expect("Can't connect.");
    println!("\rthread: binance websocket running...");
    std::io::stdout().flush().unwrap();

    let mut timestamp: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let mut next_timestamp: u64 = timestamp + 60; // Start with 60 seconds, then wait longer on future rounds

    'inner: loop {
      
      // Get socket message
      let msg = socket.read_message().expect("Error reading message");
      let msg = match msg {
        Message::Text(s) => s,
        _ => {
          println!("\rwarning: binance not connected...");
          std::io::stdout().flush().unwrap();
          break 'inner;
        },
      };

      // Parse text data
      let parsed_data: Value = serde_json::from_str(&msg).expect("Unable to parse Binance message");

      // Extract info
      let symbol: String = parsed_data["data"]["s"].as_str().unwrap().to_uppercase();
      let best_ask: f64 = parsed_data["data"]["a"].as_str().unwrap().parse::<f64>().unwrap();
      let best_bid: f64 = parsed_data["data"]["b"].as_str().unwrap().parse::<f64>().unwrap();

      // Insert price
      let mid_price: f64 = (best_ask + best_bid) / 2.0;
      prices.insert(symbol, mid_price);

      // Guard: Continue processing messages if calculation is in progress
      if is_calculating_for_thread.load(Ordering::Relaxed) {
        continue;
      }

      // Guard: Check for best symbols updates
      timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
      if timestamp >= next_timestamp {
        next_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 10; // Check every 10 seconds
        let new_tickers: Vec<String> = extract_tickers(shared_best_symbols.clone());
        if new_tickers != tickers { 
          println!("\rsymbols update, restarting connection...");
          std::io::stdout().flush().unwrap();
          socket.close(None)?;
          break 'inner; 
        }
      }

      // Update exchange rates
      let exchange_rates: Vec<(String, String, f64)> = create_exchange_rates(&exchange.symbols, &prices);

      // Start arbitrage calculation on new thread
      let is_calculating_clone = is_calculating.clone();
      let exch_clone = exchange.clone();
      if prices.len() >= 3 {
        tokio::spawn(async move {
          is_calculating_clone.store(true, Ordering::Relaxed);

          // Check for arbitrage
          let bf: BellmanFord = BellmanFord::new(&exchange_rates);
          let cycle_opt = bf.find_negative_cycle();
          if let Some(cycle) = cycle_opt {
            if cycle.len() > 0 {
              let arb_opt = validate_arbitrage_cycle(&cycle, &exch_clone).await;
              if let Some((arb_rate, symbols, directions, budget)) = arb_opt {

                // Ensure arb rate
                if arb_rate >= MIN_ARB_THRESH { 

                  // Guard: Ensure from asset is ipart of Holding Assets
                  let from_asset = cycle[0].from.as_str();
                  if !ASSET_HOLDINGS.contains(&from_asset) { panic!("Error: Asset holdings do not include symbol") }

                  // Execute and get store trigger
                  let is_store = match MODE {
                    Mode::TradeWss(is_store) | Mode::TradeWssWithSearch(is_store) => {

                      // !!! PLACE TRADE !!!
                      println!("Placing trade...");
                      let result = execute_arbitrage_cycle(
                        budget,
                        &cycle,
                        &symbols,
                        &directions, 
                        &exch_clone
                      ).await;
                      
                      if let Err(e) = result {
                          panic!("Failed to place trade: {:?}", e);
                      }

                      is_store
                    },
                    Mode::NoTradeWss(is_store) => is_store,
                    _ => false
                  };

                  // Store Result
                  if is_store {
                    let arb_surface: f64 = calculate_arbitrage_surface_rate(&cycle);
                    let _: () = store_arb_cycle(&cycle, arb_rate, arb_surface).expect("Failed to save arb");
                  }
                }
              }

              // Sleep
              std::thread::sleep(Duration::from_millis(50));
            }
          }

          is_calculating_clone.store(false, Ordering::Relaxed);
        });
      }
    }
  }
}

// #[cfg(test)]
// mod test {
//   use super::*;

//   #[tokio::test]
//   async fn it_runs_binance_ws() {
//     let init_symbols: Vec<String> = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string(), "LINKUSDT".to_string()];
//     let best_symbols = Arc::new(Mutex::new(init_symbols));
//     let _ = websocket_binance(best_symbols).await;
//   }
// }