use super::{Action, SpotSinglePairStrategy};
use super::{ orders, Candle, SpotPairWallet};
use super::orders::{Order, Transaction};

#[derive(Clone)]
pub struct BuyDips {
    exchange: String,
    sym: String,
    time_frame: chrono::Duration,
}

impl BuyDips {
    pub fn new(exchange: String, sym: String, time_frame: chrono::Duration) -> Self {
        Self {
            exchange,
            sym,
            time_frame,
        }
    }
}

impl SpotSinglePairStrategy for BuyDips {
    fn on_new_candle(&mut self, wallet :&SpotPairWallet, _outstanding_orders: &[Order], history : &[Candle]) -> Action{
        let avg = history.iter().fold(0.0, |a, b| a + b.low) / history.len() as f64;
        let current_price = history.last().expect("last candle").close;
        if current_price < avg {
            let order = orders::Order {
                exchange: self.exchange.clone(),
                symbol: self.sym.clone(),
                side: orders::Side::Buy,
                o_type: orders::Type::Market,
                volume: wallet.quote * 0.05 / current_price,
                expire: None,
                reference: 0,
            };
            return Action::NewOrder(order);
        }
        Action::None
    }
    fn on_new_transaction(&mut self, _wallet :&SpotPairWallet, _outstanding_orders: &[Order], tx: &Transaction) -> Action{
        if tx.side == orders::Side::Sell {
            return Action::None;
        }
        let price = tx.avg_price * 1.05;
        let volume = tx.volume / 1.05;
        Action::NewOrder(orders::Order {
            exchange: self.exchange.clone(),
            symbol: self.sym.clone(),
            side: orders::Side::Sell,
            o_type: orders::Type::Limit(price),
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
        &self.sym
    }
    fn time_frame(&self) -> &chrono::Duration {
        &self.time_frame
    }
}
