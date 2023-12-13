use crate::bellmanford::{BellmanFord, Edge};
use crate::models::{BookType, SmartError, SymbolInfo};
use crate::traits::{ApiCalls, BellmanFordEx, ExchangeData};
use crate::helpers;

use async_trait::async_trait;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Binance {
  pub symbols: HashMap<String, SymbolInfo>,
  pub prices: HashMap<String, f64>,
  pub exchange_rates: Vec<(String, String, f64)>,
}

#[async_trait]
impl ApiCalls for Binance {
  async fn new() -> Self {
    let symbols = Self::fetch_symbols().await.expect("Failed to fetch Binance symbols");
    let prices = Self::fetch_prices().await.expect("Failed to fetch Binance prices");
    let exchange_rates = helpers::create_exchange_rates(&symbols, &prices);
    Self { symbols, prices, exchange_rates }
  }
  
  /// Fetch Binance Symbols
  /// Retrieves Base and Quote symbol information so symbols can be broken up
  async fn fetch_symbols() -> Result<HashMap<String, SymbolInfo>, SmartError> {
    let url: &str = "https://api.binance.com/api/v3/exchangeInfo";
    let response: reqwest::Response = reqwest::get(url).await?;
    let data: serde_json::Value = response.json().await.unwrap();
    let mut symbols: HashMap<String, SymbolInfo> = HashMap::new();

    if let Some(symbol_infos) = data["symbols"].as_array() {
      for symbol_info in symbol_infos {
        if symbol_info["status"] == "TRADING" && symbol_info["isSpotTradingAllowed"].as_bool().unwrap_or(false) {
          let symbol = symbol_info["symbol"].as_str().unwrap_or_default().to_string();
          let base_asset = symbol_info["baseAsset"].as_str().unwrap_or_default().to_string();
          let quote_asset = symbol_info["quoteAsset"].as_str().unwrap_or_default().to_string();
          let base_asset_precision = symbol_info["baseAssetPrecision"].as_u64().unwrap_or_default() as u8;
          let quote_asset_precision = symbol_info["quoteAssetPrecision"].as_u64().unwrap_or_default() as u8;

          // Extract minQty, maxQty, and stepSize from LOT_SIZE filter
          let lot_size_filter = symbol_info["filters"].as_array().unwrap()
            .iter()
            .find(|&f| f["filterType"] == "LOT_SIZE")
            .unwrap_or(&serde_json::Value::Null);
          let min_qty = lot_size_filter["minQty"].as_str().unwrap_or_default().to_string();
          let max_qty = lot_size_filter["maxQty"].as_str().unwrap_or_default().to_string();
          let step_size = lot_size_filter["stepSize"].as_str().unwrap_or_default().to_string();

          // Extract min_notional and max_notional from MIN_NOTIONAL filter
          let notional_filter = symbol_info["filters"].as_array().unwrap()
            .iter()
            .find(|&f| f["filterType"] == "NOTIONAL")
            .unwrap_or(&serde_json::Value::Null);
          let min_notional = notional_filter["minNotional"].as_str().unwrap_or_default().to_string();
          let max_notional = notional_filter["maxNotional"].as_str().unwrap_or_default().to_string();

          let symbol_info = SymbolInfo {
              symbol,
              base_asset,
              quote_asset,
              base_asset_precision,
              quote_asset_precision,
              min_qty,
              max_qty,
              min_notional,
              max_notional,
              step_size,
          };

          symbols.insert(symbol_info.symbol.clone(), symbol_info);
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

  /// Place Market Order
  /// Places market order
  /// Side BUY / SELL
  async fn place_market_order(&self, symbol: &str, side: &str, quantity: f64) -> Result<reqwest::Response, reqwest::Error> {
    let api_key = "YOUR KEY";
    let secret_key = "YOUR SECRET";
    let order_type = "MARKET";
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string();

    // Create query string
    let mut query = format!("symbol={}&side={}&type={}&quantity={}&timestamp={}", symbol, side, order_type, quantity, timestamp);

    // Create signature
    let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()).unwrap();
    mac.update(query.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    // Append signature to query
    query.push_str("&signature=");
    query.push_str(&signature);

    // Send request
    let client = reqwest::Client::new();
    let res: reqwest::Response = client.post("https://api.binance.com/api/v3/order")
      .header("X-MBX-APIKEY", api_key)
      .body(query)
      .send()
      .await?;

    Ok(res)
  }
}

impl BellmanFordEx for Binance {
  fn run_bellman_ford_single(&self) -> Option<Vec<Edge>> {
    let bf = BellmanFord::new(&self.exchange_rates);
    let cycle = bf.find_negative_cycle();
    cycle
  }

  fn run_bellman_ford_multi(&self) -> Vec<Vec<Edge>> {
    let bf = BellmanFord::new(&self.exchange_rates);
    let cycles = bf.find_all_negative_cycles();
    cycles
  }
}

impl ExchangeData for Binance {
  fn symbols(&self) -> &HashMap<String, SymbolInfo> { &self.symbols }
  fn prices(&self) -> &HashMap<String, f64> { &self.prices }
  fn exchange_rates(&self) -> &Vec<(String, String, f64)> { &self.exchange_rates }
}

// #[cfg(test)]
// mod test {

//   use super::*;

//   #[tokio::test]
//   async fn it_places_a_trade() {
//     let exchange: Binance = Binance::new().await;
//     let symbol = "BTCUSDT";
//     let quantity = 0.0002;
//     let side = "BUY";

//     dbg!(quantity);
//     dbg!(side);
//     let symbol_info: &SymbolInfo = exchange.symbols.get(symbol).unwrap();
//     let price: f64 = *exchange.prices.get(symbol).unwrap();
//     let size: f64 = helpers::validate_quantity(symbol_info, quantity, price).unwrap();

//     let order = exchange.place_market_order(symbol, side, size).await;
//     dbg!(order);
//   }
// }
