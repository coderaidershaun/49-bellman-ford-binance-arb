// https://github.com/coderaidershaun/multithread-rust-arbitrage
use super::binance::Binance;
use crate::traits::ApiCalls;
use crate::helpers::create_exchange_rates;
use crate::bellmanford::BellmanFord;
use crate::models::{SymbolInfo, SmartError};

use tungstenite::{connect, Message};
use std::collections::HashMap;
use serde_json::Value;
use url::Url;


const BINANCE_WS_API: &str = "wss://stream.binance.com:9443";

pub async fn websocket_binance() -> Result<(), SmartError> {

  let symbols: HashMap<String, SymbolInfo> = Binance::fetch_symbols().await?;
  
  // Monitoring for price changes. Store last price in hashmap
  let mut prices: HashMap<String, f64> = HashMap::new();

  // Confirm URL
  let tickers = ["btcusdt", "ethusdt", "linausdt", "sandusdt", "iostusdt", "xrpusdt", "dotusdt", "btcbtc", "ethbtc", "linabtc", "sandbtc", "iostbtc", "xrpbtc", "dotbtc"];
  let ext_url: Vec<String> = tickers.iter().map(|t| format!("{}@bookTicker/", t)).collect();
  let ext_url_str = ext_url.concat();
  let mut binance_url = format!("{}/stream?streams={}", BINANCE_WS_API, ext_url_str);
  binance_url.pop();

  // Connect to websocket
  let (mut socket, _) = connect(Url::parse(&binance_url).unwrap()).expect("Can't connect.");
  println!("Successfully subscribed to Binance...");

  loop {

    // Get socket message
    let msg = socket.read_message().expect("Error reading message");
    let msg = match msg {
      Message::Text(s) => s,
      _ => { 
        println!("Binance not connected");
        continue;
      },
    };

    // Parse text data
    let parsed_data: Value = serde_json::from_str(&msg).expect("Unable to parse Binance message");

    // Extract info
    let symbol: String = parsed_data["data"]["s"].as_str().unwrap().to_uppercase();
    let best_ask: f64 = parsed_data["data"]["a"].as_str().unwrap().parse::<f64>().unwrap();
    let best_bid: f64 = parsed_data["data"]["b"].as_str().unwrap().parse::<f64>().unwrap();
    let ask_qty: f64 = parsed_data["data"]["A"].as_str().unwrap().parse::<f64>().unwrap();
    let bid_qty: f64 = parsed_data["data"]["B"].as_str().unwrap().parse::<f64>().unwrap();

    // Insert price
    let mid_price: f64 = (best_ask + best_bid) / 2.0;
    prices.insert(symbol, mid_price);
    dbg!(&prices);

    // // Check for arbitrage
    // if prices.len() >= 3 {
      
    //   let exchange_rates: Vec<(String, String, f64)> = create_exchange_rates(&symbols, &prices);
    //   let bf = BellmanFord::new(&exchange_rates);
    //   let cycle_opt = bf.find_negative_cycle();
    //   if let Some(c) = cycle_opt {
    //     if c.len() > 0 {
    //       dbg!(&exchange_rates);
    //       dbg!(c);
    //     }
    //   }
    // }
  }

}


#[cfg(test)]
mod test {

  use super::*;

  #[tokio::test]
  async fn it_runs_binance_ws() {
    websocket_binance().await;
  }

}