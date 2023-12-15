use super::bellmanford::Edge;
use super::constants::{MAX_CYCLE_LENGTH, MODE};
use super::helpers::validate_quantity;
use super::models::{Direction, SmartError, Mode};
use super::traits::{ApiCalls, BellmanFordEx, ExchangeData};

/// Execute Arbitrage Cycle
/// Executes Arbitrage Cycle.
/// Using panics as checks should happen before this function is called.
pub async fn execute_arbitrage_cycle<T>(
  budget: f64,
  cycle: &Vec<Edge>,
  symbols: &Vec<String>, 
  directions: &Vec<Direction>,
  exchange: &T
) -> Result<(), SmartError> 
  where T: BellmanFordEx + ExchangeData + ApiCalls 
{

  // Guard: Ensure mode is set to trade
  let is_trade = match MODE {
    Mode::Searcher(_, is_trade) => is_trade,
    Mode::Listener(_, is_trade) => is_trade,
  };
  if !is_trade { panic!("Tried to place trade when Mode not set to trading") }

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
  
  // Initialize
  let mut quantity: f64 = budget;
  let info_symbols = exchange.symbols();
  let general_prices = exchange.prices();

  for i in 0..symbols.len() {
    let symbol = &symbols[i];
    let direction = &directions[i];
    let leg = &cycle[i];

    // Execute Trade
    println!("---");
    println!("Placing order:");
    println!("symbol: {}", symbol);
    println!("leg: {}", i);
    println!("direction: {:?}", direction);
    println!("side: {}", direction.side());
    println!("initial quantity: {}", quantity);
    println!("---");
    
    // Adjust quantity if lower asset balance
    let asset: String = leg.from.clone();
    let asset_balance: f64 = exchange.get_asset_account_balance(&asset).await.expect("Failed to get asset balance");
    if asset_balance == 0.0 { panic!("No trading amount available for this asset") }
    if asset_balance < quantity { quantity = asset_balance };

    // Adj quantity for formatting
    let symbol_info = info_symbols.get(symbol).expect("Failed to extract symbol during live trade");
    let general_price = general_prices[symbol];
    quantity = match validate_quantity(&symbol_info, quantity, general_price, direction) {
      Ok(qty) => qty,
      Err(_e) => {
        dbg!(&_e);
        quantity
      }
    };
    
    // PLACE TRADE
    let result = exchange.place_market_order(symbol, direction, quantity).await;

    // Update next quantity to match what was received
    match result {
      Ok((status, base_amount_out, quote_amount_out)) => {

        // Guard: Ensure success else panic
        if base_amount_out == 0.0 || status.as_str() != "FILLED" {
          panic!("Order not filled: {} in {:?}, status: {}, base_amount: {}, quote_amouunt: {}", symbol, symbols, status, base_amount_out, quote_amount_out);
        }
        
        // Update quantity for next trade
        if i < symbols.len() - 1 {
          match direction {
            Direction::Forward => quantity = quote_amount_out,
            Direction::Reverse => quantity = base_amount_out,
          }
        }
      },
      Err(e) => {
        panic!("Order not filled: {} in {:?}: e: {:?}", symbol, symbols, e);
      }
    }
  }

  Ok(())
}
