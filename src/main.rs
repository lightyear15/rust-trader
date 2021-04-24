use chrono::prelude::*;
use std::path::PathBuf;
use structopt::StructOpt;
use trader::*;

#[derive(Debug, StructOpt)]
enum Trade {
    #[structopt(about = "import candles from exchage")]
    Import {
        exchange: String,
        symbol: String,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    },
    #[structopt(about = "backtest specific strategy")]
    Backtest {
        strategy: String,
        exchange: String,
        symbol: String,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    },
}

#[tokio::main]
async fn main() {
    let settings = configuration::Settings::get_configuration("trader.toml").expect("Failed at reading configuration");
    let opt = Trade::from_args();
    println!("{:?}", opt);
    match opt {
        Trade::Import {
            exchange,
            symbol,
            start,
            end,
        } => {
            println!("Import");
            let driver = drivers::create(&exchange, &settings).expect("exchange not found");
            let storage = storage::Candles::new(&settings.candle_storage).await;
            let candles = driver.get_candles(&symbol, &start.and_hms(0,0,0)).await;
            storage.store(&exchange, &symbol, &candles).await;
        }
        Trade::Backtest {
            strategy,
            exchange,
            symbol,
            start,
            end,
        } => {
            println!("Backtest");
        }
    };
}
