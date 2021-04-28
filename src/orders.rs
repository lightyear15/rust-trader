use chrono::{Duration, NaiveDateTime};

#[derive(PartialEq, Clone, Debug)]
pub enum TimeInForce {
    Gtc,
    Fok,
    Ioc,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Type {
    Market,
    Limit(f64),
    TakeProfit(f64),
    StopLoss(f64),
    StopLossLimit(f64, f64, TimeInForce),
    TakeProfitLimit(f64, f64, TimeInForce),
}

#[derive(PartialEq, Clone, Debug)]
pub enum Side {
    Buy,
    Sell,
}

pub type Id = i64;

#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub symbol: String,
    pub side: Side,
    pub order_id: Id,
    pub avg_price: f64,
    pub volume: f64,
    pub tstamp: NaiveDateTime,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Status {
    Open(Info),
    Filled(Transaction),
    Partial(Transaction),
    Canceled,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Info {
    pub symbol: String,
    pub side: Side,
    pub o_type: Type,
    pub volume: f64,
    pub expire: Option<Duration>,
    pub reference: i32,
}
