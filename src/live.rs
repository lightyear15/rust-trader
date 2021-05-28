use crate::candles::Candle;
use crate::drivers::{LiveEvent, LiveFeed, RestApi};
use crate::orders::Order;
use crate::strategies::SpotSinglePairStrategy;
use crate::wallets::SpotWallet;
use std::collections::VecDeque;

pub async fn run_live(rest: Box<dyn RestApi>, mut feed: Box<dyn LiveFeed>, mut strategy: Box<dyn SpotSinglePairStrategy>) {
    println!("starting strategy {}", strategy.name());
    let hist_size = strategy.get_candles_history_size();
    let mut cnds = rest
        .get_candles(&strategy.symbol().symbol, Some(strategy.time_frame()), None, Some(hist_size))
        .await;
    cnds.sort_by_key(|cnd| std::cmp::Reverse(cnd.tstamp));
    let mut buffer = cnds.drain(0..hist_size).collect::<VecDeque<_>>();
    let mut wallet = rest.get_wallet().await.expect("in asking for initial wallet");
    let mut orders: Vec<Order> = Vec::new();

    println!("starting wallet {:?}", wallet);

    loop {
        let msg = feed.next().await;
        match msg {
            LiveEvent::Candle(sym, candle) => {
                buffer.pop_front();
                buffer.push_back(candle);
                let _action = strategy.on_new_candle(&wallet, orders.as_slice(), buffer.make_contiguous());
                println!("{} - {:?}", sym, buffer);
            }
            LiveEvent::ReconnectionRequired => {
                println!("ReconnectionRequired")
            }
            LiveEvent::Transaction(tx) => {
                orders.retain(|ord| ord.reference != tx.order.reference);
                let _action = strategy.on_new_transaction(&wallet, orders.as_slice(), &tx);
            }
            LiveEvent::NewOrder(order) => {
                orders.push(order);
            }
            LiveEvent::Balance(spot_wallet) => {
                wallet = spot_wallet;
            }
            _ => {}
        }
    }
}
