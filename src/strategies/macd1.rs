use crate::candles::Candle;
use crate::orders::{Order, Side, Transaction, Type};
use crate::strategies::{Action, SpotSinglePairStrategy};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use std::collections::HashMap;
use std::collections::VecDeque;
use ta::indicators::MovingAverageConvergenceDivergence;
use ta::{Next, Reset};

const CAPITAL: f64 = 0.05;
#[derive(Clone)]
pub struct Macd1 {
    macd_1st_start: bool,
    macd: MovingAverageConvergenceDivergence,
    exchange: String,
    sym: Symbol,
    time_frame: chrono::Duration,
    history_len: usize,
    macd_trend: VecDeque<f64>,
    signal_trend: VecDeque<f64>,
    histo_trend: VecDeque<f64>,
    last_tx: Option<Transaction>,
}

impl Macd1 {
    pub fn new(exchange: String, sym: Symbol, time_frame: chrono::Duration, settings: HashMap<String, String>) -> Self {
        let long = settings
            .get("long")
            .expect("no long key")
            .parse::<usize>()
            .expect("long must be usize");
        let short = settings
            .get("short")
            .expect("no short key")
            .parse::<usize>()
            .expect("short must be usize");
        let smooth = settings
            .get("smooth")
            .expect("no smooth key")
            .parse::<usize>()
            .expect("smooth must be usize");
        Self {
            exchange,
            sym,
            time_frame,
            macd: MovingAverageConvergenceDivergence::new(long, short, smooth).expect("in MACD::new()"),
            macd_1st_start: true,
            history_len: long,
            macd_trend: VecDeque::new(),
            signal_trend: VecDeque::new(),
            histo_trend: VecDeque::new(),
            last_tx: None,
        }
    }
}

impl SpotSinglePairStrategy for Macd1 {
    fn name(&self) -> String {
        format!("Macd1-{}-{}-{}", self.exchange, self.sym.to_string(), self.time_frame.to_string())
    }
    fn on_new_candle(&mut self, wallet: &SpotWallet, outstanding_orders: &[Order], history: &[Candle]) -> Action {
        let last_price = history.first().unwrap().close;
        if self.macd_1st_start {
            self.macd.reset();
            for c in history.iter().skip(1).rev() {
                let res = self.macd.next(c.close);
                self.macd_trend.push_back(res.macd);
                self.signal_trend.push_back(res.signal);
                self.histo_trend.push_back(res.histogram);
            }
            self.macd_1st_start = false;
        }
        let res = self.macd.next(last_price);

        self.macd_trend.pop_front();
        self.macd_trend.push_back(res.macd);
        self.signal_trend.pop_front();
        self.signal_trend.push_back(res.signal);
        self.histo_trend.pop_front();
        self.histo_trend.push_back(res.histogram);

        if outstanding_orders.is_empty() {
            return Action::None;
        }

        let action = if let Some(ref tx) = self.last_tx {
            if confirmed_negative_cross(self.histo_trend.make_contiguous()) {
                let volume = tx.avg_price * tx.volume / last_price;
                let mut order = Order::new();
                order.exchange = self.exchange.clone();
                order.symbol = self.sym.clone();
                order.side = Side::Sell;
                order.o_type = Type::Limit(last_price);
                order.volume = volume;
                Action::NewOrder(order)
            } else {
                Action::None
            }
        } else if confirmed_positive_cross(self.histo_trend.make_contiguous()) {
            let volume = wallet.assets.get(&self.sym.quote).unwrap_or(&0.0) * CAPITAL / last_price;
            let mut order = Order::new();
            order.exchange = self.exchange.clone();
            order.symbol = self.sym.clone();
            order.side = Side::Buy;
            order.o_type = Type::Market;
            order.volume = volume;
            Action::NewOrder(order)
        } else {
            Action::None
        };

        action
    }
    fn on_new_transaction(&mut self, _outstanding_orders: &[Order], tx: &Transaction) -> Action {
        if matches!(tx.side, Side::Buy) {
            self.last_tx = Some(tx.clone());
        }
        Action::None
    }

    fn get_candles_history_size(&self) -> usize {
        self.history_len
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

// all values negative, last is positive
fn positive_cross(values: &[f64]) -> bool {
    values.iter().take(values.len() - 1).all(|value| value.is_sign_negative()) && values.last().unwrap().is_sign_positive()
}
// all values negative, last 2 are positive
fn confirmed_positive_cross(values: &[f64]) -> bool {
    values.iter().take(values.len() - 2).all(|value| value.is_sign_negative())
        && values.iter().skip(values.len() - 2).all(|value| value.is_sign_positive())
}

// all values positive, last is negative
fn negative_cross(values: &[f64]) -> bool {
    values.iter().take(values.len() - 1).all(|value| value.is_sign_positive()) && values.last().unwrap().is_sign_negative()
}
// all values positive, last 2 are negative
fn confirmed_negative_cross(values: &[f64]) -> bool {
    values.iter().take(values.len() - 2).all(|value| value.is_sign_positive())
        && values.iter().skip(values.len() - 2).all(|value| value.is_sign_negative())
}
