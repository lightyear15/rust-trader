#![allow(dead_code)]
#![allow(unused_imports)]

use chrono::NaiveDate;
use std::future::{ready, Future};
use std::pin::Pin;
use structopt::StructOpt;

mod backtest;
mod binance;
mod candles;
mod configuration;
mod drivers;
mod error;
mod import;
mod live;
mod orders;
mod storage;
mod strategies;
mod symbol;
mod utils;
mod wallets;
use crate::backtest::backtest_spot_singlepair;
use crate::configuration::{ExchangeSettings, Settings, StrategySettings};

#[derive(Debug, StructOpt)]
enum Trade {
    #[structopt(about = "import candles from exchage")]
    Import {
        exchange: String,
        symbol: String,
        start: NaiveDate,
        end: NaiveDate,
    },
    #[structopt(about = "backtest specific strategy")]
    Backtest {
        strategy: String,
        exchange: String,
        symbol: String,
        start: NaiveDate,
        end: NaiveDate,
    },
    #[structopt(about = "live trading specific strategy")]
    Live {},
}

#[actix_web::main]
async fn main() {
    let settings = Settings::get_configuration("trader.toml").expect("Failed at reading configuration");
    let opt = Trade::from_args();
    match opt {
        Trade::Import {
            exchange,
            symbol,
            start,
            end,
        } => {
            let exc_sett = settings.exchanges.get(&exchange).expect("can't find the exchange in config");
            let driver = drivers::create_rest_client(&exchange, &exc_sett).expect("exchange not found");
            let storage = storage::Candles::new(&settings.candle_storage).await;
            let res = import::import(driver.as_ref(), &storage, &exchange, &symbol, &start, &end).await;
            println!("downloaded {} candles", res);
        }
        Trade::Backtest {
            strategy,
            exchange,
            symbol,
            start,
            end,
        } => {
            let storage = storage::Candles::new(&settings.candle_storage).await;
            let cfg = settings
                .strategies
                .iter()
                .find(|settings| settings.name == strategy && settings.exchange == exchange && settings.symbol == symbol)
                .expect("no such strategy configuration");

            let exc_sett = settings.exchanges.get(&exchange).expect("can't find the exchange in config");
            let drv = drivers::create_rest_client(&exchange, exc_sett).expect("no exchange driver");
            let sym_info = drv.get_symbol_info(&symbol).await.expect("no symbol info");
            let strategy = strategies::create(&strategy, exchange, sym_info, cfg.time_frame).expect("strategies::create");
            let res = backtest_spot_singlepair(storage, strategy, start, end)
                .await
                .expect("backtest epic fail");
            println!("Backtest {:?}", res);
        }
        Trade::Live {} => {
            for strat in settings.strategies {
                let exc_sett: ExchangeSettings = settings
                    .exchanges
                    .get(&strat.exchange)
                    .expect("can't find the exchange in config")
                    .clone();

                actix_rt::Arbiter::spawn(async move {
                    run_live(strat, exc_sett).await;
                });
                //let f = async {
                //};
                //arbiter.send(run_live(strat.exchange.clone(), exc_sett, tick));
            }
            actix_rt::Arbiter::local_join().await;
        }
    };
}
async fn run_live(strategy_settings: StrategySettings, exchange_settings: ExchangeSettings) {
    let ticks: Vec<_> = vec![drivers::Tick {
        sym: strategy_settings.symbol.clone(),
        interval: strategy_settings.time_frame,
    }];
    let rest = drivers::create_rest_client(&strategy_settings.exchange, &exchange_settings).expect("in create_rest_client");
    let sym_info = rest.get_symbol_info(&strategy_settings.symbol).await.expect("no symbol info");
    let listen_key = rest.refresh_ws_token(None).await;
    let live = drivers::create_live_driver(&strategy_settings.exchange, &listen_key, ticks.as_slice())
        .await
        .expect("could not create exchange drivers");
    let strategy = strategies::create(
        &strategy_settings.name,
        strategy_settings.exchange,
        sym_info,
        strategy_settings.time_frame,
    )
    .expect("strategies::create");
    live::run_live(strategy, rest, live).await;
}
