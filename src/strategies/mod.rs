use super::{ orders, Error, candles::Candle, wallets::SpotPairWallet};
use super::orders::{Order, Transaction};

pub mod sample;
pub mod buy_dips;
pub use sample::Sample;
pub use buy_dips::BuyDips;

#[derive(Debug)]
pub enum Action {
    None,
    NewOrder(Order),
    CancelOrder(i32),
}

// a 1-symbol strategy
pub trait SpotSinglePairStrategy {
    fn on_new_candle(&mut self, wallet :&SpotPairWallet, outstanding_orders: &[Order], history : &[Candle]) -> Action;
    fn on_new_transaction(&mut self, wallet :&SpotPairWallet, outstanding_orders: &[Order], tx: &Transaction) -> Action;

    fn get_candles_history_size(&self) -> usize;
    fn exchange(&self) -> &str;
    fn symbol(&self) -> &str;
    fn time_frame(&self) -> &chrono::Duration;

    fn new_order(&self, refer :Option<i32>) -> orders::Order {
        orders::Order {
            o_type: orders::Type::Market,
            side: orders::Side::Buy,
            volume : 0.0,

            expire: None,
            exchange : String::from(self.exchange()),
            symbol : String::from(self.symbol()),
            reference: refer.unwrap_or_else(rand::random::<i32>),
        }
    }
}

pub fn create(strategy: &str, exch: String, sym :String, time_frame :chrono::Duration) -> Result<Box<dyn SpotSinglePairStrategy>, Error> {
    match strategy {
        "sample" => Ok(Box::new(Sample::new(exch, sym, time_frame))),
        "buyDips" => Ok(Box::new(BuyDips::new(exch, sym, time_frame))),
        _ => Err(Error::ErrNotFound(format!("can't find strategy {}", strategy))),
    }
}


#[derive(Debug)]
pub struct Statistics {
    pub orders : usize,
    pub transactions : usize,
    pub canceled_orders: usize,
    pub balance_start: f64,
    pub balance: f64,
    pub lowest_balance: f64,
    pub highest_balance : f64,
}

impl Statistics {
    pub fn new(balance_start: f64) -> Self {
        Self {
            orders: 0,
            transactions: 0,
            canceled_orders: 0,
            balance_start,
            balance: balance_start,
            lowest_balance : balance_start,
            highest_balance: balance_start
        }
    }
    pub fn update_with_last_price(&mut self, wallet: &SpotPairWallet, pr: f64) {
        let balance = wallet.quote + wallet.base * pr;
        self.balance = balance;
        if balance < self.lowest_balance {
            self.lowest_balance = balance;
        }
        if balance > self.highest_balance {
            self.highest_balance = balance
        }
    }
    pub fn update_with_transaction(&mut self, _tx: &orders::Transaction) {
        self.transactions += 1;
    }
    pub fn update_with_order(&mut self, _ord: &orders::Order) {
        self.orders += 1;
    }
    pub fn update_with_expired_order(&mut self, _ord: &orders::Order) {
        self.canceled_orders += 1;
    }
}
