use super::candles;
use super::configuration::Settings;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::vec::Vec;

#[async_trait]
pub trait Importer: std::fmt::Debug {
    async fn get_candles(&self, sym: &str, start: &NaiveDateTime) -> Vec<candles::Candle>;
}

pub fn create(exchange: &str, config: &Settings) -> Result<Box<dyn Importer>, super::Error> {
    match exchange {
        "binance" => Ok(Box::new(binance::Driver::new(&config.binance.api_key))),
        _ => {
            println!("i dont know");
            Err(super::Error::ErrNotFound)
        }
    }
}

pub mod binance {
    use super::candles;
    use async_trait::async_trait;
    use chrono::NaiveDateTime;
    use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, HOST, USER_AGENT};
    use reqwest::Client;

    #[derive(Debug, serde::Deserialize)]
    struct Candle {
        open_time: u64,
        open: String,
        high: String,
        low: String,
        close: String,
        volume: String,
        close_time: u64,
        quote_asset_volume: String,
        number_of_trades: u32,
        ignore1: String,
        ignore2: String,
        ignore3: String,
    }

    #[derive(Debug)]
    pub struct Driver {
        default_headers: HeaderMap,
        url: String,
        api_key: String,
        client: Client,
    }

    impl Driver {
        pub fn new(api_key: &str) -> Driver {
            let mut default_headers = HeaderMap::new();
            default_headers.insert(USER_AGENT, HeaderValue::from_static("trader/0.0.1"));
            default_headers.insert(HOST, HeaderValue::from_static("api.binance.com"));
            default_headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
            Driver {
                default_headers,
                url: String::from("https://api.binance.com"),
                //url: String::from("http://localhost:8080"),
                client: Client::new(),
                api_key: String::from(api_key),
            }
        }
    }

    #[async_trait]
    impl super::Importer for Driver {
        async fn get_candles(&self, sym: &str, start: &NaiveDateTime) -> Vec<candles::Candle> {
            let start_str = format!("{}000", start.timestamp());
            let url = self.url.clone() + "/api/v3/klines";
            let request = self
                .client
                .get(url)
                .headers(self.default_headers.clone())
                .header("X-MBX-APIKEY", &self.api_key)
                .query(&[
                    ("symbol", sym),
                    ("interval", "1m"),
                    ("startTime", &start_str),
                    ("limit", "1000"),
                    //("limit", "10"),
                ]);
            request
                .send()
                .await
                .expect("in send binance klines request")
                .json::<Vec<Candle>>()
                .await
                .expect("in json<Vec<Candle>>")
                .iter()
                .map(|cnd| candles::Candle {
                    open: cnd.open.parse::<f64>().expect("in cnd.open"),
                    low: cnd.low.parse::<f64>().expect("in cnd.low"),
                    high: cnd.high.parse::<f64>().expect("in cnd.high"),
                    close: cnd.close.parse::<f64>().expect("in cnd.close"),
                    volume: cnd.volume.parse::<f64>().expect("in cnd.volume"),
                    //tstamp: NaiveDateTime::from_timestamp(((cnd.close_time + 1) / 1000) as i64, 0),
                    tstamp: NaiveDateTime::from_timestamp((cnd.open_time  / 1000) as i64, 0),
                })
                .collect()
        }
    }
}
