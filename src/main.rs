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
    openssl_probe::init_ssl_cert_env_vars();
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
            let strategy =
                strategies::create(&strategy, exchange, sym_info, cfg.time_frame, cfg.settings.clone()).expect("strategies::create");
            let res = backtest_spot_singlepair(storage, strategy, start, end)
                .await
                .expect("backtest epic fail");
            println!("Backtest statistics {:?}", res.0);
            println!("Backtest final wallet{:?}", res.1);
        }
        Trade::Live {} => {
            for strat in settings.strategies {
                println!("main - starting strategy {} on {}", strat.name, strat.symbol);
                let exc_sett: ExchangeSettings = settings
                    .exchanges
                    .get(&strat.exchange)
                    .expect("can't find the exchange in config")
                    .clone();

                let tx_storage: String = settings.transaction_storage.clone();
                actix_rt::Arbiter::spawn(async move {
                    run_live(strat, exc_sett, &tx_storage).await;
                });
            }
            actix_rt::Arbiter::local_join().await;
        }
    };
}
async fn run_live(strategy_settings: StrategySettings, exchange_settings: ExchangeSettings, storage_url: &str) {
    let ticks: Vec<_> = vec![drivers::Tick {
        sym: strategy_settings.symbol.clone(),
        interval: strategy_settings.time_frame,
    }];
    let rest = drivers::create_rest_client(&strategy_settings.exchange, &exchange_settings).expect("in create_rest_client");
    let sym_info = rest.get_symbol_info(&strategy_settings.symbol).await.expect("no symbol info");
    let listen_key = rest.refresh_ws_token(None).await;
    let live = drivers::create_live_driver(&strategy_settings.exchange, listen_key, ticks)
        .await
        .expect("could not create exchange drivers");
    let strategy = strategies::create(
        &strategy_settings.name,
        strategy_settings.exchange,
        sym_info,
        strategy_settings.time_frame,
        strategy_settings.settings,
    )
    .expect("strategies::create");
    let mut cur_arbiter = actix_rt::Arbiter::current();
    let storage = storage::Transactions::new(storage_url, &mut cur_arbiter).await;
    live::run_live(strategy, rest, live, storage).await;
}
