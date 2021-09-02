use crate::candles::Candle;
use crate::error::Error;
use crate::orders::{Order, Side, Transaction, Type};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use std::collections::HashMap;

pub mod buy_dips;
pub mod sample;
pub mod macd1;
pub mod macd2;
pub use buy_dips::BuyDips;
pub use sample::Sample;
pub use macd1::Macd1;
pub use macd2::Macd2;

#[derive(Debug)]
pub enum Action {
    None,
    NewOrder(Order),
    CancelOrder(String, u32),
}

// a 1-symbol strategy
pub trait SpotSinglePairStrategy {
    fn name(&self) -> String;
    // candles are handed in reverse order, i.e. last candle is first item in the slice
    fn on_new_candle(&mut self, wallet: &SpotWallet, outstanding_orders: &[Order], history: &[Candle]) -> Action;
    fn on_new_transaction(&mut self, outstanding_orders: &[Order], tx: &Transaction) -> Action;

    fn get_candles_history_size(&self) -> usize;
    fn exchange(&self) -> &str;
    fn symbol(&self) -> &Symbol;
    fn time_frame(&self) -> &chrono::Duration;
}

pub fn create(
    strategy: &str,
    exch: String,
    sym: Symbol,
    time_frame: chrono::Duration,
    settings: HashMap<String, String>,
) -> Result<Box<dyn SpotSinglePairStrategy>, Error> {
    match strategy {
        "sample" => Ok(Box::new(Sample::new(exch, sym, time_frame))),
        "buyDips" => Ok(Box::new(BuyDips::new(exch, sym, time_frame, settings))),
        "macd1" => Ok(Box::new(Macd1::new(exch, sym, time_frame, settings))),
        "macd2" => Ok(Box::new(Macd2::new(exch, sym, time_frame, settings))),
        _ => Err(Error::ErrNotFound(format!("can't find strategy {}", strategy))),
    }
}

// all values <= threshold, last is > threshold
fn positive_cross(values: &[f64], threshold: f64) -> bool {
    values.iter().take(values.len() - 1).all(|value| *value <= threshold) && *values.last().unwrap() > threshold
}
// all values <= threshold, last 2 are > threshold
fn confirmed_positive_cross(values: &[f64], threshold: f64) -> bool {
    values.iter().take(values.len() - 2).all(|value| *value <= threshold)
        && values.iter().skip(values.len() - 2).all(|value| *value > threshold)
}

// all values >= threshold, last is < threshold
fn negative_cross(values: &[f64], threshold: f64) -> bool {
    values.iter().take(values.len() - 1).all(|value| *value >= threshold) && *values.last().unwrap() < threshold
}
// all values >= threshold, last 2 are < threshold
fn confirmed_negative_cross(values: &[f64], threshold: f64) -> bool {
    values.iter().take(values.len() - 2).all(|value| *value >= threshold)
        && values.iter().skip(values.len() - 2).all(|value| *value < threshold)
}
