#![allow(dead_code)]
#![allow(unused_imports)]

use chrono::NaiveDate;
use std::future::{ready, Future};
use std::pin::Pin;
use structopt::StructOpt;

mod backtest;
mod candles;
mod configuration;
mod drivers;
mod error;
mod import;
mod live;
mod orders;
mod statistics;
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
            log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
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
            println!("Backtest final wallet{:?}", res.1);
            println!("Backtest statistics {}", res.0.report());
        }
        Trade::Live {} => {
            log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
            let tx_storage: String = settings.transaction_storage.clone();
            let mut cur_arbiter = actix_rt::Arbiter::current();
            for (exchange, ex_settings) in settings.exchanges {
                let strats: Vec<_> = settings.strategies.iter().filter(|st| st.exchange == exchange).cloned().collect();
                if strats.is_empty() {
                    continue;
                }
                let storage = storage::Transactions::new(&tx_storage, &mut cur_arbiter).await;

                actix_rt::Arbiter::spawn(async move {
                    live::run_live(strats, storage, ex_settings).await;
                });
                actix_rt::time::delay_for(std::time::Duration::from_secs(5)).await;
            }
            actix_rt::Arbiter::local_join().await;
        }
    };
}
