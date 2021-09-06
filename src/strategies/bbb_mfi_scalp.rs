use crate::candles::Candle;
use crate::orders::{Order, Side, Transaction, Type};
use crate::strategies::{Action, SpotSinglePairStrategy};
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;

use chrono::Duration;
use log::debug;
use std::collections::HashMap;
use std::convert::TryInto;
use ta::indicators::{BollingerBands, MoneyFlowIndex};
use ta::{DataItem, Next, Reset};

const CAPITAL: f64 = 0.1;

#[derive(Clone)]
pub struct BBBMfiScalp {
    mfi: MoneyFlowIndex,
    bbb: BollingerBands,
    last_tx: Option<Transaction>,

    exchange: String,
    sym: Symbol,
    time_frame: chrono::Duration,
    history_len: usize,
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
        let history_len = std::cmp::max(mfi_period, bbb_size);
        Self {
            bbb: BollingerBands::new(bbb_size, bbb_multiplier).unwrap(),
            mfi: MoneyFlowIndex::new(mfi_period).unwrap(),
            last_tx: None,

            exchange,
            sym,
            time_frame,
            history_len,
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
        self.history_len
    }

    fn init(&mut self, history: &[Candle]) {
        self.bbb.reset();
        self.mfi.reset();
        for cnd in history {
            println!("giulio - init candle {:?}", cnd);
            let item: DataItem = cnd.try_into().unwrap();
            self.bbb.next(&item);
            self.mfi.next(&item);
        }
    }

    fn on_new_candle(&mut self, wallet: &SpotWallet, outstanding_orders: &[Order], history: &[Candle]) -> Action {
        println!("giulio - on_new_candle candle {:?}", history);
        let cnd = history.first().unwrap();
        let item: DataItem = cnd.try_into().unwrap();
        let price = cnd.close;
        let bbb = self.bbb.next(&item);
        let mfi = self.mfi.next(&item);

        if !outstanding_orders.is_empty() {
            return Action::None;
        }
        if let Some(tx) = self.last_tx.as_ref() {
            let diff = cnd.tstamp - tx.tstamp;
            if diff > Duration::days(1) {
                let mut order = Order::new();
                order.exchange = self.exchange.clone();
                order.symbol = self.sym.clone();
                order.side = Side::Sell;
                order.o_type = Type::Limit(tx.avg_price * 1.01);
                order.volume = tx.volume * 0.99;
            } else if price > bbb.upper && mfi > 80.0 {
                let mut order = Order::new();
                order.exchange = self.exchange.clone();
                order.symbol = self.sym.clone();
                order.side = Side::Sell;
                order.o_type = Type::Limit(price);
                order.volume = tx.volume.min(tx.avg_price * tx.volume / price);
            }
        } else if price < bbb.lower && mfi < 20.0 {
            let volume = wallet.assets.get(&self.sym.quote).unwrap_or(&0.0) * CAPITAL / price;
            let mut order = Order::new();
            order.exchange = self.exchange.clone();
            order.symbol = self.sym.clone();
            order.side = Side::Buy;
            order.o_type = Type::Market;
            order.volume = volume;
        }
        Action::None
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
