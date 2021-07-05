use crate::drivers::{LiveEvent, LiveFeed, RestApi};
use crate::orders::Order;
use crate::storage;
use crate::strategies::{Action, SpotSinglePairStrategy};
use chrono::Utc;
use log::{debug, info, warn};
use std::collections::VecDeque;

pub async fn run_live(
    mut strategy: Box<dyn SpotSinglePairStrategy>,
    rest: Box<dyn RestApi>,
    mut feed: Box<dyn LiveFeed>,
    tx_storage: storage::Transactions,
) {
    info!("starting strategy {} on symbol {:?}", strategy.name(), strategy.symbol());
    let hist_size = strategy.get_candles_history_size();
    rest.get_candles(&strategy.symbol().symbol, Some(strategy.time_frame()), None, Some(hist_size))
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
                if sym == strategy.symbol().symbol {
                    debug!("new candle event at {}", Utc::now());
                    buffer.pop_back();
                    buffer.push_front(candle);
                    strategy.on_new_candle(&wallet, orders.as_slice(), buffer.make_contiguous())
                } else {
                    debug!("ignoring new candle event at {} {}", Utc::now(), sym);
                    Action::None
                }
            }
            LiveEvent::ReconnectionRequired => {
                info!("ReconnectionRequired");
                let old_token = feed.token();
                let new_token = rest.refresh_ws_token(Some(old_token)).await;
                feed.reconnect(new_token).await;
                Action::None
            }
            LiveEvent::Transaction(tx) => {
                if tx.symbol == strategy.symbol().symbol {
                    debug!("new transaction event at {}\n\t {:?}", Utc::now(), tx);
                    orders.retain(|ord| ord.id != tx.order.id);
                    tx_storage
                        .store(strategy.exchange(), &tx)
                        .await
                        .expect("in storing new transaction");
                    strategy.on_new_transaction(orders.as_slice(), &tx)
                } else {
                    debug!("ignoring new transaction event at {} {}", Utc::now(), tx.symbol);
                    Action::None
                }
            }
            LiveEvent::NewOrder(order) => {
                if order.symbol.symbol == strategy.symbol().symbol {
                    debug!("new order event at {}\n\t {:?}", Utc::now(), order);
                    orders.push(order);
                    Action::None
                } else {
                    debug!("ignoring new order event at {} {}", Utc::now(), order.symbol.symbol);
                    Action::None
                }
            }
            LiveEvent::Balance(spot_wallet) => {
                debug!("new balance event at {}", Utc::now());
                wallet = spot_wallet;
                Action::None
            }
            LiveEvent::AssetUpdate{asset, delta} => {
                debug!("received asset change: {} {}", asset, delta);
                Action::None
            }
            _ => {
                warn!("unkown event");
                Action::None
            }
        };

        match action {
            Action::NewOrder(order) => {
                let status = rest.send_order(order).await;
                debug!("new order sent {:?}", status);
            }
            Action::CancelOrder(id) => {
                let status = rest.cancel_order(strategy.symbol().symbol.clone(), id).await;
                debug!("new cancel order sent {:?}", status);
            }
            Action::None => {}
        }
    }
}
