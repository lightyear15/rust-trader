use chrono::NaiveDateTime;
use rand::prelude::random;
use crate::symbol::Symbol;

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
    StopLoss(f64),
    //TakeProfit(f64),
    //StopLossLimit(f64, f64, TimeInForce),
    //TakeProfitLimit(f64, f64, TimeInForce),
}

#[derive(PartialEq, Clone, Debug)]
pub enum Side {
    Buy,
    Sell,
}

impl ToString for Side {
    fn to_string(&self) -> String {
        match self {
            Side::Sell => String::from("Sell"),
            Side::Buy => String::from("Buy"),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum OrderStatus {
    Accepted,
    Rejected(String),
    Filled(Transaction),
    Canceled,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Order {
    pub tstamp: Option<NaiveDateTime>,
    pub exchange: String,
    pub symbol: Symbol,
    pub side: Side,
    pub o_type: Type,
    pub volume: f64,
    pub expire: Option<chrono::NaiveDateTime>,
    pub id: u32,
    pub tx_ref: u32,
}
impl Order {
    pub fn new() -> Self {
        Self {
            tstamp : None,
            exchange: String::new(),
            symbol: Symbol::default(),
            side: Side::Buy,
            o_type: Type::Market,
            volume: 0.0,
            expire: None,
            id: random::<u16>() as u32,
            tx_ref : 0,
        }
    }
}
impl Default for Order {
    fn default() -> Self {
        Order::new()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub symbol: String,
    pub side: Side,
    pub avg_price: f64,
    pub volume: f64,
    pub tstamp: NaiveDateTime,
    pub fees: f64,
    pub fees_asset: String,
    pub order: Order,
}
impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            symbol: String::new(),
            side: Side::Buy,
            avg_price: 0.0,
            volume: 0.0,
            fees: 0.0,
            fees_asset: String::new(),
            tstamp: NaiveDateTime::MAX, // the transaction that never happened it's in the future
            order: Order::default(),
        }
    }
}
