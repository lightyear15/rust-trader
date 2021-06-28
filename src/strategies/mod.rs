use crate::candles::Candle;
use crate::error::Error;
use crate::orders::{Order, Side, Transaction, Type};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use std::collections::HashMap;

pub mod buy_dips;
pub mod sample;
pub mod macd1;
pub use buy_dips::BuyDips;
pub use sample::Sample;
pub use macd1::Macd1;

#[derive(Debug)]
pub enum Action {
    None,
    NewOrder(Order),
    CancelOrder(u32),
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
        _ => Err(Error::ErrNotFound(format!("can't find strategy {}", strategy))),
    }
}

#[derive(Debug)]
pub struct Statistics {
    pub orders: usize,
    pub transactions: usize,
    pub canceled_orders: usize,
    pub balance_start: f64,
    pub balance: f64,
    pub lowest_balance: f64,
    pub highest_balance: f64,
}

impl Statistics {
    pub fn new(balance_start: f64) -> Self {
        Self {
            orders: 0,
            transactions: 0,
            canceled_orders: 0,
            balance_start,
            balance: balance_start,
            lowest_balance: balance_start,
            highest_balance: balance_start,
        }
    }
    pub fn update_with_last_prices(&mut self, wallet: &SpotWallet, prices: &HashMap<String, f64>) {
        let balance = wallet.assets.iter().fold(0.0, |balance, (sym, price)| {
            balance + prices.get(sym).expect("coin in wallet missing from price list") * price
        });
        self.balance = balance;
        if balance < self.lowest_balance {
            self.lowest_balance = balance;
        }
        if balance > self.highest_balance {
            self.highest_balance = balance
        }
    }
    pub fn update_with_transaction(&mut self, _tx: &Transaction) {
        self.transactions += 1;
    }
    pub fn update_with_order(&mut self, _ord: &Order) {
        self.orders += 1;
    }
    pub fn update_with_expired_order(&mut self, _ord: &Order) {
        self.canceled_orders += 1;
    }
}
