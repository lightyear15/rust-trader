use crate::drivers::{LiveEvent, LiveFeed, RestApi};
use crate::orders::Order;
use crate::strategies::{SpotSinglePairStrategy, Action};
use std::collections::VecDeque;

pub async fn run_live(mut strategy: Box<dyn SpotSinglePairStrategy>, rest: Box<dyn RestApi>, mut feed: Box<dyn LiveFeed>) {
    println!("starting strategy {}", strategy.name());
    println!("on symbol {:?}", strategy.symbol());
    let hist_size = strategy.get_candles_history_size();
    rest
        .get_candles(&strategy.symbol().symbol, Some(strategy.time_frame()), None, Some(hist_size))
        .await;
    let mut cnds = rest
        .get_candles(&strategy.symbol().symbol, Some(strategy.time_frame()), None, Some(hist_size))
        .await;
    cnds.sort_by_key(|cnd| std::cmp::Reverse(cnd.tstamp));
    let mut buffer = cnds.drain(0..hist_size).collect::<VecDeque<_>>();
    let mut wallet = rest.get_wallet().await.expect("in asking for initial wallet");
    let mut orders: Vec<Order> = Vec::new();

    loop {
        let msg = feed.next().await;
        let action = match msg {
            LiveEvent::Candle(sym, candle) => {
                buffer.pop_front();
                buffer.push_back(candle);
                println!("{} - {:?}", sym, buffer);
                strategy.on_new_candle(&wallet, orders.as_slice(), buffer.make_contiguous())
            }
            LiveEvent::ReconnectionRequired => {
                println!("ReconnectionRequired");
                return;
            }
            LiveEvent::Transaction(tx) => {
                orders.retain(|ord| ord.reference != tx.order.reference);
                strategy.on_new_transaction(orders.as_slice(), &tx)
            }
            LiveEvent::NewOrder(order) => {
                orders.push(order);
                Action::None
            }
            LiveEvent::Balance(spot_wallet) => {
                wallet = spot_wallet;
                Action::None
            }
            _ => {
                println!("unkown event");
                Action::None
            }
        };

        match action {
            Action::NewOrder(order) => {
                println!("received a new order action {:?}", order);
            }
            Action::CancelOrder(reference) => {
                println!("received a cancel order action {:?}", reference);
            }
            Action::None => {
                println!("action of doing nothing");
            }
        }
    }
}
