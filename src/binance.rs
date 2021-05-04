use super::{candles, drivers, Error, Symbol};
use async_trait::async_trait;
use chrono::{NaiveDateTime,Duration};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, HOST, USER_AGENT};
use reqwest::Client;

#[derive(Debug, serde::Deserialize)]
struct Candle {
    open_time: u64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
    close_time: u64,
    quote_asset_volume: String,
    number_of_trades: u32,
    ignore1: String,
    ignore2: String,
    ignore3: String,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct SymbolInfo {
    symbol: String,
    #[serde(alias = "baseAsset")]
    base: String,
    #[serde(alias = "baseAssetPrecision")]
    base_precision: usize,
    #[serde(alias = "quoteAsset")]
    quote: String,
    #[serde(alias = "quoteAssetPrecision")]
    quote_precision: usize,
}
impl std::convert::From<SymbolInfo> for Symbol {
    fn from(sym : SymbolInfo) -> Self {
        Symbol {
            pretty : format!("{}-{}", &sym.base, &sym.quote),
            symbol: sym.symbol,
            base: sym.base,
            base_decimals: sym.base_precision,
            quote: sym.quote,
            quote_decimals: sym.quote_precision,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rest {
    default_headers: HeaderMap,
    url: String,
    api_key: String,
    client: Client,
}

impl Rest {
    pub fn new(api_key: &str) -> Rest {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(USER_AGENT, HeaderValue::from_static("trader/0.0.1"));
        default_headers.insert(HOST, HeaderValue::from_static("api.binance.com"));
        default_headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        Rest {
            default_headers,
            url: String::from("https://api.binance.com"),
            //url: String::from("http://localhost:8080"),
            client: Client::new(),
            api_key: String::from(api_key),
        }
    }
}

#[async_trait]
impl drivers::Importer for Rest {
    async fn get_candles(&self, sym: &str, start: &NaiveDateTime) -> Vec<candles::Candle> {
        let start_str = format!("{}000", start.timestamp());
        let url = self.url.clone() + "/api/v3/klines";
        let request = self
            .client
            .get(url)
            .headers(self.default_headers.clone())
            //.header("X-MBX-APIKEY", &self.api_key)
            .query(&[
                   ("symbol", sym),
                   ("interval", "1m"),
                   ("startTime", &start_str),
                   ("limit", "1000"),
                   //("limit", "10"),
            ]);
        request
            .send()
            .await
            .expect("in send binance klines request")
            .json::<Vec<Candle>>()
            .await
            .expect("in json<Vec<Candle>>")
            .iter()
            .map(|cnd| candles::Candle {
                open: cnd.open.parse::<f64>().expect("in cnd.open"),
                low: cnd.low.parse::<f64>().expect("in cnd.low"),
                high: cnd.high.parse::<f64>().expect("in cnd.high"),
                close: cnd.close.parse::<f64>().expect("in cnd.close"),
                volume: cnd.volume.parse::<f64>().expect("in cnd.volume"),
                tstamp: NaiveDateTime::from_timestamp((cnd.open_time / 1000) as i64, 0),
                tframe: Duration::minutes(1),
            })
        .collect()
    }
}

#[async_trait]
impl drivers::SymbolParser for Rest {
    async fn get_symbol(&self, sym: &str) -> Result<Symbol, Error> {
        let url = self.url.clone() + "api/v3/exchangeInfo";
        let request = self.client.get(url).headers(self.default_headers.clone());
        request
            .send()
            .await
            .expect("in send binance exchange info request")
            .json::<Vec<SymbolInfo>>()
            .await
            .expect("in json::<Vec<SymbolInfo>>")
            .drain(0..)
            .find(|sym_info| sym_info.symbol == sym)
            .map(|sym_info| sym_info.into())
            .ok_or(Error::ErrNotFound(format!("can't find symbol {}", sym)))
    }
}
