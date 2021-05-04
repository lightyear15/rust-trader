use super::{binance, candles, configuration::ExchangeSettings, Error, Symbol, orders};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::vec::Vec;

#[async_trait]
pub trait Importer: std::fmt::Debug {
    async fn get_candles(&self, sym: &str, start: &NaiveDateTime) -> Vec<candles::Candle>;
}

pub fn create_importer(exchange: &str, config: &ExchangeSettings) -> Result<Box<dyn Importer>, super::Error> {
    match exchange {
        "binance" => Ok(Box::new(binance::Rest::new(&config.api_key))),
        _ => {
            Err(super::Error::ErrNotFound(format!("can't find driver {}", exchange)))
        }
    }
}

#[async_trait]
pub trait SymbolParser: std::fmt::Debug {
    async fn get_symbol(&self, sym: &str) -> Result<Symbol, Error>;
}

pub fn create_symbol_parser(exchange: &str, config: &ExchangeSettings) -> Result<Box<dyn SymbolParser>, super::Error> {
    match exchange {
        "binance" => Ok(Box::new(binance::Rest::new(&config.api_key))),
        _ => {
            Err(super::Error::ErrNotFound(format!("can't find driver {}", exchange)))
        }
    }
}


pub enum LiveSpotMessage {
    Transaction(orders::Transaction),
    Candle(String, candles::Candle)
}

#[async_trait]
pub trait LiveExchange {
    async fn next(&mut self) -> LiveSpotMessage;
    async fn submit(&self, order :&orders::Order);
    async fn cancel(&self, order_reference: i32);
} 
