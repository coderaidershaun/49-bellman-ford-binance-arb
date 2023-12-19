use crate::bellmanford::{BellmanFord, Edge};
use crate::constants::FIAT_EXCLUSION;
use crate::models::{Direction, SmartError, SymbolInfo};
use crate::traits::{ApiCalls, BellmanFordEx, ExchangeData};
use crate::helpers;

use async_trait::async_trait;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use dotenv::dotenv;
use std::env;

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
          
          // Guard: Ensure no fiat currency
          if FIAT_EXCLUSION.contains(&base_asset.as_str()) || FIAT_EXCLUSION.contains(&quote_asset.as_str()) { continue; }

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
  async fn get_orderbook_depth(&self, symbol: &str, direction: &Direction) -> Result<Vec<(f64, f64)>, SmartError> {
    let url: String = format!("https://api.binance.com/api/v3/depth?symbol={}", symbol);
    let resp: reqwest::Response = reqwest::get(&url).await?;

    if resp.status().is_success() {
      let data_res: Result<serde_json::Value, reqwest::Error> = resp.json().await;
      let data = match data_res {
        Ok(data) => data,
        Err(e) => panic!("Failed to extract orderbook: {:?}", e)
      };

      let book: &str = direction.orderbook();
      let order_book = data[book].as_array().ok_or("Invalid JSON structure").map_err(|e| SmartError::Runtime(e.to_string()))?;

      let mut result = vec![];
      for item in order_book {
        let price = item[0].as_str().ok_or("Invalid price format").map_err(|e| SmartError::Runtime(e.to_string()))?.parse::<f64>()?;
        let qty = item[1].as_str().ok_or("Invalid quantity format").map_err(|e| SmartError::Runtime(e.to_string()))?.parse::<f64>()?;
        result.push((price, qty));
      }

      if book == "asks" {
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
  async fn place_market_order(&self, symbol: &str, direction: &Direction, quantity: f64) -> Result<(String, f64, f64), SmartError> {
    dotenv().ok();

    let api_key = env::var("BINANCE_API_KEY")
      .expect("BINANCE_API_KEY not found in .env file");

    let api_secret = env::var("BINANCE_API_SECRET")
      .expect("BINANCE_API_SECRET not found in .env file");

    let order_type = "MARKET";
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string();

    // Create query string
    // i.e. BTCUSDT Reverse: I want to acquire BTC but the amount I have is in USDT: quoteOrderQty
    // i.e. BTCUSDT Forward: I want to acquire USDT and the amount that I have is BTC: quantity
    let mut query = match direction {
      Direction::Forward => format!("symbol={}&side={}&type={}&quantity={}&timestamp={}", symbol, direction.side(), order_type, quantity, timestamp),
      Direction::Reverse => format!("symbol={}&side={}&type={}&quoteOrderQty={}&timestamp={}", symbol, direction.side(), order_type, quantity, timestamp),
    };

    // Create signature
    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes()).unwrap();
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

    let res_text: String = res.text().await?;

    dbg!(&res_text);

    let res_json: serde_json::Value = serde_json::from_str(res_text.as_str())?;
    let status: String = res_json["status"].as_str().expect("Status not returned for order placed").to_owned();
    let executed_base_qty_str: &str = res_json["executedQty"].as_str().expect("Missing executed base quantity");
    let executed_base_qty: f64 = executed_base_qty_str.parse::<f64>().expect("Failed to parse executed base amount out");
    let executed_quote_qty_str: &str = res_json["cummulativeQuoteQty"].as_str().expect("Missing executed quote quantity");
    let executed_quote_qty: f64 = executed_quote_qty_str.parse::<f64>().expect("Failed to parse executed quote amount out");

    Ok((status, executed_base_qty, executed_quote_qty))
  }

  /// Get Asset Account Balance
  /// Retrieves Spot Balance for given asset (used for checking amounts available to trade)
  async fn get_asset_account_balance(&self, asset: &str) -> Result<f64, SmartError> {
    dotenv().ok();

    let api_key = env::var("BINANCE_API_KEY")
      .expect("BINANCE_API_KEY not found in .env file");

    let api_secret = env::var("BINANCE_API_SECRET")
      .expect("BINANCE_API_SECRET not found in .env file");

    // Constuct Query
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis().to_string();
    let mut query = format!("timestamp={}", timestamp);

    // Create signature
    let mut mac = Hmac::<Sha256>::new_from_slice(api_secret.as_bytes()).unwrap();
    mac.update(query.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    // Append signature to query
    query.push_str("&signature=");
    query.push_str(&signature);

    // Send request
    let url = format!("https://api.binance.com/api/v3/account?{}", query);
    let client = reqwest::Client::new();
    let res: reqwest::Response = client.get(url)
      .header("X-MBX-APIKEY", api_key)
      .send()
      .await?;

    let res_text = res.text().await?;
    let account_info: serde_json::Value = serde_json::from_str(&res_text)?;
    let balances: &Vec<serde_json::Value> = account_info["balances"].as_array().expect("Failed to find balances");
    let mut free_balance = 0.0;
    for item in balances {
      let coin = item["asset"].as_str().expect("Failed to locate asset");
      if coin == asset {
        free_balance = item["free"].as_str().expect("Failed to locate available amount").parse::<f64>().expect("Filed to parse balance");
        break;
      }
    }    

    Ok(free_balance)
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

#[cfg(test)]
mod test {
  use super::*;

  #[tokio::test]
  async fn it_creates_binance_instance() {
    std::thread::sleep(std::time::Duration::from_millis(100));
    let exchange: Binance = Binance::new().await;
    assert!(exchange.symbols.len() > 0);
    assert!(exchange.prices.len() > 0);
    assert!(exchange.exchange_rates.len() > 0);
  }

  #[tokio::test]
  async fn it_extracts_binance_orderbook() {
    std::thread::sleep(std::time::Duration::from_millis(100));
    let exchange: Binance = Binance::new().await;
    let orderbook_bids: Vec<(f64, f64)> = exchange.get_orderbook_depth("BTCUSDT", &Direction::Forward).await.unwrap();
    let orderbook_asks: Vec<(f64, f64)> = exchange.get_orderbook_depth("BTCUSDT", &Direction::Reverse).await.unwrap();
    assert!(orderbook_asks[0].0 > orderbook_bids[0].0);
    assert!(orderbook_asks[0].0 < orderbook_asks[1].0);
    assert!(orderbook_bids[0].0 > orderbook_bids[1].0);
  }

  #[tokio::test]
  async fn it_runs_bellman_ford_single_and_multi() {
    std::thread::sleep(std::time::Duration::from_millis(100));
    let exchange: Binance = Binance::new().await;
    let cycle = exchange.run_bellman_ford_single();
    let cycles = exchange.run_bellman_ford_multi();
    assert!(cycle.is_some());
    assert!(cycles.len() > 0);
  }

  // #[tokio::test]
  // async fn it_places_a_trade() {
  //   let exchange: Binance = Binance::new().await;
  //   let symbol = "BTCUSDT";
  //   let quantity = 20.0; // 20 USDT
  //   let direction = Direction::Reverse;

  //   let symbol_info: &SymbolInfo = exchange.symbols.get(symbol).unwrap();
  //   let price: f64 = *exchange.prices.get(symbol).unwrap();
  //   let quantity: f64 = helpers::validate_quantity(symbol_info, quantity, price, &direction).unwrap();

  //   let (status, base_amount_out, quote_amount_out) = exchange.place_market_order(symbol, &direction, quantity).await.unwrap();
  //   assert!(status.as_str() == "FILLED");
  //   assert!(base_amount_out > 0.0);
  //   assert!(quote_amount_out > 0.0);
  // }

  // #[tokio::test]
  // async fn it_gets_account_balance() {
  //   let exchange: Binance = Binance::new().await;
  //   let asset = "USDT";
  //   let balance = exchange.get_asset_account_balance(asset).await.unwrap();
  //   assert!(balance > 0.0);
  // }
}
