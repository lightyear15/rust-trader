use super::{binance, candles, configuration::Settings, Error, Symbol};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::vec::Vec;

#[async_trait]
pub trait Importer: std::fmt::Debug {
    async fn get_candles(&self, sym: &str, start: &NaiveDateTime) -> Vec<candles::Candle>;
}

pub fn create_importer(exchange: &str, config: &Settings) -> Result<Box<dyn Importer>, super::Error> {
    match exchange {
        "binance" => Ok(Box::new(binance::Rest::new(&config.binance.api_key))),
        _ => {
            Err(super::Error::ErrNotFound(format!("can't find driver {}", exchange)))
        }
    }
}

#[async_trait]
pub trait SymbolParser: std::fmt::Debug {
    async fn get_symbol(&self, sym: &str) -> Result<Symbol, Error>;
}

pub fn create_symbol_parser(exchange: &str, config: &Settings) -> Result<Box<dyn SymbolParser>, super::Error> {
    match exchange {
        "binance" => Ok(Box::new(binance::Rest::new(&config.binance.api_key))),
        _ => {
            Err(super::Error::ErrNotFound(format!("can't find driver {}", exchange)))
        }
    }
}
