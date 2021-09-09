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
use ta::{DataItem, Next, Period, Reset};

const CAPITAL: f64 = 0.1;
const GAIN: f64 = 1.01;

#[derive(Clone)]
pub struct BBBMfiScalp {
    mfi: MoneyFlowIndex,
    bbb: BollingerBands,

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
        Self {
            bbb: BollingerBands::new(bbb_size, bbb_multiplier).unwrap(),
            mfi: MoneyFlowIndex::new(mfi_period).unwrap(),

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

        if !outstanding_orders.is_empty() {
            return Action::None;
        }
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
            let price = tx.avg_price * GAIN;
            let volume = tx.volume / GAIN;
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
