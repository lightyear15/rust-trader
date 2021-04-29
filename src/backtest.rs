use super::{orders, orders::Order, orders::Transaction, storage, strategies, wallets, Error, Strategy};
use chrono::NaiveDate;

pub async fn backtest(
    storage: &storage::Candles,
    strategy: &mut dyn Strategy,
    start: &NaiveDate,
    end: &NaiveDate,
) -> Result<f64, Error> {
    // set up
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
        let mut cnds = storage
            .get(
                strategy.exchange(),
                strategy.symbol(),
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
        last_price = last.close;

        let mut action = strategy.on_new_candle(&wallet, cnds.as_slice());

        tstamp += *(strategy.time_frame());
        start_time = tstamp - (*(strategy.time_frame()) * depth as i32);
        // dealing with the action
        while let strategies::Action::NewOrder(order) = &action {
                if let Some(tx) = process_new_order(&order, last.close, &tstamp) {
                    wallet = update_wallet(&tx, &wallet);
                    action = strategy.on_new_transaction(&wallet, &tx);
                    transactions.push(tx);
                } else {
                    println!("unfillfilled order");
                    outstanding_orders.push(order.clone());
                };
        }
    }
    println!("closing with total {}", wallet.quote + wallet.base * last_price);
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
            if price >= *buy_price {
                Some(tx)
            } else {
                None
            }
        }
        (orders::Side::Sell, orders::Type::Limit(sell_price)) => {
            if price <= *sell_price {
                Some(tx)
            } else {
                None
            }
        }
        (_, _) => None,
    }
}

fn update_wallet(tx: &Transaction, wallet: &wallets::SimplePairWallet) -> wallets::SimplePairWallet {
    match tx.side {
        orders::Side::Buy => wallets::SimplePairWallet {
            base: wallet.base + tx.volume,
            quote: wallet.quote - (tx.avg_price * tx.volume),
        },
        orders::Side::Sell => wallets::SimplePairWallet {
            base: wallet.base - tx.volume,
            quote: wallet.quote + (tx.avg_price * tx.volume),
        },
    }
}
