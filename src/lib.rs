#![allow(dead_code)]

use chrono::{Duration, NaiveDateTime};
use rand::Rng;

pub mod binance;
pub mod candles;
pub mod configuration;
pub mod drivers;
pub mod orders;
pub mod storage;
pub mod strategies;
pub mod wallets;
pub mod backtest;
//pub mod brokers;
//pub mod csv_file;
//pub mod market;
//pub mod replayer;

pub use strategies::SpotSinglePairStrategy;
pub use backtest::backtest;
//pub use replayer::Replayer;
//pub use csv_file::CSVFile;

#[derive(Debug)]
pub enum Error {
    ErrNotFound(String),
    ErrTimeFrameNotSupported,
    Unexpected(Box<dyn std::error::Error>),
    Unknown,       // to be removed
    Unimplemented, // to be removed
    Done,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Symbol {
    symbol: String,
    pretty: String,
    base: String,
    base_decimals: usize,
    quote: String,
    quote_decimals: usize,
}

impl std::string::ToString for Symbol {
    fn to_string(&self) -> String {
        self.pretty.clone()
    }
}

fn generate_random_tstamp(start: &NaiveDateTime, end: &NaiveDateTime) -> NaiveDateTime {
    let time_frame = end.signed_duration_since(*start);
    let mut rng = rand::thread_rng();
    let rand_time = Duration::seconds(rng.gen_range(0..time_frame.num_seconds()));
    *start + rand_time
}
