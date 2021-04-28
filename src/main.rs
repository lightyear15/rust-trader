use chrono::{Duration, NaiveDate};
use structopt::StructOpt;
use trader::*;

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
}

#[tokio::main]
async fn main() {
    let settings = configuration::Settings::get_configuration("trader.toml").expect("Failed at reading configuration");
    let opt = Trade::from_args();
    match opt {
        Trade::Import {
            exchange,
            symbol,
            start,
            end,
        } => {
            let driver = drivers::create_importer(&exchange, &settings).expect("exchange not found");
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
                .strategies.iter()
                .find(|settings| settings.name == strategy && settings.exchange == exchange && settings.symbol == symbol)
                .expect("no such strategy configuration");

            let res = backtest(&storage, &strategy, &exchange, &symbol, &cfg.time_frame, &start, &end).await.expect("backtest epic fail");
            println!("Backtest {}", res);
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

async fn backtest(
    storage: &storage::Candles,
    strat: &str,
    exchange: &str,
    sym: &str,
    time_frame: &chrono::Duration,
    start: &NaiveDate,
    end: &NaiveDate,
) -> Result<f64, Error> {
    let start_t = start.and_hms(0, 0, 0);
    let end_t = end.and_hms(0, 0, 0);

    let mut strategy = strategies::create(strat).expect("strategy does not exist");

    let cnds = storage.get(&exchange, &sym, &start_t, &end_t, time_frame, 3).await;
    for c in cnds {
        let action = strategy.on_new_candle();
        println!("{:?} -> {:?}", c, action);
    }
    Ok(0.0)
}
