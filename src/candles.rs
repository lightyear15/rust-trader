use serde::Deserialize;
use std::fmt;
use chrono::{NaiveDateTime, Duration};

#[derive(Debug, Clone, Copy)]
pub struct Candle {
    pub tstamp: NaiveDateTime, // refers to start timestamp
    pub tframe: Duration,

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

impl Candle {
    pub fn get_time_interval(&self) -> (NaiveDateTime, NaiveDateTime) {
        (self.tstamp.clone(), self.tstamp + self.tframe)
    }
}

fn deserialize_from_str<'de, D>(deserializer: D) -> Result<chrono::NaiveDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(serde::de::Error::custom)
}
