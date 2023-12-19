use super::models::Mode;

/// Searcher: Trades entire pool of assets
/// Listener: Listens to and trades specific pool of assets
pub const MODE: Mode = Mode::Searcher(true, false); // bool = is save results, bool = is trade

pub const ASSET_HOLDINGS: [&str; 2] = ["USDT", "BTC"];
pub const FIAT_EXCLUSION: [&str; 13] = ["ARS", "BIDR", "BRL", "EUR", "GBP", "IDRT", "NGN", "PLN", "RON", "RUB", "TRY", "UAH", "ZAR"];
pub const USD_BUDGET: f64 = 25.0; // USD equivalent in each asset holding
pub const MAX_CYCLE_LENGTH: usize = 5;
pub const MIN_ARB_THRESH: f64 = 1.015; // i.e. 1.015 for 1.5%
