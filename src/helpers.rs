use std::collections::HashMap;

/// Create Exchange Rates
/// Combines symbol and price information to create exchange rates
pub fn create_exchange_rates(
  symbols: &HashMap<String, (String, String)>,
  prices: &HashMap<String, f64>
) -> Vec<(String, String, f64)> {
  let mut exchange_rates = Vec::new();
  for (symbol, (base, quote)) in symbols {
    if let Some(&rate) = prices.get(symbol) {
      exchange_rates.push((base.clone(), quote.clone(), rate));
      if rate != 0.0 {  // Prevent division by zero
        exchange_rates.push((quote.clone(), base.clone(), 1.0 / rate));
      }
    }
  }
  exchange_rates
}