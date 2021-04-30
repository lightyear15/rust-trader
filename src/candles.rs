use serde::Deserialize;
//use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Candle {
    pub tstamp: chrono::NaiveDateTime, // refers to start timestamp
    pub tframe: chrono::Duration,

    pub open: f64,
    pub close: f64,
    pub low: f64,
    pub high: f64,
    pub volume :f64,
}

impl fmt::Display for Candle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return write!(f, "[{} [{} -> {}]", self.tstamp, self.open, self.close);
    }
}

/*
pub struct CandleHistory {
    pub history: Vec<Candle>,
    pub symbol: String,
    pub time_frame: chrono::Duration,
}

impl CandleHistory {
    pub fn import_from_csv(symbol: &str, frame: &chrono::Duration, filename: &std::path::Path) -> Result<Self, Box<dyn Error>> {
        let candles: Vec<Candle> = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(filename)?
            .deserialize()
            .collect::<Result<Vec<Candle>, csv::Error>>()?;
        Ok(CandleHistory {
            history: candles,
            symbol: String::from(symbol),
            time_frame: *frame,
        })
    }
    pub fn get_nth_candle(&self, n: usize) -> Option<&Candle> {
        self.history.get(n)
    }
    pub fn get_oldest_candles(&self, n: usize) -> Vec<Candle> {
        let sz = std::cmp::min(n, self.history.len());
        self.history[0..sz].to_vec()
    }
    pub fn get_last_candles(&self, n: usize) -> Vec<Candle> {
        let sz = std::cmp::min(0, self.history.len() - n);
        self.history[sz..].to_vec()
    }
}

impl fmt::Display for CandleHistory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} - {} time_frame", self.symbol, self.time_frame)?;
        if self.history.is_empty() {
            return write!(f, "\nempty history");
        }
        write!(
            f,
            "\n From {} to {}",
            self.history.first().unwrap().tstamp,
            self.history.last().unwrap().tstamp
        )
    }
}
*/

fn deserialize_from_str<'de, D>(deserializer: D) -> Result<chrono::NaiveDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(serde::de::Error::custom)
}
