use super::bellmanford::Edge;
use super::models::{BookType, SmartError, SymbolInfo};

use async_trait::async_trait;
use std::collections::HashMap;

pub trait ExchangeData {
  fn symbols(&self) -> &HashMap<String, SymbolInfo>;
  fn prices(&self) -> &HashMap<String, f64>;
  fn exchange_rates(&self) -> &Vec<(String, String, f64)>;
}

#[async_trait]
pub trait ApiCalls {
  async fn new() -> Self;
  async fn fetch_symbols() -> Result<HashMap<String, SymbolInfo>, SmartError>;
  async fn fetch_prices() -> Result<HashMap<String, f64>, SmartError>;
  async fn get_orderbook_depth(&self, symbol: &str, book_type: BookType) -> Result<Vec<(f64, f64)>, SmartError>;
  async fn place_market_order(&self, symbol: &str, side: &str, quantity: f64) -> Result<reqwest::Response, reqwest::Error>;
}

pub trait BellmanFordEx {
  fn run_bellman_ford_single(&self) -> Option<Vec<Edge>>;
  fn run_bellman_ford_multi(&self) -> Vec<Vec<Edge>>;
}
