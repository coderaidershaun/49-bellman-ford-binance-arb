use super::arb_execution::execute_arbitrage_cycle;
use super::constants::{ASSET_HOLDINGS, USD_BUDGET, MIN_ARB_THRESH, MAX_CYCLE_LENGTH, MODE};
use super::bellmanford::Edge;
use super::exchanges::binance::Binance;
use super::models::{ArbData, Direction, Mode, SmartError};
use super::traits::{ApiCalls, BellmanFordEx, ExchangeData};

use csv::WriterBuilder;
use futures::future::join_all;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::fs::OpenOptions;
use std::io::Write;
use std::collections::HashSet;

/// Calculate Weighted Average Price
/// Calculates the depth of the orderbook to get a real rate
fn calculate_weighted_average_price(
    orderbook: &Vec<(f64, f64)>,
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

    // Guard: Ensure quantity is not zero
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

/// Calculate Arbitrage
/// Calculates arbitrage given relevant inputs and orderbooks
fn calculate_arbitrage(
    orderbooks: &Vec<Vec<(f64, f64)>>,
    symbols: &Vec<String>,
    directions: &Vec<Direction>,
    budget: f64
) -> Option<f64> {

    // Initialize
    let mut real_rate = 1.0;
    let mut amount_in = budget;

    // Perform arbitrage calculation
    for i in 0..symbols.len() {
        let direction = &directions[i];
        let orderbook = &orderbooks[i];

        // Calculate Average Price and quantity out
        let trade_res: Option<(f64, f64, f64)> = calculate_weighted_average_price(orderbook, amount_in, &direction);

        // Extract values
        let (weighted_price, trade_qty) = match trade_res {
            Some((wp, _, qty)) => (wp, qty),
            None => return None
        };

        // Update amount in for next leg budget amount
        amount_in = trade_qty;

        // Calculate Real Rate
        match direction {
            Direction::Forward => real_rate *= weighted_price,
            Direction::Reverse => real_rate *= 1.0 / weighted_price,
        }
    }

    // Return results
    Some(real_rate)
}


/// Validate Arbitrage Cycle
/// Validates arbitrage cycle has enough depth
pub async fn validate_arbitrage_cycle<T: BellmanFordEx>(cycle: &Vec<Edge>, exchange: &T) 
    -> Option<(f64, Vec<String>, Vec<Direction>, f64)> 
where T: BellmanFordEx + ExchangeData + ApiCalls 
{

    // Guard: Ensure cycle
    if cycle.len() == 0 { return None };

    // Guard: Ensure asset holding
    let from = cycle[0].from.as_str();
    if !ASSET_HOLDINGS.contains(&from) {
        // eprintln!("Asset not in holding: {}", from);
        return None
    }

    // Get starting budget
    let budget = match from {
        "BTC" => USD_BUDGET / exchange.prices().get("BTCUSDT").expect("Expected price for BTCUSDT").to_owned(),
        "ETH" => USD_BUDGET / exchange.prices().get("ETHUSDT").expect("Expected price for ETHUSDT").to_owned(),
        "BNB" => USD_BUDGET / exchange.prices().get("BNBUSDT").expect("Expected price for BNBUSDT").to_owned(),
        "LINK" => USD_BUDGET / exchange.prices().get("LINKUSDT").expect("Expected price for LINKUSDT").to_owned(),
        "USDT" => USD_BUDGET,
        "BUSD" => USD_BUDGET,
        "USDC" => USD_BUDGET,
        _ => {
            eprintln!("{} not recognised as meaningful starting point", from);
            return None
        }
    };

    // Initialize
    let mut symbols: Vec<String> = vec![];
    let mut directions: Vec<Direction> = vec![];
    let mut orderbooks: Vec<Vec<(f64, f64)>> = vec![];

    // Extract info for parallel async orderbook fetching
    for leg in cycle {
        let symbol_1 = format!("{}{}", leg.to, leg.from);
        let symbol_2 = format!("{}{}", leg.from, leg.to);
        let symbol = if exchange.symbols().contains_key(symbol_1.as_str()) { symbol_1 } else { symbol_2 };
        let direction = if symbol.starts_with(leg.from.as_str()) { 
            Direction::Forward // Uses Asks orderbooks
        } else { 
            Direction::Reverse // Uses Bids orderbooks
        };

        symbols.push(symbol);
        directions.push(direction);
    }

    // Build futures for orderbook asyncronous extraction
    let futures: Vec<_> = symbols.iter().zip(directions.iter())
        .map(|(symbol, direction)| exchange.get_orderbook_depth(symbol.as_str(), direction))
        .collect();

    // Call api for orderbooks
    let results: Vec<Result<Vec<(f64, f64)>, SmartError>> = join_all(futures).await;

    // Guard: Ensure orderbook results
    for result in results {
        match result {
            Ok(book) => orderbooks.push(book),
            Err(e) => {
                eprintln!("Error fetching order book: {:?}", e);
                return None
            },
        }
    }

    // Calculate Arbitrage
    let Some(real_rate) = calculate_arbitrage(&orderbooks, &symbols, &directions, budget) else { return None };

    // Return result
    Some((real_rate, symbols, directions, budget))
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

    // Save data
    let current_dir = std::env::current_dir()?;
    let base_path = current_dir.to_str().unwrap();
    let file_path = format!("{}/arbitrage_data.csv", base_path);
    let file_exists = std::path::Path::new(file_path.as_str()).exists();
    let file: std::fs::File = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file_path)?;
    let mut wtr = WriterBuilder::new()
        .has_headers(!file_exists)
        .from_writer(file);
    wtr.serialize(data)?;
    wtr.flush()?;

    Ok(())
}

