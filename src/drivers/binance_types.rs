use crate::candles;
use crate::orders;
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use scan_fmt::scan_fmt;
use std::collections::HashMap;

#[derive(Debug, serde::Deserialize, Clone)]
#[serde(tag = "filterType")]
enum SymbolFilter {
    #[serde(alias = "LOT_SIZE")]
    LotSize {
        #[serde(alias = "minQty")]
        min_qty: String,
        #[serde(alias = "maxQty")]
        max_qty: String,
        #[serde(alias = "stepSize")]
        step_size: String,
    },
    #[serde(alias = "MARKET_LOT_SIZE")]
    MarketLotSize {
        #[serde(alias = "minQty")]
        min_qty: String,
        #[serde(alias = "maxQty")]
        max_qty: String,
        #[serde(alias = "stepSize")]
        step_size: String,
    },
    #[serde(alias = "PRICE_FILTER")]
    PriceFilter {
        #[serde(alias = "minPrice")]
        min_price: String,
        #[serde(alias = "maxPrice")]
        max_price: String,
        #[serde(alias = "tickSize")]
        tick_price: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub(super) struct SymbolInfo {
    pub(super) symbol: String,
    #[serde(alias = "baseAsset")]
    base: String,
    #[serde(alias = "baseAssetPrecision")]
    base_precision: usize,
    #[serde(alias = "quoteAsset")]
    quote: String,
    #[serde(alias = "quoteAssetPrecision")]
    quote_precision: usize,
    filters: Vec<SymbolFilter>,
}
impl From<SymbolInfo> for Symbol {
    fn from(info: SymbolInfo) -> Self {
        let mut min_volume: f64 = 0.0;
        let mut price_min: f64 = 0.0;
        let mut volume_step: f64 = 0.0;
        let mut price_tick: f64 = 0.0;
        for filter in info.filters {
            match filter {
                SymbolFilter::LotSize {
                    min_qty,
                    max_qty: _,
                    step_size,
                } => {
                    min_volume = min_qty.parse::<f64>().expect("min_qty not an f64");
                    volume_step = step_size.parse::<f64>().expect("step size not an f64");
                }
                SymbolFilter::PriceFilter {
                    min_price,
                    max_price: _,
                    tick_price,
                } => {
                    price_min = min_price.parse::<f64>().expect("min_price not an f64");
                    price_tick = tick_price.parse::<f64>().expect("price tick not an f64");
                }
                _ => {}
            }
        }
        Self {
            pretty: format!("{}-{}", &info.base, &info.quote),
            symbol: info.symbol,
            base: info.base,
            base_decimals: info.base_precision,
            quote: info.quote,
            quote_decimals: info.quote_precision,
            min_volume,
            min_price: price_min,
            volume_step,
            price_tick,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct Candle {
    #[serde(alias = "t")]
    tstamp_open: u64,
    #[serde(alias = "o")]
    open: String,
    #[serde(alias = "h")]
    high: String,
    #[serde(alias = "l")]
    low: String,
    #[serde(alias = "c")]
    close: String,
    #[serde(alias = "v")]
    volume: String,
    #[serde(alias = "T")]
    tstamp_close: u64,
    #[serde(alias = "q")]
    quote_asset_volume: String,
    #[serde(alias = "n")]
    number_of_trades: u32,
    #[serde(default)]
    ignore1: String,
    #[serde(default)]
    ignore2: String,
    #[serde(default)]
    ignore3: String,
    #[serde(alias = "x", default)]
    kline_close: bool,
}
impl From<Candle> for candles::Candle {
    fn from(cnd: Candle) -> Self {
        if cnd.tstamp_close < cnd.tstamp_open {
            panic!("close {}, open {}", cnd.tstamp_close, cnd.tstamp_open);
        }
        Self {
            open: cnd.open.parse::<f64>().expect("in cnd.open"),
            low: cnd.low.parse::<f64>().expect("in cnd.low"),
            high: cnd.high.parse::<f64>().expect("in cnd.high"),
            close: cnd.close.parse::<f64>().expect("in cnd.close"),
            volume: cnd.volume.parse::<f64>().expect("in cnd.volume"),
            tstamp: NaiveDateTime::from_timestamp((cnd.tstamp_open / 1000) as i64, 0),
            tframe: Duration::milliseconds((cnd.tstamp_close - cnd.tstamp_open) as i64 + 1),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(super) enum Side {
    #[serde(alias = "BUY")]
    Buy,
    #[serde(alias = "SELL")]
    Sell,
}
impl From<Side> for orders::Side {
    fn from(side: Side) -> Self {
        match side {
            Side::Buy => orders::Side::Buy,
            Side::Sell => orders::Side::Sell,
        }
    }
}
impl From<orders::Side> for Side {
    fn from(side: orders::Side) -> Self {
        match side {
            orders::Side::Buy => Side::Buy,
            orders::Side::Sell => Side::Sell,
        }
    }
}
impl ToString for Side {
    fn to_string(&self) -> String {
        match self {
            Side::Sell => String::from("SELL"),
            Side::Buy => String::from("BUY"),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct LiveMessage {
    //{"stream":"btceur@kline_1m","data": {}}
    stream: String,
    pub data: LiveMessageType,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "e")]
pub(super) enum LiveMessageType {
    #[serde(alias = "kline")]
    LiveCandle(LiveCandle),
    #[serde(alias = "executionReport")]
    OrderUpdate(LiveOrderUpdate),
    #[serde(alias = "outboundAccountPosition")]
    AccountUpdate(LiveAccountUpdate),
    #[serde(alias = "balanceUpdate")]
    BalanceUpdate(BalanceUpdate),
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct LiveCandle {
    #[serde(alias = "E")]
    tstamp_open: u64,
    #[serde(alias = "s")]
    symbol: String,
    #[serde(alias = "k")]
    candle: Candle,
}
impl LiveCandle {
    pub(super) fn is_closed(&self) -> bool {
        self.candle.kline_close
    }
    pub(super) fn name(&self) -> String {
        self.symbol.clone()
    }
}
impl From<LiveCandle> for candles::Candle {
    fn from(msg: LiveCandle) -> Self {
        let start = NaiveDateTime::from_timestamp((msg.candle.tstamp_open / 1000) as i64, 0);
        let stop = NaiveDateTime::from_timestamp((msg.candle.tstamp_close / 1000) as i64, 0);
        let dur = stop - start + Duration::milliseconds(1);
        Self {
            open: msg.candle.open.parse::<f64>().expect("in cnd.open"),
            low: msg.candle.low.parse::<f64>().expect("in cnd.low"),
            high: msg.candle.high.parse::<f64>().expect("in cnd.high"),
            close: msg.candle.close.parse::<f64>().expect("in cnd.close"),
            volume: msg.candle.volume.parse::<f64>().expect("in cnd.volume"),
            tstamp: start,
            tframe: dur,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct LiveOrderUpdate {
    #[serde(alias = "E")]
    tstamp: u64,
    #[serde(alias = "s")]
    symbol: String,
    #[serde(alias = "c")]
    order_id: String,
    #[serde(alias = "X")]
    pub order_status: OrderStatus,
    #[serde(alias = "S")]
    side: Side,
    #[serde(alias = "Z")]
    cumulative_price: String,
    #[serde(alias = "z")]
    cumulative_quantity: String,
    // order related stuff
    #[serde(alias = "q")]
    order_quantity: String,
    #[serde(alias = "p")]
    order_price: String,
    #[serde(alias = "o")]
    order_type: Type,
    #[serde(alias = "n")]
    commission_amount: String,
    #[serde(alias = "N")]
    commission_asset: Option<String>,
}
impl From<LiveOrderUpdate> for orders::Transaction {
    fn from(msg: LiveOrderUpdate) -> Self {
        let tot_quantity = msg.cumulative_quantity.parse::<f64>().expect("in cumulative_quantity");
        let tot_price = msg.cumulative_price.parse::<f64>().expect("in cumulative_price");
        let fees = msg.commission_amount.parse::<f64>().expect("in commission_asset");
        let mut id: u32 = 0;
        let mut tx_ref : u32 = 0;
        if let Ok((sc_id, sc_tx_ref)) = scan_fmt!(&msg.order_id, "{d}_{d}", u32, u32) {
            id = sc_id;
            tx_ref = sc_tx_ref;
        } else if let Ok(sc_id) = msg.order_id.parse::<u32>() {
            id = sc_id;
        }
        Self {
            tstamp: NaiveDateTime::from_timestamp((msg.tstamp / 1000) as i64, 0),
            symbol: msg.symbol.clone(),
            side: msg.side.clone().into(),
            avg_price: tot_price / tot_quantity,
            volume: tot_quantity,
            fees,
            fees_asset: msg.commission_asset.unwrap_or_default(),
            order: orders::Order {
                volume: msg.order_quantity.parse::<f64>().expect("in msg.order_quantity"),
                exchange: String::from("binance"),
                expire: None,
                side: msg.side.into(),
                symbol: Symbol::new(msg.symbol),
                id,
                o_type: to_type(&msg.order_type, msg.order_price.parse::<f64>().expect("in msg.order_price")),
                tx_ref,
            },
        }
    }
}

fn to_type(msg_o_type: &Type, o_price: f64) -> orders::Type {
    match msg_o_type {
        Type::Limit => orders::Type::Limit(o_price),
        Type::Market => orders::Type::Market,
        _ => panic!("unknown type"),
    }
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct LiveAccountUpdate {
    #[serde(alias = "E")]
    tstamp: u64,
    #[serde(alias = "B")]
    balances: Vec<Balance>,
}
impl From<LiveAccountUpdate> for SpotWallet {
    fn from(msg: LiveAccountUpdate) -> Self {
        msg.balances.into()
    }
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct Balance {
    #[serde(alias = "a")]
    asset: String,
    #[serde(alias = "f")]
    free: String,
    #[serde(alias = "l")]
    locked: String,
}
impl From<Vec<Balance>> for SpotWallet {
    fn from(mut msg: Vec<Balance>) -> Self {
        Self {
            assets: msg
                .drain(0..)
                .map(|balance| (balance.asset, balance.free.parse::<f64>().expect("in balance.free")))
                .collect::<HashMap<_, _>>(),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct BalanceUpdate {
    #[serde(alias = "E")]
    tstamp: u64,
    #[serde(alias = "a")]
    pub asset: String,
    #[serde(alias = "d")]
    pub delta: String,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub(super) struct ExchangeInfo {
    pub symbols: Vec<SymbolInfo>,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub(super) struct ListenKey {
    #[serde(alias = "listenKey")]
    pub listen_key: String,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct AccountStatusData {
    pub balances: Vec<Balance>,
    #[serde(alias = "totalAssetOfBtc")]
    total: String,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct AccountStatusSnapshot {
    pub data: AccountStatusData,
    #[serde(alias = "updateTime")]
    pub tstamp: u64,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct AccountStatus {
    #[serde(alias = "snapshotVos")]
    pub snapshot: Vec<AccountStatusSnapshot>,
    msg: String,
    code: i16,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(super) enum Type {
    #[serde(alias = "MARKET")]
    Market,
    #[serde(alias = "LIMIT")]
    Limit,
    #[serde(alias = "STOP_LOSS")]
    StopLoss,
}

#[derive(Debug, serde::Deserialize)]
pub(super) enum OrderStatus {
    #[serde(alias = "NEW")]
    New,
    #[serde(alias = "PARTIALLY_FILLED")]
    PartiallyFilled,
    #[serde(alias = "FILLED")]
    Filled,
    #[serde(alias = "CANCELED")]
    Canceled,
    #[serde(alias = "PENDING_CANCEL")]
    PendingCancel,
    #[serde(alias = "REJECTED")]
    Rejected,
    #[serde(alias = "EXPIRED")]
    Expired,
}
