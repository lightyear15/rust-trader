#![allow(dead_code)]

use chrono::NaiveDate;
use structopt::StructOpt;
use std::thread;

mod backtest;
mod binance;
mod candles;
mod configuration;
mod drivers;
mod error;
mod live;
mod orders;
mod storage;
mod strategies;
mod symbol;
mod utils;
mod wallets;
use crate::backtest::backtest;
use crate::configuration::Settings;

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
            let driver = drivers::create_rest_client(&exchange, &exc_sett).await.expect("exchange not found");
            let storage = storage::Candles::new(&settings.candle_storage).await;
            let res = import(driver.as_ref(), &storage, &exchange, &symbol, &start, &end).await;
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
            let drv = drivers::create_rest_client(&exchange, exc_sett).await.expect("no exchange driver");
            let sym_info =drv.get_symbol_info(&symbol).await.expect("no symbol info");
            let strategy = strategies::create(&strategy, exchange, sym_info, cfg.time_frame).expect("strategies::create");
            let res = backtest(storage, strategy, start, end).await.expect("backtest epic fail");
            println!("Backtest {:?}", res);
        }
        Trade::Live {} => {
            for strat in settings.strategies {
                let exc_sett = settings.exchanges.get(&strat.exchange).expect("can't find the exchange in config");
                let tick: Vec<_> = vec![drivers::Tick {
                    sym: strat.symbol.clone(),
                    interval: strat.time_frame,
                }];
                let (rest, live) = drivers::create_live_drivers(&strat.exchange, exc_sett, tick.as_slice())
                    .await
                    .expect("could not create exchange drivers");
                let sym_info =rest.get_symbol_info(&strat.symbol).await.expect("no symbol info");
                let strategy =
                    strategies::create(&strat.name, strat.exchange, sym_info, strat.time_frame).expect("strategies::create");
                live::run_live(rest, live, strategy).await;
            }
        }
    };
}

async fn import(
    driver: &dyn drivers::RestApi,
    storage: &storage::Candles,
    exchange: &str,
    sym: &str,
    start: &NaiveDate,
    end: &NaiveDate,
) -> u64 {
    // TODO: check if candles already exists
    println!("importing candles for {} days", end.signed_duration_since(*start).num_days());
    let mut total: u64 = 0;
    let mut tstamp = start.and_hms(0, 0, 0);
    let end_t = end.and_hms(0, 0, 0);
    while tstamp < end_t {
        let candles = driver.get_candles(sym, None, Some(&tstamp), None).await;
        if candles.is_empty() {
            panic!("not getting any candles");
        }
        total += storage.store(exchange, sym, &candles).await.expect("in storing data to DB");
        tstamp = candles.last().expect("last not present").tstamp;
        println!("{}", tstamp);
    }
    total
}
