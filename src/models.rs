#[derive(thiserror::Error, Debug)]
pub enum SmartError {
  #[error("Runtime error check failed")]
  Runtime(String),
  #[error(transparent)]
  Csv(#[from] csv::Error),
  #[error(transparent)]
  Reqwest(#[from] reqwest::Error),
  #[error(transparent)]
  Websocket(#[from] tungstenite::Error),
  #[error(transparent)]
  ParseFloat(#[from] std::num::ParseFloatError),
  #[error(transparent)]
  Io(#[from] std::io::Error),
  #[error(transparent)]
  SerdeJson(#[from] serde_json::Error)
}

pub type IsStore = bool;
pub type IsTrade = bool;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Mode {
  Searcher(IsStore, IsTrade),
  Listener(IsStore, IsTrade),
}

#[derive(Debug, PartialEq)]
pub enum Direction {
  Forward,
  Reverse
}

impl Direction {
  pub fn orderbook(&self) -> &'static str {
    match self {
      Self::Forward => "bids",
      Self::Reverse => "asks"
    }
  }

  pub fn side(&self) -> &'static str {
    match self {
      Self::Forward => "SELL",
      Self::Reverse => "BUY"
    }
  }
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
  pub symbol: String,
  pub base_asset: String,
  pub quote_asset: String,
  pub base_asset_precision: u8,
  pub quote_asset_precision: u8,
  pub min_qty: String,
  pub max_qty: String,
  pub min_notional: String,
  pub max_notional: String,
  pub step_size: String,
}

#[derive(Debug)]
pub struct ExchangeRate {
  pub symbol: String,
  pub from: String,
  pub to: String,
  pub best_bid_price: f64,
  pub best_ask_price: f64,
  pub best_bid_size: f64,
  pub best_ask_size: f64
}

#[derive(Debug, serde::Serialize)]
pub struct ArbData {
  pub timestamp: u64,
  pub arb_length: usize,
  pub arb_rate: f64,
  pub arb_surface: f64,
  pub asset_0: Option<String>,
  pub asset_1: Option<String>,
  pub asset_2: Option<String>,
  pub asset_3: Option<String>,
  pub asset_4: Option<String>,
  pub asset_5: Option<String>,
  pub asset_6: Option<String>,
  pub asset_7: Option<String>
}
