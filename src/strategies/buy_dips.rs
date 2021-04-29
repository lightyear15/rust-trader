use super::{candles, orders, wallets, Action, Strategy};

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

impl Strategy for BuyDips {
    fn on_new_candle(&mut self, wallet: &wallets::SimplePairWallet, history: &[candles::Candle]) -> Action {
        let avg = history.iter().fold(0.0, |a, b| a + b.low) / history.len() as f64;
        let current_price = history.last().expect("last candle").close;
        if current_price < avg {
            let order = orders::Order {
                symbol: self.sym.clone(),
                side: orders::Side::Buy,
                o_type: orders::Type::Market,
                volume: wallet.quote * 0.05 / current_price,
                expire: None,
                reference: 0,
            };
            println!("buying the dip at {} !!!", current_price);
            return Action::NewOrder(order);
        }
        Action::None
    }
    fn on_new_transaction(&mut self, wallet: &wallets::SimplePairWallet, tx: &orders::Transaction) -> Action {
        if tx.side == orders::Side::Sell {
            return Action::None;
        }
        let price = tx.avg_price * 1.05;
        let volume = tx.volume / 1.05;
        Action::NewOrder(orders::Order {
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
