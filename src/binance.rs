use super::bellmanford::{BellmanFord, Edge};
use super::models::{BookType, SmartError};
use super::traits::{ApiCalls, BellmanFordEx, ExchangeData};
use super::helpers;

use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Binance {
  pub symbols: HashMap<String, (String, String)>,
  pub prices: HashMap<String, f64>,
  pub exchange_rates: Vec<(String, String, f64)>,
}

#[async_trait]
impl ApiCalls for Binance {
  async fn new() -> Self {
    println!("extracting binance exchange rates...");
    let symbols = Self::fetch_symbols().await.expect("Failed to fetch Binance symbols");
    let prices = Self::fetch_prices().await.expect("Failed to fetch Binance prices");
    let exchange_rates = helpers::create_exchange_rates(&symbols, &prices);
    Self { symbols, prices, exchange_rates }
  }
  
  /// Fetch Binance Symbols
  /// Retrieves Base and Quote symbol information so symbols can be broken up
  async fn fetch_symbols() -> Result<HashMap<String, (String, String)>, SmartError> {
    let url: &str = "https://api.binance.com/api/v3/exchangeInfo";
    let response: reqwest::Response = reqwest::get(url).await?;
    let data: serde_json::Value = response.json().await.unwrap();
    let mut symbols: HashMap<String, (String, String)> = HashMap::new();
    if let Some(symbol_infos) = data["symbols"].as_array() {
      for symbol_info in symbol_infos {
        if let Some(status) = symbol_info["status"].as_str() {
          if status == "TRADING" {
            let symbol = symbol_info["symbol"].as_str().unwrap_or_default().to_string();
            let base_asset = symbol_info["baseAsset"].as_str().unwrap_or_default().to_string();
            let quote_asset = symbol_info["quoteAsset"].as_str().unwrap_or_default().to_string();
            symbols.insert(symbol, (base_asset, quote_asset));
          }
        }
      }
    }
  
    Ok(symbols)
  }
  
  /// Fetch Binance Prices
  /// Retrieves current prices for assets
  async fn fetch_prices() -> Result<HashMap<String, f64>, SmartError> {
    let url = "https://api.binance.com/api/v3/ticker/price";
    let response = reqwest::get(url).await?;
    let data: serde_json::Value = response.json().await?;
    let mut prices = HashMap::new();
    if let Some(price_items) = data.as_array() {
      for item in price_items {
        let symbol = item["symbol"].as_str().unwrap_or_default().to_string();
        let price = item["price"].as_str().unwrap_or_default().parse::<f64>()?;
        prices.insert(symbol, price);
      }
    }
  
    Ok(prices)
  }

  /// Get Orderbook Depth
  /// Retrieves orderbook depth for either bids or asks
  async fn get_orderbook_depth(&self, symbol: &str, book_type: BookType) -> Result<Vec<(f64, f64)>, SmartError> {
    let url: String = format!("https://api.binance.com/api/v3/depth?symbol={}", symbol);
    let resp: reqwest::Response = reqwest::get(&url).await?;

    if resp.status().is_success() {
      let data: serde_json::Value = resp.json().await?;
      let order_book = data[book_type.as_str()].as_array().ok_or("Invalid JSON structure").map_err(|e| SmartError::Runtime(e.to_string()))?;

      let mut result = vec![];
      for item in order_book {
        let price = item[0].as_str().ok_or("Invalid price format").map_err(|e| SmartError::Runtime(e.to_string()))?.parse::<f64>()?;
        let qty = item[1].as_str().ok_or("Invalid quantity format").map_err(|e| SmartError::Runtime(e.to_string()))?.parse::<f64>()?;
        result.push((price, qty));
      }

      if let BookType::Asks = book_type {
        result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
      } else {
        result.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
      }

      Ok(result)
    } else {
      Err(SmartError::Runtime("Failed to fetch data".to_string()))
    }
  }
}

impl BellmanFordEx for Binance {
  fn run_bellman_ford_single(&self) -> Option<Vec<Edge>> {
    println!("running bellman ford...");
    let bf = BellmanFord::new(&self.exchange_rates);
    let cycles = bf.find_negative_cycle();
    cycles
  }

  fn run_bellman_ford_multi(&self) -> Vec<Vec<Edge>> {
    println!("running bellman ford...");
    let bf = BellmanFord::new(&self.exchange_rates);
    let cycles = bf.find_all_negative_cycles();
    cycles
  }
}

impl ExchangeData for Binance {
  fn symbols(&self) -> &HashMap<String, (String, String)> { &self.symbols }
  fn prices(&self) -> &HashMap<String, f64> { &self.prices }
  fn exchange_rates(&self) -> &Vec<(String, String, f64)> { &self.exchange_rates }
}
