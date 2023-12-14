use super::bellmanford::Edge;
use super::constants::{MAX_CYCLE_LENGTH, MODE};
use super::models::{BookType, SmartError, Mode};
use super::traits::{ApiCalls, BellmanFordEx, ExchangeData};

/// Execute Arbitrage Cycle
/// Executes Arbitrage Cycle.
/// Using panics as checks should happen before this function is called.
pub async fn execute_arbitrage_cycle<T>(
  cycle: &Vec<Edge>,
  symbols: &Vec<String>, 
  quantities: &Vec<f64>, 
  book_types: &Vec<BookType>,
  exchange: &T
) -> Result<(), SmartError> 
  where T: BellmanFordEx + ExchangeData + ApiCalls 
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
  
  for i in 0..symbols.len() {
    let symbol = &symbols[i];
    let side = match book_types[i] {
      BookType::Asks => "SELL", // Forward Trade
      BookType::Bids => "BUY" // Reverse Trade
    };

    // Update Quantity based on account balance 
    // Uses smaller balance if account balance is smaller than what was hoped for
    let quantity: f64 = quantities[i];

    // Execute Trade
    // std::thread::sleep(std::time::Duration::from_millis(100));
    println!("---");
    println!("Placing order:");
    println!("symbol: {}", symbol);
    println!("leg: {}", i);
    println!("side: {}", side);
    println!("quantity: {}", quantity);
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
