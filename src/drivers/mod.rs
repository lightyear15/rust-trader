use super::candles;
use super::configuration::Settings;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::vec::Vec;

#[async_trait]
pub trait Importer: std::fmt::Debug {
    async fn get_candles(&self, sym: &str, start: &NaiveDateTime, end: &NaiveDateTime) -> Vec<candles::Candle>;
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
    use reqwest::{multipart::Form, Client};

    #[derive(Debug, serde::Deserialize)]
    struct Candle {
        open_time: u64,
        open: String,
        high: String,
        low: String,
        close: String,
        volume: String,
        close_time: u64,
        quote_asset_volume : String,
        number_of_trades: u32,
        ignore1: String,
        ignore2: String,
        ignore3: String,
    }

    #[derive(Debug)]
    pub struct Driver {
        url: String,
        api_key: String,
        client: Client,
    }

    impl Driver {
        pub fn new(api_key: &str) -> Driver {
            Driver {
                url: String::from("https://api.binance.com"),
                client: Client::new(),
                api_key: String::from(api_key),
            }
        }
    }

    #[async_trait]
    impl super::Importer for Driver {
        async fn get_candles(&self, sym: &str, start: &NaiveDateTime, end: &NaiveDateTime) -> Vec<candles::Candle> {
            let symbol = String::from(sym);
            let form = Form::new()
                .text("symbol", symbol)
                .text("interval", "1m")
                .text("startTime", format!("{}000", start.timestamp()))
                .text("endTime", format!("{}000", end.timestamp()))
                .text("limit", "1000");
            let url = self.url.clone() + "/api/v3/klines";
            let _response = self
                .client
                .get(url)
                .header("X-MBX-APIKEY", &self.api_key)
                .multipart(form)
                .send()
                .await
                .expect("Binance klines request")
                .text()
                //.json::<Vec<Vec<String>>>()
                .await;
            println!("{:?}", _response);
            Vec::new()
        }
    }
}
