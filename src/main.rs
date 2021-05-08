#![allow(dead_code)]

use chrono::NaiveDate;
use structopt::StructOpt;

mod configuration;
mod drivers;
mod storage;
mod strategies;
mod backtest;
mod wallets;
mod utils;
mod orders;
mod error;
mod candles;
mod binance;
mod symbol;
use crate::configuration::Settings;
use crate::backtest::backtest;

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
            let driver = drivers::create_importer(&exchange, &exc_sett).expect("exchange not found");
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

            let strategy = strategies::create(&strategy, exchange, symbol, cfg.time_frame).expect("strategies::create");
            let res = backtest(storage, strategy, start, end).await.expect("backtest epic fail");
            println!("Backtest {:?}", res);
        }
        Trade::Live {} => {
            for strat in settings.strategies {
                let exc_sett = settings.exchanges.get(&strat.exchange).expect("can't find the exchange in config");
                let symbol = drivers::create_symbol_parser(&strat.exchange, exc_sett)
                    .expect("exchange not found")
                    .get_symbol(&strat.symbol)
                    .await
                    .expect("symbol not found");

                println!("starting strategy {} for {} on {}", strat.name, symbol.to_string(), strat.exchange);
            }
        }
    };
}

async fn import(
    driver: &dyn drivers::Importer,
    storage: &storage::Candles,
    exchange: &str,
    sym: &str,
    start: &NaiveDate,
    end: &NaiveDate,
) -> u64 {
    println!("importing candles for {} days", end.signed_duration_since(*start).num_days());
    let mut total: u64 = 0;
    let mut tstamp = start.and_hms(0, 0, 0);
    let end_t = end.and_hms(0, 0, 0);
    while tstamp < end_t {
        let candles = driver.get_candles(sym, &tstamp).await;
        if candles.is_empty() {
            panic!("not getting any candles");
        }
        total += storage.store(exchange, sym, &candles).await.expect("in storing data to DB");
        tstamp = candles.last().expect("last not present").tstamp;
        println!("{}", tstamp);
    }
    total
}
