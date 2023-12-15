use super::models::Mode;

/// TradeSearch: Will search for arbitrage and try to execute trade
/// TradeWss: Will listen to pairs via Websocket for arbitrage and try to execute trade
/// TradeWssWithSearch: Will both search and listen to Websocket for arbitrage, with trading from WSS findings
/// NoTradeSearch: Will search for arbitrage but not trade
/// NoTradeWss:  Will listen to Websocket for arbitrage but not trade
/// NoTradeBoth: Will both search and listen to Websocket for arbitrage but not trade
pub const MODE: Mode = Mode::NoTradeBoth(true); // bool = is save results

pub const ASSET_HOLDINGS: [&str; 2] = ["USDT", "BTC"];
pub const FIAT_EXCLUSION: [&str; 13] = ["ARS", "BIDR", "BRL", "EUR", "GBP", "IDRT", "NGN", "PLN", "RON", "RUB", "TRY", "UAH", "ZAR"];
pub const USD_BUDGET: f64 = 50.0; // USD equivalent in each asset holding
pub const MAX_SYMBOLS_WATCH: usize = 5; // Number of assets to scan for arbitrage on
pub const MAX_CYCLE_LENGTH: usize = 5;
pub const MIN_ARB_SEARCH: f64 = 1.000000001; // i.e. 1.005 for 0.01%
pub const MIN_ARB_THRESH: f64 = 1.000000015; // i.e. 1.015 for 1.5%
pub const UPDATE_SYMBOLS_SECONDS: u64 = 300; // Set at Minimum of 60 seconds in production for shared data symbol update (Only if Both Search and Wss)
