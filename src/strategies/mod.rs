use super::{ orders, Error, candles, wallets};

pub mod sample;
pub mod buy_dips;
pub use sample::Sample;
pub use buy_dips::BuyDips;

#[derive(Debug)]
pub enum Action {
    None,
    NewOrder(orders::Order),
}

// a 1-symbol strategy
pub trait Strategy {
    fn on_new_candle(&mut self, wallet :&wallets::SpotPairWallet, history : &[candles::Candle]) -> Action;
    fn on_new_transaction(&mut self, wallet :&wallets::SpotPairWallet, tx: &orders::Transaction) -> Action;

    fn get_candles_history_size(&self) -> usize;
    fn exchange(&self) -> &str;
    fn symbol(&self) -> &str;
    fn time_frame(&self) -> &chrono::Duration;
}

pub fn create(strategy: &str, exch: String, sym :String, time_frame :chrono::Duration) -> Result<Box<dyn Strategy>, Error> {
    match strategy {
        "sample" => Ok(Box::new(Sample::new(exch, sym, time_frame))),
        "buyDips" => Ok(Box::new(BuyDips::new(exch, sym, time_frame))),
        _ => Err(Error::ErrNotFound),
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
    pub fn update_with_last_price(&mut self, wallet: &wallets::SpotPairWallet, pr: f64) {
        let balance = wallet.quote + wallet.base * pr;
        self.balance = balance;
        if balance < self.lowest_balance {
            self.lowest_balance = balance;
        }
        if balance > self.highest_balance {
            self.highest_balance = balance
        }
    }
}
