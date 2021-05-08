use crate::{binance, candles, orders};
use crate::configuration::ExchangeSettings;
use crate::error::Error;
use crate::symbol::Symbol;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use std::vec::Vec;

#[async_trait(?Send)]
pub trait Importer {
    async fn get_candles(&self, sym: &str, start: &NaiveDateTime) -> Vec<candles::Candle>;
}

pub fn create_importer(exchange: &str, config: &ExchangeSettings) -> Result<Box<dyn Importer>, Error> {
    match exchange {
        "binance" => Ok(Box::new(binance::Rest::new(&config.api_key))),
        _ => {
            Err(Error::ErrNotFound(format!("can't find driver {}", exchange)))
        }
    }
}

#[async_trait(?Send)]
pub trait SymbolParser {
    async fn get_symbol(&self, sym: &str) -> Result<Symbol, Error>;
}

pub fn create_symbol_parser(exchange: &str, config: &ExchangeSettings) -> Result<Box<dyn SymbolParser>, Error> {
    match exchange {
        "binance" => Ok(Box::new(binance::Rest::new(&config.api_key))),
        _ => {
            Err(Error::ErrNotFound(format!("can't find driver {}", exchange)))
        }
    }
}


pub enum LiveSpotMessage {
    Transaction(orders::Transaction),
    Candle(String, candles::Candle)
}

#[async_trait(?Send)]
pub trait LiveExchange {
    async fn next(&mut self) -> LiveSpotMessage;
    async fn submit(&self, order :&orders::Order);
    async fn cancel(&self, order_reference: i32);
} 
