use crate::wallets::SpotWallet;
use crate::candles::Candle;
use crate::orders::{Order, Transaction, Side, Type};
use crate::strategies::{SpotSinglePairStrategy, Action};
use crate::symbol::Symbol;

#[derive(Clone)]
pub struct BuyDips {
    exchange: String,
    sym: Symbol,
    time_frame: chrono::Duration,
}

impl BuyDips {
    pub fn new(exchange: String, sym: Symbol, time_frame: chrono::Duration) -> Self {
        Self {
            exchange,
            sym,
            time_frame,
        }
    }
}

impl SpotSinglePairStrategy for BuyDips {
    fn name(&self) -> String {
        format!("BuyDips-{}-{}-{}",self.exchange, self.sym.to_string(), self.time_frame.to_string())
    }
    fn on_new_candle(&mut self, wallet :&SpotWallet, _outstanding_orders: &[Order], history : &[Candle]) -> Action{
        let avg = history.iter().fold(0.0, |a, b| a + b.low) / history.len() as f64;
        let current_price = history.last().expect("last candle").close;
        if current_price < avg {
            let order = Order {
                exchange: self.exchange.clone(),
                symbol: self.sym.symbol,
                side: Side::Buy,
                o_type: Type::Market,
                volume: wallet.assets.get(&self.sym.quote).unwrap_or(&0.0) * 0.05 / current_price,
                expire: None,
                reference: 0,
            };
            return Action::NewOrder(order);
        }
        Action::None
    }
    fn on_new_transaction(&mut self, _wallet :&SpotWallet, _outstanding_orders: &[Order], tx: &Transaction) -> Action{
        if tx.side == Side::Sell {
            return Action::None;
        }
        let price = tx.avg_price * 1.05;
        let volume = tx.volume / 1.05;
        Action::NewOrder(Order {
            exchange: self.exchange.clone(),
            symbol: self.sym.symbol.clone(),
            side: Side::Sell,
            o_type: Type::Limit(price),
            volume,
            expire: None,
            reference: 0,
        })
    }
    fn get_candles_history_size(&self) -> usize {
        5
    }

    fn exchange(&self) -> &str {
        &self.exchange
    }
    fn symbol(&self) -> &str {
        &self.sym.symbol
    }
    fn time_frame(&self) -> &chrono::Duration {
        &self.time_frame
    }
}
