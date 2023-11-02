
const MARKET_ETH_PERP: &str = "Fgh9JSZ2qfSjCw9RPJ85W2xbihsp2muLvfRztzoVR7f1";
const MARKET_SOL_PERP: &str = "ESdnpnNLgTkBCZRuTJkZLi5wKEZ2z47SG3PJrhundSQ2";

const MINT_ADDRESS_USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
const MINT_ADDRESS_ETH: &str = "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs";
const MINT_ADDRESS_SOL: &str = "So11111111111111111111111111111111111111112";


// ETH
// 1 bps = 0.0001 = 0.01%
// pub const PROFIT_THRESHOLD: f64 = 0.005; // 50 bps
// pub const BASE_QTY_UI: f64 = 0.001;
// pub const PERP_ALLOWANCE_THRESHOLD_BASE_UI: f64 = 0.02;
// pub const BASE_DECIMALS: u8 = 8;
// pub const MARKET: &str = MARKET_ETH_PERP;
// pub const PERP_MARKET_NAME: &'static str = "ETH-PERP";
// pub const TOKEN_NAME: &'static str = "ETH (Portal)";
// pub const MINT_ADDRESS_INPUT: &str = MINT_ADDRESS_USDC;
// pub const MINT_ADDRESS_OUTPUT: &str = MINT_ADDRESS_ETH;

// SOL
// 1 bps = 0.0001 = 0.01%
pub const PROFIT_THRESHOLD: f64 = 0.002; // 20 bps
pub const BASE_QTY_UI: f64 = 0.1; // 2 USD
pub const PERP_ALLOWANCE_THRESHOLD_BASE_UI: f64 = 1.1;
pub const BASE_DECIMALS: u8 = 9;
pub const MARKET: &str = MARKET_SOL_PERP;
pub const PERP_MARKET_NAME: &'static str = "SOL-PERP";
pub const TOKEN_NAME: &'static str = "SOL";
pub const MINT_ADDRESS_INPUT: &str = MINT_ADDRESS_USDC;
pub const MINT_ADDRESS_OUTPUT: &str = MINT_ADDRESS_SOL;