/// Calculate Arbitrage Surface Rate
/// Calculates the surface rate of an arbitrage opportunity
pub fn calculate_arbitrage_surface_rate(cycle: &Vec<Edge>) -> f64 {
    cycle.iter().fold(1.0, |acc, edge| acc * f64::exp(-edge.weight)) - 1.0
}

/// Arb Scanner
/// Scans and executes (if requested) for arbitrage
pub async fn arb_scanner() -> Result<(), SmartError> {
    println!("scanning for abitrage...");

    loop {
        std::thread::sleep(Duration::from_millis(50));

        let exchange = Binance::new().await;
        let cycles = exchange.run_bellman_ford_multi();
        for cycle in cycles {

            print!("\ranalyzing cycle of length {}...", cycle.len());
            std::io::stdout().flush().unwrap();

            // Guard: Ensure cycle length
            if cycle.len() > MAX_CYCLE_LENGTH { continue; }

            let arb_opt = validate_arbitrage_cycle(&cycle, &exchange).await;
            if let Some((arb_rate, symbols, directions, budget)) = arb_opt {

                // Guard: Ensure arb rate
                dbg!(&arb_rate);
                if arb_rate < MIN_ARB_THRESH { continue; }

                // Guard: Ensure from asset is ipart of Holding Assets
                let from_asset = cycle[0].from.as_str();
                if !ASSET_HOLDINGS.contains(&from_asset) { panic!("Error: Asset holdings do not include symbol") }
                
                // Execute and get store trigger
                let (is_store, is_trade) = match MODE {
                Mode::Searcher(is_store, is_trade) => (is_store, is_trade),
                _ => (false, false)
                };

                // !!! PLACE TRADE !!!
                if is_trade {
                    println!("\nPlacing trade...");
                    let result = execute_arbitrage_cycle(
                        budget,
                        &cycle,
                        &symbols,
                        &directions,
                        &exchange
                    ).await;
                    
                    if let Err(e) = result {
                        panic!("Failed to place trade: {:?}", e);
                    }
                }

                // Store Result
                if is_store {
                    let arb_surface: f64 = calculate_arbitrage_surface_rate(&cycle);
                    let _: () = store_arb_cycle(&cycle, arb_rate, arb_surface)?;
                }
            }
        }
    }
}

 #[cfg(test)]
 mod test {
    use super::*;

    #[tokio::test]
    async fn it_calculates_weighted_price_metrics() {
        std::thread::sleep(Duration::from_millis(100));
        let exchange = Binance::new().await;
        let symbol: &str = "BTCUSDT";
        let budget: f64 = 50.0; // USDT
        let direction = Direction::Reverse;
        let orderbook = exchange.get_orderbook_depth(symbol, &direction).await.unwrap();
        let result = calculate_weighted_average_price(&orderbook, budget, &direction);
        match result {
            Some((weighted_average_price, total_cost, total_quantity)) => {
                dbg!(&weighted_average_price);
                dbg!(&total_cost);
                dbg!(&total_quantity);

                assert!(weighted_average_price > 0.0);
                assert!(total_cost > 0.0);
                assert!(total_quantity > 0.0);
            },
            None => panic!("No weighted average price metrics")
        };
    }

    #[tokio::test]
    async fn it_validates_arbitrage_cycle() {
        std::thread::sleep(Duration::from_millis(100));
        let exchange = Binance::new().await;
        let cycle = exchange.run_bellman_ford_single().unwrap();
        let result = validate_arbitrage_cycle(&cycle, &exchange).await;
        match result {
            Some((real_rate, symbols, directions, budget)) => {
                assert!(real_rate > 0.0);
                assert!(symbols.len() > 0);
                assert!(directions.len() > 0);
                assert!(budget > 0.0);
            },
            None => println!("No real arbitrage opportunity")
        };
    }

    #[tokio::test]
    async fn it_stores_an_arb_cycle() {
        std::thread::sleep(Duration::from_millis(100));
        let exchange = Binance::new().await;
        let cycle = exchange.run_bellman_ford_single().unwrap();
        let _result: () = store_arb_cycle(&cycle, 1.1, 0.1).unwrap();
    }
 }
