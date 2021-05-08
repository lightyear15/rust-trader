use crate::error::Error;
use crate::symbol::Symbol;
use crate::{candles, drivers};
use actix::{Actor, ActorContext, AsyncContext, Running};
use actix_web_actors::ws;
use async_trait::async_trait;
use chrono::{Duration, NaiveDateTime};
use std::time::Instant;

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
struct ExchangeInfo {
    symbols: Vec<SymbolInfo>,
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

#[derive(Clone)]
pub struct Rest {
    url: String,
    api_key: String,
    client: awc::Client,
}

impl Rest {
    pub fn new(api_key: &str) -> Rest {
        let client = awc::Client::builder()
            .header("User-Agent", "trader/0.0.1")
            .header("Host", "api.binance.com")
            .header("Accept", "*/*")
            .finish();
        Rest {
            url: String::from("https://api.binance.com"),
            //url: String::from("http://localhost:8080"),
            client,
            api_key: String::from(api_key),
        }
    }
}

#[async_trait(?Send)]
impl drivers::Importer for Rest {
    async fn get_candles(&self, sym: &str, start: &NaiveDateTime) -> Vec<candles::Candle> {
        let start_str = format!("{}000", start.timestamp());
        let url = self.url.clone() + "/api/v3/klines";
        let request = self
            .client
            .get(url)
            .set_header("X-MBX-APIKEY", self.api_key.as_str())
            .query(&[
                ("symbol", sym),
                ("interval", "1m"),
                ("startTime", &start_str),
                ("limit", "1000"),
                //("limit", "10"),
            ])
            .expect("in adding queries");
        request
            .send()
            .await
            .expect("in send binance klines request")
            .json::<Vec<Candle>>()
            .limit(128_000_000)
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

#[async_trait(?Send)]
impl drivers::SymbolParser for Rest {
    async fn get_symbol(&self, sym: &str) -> Result<Symbol, Error> {
        let url = self.url.clone() + "/api/v3/exchangeInfo";
        let request = self.client.get(url);
        let mut info = request
            .send()
            .await
            .expect("in send binance exchange info request")
            .json::<ExchangeInfo>()
            .limit(128_000_000)
            .await
            .expect("in json::<ExchangeInfo>");

        let x = info
            .symbols
            .drain(0..)
            .find(|sym_info| sym_info.symbol == sym)
            .map(to_symbol)
            .ok_or_else(|| Error::ErrNotFound(format!("can't find symbol {}", sym)));
        x
    }
}

fn to_symbol(sym: SymbolInfo) -> Symbol {
    Symbol {
        pretty: format!("{}-{}", &sym.base, &sym.quote),
        symbol: sym.symbol,
        base: sym.base,
        base_decimals: sym.base_precision,
        quote: sym.quote,
        quote_decimals: sym.quote_precision,
    }
}

struct Tick {
    sym: String,
    interval: Duration,
}

pub struct Live {
    ticks: Vec<Tick>,
    url: String,
    api_key: String,
    secret_key: String,
    hb: Instant,
    //client: awc::Client,
}

impl Actor for Live {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::hours(24).to_std().unwrap(), |act, ctx| {});
    }
    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        Running::Stop
    }
}

impl Live {
    pub fn new() -> Self {
        Self {
            hb: Instant::now(),
            ticks: Vec::new(),
            url: String::new(),
            api_key: String::new(),
            secret_key: String::new(),
        }
    }
    fn hb(&self, cont: &mut ws::WebsocketContext<Self>) {
        // TODO find min among ticks
        let hb_inte = Duration::hours(24).to_std().unwrap();
        let max_ttl = Duration::minutes(3).to_std().unwrap();
        cont.run_interval(hb_inte, move |act, ctx| {
            let nnow = Instant::now();
            if nnow.duration_since(act.hb) > max_ttl {
                println!("Disconnecting failed heartbeat");
                ctx.stop();
                return;
            }

            ctx.ping(b"hello");
        });
    }
}

impl actix::StreamHandler<Result<ws::Message, ws::ProtocolError>> for Live {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(b)) => {
                self.hb = Instant::now();
                ctx.pong(&b);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Text(text)) => {
                self.hb = Instant::now();
            }
            Ok(_) => {
                println!("don't really know what to do");
            }
            Err(e) => {
                panic!("binance Live {}", e);
            }
        }
    }
}
