use super::constants::{ASSET_HOLDINGS, USD_BUDGET};
use super::models::{ArbData, BookType, Direction, SmartError};
use super::traits::{ApiCalls, BellmanFordEx, ExchangeData};
use super::bellmanford::Edge;

use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::OpenOptions;
use std::collections::HashSet;
use csv::Writer;

/// Calculate Weighted Average Price
/// Calculates the depth of the orderbook to get a real rate
fn calculate_weighted_average_price(
    orderbook: Vec<(f64, f64)>,
    budget: f64,
    direction: &Direction,
) -> Option<(f64, f64, f64)> {
    let mut total_cost = 0.0;
    let mut total_quantity = 0.0;

    for &(price, quantity) in orderbook.iter() {

        // Effective quantity is the amount of the quote asset you receive (or spend in reverse)
        let effective_quantity = match direction {
            Direction::Reverse => quantity,
            Direction::Forward => quantity * price,
        };

        // Cost is the amount of the base asset you spend (or receive in forward)
        let cost = match direction {
            Direction::Reverse => quantity * price,
            Direction::Forward => quantity,
        };

        // Check if adding this order exceeds the budget
        if total_cost + cost > budget {
            let remaining_budget = budget - total_cost;

            // Adjust the remaining quantity based on the direction of the trade
            let remaining_quantity = match direction {
                Direction::Reverse => remaining_budget / price,
                Direction::Forward => remaining_budget * price, // In forward, get quote asset amount
            };

            total_cost += remaining_budget;
            total_quantity += remaining_quantity;
            break;

        } else {
            total_cost += cost;
            total_quantity += effective_quantity;
        }

        if total_cost >= budget {
            break;
        }
    }

    if total_quantity == 0.0 {
        return None;
    }

    // Weighted average price calculation
    let weighted_average_price = match direction {
        Direction::Reverse => total_cost / total_quantity,
        Direction::Forward => total_quantity / total_cost,
    };
  
    Some((weighted_average_price, total_cost, total_quantity))
}

/// Validate Arbitrage Cycle
/// Validates arbitrage cycle has enough depth
pub async fn validate_arbitrage_cycle<T: BellmanFordEx>(cycle: &Vec<Edge>, exchange: &T) -> Option<f64> 
where T: BellmanFordEx + ExchangeData + ApiCalls {

    // Guard: Ensure cycle
    if cycle.len() == 0 { return None };

    // Guard: Ensure asset holding
    if !ASSET_HOLDINGS.contains(&cycle[0].from.as_str()) {
        return None
    }

    // Get starting budget
    let mut budget = match cycle[0].from.as_str() {
        "BTC" => USD_BUDGET / exchange.prices().get("BTCUSDT").expect("Expected price for BTCUSDT").to_owned(),
        "ETH" => USD_BUDGET / exchange.prices().get("ETHUSDT").expect("Expected price for ETHUSDT").to_owned(),
        "BNB" => USD_BUDGET / exchange.prices().get("BNBUSDT").expect("Expected price for BNBUSDT").to_owned(),
        "USDT" => USD_BUDGET,
        "BUSD" => USD_BUDGET,
        "USDC" => USD_BUDGET,
        _ => return None
    };

    // Initialize
    let mut real_rate = 1.0;

    // Assess trade
    for leg in cycle {

        // Initialize Symbol
        let symbol = if exchange.symbols().contains_key(format!("{}{}", leg.to, leg.from).as_str()) {
            format!("{}{}", leg.to, leg.from)
        } else {
            format!("{}{}", leg.from, leg.to)
        };

        // Initialize Direction
        // forward: price in to, qty in from
        // reverse: price in from, qty in to
        let (direction, book_type) = if symbol.starts_with(leg.from.as_str()) {
            (Direction::Forward, BookType::Asks)
        } else {
            (Direction::Reverse, BookType::Bids)
        };

        // Extract orderbook
        let orderbook = exchange.get_orderbook_depth(symbol.as_str(), book_type).await.expect("Failed to extract orderbook");

        // Calculate Average Price
        let Some((weighted_price, _, total_qty)) = calculate_weighted_average_price(
            orderbook, budget, &direction
        ) else {
            return None
        };

        // Update budget
        budget = total_qty;

        // Calculate Real Rate
        match direction {
            Direction::Forward => real_rate *= weighted_price,
            Direction::Reverse => real_rate *= 1.0 / weighted_price,
        }
    }

    // Return result
    if real_rate > 1.0 { Some(real_rate) } else { None }
}

/// Store Arb
/// Stores Arb found in table for later analysis
pub fn store_arb_cycle(cycle: &Vec<Edge>, arb_rate: f64, arb_surface: f64) -> Result<(), SmartError> {

    // Get unique assets
    let mut assets_hs: HashSet<String> = HashSet::new();
    for leg in cycle {
        assets_hs.insert(leg.from.clone());
        assets_hs.insert(leg.to.clone());
    }
    
    let timestamp: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let arb_length = cycle.len();
    let arb_assets: Vec<&String> = assets_hs.iter().collect();
    
    let asset_0 = if arb_assets.len() > 0 { Some(arb_assets[0].to_owned()) } else { None };
    let asset_1 = if arb_assets.len() > 1 { Some(arb_assets[1].to_owned()) } else { None };
    let asset_2 = if arb_assets.len() > 2 { Some(arb_assets[2].to_owned()) } else { None };
    let asset_3 = if arb_assets.len() > 3 { Some(arb_assets[3].to_owned()) } else { None };
    let asset_4 = if arb_assets.len() > 4 { Some(arb_assets[4].to_owned()) } else { None };
    let asset_5 = if arb_assets.len() > 5 { Some(arb_assets[5].to_owned()) } else { None };
    let asset_6 = if arb_assets.len() > 6 { Some(arb_assets[6].to_owned()) } else { None };
    let asset_7 = if arb_assets.len() > 7 { Some(arb_assets[7].to_owned()) } else { None };

    // Create an ArbData instance
    let data: ArbData = ArbData {
        timestamp,
        arb_length,
        arb_rate,
        arb_surface,
        asset_0,
        asset_1,
        asset_2,
        asset_3,
        asset_4,
        asset_5,
        asset_6,
        asset_7
    };

    // Create or append to a CSV file
    let file: std::fs::File = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("arbitrage_data.csv")?;

    // Write the data to the CSV file
    let mut wtr = Writer::from_writer(file);
    wtr.serialize(data)?;

    // Ensure all data is flushed to the file
    wtr.flush()?;

    Ok(())
}
