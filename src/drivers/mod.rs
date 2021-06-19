use crate::configuration::ExchangeSettings;
use crate::error::Error;
use crate::symbol::Symbol;
use crate::{candles, orders, wallets};
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime};
use crate::orders::{Order, Transaction, OrderStatus};
use std::vec::Vec;

pub mod binance;
pub mod binance_types;

#[async_trait(?Send)]
pub trait RestApi {
    async fn get_candles(
        &self,
        sym: &str,
        interval: Option<&Duration>,
        start: Option<&NaiveDateTime>,
        limit: Option<usize>,
    ) -> Vec<candles::Candle>;
    async fn get_symbol_info(&self, sym: &str) -> Result<Symbol, Error>;
    async fn get_wallet(&self) -> Result<wallets::SpotWallet,Error>;
    async fn refresh_ws_token(&self, old_token: Option<String>) -> String;
    async fn send_order(&self, order : Order) -> OrderStatus;
    async fn cancel_order(&self, symbol: String, order_id : u32) -> OrderStatus;
}

pub fn create_rest_client(exchange: &str, config: &ExchangeSettings) -> Result<Box<dyn RestApi>, Error> {
    match exchange {
        "binance" => Ok(Box::new(binance::Rest::new(&config.api_key, &config.secret_key))),
        _ => Err(Error::ErrNotFound(format!("can't find driver {}", exchange))),
    }
}

#[derive(Debug)]
pub enum LiveEvent {
    ReconnectionRequired,
    None,
    Generic(String),
    Transaction(orders::Transaction),
    NewOrder(orders::Order),
    Candle(String, candles::Candle),
    Balance(wallets::SpotWallet),
}

#[derive(Clone)]
pub struct Tick {
    pub sym: String,
    pub interval: chrono::Duration,
}

#[async_trait(?Send)]
pub trait LiveFeed {
    async fn next(&mut self) -> LiveEvent;
    fn token(&self) -> String;
    async fn reconnect(&mut self, new_key: String);

}

pub async fn create_live_driver(exchange :&str, listen_key: String, ticks: Vec<Tick>) -> Result<Box<dyn LiveFeed>, Error> {
    match exchange {
        "binance" => {
            let live = Box::new(binance::Live::new(ticks, listen_key).await);
            Ok(live)
        }
        _ => Err(Error::ErrNotFound(format!("can't find driver {}", exchange))),
    }
}
