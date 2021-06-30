use crate::candles::Candle;
use crate::orders::{Order, Side, Transaction, Type};
use crate::strategies::{Action, SpotSinglePairStrategy};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use std::collections::HashMap;

#[derive(Clone)]
pub struct BuyDips {
    exchange: String,
    sym: Symbol,
    time_frame: chrono::Duration,
    // custom params
    ongoing_ops: usize,

    period: usize,
    gain_factor: f64,
    max_ops: usize,
}

impl BuyDips {
    pub fn new(exchange: String, sym: Symbol, time_frame: chrono::Duration, settings: HashMap<String, String>) -> Self {
        Self {
            exchange,
            sym,
            time_frame,
            ongoing_ops: 0,
            period: settings
                .get("period")
                .expect("missing period<usize>")
                .parse::<usize>()
                .expect("period must be <usize>"),
            gain_factor: settings
                .get("gain_factor")
                .expect("missing gain_factor<f64>")
                .parse::<f64>()
                .expect("gain_factor must be <f64>"),
            max_ops: settings
                .get("max_ops")
                .expect("missing max_ops<usize>")
                .parse::<usize>()
                .expect("max_ops must be <usize>"),
        }
    }
}

impl SpotSinglePairStrategy for BuyDips {
    fn name(&self) -> String {
        format!("BuyDips-{}-{}-{}", self.exchange, self.sym.to_string(), self.time_frame.to_string())
    }
    fn on_new_candle(&mut self, wallet: &SpotWallet, _outstanding_orders: &[Order], history: &[Candle]) -> Action {
        if self.ongoing_ops >= self.max_ops {
            return Action::None;
        }
        let (total, volume) = history.iter().fold((0.0, 0.0), |(total, volume), b| {
            let t = (b.low + b.high) / 2.0 * b.volume;
            (total + t, volume + b.volume)
        });
        let avg = total / volume;
        let current_price = history.first().expect("last candle").close;
        if current_price < (avg / (1.0 + self.gain_factor)) {
            let mut order = Order::new();
            order.exchange = self.exchange.clone();
            order.symbol = self.sym.clone();
            order.side = Side::Buy;
            order.o_type = Type::Market;
            order.volume = wallet.assets.get(&self.sym.quote).unwrap_or(&0.0) * self.gain_factor / current_price;
            order.expire = None;
            self.ongoing_ops += 1;
            return Action::NewOrder(order);
        }
        Action::None
    }
    fn on_new_transaction(&mut self, _outstanding_orders: &[Order], tx: &Transaction) -> Action {
        if tx.side == Side::Sell {
            self.ongoing_ops -= 1;
            return Action::None;
        }
        let price = tx.avg_price * (1.0 + self.gain_factor);
        let volume = tx.volume / (1.0 + self.gain_factor);
        let mut order = Order::new();
        order.exchange = self.exchange.clone();
        order.symbol = self.sym.clone();
        order.side = Side::Sell;
        order.o_type = Type::Limit(price);
        order.volume = volume;
        order.expire = None;
        order.tx_ref = tx.order.id;
        Action::NewOrder(order)
    }
    fn get_candles_history_size(&self) -> usize {
        self.period
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
