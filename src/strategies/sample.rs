use crate::candles::Candle;
use crate::orders::{Order, Transaction};
use crate::strategies::{Action, SpotSinglePairStrategy};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;

#[derive(Clone)]
pub struct Sample {
    index: u32,
    exchange: String,
    sym: Symbol,
    time_frame: chrono::Duration,
}

impl Sample {
    pub fn new(exchange: String, sym: Symbol, time_frame: chrono::Duration) -> Self {
        Self {
            exchange,
            sym,
            time_frame,
            index: 0,
        }
    }
}

impl SpotSinglePairStrategy for Sample {
    fn name(&self) -> String {
        format!("Sample-{}-{}-{}", self.exchange, self.sym.to_string(), self.time_frame.to_string())
    }
    fn on_new_candle(&mut self, _wallet: &SpotWallet, _outstanding_orders: &[Order], history: &[Candle]) -> Action {
        println!("at iteration {}", self.index);
        for c in history {
            println!("{:?}", c);
        }
        self.index += 1;
        Action::None
    }
    fn on_new_transaction(&mut self, _outstanding_orders: &[Order], _tx: &Transaction) -> Action {
        Action::None
    }

    fn get_candles_history_size(&self) -> usize {
        3
    }

    fn exchange(&self) -> &str {
        &self.exchange
    }
    fn symbol(&self) -> &Symbol {
        &self.sym
    }
    fn time_frame(&self) -> &chrono::Duration {
        &self.time_frame
    }
}
