// https://github.com/coderaidershaun/multithread-rust-arbitrage
use crate::arbitrage::{validate_arbitrage_cycle, store_arb_cycle, calculate_arbitrage_surface_rate};
use crate::bellmanford::BellmanFord;
use crate::constants::MIN_ARB_THRESH;
use crate::helpers::create_exchange_rates;
use crate::models::SmartError;
use crate::traits::ApiCalls;
use super::binance::Binance;

use tungstenite::{connect, Message};
use std::collections::HashMap;
use serde_json::Value;
use url::Url;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
    println!("thread: binance websocket running...");

    let mut timestamp: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let mut next_timestamp: u64 = timestamp + 60; // Start with 60 seconds, then wait longer on future rounds

    'inner: loop {
      
      // Get socket message
      let msg = socket.read_message().expect("Error reading message");
      let msg = match msg {
        Message::Text(s) => s,
        _ => { 
          println!("warning: binance not connected...");
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
          println!("symbols update, restarting connection...");
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
          if let Some(c) = cycle_opt {
            if c.len() > 0 {
              let arb_opt = validate_arbitrage_cycle(&c, &exch_clone).await;
              if let Some(arb) = arb_opt {
                if arb.0 >= MIN_ARB_THRESH {
                  let surface_rate = calculate_arbitrage_surface_rate(&c);
                  let _: () = store_arb_cycle(&c, arb.0, surface_rate).expect("Failed to store results");
                }
              }

              // Sleep
              std::thread::sleep(Duration::from_millis(100));
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
//     // websocket_binance().await;
//   }

// }