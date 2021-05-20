use crate::configuration::ExchangeSettings;
use crate::error::Error;
use crate::symbol::Symbol;
use crate::{binance, candles, orders, wallets};
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime};
use std::vec::Vec;

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
}

pub async fn create_rest_client(exchange: &str, config: &ExchangeSettings) -> Result<Box<dyn RestApi>, Error> {
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
    async fn submit(&self, order: &orders::Order);
    async fn cancel(&self, order_reference: i32);
}

pub async fn create_live_drivers(
    exchange: &str,
    config: &ExchangeSettings,
    ticks: &[Tick],
) -> Result<(Box<dyn RestApi>, Box<dyn LiveFeed>), Error> {
    match exchange {
        "binance" => {
            let rest = Box::new(binance::Rest::new(&config.api_key, &config.secret_key));
            let listen_key = rest.refresh_ws_token(None).await;
            let live = Box::new(binance::Live::new(ticks, &listen_key).await);
            Ok((rest, live))
        }
        _ => Err(Error::ErrNotFound(format!("can't find driver {}", exchange))),
    }
}
