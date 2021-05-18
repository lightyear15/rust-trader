use crate::candles::Candle;
use crate::error::Error;
use crate::orders::{Order, Side, Transaction, Type};
use crate::strategies::SpotSinglePairStrategy;
use crate::strategies::{Action, Statistics};
use crate::symbol::Symbol;
use crate::{storage, utils, wallets};
use chrono::{Duration, NaiveDate};
use std::collections::HashMap;

const STARTING_BALANCE: f64 = 10000.0;

pub async fn backtest(
    storage: storage::Candles,
    mut strategy: Box<dyn SpotSinglePairStrategy>,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Statistics, Error> {
    // set up
    let end_t = end.and_hms(0, 0, 0);
    let mut start_time = start.and_hms(0, 0, 0);
    let depth = strategy.get_candles_history_size();
    let mut tstamp = start_time + (*(strategy.time_frame()) * (depth as i32));

    // preparing the environment
    let mut wallet = wallets::SpotWallet { assets: HashMap::new() };
    wallet.assets.insert(strategy.symbol().quote.clone(), STARTING_BALANCE);
    wallet.assets.insert(strategy.symbol().base.clone(), 0.0);
    let mut outstanding_orders: Vec<Order> = Vec::new();
    let mut transactions: Vec<Transaction> = Vec::new();

    // performance tracking
    let mut stats = Statistics::new(STARTING_BALANCE);

    while tstamp < end_t {
        // gather current candles
        let mut cnds = storage
            .get(
                strategy.exchange(),
                &strategy.symbol().symbol,
                &start_time,
                &end_t,
                strategy.time_frame(),
                depth,
            )
            .await;
        if cnds.len() < depth {
            break;
        }
        cnds.reverse();
        let last = cnds.last().expect("len == 0 ??, impossible!!");

        // check outstanding orders with current candle
        // any expired orders?
        let mut i = 0;
        while i < outstanding_orders.len() {
            if is_expired(&outstanding_orders[i], last) {
                let ord = outstanding_orders.remove(i);
                stats.update_with_expired_order(&ord);
            } else {
                i += 1;
            }
        }
        // find first order that can be fullfilled
        loop {
            let mut frst_tx: Option<Transaction> = None;
            for or in &outstanding_orders {
                if order_in_candle(or, last) {
                    let pot_tx = process_order(&or, last, &storage).await.expect("process_order");
                    let frst_tstamp = frst_tx.as_ref().map(|t| t.tstamp).unwrap_or(chrono::naive::MAX_DATETIME);
                    if frst_tstamp > pot_tx.tstamp {
                        frst_tx = Some(pot_tx);
                    }
                }
            }
            if let Some(tx) = frst_tx {
                if let Some(tp_sl_or) = order_from_tp_sl_tx(&tx) {
                    outstanding_orders.push(tp_sl_or);
                }
                update_wallet(&tx, strategy.symbol(), &mut wallet);
                outstanding_orders.retain(|or| or.reference != tx.order.reference);
                stats.update_with_transaction(&tx);

                let action = strategy.on_new_transaction(&wallet, outstanding_orders.as_slice(), &tx);
                transactions.push(tx);
                match action {
                    Action::NewOrder(or) => {
                        stats.update_with_order(&or);
                        outstanding_orders.push(or);
                    }
                    Action::CancelOrder(reference) => {
                        outstanding_orders.retain(|or| or.reference != reference);
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }

        // processing new candle signal
        let mut price_update: HashMap<String, f64> = HashMap::new();
        price_update.insert(strategy.symbol().base.clone(), last.close);
        price_update.insert(strategy.symbol().quote.clone(), 1.0);
        stats.update_with_last_prices(&wallet, &price_update);
        let action = strategy
            .as_mut()
            .on_new_candle(&wallet, outstanding_orders.as_slice(), cnds.as_slice());
        match action {
            Action::NewOrder(or) => {
                stats.update_with_order(&or);
                outstanding_orders.push(or);
            }
            Action::CancelOrder(reference) => {
                outstanding_orders.retain(|or| or.reference != reference);
            }
            _ => {}
        }

        tstamp += *(strategy.time_frame());
        start_time = tstamp - (*(strategy.time_frame()) * depth as i32);
    }
    Ok(stats)
}

fn order_in_candle(ord: &Order, last: &Candle) -> bool {
    match (&ord.o_type, &ord.side) {
        (Type::Market, _) => true,
        (Type::Limit(buy_p), Side::Buy) => *buy_p >= last.low,
        (Type::Limit(sell_p), Side::Sell) => *sell_p <= last.high,
        //(_, _) => false,
    }
}
fn is_expired(ord: &Order, last: &Candle) -> bool {
    ord.expire.map_or(false, |date| last.tstamp > date)
}

async fn process_order(ord: &Order, last: &Candle, store: &storage::Candles) -> Result<Transaction, Error> {
    let mut tx = Transaction {
        symbol: ord.symbol.clone(),
        side: ord.side.clone(),
        order: ord.clone(),
        avg_price: last.open,
        volume: ord.volume,
        tstamp: last.tstamp,
    };
    let end_t = last.tstamp + last.tframe;
    match (&ord.o_type, &ord.side) {
        (Type::Market, _) => Ok(tx),
        (Type::Limit(buy_p), Side::Buy) => {
            let t = store
                .find_lower(&ord.exchange, &ord.symbol, &last.tstamp, &end_t, *buy_p)
                .await
                .ok_or_else(|| Error::ErrNotFound(format!("can't find lower for {}", *buy_p)))?;
            tx.avg_price = *buy_p;
            tx.tstamp = utils::generate_random_tstamp(&t, &(t + Duration::minutes(1)));
            Ok(tx)
        }
        (Type::Limit(sell_p), Side::Sell) => {
            let t = store
                .find_higher(&ord.exchange, &ord.symbol, &last.tstamp, &end_t, *sell_p)
                .await
                .ok_or_else(|| Error::ErrNotFound(format!("can't find higher for {}", *sell_p)))?;
            tx.avg_price = *sell_p;
            tx.tstamp = utils::generate_random_tstamp(&t, &(t + Duration::minutes(1)));
            Ok(tx)
        } //(_, _) => Err(Error::Done),
    }
}

fn update_wallet(tx: &Transaction, sym : &Symbol, wallet: &mut wallets::SpotWallet) {
    assert_eq!(tx.symbol, sym.symbol);
    match tx.side {
        Side::Buy => {
            *wallet.assets.get_mut(&sym.quote).expect("no quote in wallet") -= tx.avg_price * tx.volume;
            *wallet.assets.get_mut(&sym.base).expect("no base in wallet") += tx.volume;
        },
        Side::Sell => {
            *wallet.assets.get_mut(&sym.quote).expect("no quote in wallet") += tx.avg_price * tx.volume;
            *wallet.assets.get_mut(&sym.base).expect("no base in wallet") -= tx.volume;
        },
    };
}

fn order_from_tp_sl_tx(_tx: &Transaction) -> Option<Order> {
    None
}
