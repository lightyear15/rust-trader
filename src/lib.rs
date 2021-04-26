#![allow(dead_code)]

use chrono::{Duration, NaiveDateTime};
use rand::Rng;

pub mod candles;
pub mod configuration;
pub mod drivers;
pub mod storage;
pub mod binance;
//pub mod brokers;
//pub mod csv_file;
//pub mod market;
//pub mod order;
//pub mod replayer;
//pub mod strategies;
//pub mod strategy;

//pub use strategy::Strategy;
//pub use replayer::Replayer;
//pub use csv_file::CSVFile;

#[derive(Debug)]
pub enum Error {
    ErrNotFound,
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
//#[allow(dead_code)]
//pub struct Transaction {
//symbol : String,
//price : f64,
//volume : f64,
//reference: i32
//}

//#[allow(dead_code)]
//pub struct Position {
//average_entry_price: f64,
//volume : f64,
//}

//#[allow(dead_code)]
//impl Position {
//fn new() -> Self {
//Position{average_entry_price: 0.0, volume: 0.0}
//}
//fn collect(&self, tx: &Transaction) -> Self {
//let tot_volume = self.volume + tx.volume;
//let avg_price = (self.average_entry_price * self.volume + tx.price * tx.volume) / tot_volume;
//Position{ average_entry_price : avg_price, volume: tot_volume}
//}
//}
