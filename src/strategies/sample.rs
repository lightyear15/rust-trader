use crate::wallets::SpotPairWallet;
use crate::candles::Candle;
use crate::orders::{Order, Transaction};
use crate::strategies::{SpotSinglePairStrategy, Action};

#[derive(Clone)]
pub struct Sample {
    index: u32,
    exchange: String,
    sym: String,
    time_frame: chrono::Duration,
}

impl Sample {
    pub fn new(exchange :String, sym:String, time_frame:chrono::Duration) -> Self {
        Self{exchange, sym, time_frame, index: 0}
    }
}

impl SpotSinglePairStrategy for Sample {
    fn name(&self) -> String {
        format!("Sample-{}-{}-{}",self.exchange, self.sym, self.time_frame.to_string())
    }
    fn on_new_candle(&mut self, _wallet :&SpotPairWallet, _outstanding_orders: &[Order], history : &[Candle]) -> Action{
        println!("at iteration {}", self.index);
        for c in history {
            println!("{:?}", c);
        }
        self.index += 1;
        Action::None
    }
    fn on_new_transaction(&mut self, _wallet :&SpotPairWallet, _outstanding_orders: &[Order], _tx: &Transaction) -> Action{
        Action::None
    }

    fn get_candles_history_size(&self) -> usize {
        3
    }

    fn exchange(&self) -> &str {
        &self.exchange
    }
    fn symbol(&self) -> &str {
        &self.sym
    }
    fn time_frame(&self) -> &chrono::Duration {
        &self.time_frame
    }
}
