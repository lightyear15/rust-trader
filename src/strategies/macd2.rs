use super::{confirmed_negative_cross, confirmed_positive_cross};
use crate::candles::Candle;
use crate::orders::{Order, Side, Transaction, Type};
use crate::strategies::{Action, SpotSinglePairStrategy};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use log::debug;
use std::collections::HashMap;
use std::collections::VecDeque;
use ta::indicators::MovingAverageConvergenceDivergence;
use ta::{Next, Reset};

const CAPITAL: f64 = 0.05;
#[derive(Clone)]
pub struct Macd2 {
    macd_1st_start: bool,
    slow_macd: MovingAverageConvergenceDivergence,
    fast_macd: MovingAverageConvergenceDivergence,
    exchange: String,
    sym: Symbol,
    time_frame: chrono::Duration,
    history_len: usize,
    slow_macd_trend: VecDeque<f64>,
    slow_signal_trend: VecDeque<f64>,
    slow_histo_trend: VecDeque<f64>,
    fast_macd_trend: VecDeque<f64>,
    fast_signal_trend: VecDeque<f64>,
    fast_histo_trend: VecDeque<f64>,
    last_tx: Option<Transaction>,
}

impl Macd2 {
    pub fn new(exchange: String, sym: Symbol, time_frame: chrono::Duration, settings: HashMap<String, String>) -> Self {
        let slow_long = settings
            .get("slow_long")
            .expect("no slow_long key")
            .parse::<usize>()
            .expect("slow_long must be usize");
        let slow_short = settings
            .get("slow_short")
            .expect("no slow_short key")
            .parse::<usize>()
            .expect("slow_short must be usize");
        let slow_smooth = settings
            .get("slow_smooth")
            .expect("no slow_smooth key")
            .parse::<usize>()
            .expect("slow_smooth must be usize");
        let fast_long = settings
            .get("fast_long")
            .expect("no fast_long key")
            .parse::<usize>()
            .expect("fast_long must be usize");
        let fast_short = settings
            .get("fast_short")
            .expect("no fast_short key")
            .parse::<usize>()
            .expect("fast_short must be usize");
        let fast_smooth = settings
            .get("fast_smooth")
            .expect("no fast_smooth key")
            .parse::<usize>()
            .expect("fast_smooth must be usize");
        debug!("tstamp,price,slow_macd,slow_signal,slow_histogram,fast_macd,fast_signal,fast_histogram,positive,negative");
        Self {
            exchange,
            sym,
            time_frame,
            slow_macd: MovingAverageConvergenceDivergence::new(slow_long, slow_short, slow_smooth).expect("in MACD::new()"),
            fast_macd: MovingAverageConvergenceDivergence::new(fast_long, fast_short, fast_smooth).expect("in MACD::new()"),
            macd_1st_start: true,
            history_len: slow_long,
            slow_macd_trend: VecDeque::new(),
            slow_signal_trend: VecDeque::new(),
            slow_histo_trend: VecDeque::new(),
            fast_macd_trend: VecDeque::new(),
            fast_signal_trend: VecDeque::new(),
            fast_histo_trend: VecDeque::new(),
            last_tx: None,
        }
    }
}

