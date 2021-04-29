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
    fn on_new_candle(&mut self, wallet :&wallets::SimplePairWallet, history : &[candles::Candle]) -> Action;
    fn on_new_transaction(&mut self, wallet :&wallets::SimplePairWallet, tx: &orders::Transaction) -> Action;

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
