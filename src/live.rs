use crate::candles::Candle;
use crate::configuration::{ExchangeSettings, Settings, StrategySettings};
use crate::drivers::{create_live_driver, create_rest_client, LiveEvent, LiveFeed, RestApi, Tick};
use crate::orders::Order;
use crate::storage;
use crate::strategies;
use crate::strategies::{Action, SpotSinglePairStrategy};
use chrono::Utc;
use log::{debug, error, info, warn};
use std::collections::{HashMap, VecDeque};
use std::iter::Extend;

pub async fn run_live(
    strategies_settings: Vec<StrategySettings>,
    mut tx_storage: storage::Transactions,
    exchange_settings: ExchangeSettings,
) {
    if strategies_settings.is_empty() {
        return;
    }
    let exchange = strategies_settings.first().unwrap().exchange.clone();
    if strategies_settings.iter().any(|st| st.exchange != exchange) {
        panic!("strategies not on same exchange");
    }
    // init rest exchange api
    let rest = create_rest_client(&exchange, &exchange_settings).expect("in create_rest_client");
    // init strategies
    let mut strategies: HashMap<String, Box<dyn SpotSinglePairStrategy>> = HashMap::new();
    let mut buffers: HashMap<String, VecDeque<Candle>> = HashMap::new();
    let mut orders: HashMap<String, Vec<Order>> = HashMap::new();
    let mut ticks: Vec<Tick> = Vec::new();
    for st in strategies_settings {
        let sym_info = rest.get_symbol_info(&st.symbol).await.expect("no symbol info");
        let mut strategy =
            strategies::create(&st.name, st.exchange, sym_info, st.time_frame, st.settings).expect("strategies::create");
        let sym = strategy.symbol().symbol.clone();
        let t_frame = *strategy.time_frame();
        //init
        let init_size = strategy.get_candles_init_size();
        if init_size == 0 {
            info!("{}: no init needed ", strategy.name());
        } else {
            let mut init_cnds = rest.get_candles(&sym, Some(&t_frame), None, Some(init_size)).await;
            init_cnds.sort_by_key(|cnd| cnd.tstamp);
            strategy.initialize(init_cnds.as_slice());
            info!("{}: init needed {} - {}", strategy.name(), init_size, init_cnds.len());
        }
        // runtime prep
        let hist_size = strategy.get_candles_history_size();
        ticks.push(Tick {
            sym: sym.clone(),
            interval: t_frame,
        });

        let mut cnds = rest.get_candles(&sym, Some(&t_frame), None, Some(hist_size)).await;
        cnds.sort_by_key(|cnd| std::cmp::Reverse(cnd.tstamp));
        let buffer = cnds.drain(0..hist_size).collect::<VecDeque<_>>();
        info!("strategy {} on {} at {} started", strategy.name(), sym.clone(), t_frame);
        buffers.insert(sym.clone(), buffer);
        let outstanding_orders = rest.get_outstanding_orders(&sym).await;
        info!("found {} outstanding orders for {}", outstanding_orders.len(), sym);
        orders.insert(sym.clone(), outstanding_orders);
        strategies.insert(sym, strategy);
    }
    // init wallet
    let mut wallet = rest.get_wallet().await.expect("in asking for initial wallet");

    // init live feed client
    let listen_key = rest.refresh_ws_token(None).await;
    let mut feed = create_live_driver(&exchange, listen_key, ticks)
        .await
        .expect("could not create exchange drivers");

    // main loop
    loop {
        let msg = feed.next().await;
        let action = match msg {
            LiveEvent::Candle(sym, candle) => {
                if let Some(st) = strategies.get_mut(&sym) {
                    let buf = buffers.get_mut(&sym).expect("symbol not found in buffers");
                    if candle.tstamp == buf.front().unwrap().tstamp {
                        error!("{} - repeated candle {:?} {:?}", sym, candle, buf.front().unwrap());
                        buf.pop_front();
                    }
                    debug!("{} - new candle event at {}", sym, Utc::now());
                    buf.pop_back();
                    buf.push_front(candle);
                    let ords = orders.get(&sym).expect("symbol not found in orders").as_slice();
                    st.on_new_candle(&wallet, ords, buf.make_contiguous())
                } else {
                    debug!("ignoring new candle event at {} {}", Utc::now(), sym);
                    Action::None
                }
            }
            LiveEvent::Transaction(tx) => {
                if let Some(st) = strategies.get_mut(&tx.symbol) {
                    debug!("new transaction event at {}\n\t {:?}", Utc::now(), tx);
                    let ords = orders.get_mut(&tx.symbol).expect("symbol not found in orders");
                    ords.retain(|ord| ord.id != tx.order.id);
                    tx_storage.store(st.exchange(), &tx).await.expect("in storing new transaction");
                    st.on_new_transaction(ords.as_slice(), &tx)
                } else {
                    debug!("ignoring new transaction event at {} {}", Utc::now(), tx.symbol);
                    Action::None
                }
            }
            LiveEvent::NewOrder(order) => {
                if strategies.contains_key(&order.symbol.symbol) {
                    debug!("new order event at {}\n\t {:?}", Utc::now(), order);
                    let ords = orders.get_mut(&order.symbol.symbol).expect("symbol not found in orders");
                    ords.push(order);
                    Action::None
                } else {
                    debug!("ignoring new order event at {} {}", Utc::now(), order.symbol.symbol);
                    Action::None
                }
            }
            LiveEvent::BalanceUpdate(spot_wallet) => {
                debug!("new balance event at {}", Utc::now());
                wallet.assets.extend(spot_wallet.assets.into_iter());
                Action::None
            }
            LiveEvent::AssetUpdate { asset, delta } => {
                debug!("received asset change: {} {}", asset, delta);
                Action::None
            }
            LiveEvent::TokenRefreshRequired => {
                debug!("{} - Token refresh required", exchange);
                let token = feed.token();
                rest.refresh_ws_token(Some(token)).await;
                Action::None
            }
            LiveEvent::ReconnectionRequired => {
                debug!("{} - ReconnectionRequired", exchange);
                let new_token = rest.refresh_ws_token(None).await;
                feed.reconnect(new_token).await;
                Action::None
            }
            _ => {
                warn!("unknown  event");
                Action::None
            }
        };
        match action {
            Action::NewOrder(order) => {
                let status = rest.send_order(order).await;
                debug!("new order sent {:?}", status);
            }
            Action::CancelOrder(symbol, id) => {
                let status = rest.cancel_order(symbol, id).await;
                debug!("new cancel order sent {:?}", status);
            }
            Action::None => {}
        }
    }
}
