use chrono::NaiveDateTime;

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
    //TakeProfit(f64),
    //StopLoss(f64),
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

pub type Id = i64;

#[derive(PartialEq, Clone, Debug)]
pub enum OrderStatus {
    Accepted,
    Rejected,
    Filled(Transaction),
    Canceled,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Order {
    pub exchange: String,
    pub symbol: String,
    pub side: Side,
    pub o_type: Type,
    pub volume: f64,
    pub expire: Option<chrono::NaiveDateTime>,
    pub reference: i32,
}
impl Default for Order {
    fn default() -> Self {
        Order {
            exchange: String::new(),
            symbol: String::new(),
            side: Side::Buy,
            o_type: Type::Market,
            volume: 0.0,
            expire: None,
            reference: 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub symbol: String,
    pub side: Side,
    pub avg_price: f64,
    pub volume: f64,
    pub tstamp: NaiveDateTime,
    pub order: Order,
}
impl Default for Transaction {
    fn default() -> Self {
        Transaction {
            symbol: String::new(),
            side: Side::Buy,
            avg_price: 0.0,
            volume: 0.0,
            tstamp: chrono::naive::MAX_DATETIME, // the transaction that never happened it's in the future
            order: Order::default(),
        }
    }
}
