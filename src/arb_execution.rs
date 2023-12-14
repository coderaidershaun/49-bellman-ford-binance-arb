use crate::models::Mode;

use super::constants::{ASSET_HOLDINGS, MAX_CYCLE_LENGTH, MODE};
use super::models::{BookType, SmartError};
use super::traits::ApiCalls;

/// Execute Arbitrage Cycle
/// Executes Arbitrage Cycle.
/// Using panics as checks should happen before this function is called.
pub async fn execute_arbitrage_cycle<T>(
  from_asset: &str,
  symbols: &Vec<String>, 
  quantities: &Vec<f64>, 
  book_types: &Vec<BookType>,
  exchange: &T
) -> Result<(), SmartError> 
  where T: ApiCalls 
{

  // Guard: Ensure mode is set to trade
  match MODE {
    Mode::TradeSearch(_) => {},
    Mode::TradeWss(_) => {},
    _ => panic!("Error: Trade attempted when Mode not set to trading.")
  }

  // Guard: Ensure correct cycle length
  if symbols.len() > MAX_CYCLE_LENGTH {
    panic!("Error: Too many cycles. Max length set to {} in concstants", MAX_CYCLE_LENGTH)
  }

  // Guard: Ensure symbols len
  if symbols.len() < 3 {
    panic!("Error: Trade attempted when not enough cycle legs to complete trade")
  }

  // Guard: Ensure holdings includes first trade from
  if !ASSET_HOLDINGS.contains(&from_asset) {
    panic!("Error: Asset holdings do not include symbol")
  }
  
  for i in 0..symbols.len() {
    let symbol = &symbols[i];
    let quantity = quantities[i];
    let side = match book_types[i] {
      BookType::Asks => "SELL",
      BookType::Bids => "BUY"
    };

    println!("---");
    println!("Placing order:");
    println!("symbol: {}", symbol);
    println!("leg: {}", i);
    println!("quantity: {}", quantity);
    println!("side: {}", side);
    println!("---");

    let result = exchange.place_market_order(symbol, side, quantity).await;

    match result {
      Ok(res) => {
        let res_text: String = res.text().await.unwrap();
        if !res_text.contains("FILLED") { 
          panic!("Order not filled: {:?}", res_text); 
        }
      },
      Err(e) => panic!("Failed to execute trade: {:?}", e)
    }
  }

  Ok(())
}
