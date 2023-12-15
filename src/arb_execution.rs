use super::constants::{MAX_CYCLE_LENGTH, MODE};
use super::helpers::validate_quantity;
use super::models::{Direction, SmartError, Mode};
use super::traits::{ApiCalls, BellmanFordEx, ExchangeData};

/// Execute Arbitrage Cycle
/// Executes Arbitrage Cycle.
/// Using panics as checks should happen before this function is called.
pub async fn execute_arbitrage_cycle<T>(
  budget: f64,
  symbols: &Vec<String>, 
  directions: &Vec<Direction>,
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

  // Guard: Ensure symbols lenght matches directions length
  assert_eq!(symbols.len(), directions.len());
  
  // Initialize quantity
  let mut quantity: f64 = budget;
  dbg!(symbols);

  for i in 0..symbols.len() {
    let symbol = &symbols[i];
    let direction = &directions[i];

    // Execute Trade
    println!("---");
    println!("Placing order:");
    println!("symbol: {}", symbol);
    println!("leg: {}", i);
    println!("direction: {:?}", direction);
    println!("side: {}", direction.side());
    println!("quantity: {}", quantity);
    println!("---");

    let result = exchange.place_market_order(symbol, direction, quantity).await;
    let info_symbols = exchange.symbols();
    let general_prices = exchange.prices();

    // Update next quantity to match what was received
    match result {
      Ok((status, base_amount_out, quote_amount_out)) => {

        // Guard: Ensure success else panic
        if base_amount_out == 0.0 || status.as_str() != "FILLED" {
          panic!("Order not filled: {:?}, {}, {}, {}", symbols, status, base_amount_out, quote_amount_out);
        }
        
        // Update quantity for next trade
        if i < symbols.len() - 1 {
          match direction {
            Direction::Forward => quantity = quote_amount_out,
            Direction::Reverse => quantity = base_amount_out,
          }

          // Ensure that the quantity intended to be sent to the exchange is correct
          dbg!(&i);
          dbg!(&symbols.len());
          dbg!(&directions.len());
          if direction == &Direction::Reverse {
            let symbol_info = info_symbols.get(symbol).expect("Failed to extract symbol during live trade");
            let general_price = general_prices[symbol];
            quantity = match validate_quantity(&symbol_info, quantity, general_price) {
              Ok(qty) => qty,
              Err(_e) => {
                  panic!("Failed to validate quantity: {:?}", _e);
              }
            };
          }
        }
      },
      Err(e) => panic!("Failed to execute trade: {:?}", e)
    }
  }

  Ok(())
}
