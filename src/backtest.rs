use crate::candles::Candle;
use crate::error::Error;
use crate::orders::{Order, Side, Transaction, Type};
use crate::statistics::Statistics;
use crate::strategies::Action;
use crate::strategies::SpotSinglePairStrategy;
use crate::symbol::Symbol;
use crate::{storage, utils, wallets};
use chrono::{Duration, NaiveDate};
use std::collections::HashMap;

const STARTING_BALANCE: f64 = 10000.0;

pub async fn backtest_spot_singlepair(
    storage: storage::Candles,
    mut strategy: Box<dyn SpotSinglePairStrategy>,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<(Statistics, wallets::SpotWallet), Error> {
    // set up
    let end_t = end.and_hms(0, 0, 0);
    let start_t = start.and_hms(0, 0, 0);
    let mut start_time = start_t;
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

    let mut bar = progress::Bar::new();
    bar.set_job_title("backtesting");
    while tstamp < end_t {
        let perc = (tstamp - start_t).num_minutes() * 100 / (end_t - start_t).num_minutes();
        bar.reach_percent(perc as i32);

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
        let last = cnds.first().unwrap();

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
        // fullfilling any of the outstanding orders
        loop {
            let mut next_tx: Option<Transaction> = None;
            // find first order that can be fullfilled
            for or in &outstanding_orders {
                let tx = if order_in_candle(or, last) {
                    // order price limit is within the current candle (or order is MARKET)
                    generate_tx_from_order(&or, last, &storage).await.expect("process_order")
                } else {
                    Transaction::default()
                };
                let frst_tstamp = next_tx.as_ref().map(|t| t.tstamp).unwrap_or(chrono::naive::MAX_DATETIME);
                if frst_tstamp > tx.tstamp {
                    next_tx = Some(tx);
                }
            }
            if let Some(tx) = next_tx {
                if let Some(tp_sl_or) = order_from_tp_sl_tx(&tx) {
                    outstanding_orders.push(tp_sl_or);
                }

                outstanding_orders.retain(|or| or.id != tx.order.id);
                let action = strategy.on_new_transaction(outstanding_orders.as_slice(), &tx);

                stats.update_with_transaction(&tx);
                update_wallet(&tx, strategy.symbol(), &mut wallet);

                transactions.push(tx);
                on_action(action, &mut stats, &mut outstanding_orders);
            } else {
                break;
            }
        }

        // processing new candle signal
        let mut price_update: HashMap<String, f64> = HashMap::new();
        price_update.insert(strategy.symbol().base.clone(), last.close);
        price_update.insert(strategy.symbol().quote.clone(), 1.0);
        stats.update_with_last_prices(&wallet, &price_update);
        let action = strategy.on_new_candle(&wallet, outstanding_orders.as_slice(), cnds.as_slice());
        on_action(action, &mut stats, &mut outstanding_orders);

        tstamp += *(strategy.time_frame());
        start_time = tstamp - (*(strategy.time_frame()) * depth as i32);
    }
    bar.jobs_done();
    Ok((stats, wallet))
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

async fn generate_tx_from_order(ord: &Order, last: &Candle, store: &storage::Candles) -> Result<Transaction, Error> {
    let mut tx = Transaction {
        symbol: ord.symbol.symbol.clone(),
        side: ord.side.clone(),
        order: ord.clone(),
        avg_price: last.open,
        fees: 0.0,
        fees_asset: ord.symbol.quote.clone(),
        volume: ord.volume,
        tstamp: last.tstamp,
    };
    let end_t = last.tstamp + last.tframe;
    match (&ord.o_type, &ord.side) {
        (Type::Market, _) => Ok(tx),
        (Type::Limit(buy_p), Side::Buy) => {
            let t = store
                .find_lower(&ord.exchange, &ord.symbol.symbol, &last.tstamp, &end_t, *buy_p)
                .await
                .ok_or_else(|| Error::ErrNotFound(format!("can't find lower for {}", *buy_p)))?;
            tx.avg_price = *buy_p;
            tx.tstamp = t;
            Ok(tx)
        }
        (Type::Limit(sell_p), Side::Sell) => {
            let t = store
                .find_higher(&ord.exchange, &ord.symbol.symbol, &last.tstamp, &end_t, *sell_p)
                .await
                .ok_or_else(|| Error::ErrNotFound(format!("can't find higher for {}", *sell_p)))?;
            tx.avg_price = *sell_p;
            tx.tstamp = t;
            Ok(tx)
        } //(_, _) => Err(Error::Done),
    }
}

fn update_wallet(tx: &Transaction, sym: &Symbol, wallet: &mut wallets::SpotWallet) {
    assert_eq!(tx.symbol, sym.symbol);
    match tx.side {
        Side::Buy => {
            *wallet.assets.get_mut(&sym.quote).expect("no quote in wallet") -= tx.avg_price * tx.volume;
            *wallet.assets.get_mut(&sym.base).expect("no base in wallet") += tx.volume;
        }
        Side::Sell => {
            *wallet.assets.get_mut(&sym.quote).expect("no quote in wallet") += tx.avg_price * tx.volume;
            *wallet.assets.get_mut(&sym.base).expect("no base in wallet") -= tx.volume;
        }
    };
    wallet.assets.values().for_each(|v| {
        if v.is_sign_negative() {
            panic!("wallet is negative {:?} with tx {:?}", wallet, tx)
        }
    });
}

fn order_from_tp_sl_tx(_tx: &Transaction) -> Option<Order> {
    None
}

fn on_action(action: Action, stats: &mut Statistics, outstanding_orders: &mut Vec<Order>) {
    match action {
        Action::NewOrder(or) => {
            //println!("received new order {:?}", or);
            stats.update_with_order(&or);
            outstanding_orders.push(or);
        }
        Action::CancelOrder(_, id) => {
            //println!("received cancel order {}", reference);
            outstanding_orders.retain(|or| or.id != id);
        }
        _ => {}
    }
}
