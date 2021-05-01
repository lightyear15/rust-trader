use super::orders::{Order, Side, Transaction, Type};
use super::strategies::Action;
use super::{candles::Candle, storage, wallets, Error, Strategy};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use tokio::runtime::Runtime;

pub async fn backtest(
    storage: storage::Candles,
    mut strategy: Box<dyn Strategy>,
    start: NaiveDate,
    end: NaiveDate,
    ) -> Result<f64, Error> {
    // set up
    let rt = Runtime::new().unwrap();
    let end_t = end.and_hms(0, 0, 0);
    let mut start_time = start.and_hms(0, 0, 0);
    let depth = strategy.get_candles_history_size();
    let mut tstamp = start_time + (*(strategy.time_frame()) * (depth as i32));

    // preparing the environment
    let mut wallet = wallets::SimplePairWallet {
        base: 0.0,
        quote: 10000.0,
    };
    let mut outstanding_orders: Vec<Order> = Vec::new();
    let mut transactions: Vec<Transaction> = Vec::new();

    // performance tracking
    let mut last_price: f64 = 0.0;

    while tstamp < end_t {
        let mut cnds = storage.get(strategy.exchange(), strategy.symbol(), &start_time, &end_t, strategy.time_frame(), depth).await;
        if cnds.len() < depth {
            break;
        }
        cnds.reverse();

        let last = cnds.last().expect("len == 0 ??, impossible!!");
        last_price = last.close;

        // processing outstanding orders
        outstanding_orders = outstanding_orders
            .into_iter()
            .filter(|order| is_not_expired(order, last))
            .collect();
        let mut new_tx: Vec<Transaction> = outstanding_orders
            .drain_filter(|order| order_in_candle(order, last))
            .map(|order| rt.block_on(process_order(&order, last, &storage)).expect("in process order")).collect();
        // check if transactions were from a TakeProfit or StopLoss
        let mut tp_sl_orders :Vec<Order> = Vec::new();
        let mut tp_sl_tx :Vec<Transaction> = tp_sl_orders
            .drain_filter(|order| order_in_candle(order, last))
            .map(|order| rt.block_on(process_order(&order, last, &storage)).expect("in process tp/sl orders")).collect();

        outstanding_orders.append(&mut tp_sl_orders);
        new_tx.append(&mut tp_sl_tx);
        new_tx.sort_by_key(|tx|{tx.tstamp});

        for tx in &new_tx {
            wallet = update_wallet(tx, &wallet);
            if let Action::NewOrder(or) = strategy.as_mut().on_new_transaction(&wallet, tx) {
                outstanding_orders.push(or);
            }
        }
        transactions.append(&mut new_tx);

        if let Action::NewOrder(or) = strategy.as_mut().on_new_candle(&wallet, cnds.as_slice()) {
            outstanding_orders.push(or);
        }

        tstamp += *(strategy.time_frame());
        start_time = tstamp - (*(strategy.time_frame()) * depth as i32);
    }
    Err(Error::Done)
}

fn order_in_candle(ord: &Order, last: &Candle) -> bool {
    match (&ord.o_type, &ord.side) {
        (Type::Market, _) => true,
        (Type::Limit(buy_p), Side::Buy) => *buy_p >= last.low,
        (Type::Limit(sell_p), Side::Sell) => *sell_p <= last.high,
        (_, _) => false,
    }
}
fn is_not_expired(ord: &Order, last: &Candle) -> bool {
    ord.expire.map_or(true, |date| last.tstamp < date)
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
                .ok_or(Error::ErrNotFound)?;
            tx.avg_price = *buy_p;
            tx.tstamp = t;
            Ok(tx)
        }
        (Type::Limit(sell_p), Side::Sell) => {
            let t = store
                .find_higher(&ord.exchange, &ord.symbol, &last.tstamp, &end_t, *sell_p)
                .await
                .ok_or(Error::ErrNotFound)?;
            tx.avg_price = *sell_p;
            tx.tstamp = t;
            Ok(tx)
        }
        (_, _) => Err(Error::Done),
    }
}

fn update_wallet(tx: &Transaction, wallet: &wallets::SimplePairWallet) -> wallets::SimplePairWallet {
    match tx.side {
        Side::Buy => wallets::SimplePairWallet {
            base: wallet.base + tx.volume,
            quote: wallet.quote - (tx.avg_price * tx.volume),
        },
        Side::Sell => wallets::SimplePairWallet {
            base: wallet.base - tx.volume,
            quote: wallet.quote + (tx.avg_price * tx.volume),
        },
    }
}
/*

// outstanding orders
let fullfillable = outstanding_orders.drain_filter(|order| rt.block_on(is_order_fullfillable(order, last, storage)));
let (outstanding, fullfilled) = outstanding_orders
.iter()
.partition(|order| rt.block_on(is_order_fullfillable(order, last, storage)));

let mut action = strategy.on_new_candle(&wallet, cnds.as_slice());

tstamp += *(strategy.time_frame());
start_time = tstamp - (*(strategy.time_frame()) * depth as i32);
// dealing with the action
while let Action::NewOrder(order) = &action {
if let Some(tx) = process_new_order(&order, last.close, &tstamp) {
wallet = update_wallet(&tx, &wallet);
action = strategy.on_new_transaction(&wallet, &tx);
transactions.push(tx);
} else {
outstanding_orders.push(order.clone());
action = Action::None;
};
}
}
println!("closing with total {}", wallet.quote + wallet.base * last_price);
for tx in transactions {
println!("transaction {:?}", tx);
}
Ok(0.0)
}

// TODO: reject if cost > wallet
fn process_new_order(order: &Order, price: f64, current: &chrono::NaiveDateTime) -> Option<Transaction> {
let tx = Transaction {
symbol: order.symbol.clone(),
side: order.side.clone(),
order_id: 0,
avg_price: price,
volume: order.volume,
tstamp: *current,
};
match (&order.side, &order.o_type) {
(_, orders::Type::Market) => Some(tx),
(orders::Side::Buy, orders::Type::Limit(buy_price)) => {
if price <= *buy_price {
Some(tx)
} else {
None
}
}
(orders::Side::Sell, orders::Type::Limit(sell_price)) => {
if price >= *sell_price {
Some(tx)
} else {
None
}
}
(_, _) => None,
}
}


async fn is_order_fullfillable(order: &Order, last_candle: &Candle, storage: &storage::Candles) -> bool {
match (&order.o_type, &order.side) {
(orders::Type::Market, _) => true,
(orders::Type::Limit(pr), orders::Side::Sell) => storage
.find_higher(
&order.exchange,
&order.symbol,
&last_candle.tstamp,
&(last_candle.tstamp + last_candle.tframe),
*pr,
    )
    .await
    .is_some(),
    (orders::Type::Limit(pr), orders::Side::Buy) => storage
.find_lower(
    &order.exchange,
    &order.symbol,
    &last_candle.tstamp,
    &(last_candle.tstamp + last_candle.tframe),
    *pr,
    )
    .await
    .is_some(),
    }
}
*/
