pub mod candle_buffer;
pub mod orderbook;
pub mod trade_stream;

// Re-export the Candle struct for convenient access (e.g. `use crate::market_data::Candle`).
pub use candle_buffer::{Candle, CandleBuffer, CandleKey};
pub use orderbook::OrderBookManager;
pub use trade_stream::TradeStreamProcessor;
