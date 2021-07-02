use crate::candles;
use crate::symbol::Symbol;
use crate::orders;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};

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
        let mut min_volume : f64 = 0.0;
        let mut price_min : f64 = 0.0;
        let mut volume_step : f64 = 0.0;
        let mut price_tick : f64 = 0.0;
        for filter in info.filters {
            match filter {
                SymbolFilter::LotSize {
                    min_qty,
                    max_qty: _,
                    step_size,
                } => {
                    min_volume = min_qty.parse::<f64>().expect("min_qty not an f64");
                    volume_step = step_size.parse::<f64>().expect("step size not an f64");
                },
                SymbolFilter::PriceFilter {
                    min_price,
                    max_price: _,
                    tick_price,
                } => {
                    price_min = min_price.parse::<f64>().expect("min_price not an f64");
                    price_tick = tick_price.parse::<f64>().expect("price tick not an f64");
                },
                _ => {},
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

#[derive(Debug, serde::Deserialize)]
pub(super) struct LiveCandleMsg {
    #[serde(alias = "E")]
    tstamp_open: u64,
    #[serde(alias = "s")]
    symbol: String,
    #[serde(alias = "k")]
    candle: Candle,
}
impl LiveCandleMsg {
    pub fn is_closed(&self) -> bool {
        self.candle.kline_close
    }
    pub fn name(&self) -> String {
        self.symbol.clone()
    }
}
impl From<LiveCandleMsg> for candles::Candle {
    fn from(msg: LiveCandleMsg) -> Self {
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


#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum Side {
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
