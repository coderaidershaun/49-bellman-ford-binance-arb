use super::models::{Direction, SymbolInfo};
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
pub fn validate_quantity(symbol_info: &SymbolInfo, quantity: f64, general_price: f64, direction: &Direction) -> Result<f64, String> {
  let min_qty: f64 = symbol_info.min_qty.parse().map_err(|_| "Invalid min_qty")?;
  let max_qty: f64 = symbol_info.max_qty.parse().map_err(|_| "Invalid max_qty")?;
  let step_size: f64 = symbol_info.step_size.parse().map_err(|_| "Invalid step_size")?;

  let decimal_places: i32 = match direction {
    Direction::Forward => symbol_info.base_asset_precision as i32,
    Direction::Reverse => symbol_info.quote_asset_precision as i32,
  };

  let mut quantity: f64 = quantity;

  // Ensure Precision: Check if the quantity aligns with the precision
  let precision_factor = 10f64.powi(decimal_places);
  quantity = (quantity * precision_factor).round() / precision_factor;

  // Ensure Step Size: Check if the quantity aligns with the step size and adjust if necessary
  if direction == &Direction::Forward {
    if (quantity / step_size).fract() > 0.0 {
      quantity = (quantity / step_size).trunc() * step_size;
    }
  }

  // Format quantity to account for Rust f64 and f32 inherent imprecision
  let formatted_quantity = format!("{:.*}", decimal_places as usize, quantity);
  quantity = formatted_quantity.parse::<f64>().expect("Failed to parse formatted quantity back into f64");

  // Guard: Ensure quantity remaining is not zero
  if quantity == 0.0 {
    return Err(format!("Effective quantity after trade would leave zero: {} {} {}", symbol_info.symbol, quantity * general_price, symbol_info.max_notional));
  }

  // Checks for base asset
  if direction == &Direction::Forward {

    // Guard: Check if the quantity is greater than or equal to the minimum size
    if quantity < min_qty {
      return Err(format!("Quantity is less than the minimum required: {} {} {}", symbol_info.symbol, quantity, min_qty));
    }

    // Guard: Check if the quantity is less than or equal to the maximum
    if quantity > max_qty {
      return Err(format!("Quantity exceeds the maximum limit: {} {} {}", symbol_info.symbol, quantity, max_qty));
    }

    // Guard: Check if the quantity aligns with minimum notional value
    if quantity * general_price < symbol_info.min_notional.parse().expect("Failed to parse min notional value") {
      return Err(format!("Total trade value under minimum notional value: {} {} {}", symbol_info.symbol, quantity * general_price, symbol_info.min_notional));
    }

    // Guard: Check if the quantity aligns with minimum notional value
    if quantity * general_price > symbol_info.max_notional.parse().expect("Failed to parse max notional value") {
      return Err(format!("Total trade value over maximum notional value: {} {} {}", symbol_info.symbol, quantity * general_price, symbol_info.max_notional));
    }
  } else {
    // Guard: Check if the quantity aligns with minimum notional value
    if quantity < symbol_info.min_notional.parse().expect("Failed to parse min notional value") {
      return Err(format!("Total trade value under minimum notional value: {} {} {}", symbol_info.symbol, quantity, symbol_info.min_notional));
    }

    // Guard: Check if the quantity aligns with minimum notional value
    if quantity > symbol_info.max_notional.parse().expect("Failed to parse max notional value") {
      return Err(format!("Total trade value over maximum notional value: {} {} {}", symbol_info.symbol, quantity, symbol_info.max_notional));
    }
  }

  Ok(quantity)
}
