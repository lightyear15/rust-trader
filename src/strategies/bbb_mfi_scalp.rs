use crate::candles::Candle;
use crate::orders::{Order, Side, Transaction, Type};
use crate::strategies::{Action, SpotSinglePairStrategy};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use log::debug;
use std::collections::HashMap;
use std::convert::TryInto;
use ta::indicators::{BollingerBands, MoneyFlowIndex};
use ta::{DataItem, Next, Period, Reset};
use log::error;

const CAPITAL: f64 = 0.1;

#[derive(Clone)]
pub struct BBBMfiScalp {
    mfi: MoneyFlowIndex,
    bbb: BollingerBands,
    max_outstanding_orders : usize,
    starting_time: chrono::NaiveDateTime,
    take_profit: f64,

    exchange: String,
    sym: Symbol,
    time_frame: chrono::Duration,
}

impl BBBMfiScalp {
    pub fn new(exchange: String, sym: Symbol, time_frame: chrono::Duration, settings: HashMap<String, String>) -> Self {
        let mfi_period = settings
            .get("mfi_period")
            .expect("no mfi_period key")
            .parse::<usize>()
            .expect("mfi_period must be usize");
        let bbb_size = settings
            .get("bbb_size")
            .expect("no bbb_size key")
            .parse::<usize>()
            .expect("bbb_size must be usize");
        let bbb_multiplier = settings
            .get("bbb_multiplier")
            .expect("no bbb_multiplier key")
            .parse::<f64>()
            .expect("bbb_multiplier must be f64");
        let max_outstanding_orders = settings
            .get("max_outstanding_orders")
            .expect("no max_outstanding_orders key")
            .parse::<usize>()
            .expect("max_outstanding_orders must be usize");
        let take_profit_perc = settings
            .get("take_profit")
            .expect("no take_profit key")
            .parse::<usize>()
            .expect("take_profit must be usize");
        if take_profit_perc <=0 || take_profit_perc > 100 {
            error!("take_profit must be (0,100]");
            std::process::exit(1);
        }
        let take_profit: f64 = take_profit_perc as f64 / 100.0;
        Self {
            bbb: BollingerBands::new(bbb_size, bbb_multiplier).unwrap(),
            mfi: MoneyFlowIndex::new(mfi_period).unwrap(),
            max_outstanding_orders, 
            starting_time: NaiveDateTime::default(),
            take_profit,

            exchange,
            sym,
            time_frame,
        }
    }
}

impl SpotSinglePairStrategy for BBBMfiScalp {
    fn name(&self) -> String {
        format!(
            "BBBMfiScalp-{}-{}-{}",
            self.exchange,
            self.sym.to_string(),
            self.time_frame.to_string()
        )
    }

    fn get_candles_init_size(&self) -> usize {
        std::cmp::max(self.bbb.period(), self.mfi.period())
    }

    fn initialize(&mut self, history: &[Candle]) {
        debug!("BBBMfiScalp::init with {} candles", history.len());
        self.bbb.reset();
        self.mfi.reset();
        for cnd in history {
            let item: DataItem = cnd.try_into().unwrap();
            self.bbb.next(&item);
            self.mfi.next(&item);
        }
    }

    fn on_new_candle(&mut self, wallet: &SpotWallet, outstanding_orders: &[Order], history: &[Candle]) -> Action {
        debug!("on_new_candle with history depth - {} ", history.len());
        let cnd = history.first().unwrap();
        let item: DataItem = cnd.try_into().unwrap();
        let price = cnd.close;
        let bbb = self.bbb.next(&item);
        let mfi = self.mfi.next(&item);

        // rate limiter
        if  outstanding_orders.len() > self.max_outstanding_orders {
            return Action::None;
        } 
        let youngest_order = outstanding_orders.last().map(|o| o.tstamp).flatten().unwrap_or(self.starting_time);
        if cnd.tstamp - youngest_order <  chrono::Duration::hours(12) {
            return Action::None;
        }
        //decision making
        if price < bbb.lower && mfi < 20.0 {
            let volume = wallet.assets.get(&self.sym.quote).unwrap_or(&0.0) * CAPITAL / price;
            let mut order = Order::new();
            order.exchange = self.exchange.clone();
            order.symbol = self.sym.clone();
            order.side = Side::Buy;
            order.o_type = Type::Market;
            order.volume = volume;
            Action::NewOrder(order)
        } else {
            Action::None
        }
    }

    fn on_new_transaction(&mut self, _outstanding_orders: &[Order], tx: &Transaction) -> Action {
        if matches!(tx.side, Side::Buy) {
            let price = tx.avg_price * (1.0 + self.take_profit);
            let volume = tx.volume / (1.0 + self.take_profit);
            let mut order = Order::new();
            order.exchange = self.exchange.clone();
            order.symbol = self.sym.clone();
            order.side = Side::Sell;
            order.o_type = Type::Limit(price);
            order.volume = volume;
            order.tx_ref = tx.order.id;
            Action::NewOrder(order)
        } else {
            Action::None
        }
    }

    fn get_candles_history_size(&self) -> usize {
        1
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
