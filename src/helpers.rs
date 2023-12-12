use super::models::SymbolInfo;
use std::collections::HashMap;

/// Create Exchange Rates
/// Combines symbol and price information to create exchange rates
pub fn create_exchange_rates(
  symbols: &HashMap<String, SymbolInfo>,
  prices: &HashMap<String, f64>
) -> Vec<(String, String, f64)> {
  let mut exchange_rates = Vec::new();
  for (symbol, symbol_info) in symbols {
    if let Some(&rate) = prices.get(symbol) {
      exchange_rates.push((symbol_info.base_asset.clone(), symbol_info.quote_asset.clone(), rate));
      if rate != 0.0 {  // Prevent division by zero
        exchange_rates.push((symbol_info.quote_asset.clone(), symbol_info.base_asset.clone(), 1.0 / rate));
      }
    }
  }
  exchange_rates
}

/// Validate Quantity
/// Validates that the quantity being requested matches exchange criteria
pub fn validate_quantity(symbol_info: &SymbolInfo, quantity: f64, price: f64) -> Result<f64, String> {
  let min_qty: f64 = symbol_info.min_qty.parse().map_err(|_| "Invalid min_qty")?;
  let max_qty: f64 = symbol_info.max_qty.parse().map_err(|_| "Invalid max_qty")?;
  let step_size: f64 = symbol_info.step_size.parse().map_err(|_| "Invalid step_size")?;

  dbg!(&symbol_info);

  let mut quantity = quantity;

  // Check if the quantity is greater than or equal to the minimum size
  if quantity < min_qty {
    return Err("Quantity is less than the minimum required".to_string());
  }

  // Check if the quantity is less than or equal to the maximum
  if quantity > max_qty {
    return Err("Quantity exceeds the maximum limit".to_string());
  }

  // Check if the quantity aligns with the step size
  if (quantity / step_size).fract() > 0.0 {
    return Err("Quantity does not align with the step size".to_string());
  }

  // Check if the quantity aligns with minimum notional value
  if quantity * price < symbol_info.min_notional.parse().expect("Failed to parse min notional value") {
    return Err("Total trade value under minimum notional value".to_string());
  }

  // Check if the quantity aligns with minimum notional value
  if quantity * price > symbol_info.max_notional.parse().expect("Failed to parse max notional value") {
    return Err("Total trade value over maximum notional value".to_string());
  }

  // Check if the quantity aligns with the precision
  let precision_factor = 10f64.powi(symbol_info.base_asset_precision as i32);
  if (quantity * precision_factor).fract() > 0.0 {
    quantity = (quantity * precision_factor).round() / precision_factor;
    // return Err("Quantity does not align with the required precision".to_string());
  }

  Ok(quantity)
}