impl SpotSinglePairStrategy for Macd2 {
    fn name(&self) -> String {
        format!("macd2-{}-{}-{}", self.exchange, self.sym.to_string(), self.time_frame.to_string())
    }
    fn on_new_candle(&mut self, wallet: &SpotWallet, outstanding_orders: &[Order], history: &[Candle]) -> Action {
        let last_cnd = history.first().unwrap();
        let last_price = last_cnd.close;
        let tstamp = last_cnd.tstamp;
        if self.macd_1st_start {
            self.slow_macd.reset();
            self.fast_macd.reset();
            self.slow_macd_trend.clear();
            self.slow_signal_trend.clear();
            self.slow_histo_trend.clear();
            self.fast_macd_trend.clear();
            self.fast_signal_trend.clear();
            self.fast_histo_trend.clear();
            for c in history.iter().skip(1).rev() {
                let slow_res = self.slow_macd.next(c.close);
                let fast_res = self.fast_macd.next(c.close);
                self.slow_macd_trend.push_back(slow_res.macd);
                self.slow_signal_trend.push_back(slow_res.signal);
                self.slow_histo_trend.push_back(slow_res.histogram);
                self.fast_macd_trend.push_back(fast_res.macd);
                self.fast_signal_trend.push_back(fast_res.signal);
                self.fast_histo_trend.push_back(fast_res.histogram);
                debug!(
                    "'{}',{},{},{},{},{},{},{}",
                    c.tstamp.format("%Y-%m-%d %H:%M:%S"),
                    c.close,
                    slow_res.macd,
                    slow_res.signal,
                    slow_res.histogram,
                    fast_res.macd,
                    fast_res.signal,
                    fast_res.histogram
                );
            }
            while self.slow_histo_trend.len() > 5 {
                self.slow_histo_trend.pop_front();
            }
            while self.slow_macd_trend.len() > 5 {
                self.slow_macd_trend.pop_front();
            }
            while self.slow_signal_trend.len() > 5 {
                self.slow_signal_trend.pop_front();
            }
            while self.fast_histo_trend.len() > 5 {
                self.fast_histo_trend.pop_front();
            }
            while self.fast_macd_trend.len() > 5 {
                self.fast_macd_trend.pop_front();
            }
            while self.fast_signal_trend.len() > 5 {
                self.fast_signal_trend.pop_front();
            }
            self.macd_1st_start = false;
        }
        let slow_res = self.slow_macd.next(last_price);
        let fast_res = self.fast_macd.next(last_price);

        self.slow_macd_trend.pop_front();
        self.slow_macd_trend.push_back(slow_res.macd);
        self.slow_signal_trend.pop_front();
        self.slow_signal_trend.push_back(slow_res.signal);
        self.slow_histo_trend.pop_front();
        self.slow_histo_trend.push_back(slow_res.histogram);
        self.fast_macd_trend.pop_front();
        self.fast_macd_trend.push_back(fast_res.macd);
        self.fast_signal_trend.pop_front();
        self.fast_signal_trend.push_back(fast_res.signal);
        self.fast_histo_trend.pop_front();
        self.fast_histo_trend.push_back(fast_res.histogram);

        //let histos = self.histo_trend.make_contiguous();
        let buy_signal = confirmed_positive_cross(self.fast_histo_trend.make_contiguous(), 0.0);
        let sell_signal = confirmed_negative_cross(self.slow_histo_trend.make_contiguous(), 0.0);

        debug!(
            "'{}',{},{},{},{},{},{},{},{},{}",
            tstamp.format("%Y-%m-%d %H:%M:%S"),
            last_price,
            slow_res.macd,
            slow_res.signal,
            slow_res.histogram,
            fast_res.macd,
            fast_res.signal,
            fast_res.histogram,
            buy_signal,
            sell_signal,
        );

        // END COMPUTATION
        if !outstanding_orders.is_empty() {
            return Action::None;
        }

        let action = if sell_signal && self.last_tx.is_some() {
            let tx = self.last_tx.as_ref().unwrap();
            let volume = tx.volume.min(tx.avg_price * tx.volume / last_price);
            if *wallet.assets.get(&self.sym.base).expect("no base") < volume {
                panic!(
                    "volume {:?} in wallet {:?} tx {:?} - lastprice {:?}",
                    volume, wallet, tx, last_price
                );
            }
            let mut order = Order::new();
            order.exchange = self.exchange.clone();
            order.symbol = self.sym.clone();
            order.side = Side::Sell;
            order.o_type = Type::Limit(last_price);
            order.volume = volume;
            order.tx_ref = tx.order.id;
            Action::NewOrder(order)
        } else if buy_signal && self.last_tx.is_none() {
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
        } else {
            self.last_tx = None;
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
