use crate::candles::Candle;
use crate::orders::{Order, Side, Transaction, Type};
use crate::strategies::{Action, SpotSinglePairStrategy};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use log::debug;
use std::collections::{VecDeque, HashMap};
use ta::indicators::MovingAverageConvergenceDivergence;
use ta::{Next, Reset};
use super::positive_cross;

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
        debug!("tstamp,price,macd,signal,histogram,buy,sell");
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
        let last_cnd = history.first().unwrap();
        let last_price = last_cnd.close;
        let tstamp = last_cnd.tstamp;
        if self.macd_1st_start {
            self.macd.reset();
            self.macd_trend.clear();
            self.signal_trend.clear();
            self.histo_trend.clear();
            for c in history.iter().skip(1).rev() {
                let res = self.macd.next(c.close);
                self.macd_trend.push_back(res.macd);
                self.signal_trend.push_back(res.signal);
                self.histo_trend.push_back(res.histogram);
                debug!(
                    "'{}',{},{},{},{}",
                    c.tstamp.format("%Y-%m-%d %H:%M:%S"),
                    c.close,
                    res.macd,
                    res.signal,
                    res.histogram
                    );
            }
            while self.histo_trend.len() > 5 {
                self.histo_trend.pop_front();
            }
            while self.macd_trend.len() > 5 {
                self.macd_trend.pop_front();
            }
            while self.signal_trend.len() > 5 {
                self.signal_trend.pop_front();
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

        let histos = self.histo_trend.make_contiguous();
        let buy_signal = positive_cross(histos, 0.0);
        let sell_signal = histos.last().unwrap() < histos.get(histos.len() - 2).unwrap();

        debug!(
            "'{}',{},{},{},{},{},{}",
            tstamp.format("%Y-%m-%d %H:%M:%S"),
            last_price,
            res.macd,
            res.signal,
            res.histogram,
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
