use crate::candles;
use crate::drivers::{LiveEvent, LiveFeed, RestApi, Tick};
use crate::error::Error;
use crate::orders::Order;
use crate::symbol::Symbol;
use async_trait::async_trait;
use awc::ws::{Codec, Frame};
use awc::{BoxedSocket, Client};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::sign::Signer;
use serde_json;

#[derive(Clone)]
pub struct Rest {
    url: String,
    api_key: String,
    secret: PKey<Private>,
    client: Client,
}

impl Rest {
    pub fn new(api_key: &str, secret: &str) -> Rest {
        let secret = PKey::hmac(secret.as_bytes()).expect("cannot create private key from secret");
        let client = Client::builder()
            .header("User-Agent", "trader/0.0.1")
            .header("Host", "api.binance.com")
            .header("X-MBX-APIKEY", api_key)
            .finish();
        Rest {
            url: String::from("https://api.binance.com"),
            client,
            api_key: String::from(api_key),
            secret,
        }
    }
}

#[async_trait(?Send)]
impl RestApi for Rest {
    async fn refresh_ws_token(&self, old_token: Option<String>) -> String {
        let url = self.url.clone() + "/api/v3/userDataStream";
        let request = if let Some(token) = old_token {
            self.client
                .put(url)
                .query(&["listen_key", &token])
                .expect("in building put ws token")
        } else {
            self.client.post(url)
        };
        let key = request
            .send()
            .await
            .expect("in sending listen_key request")
            .json::<ListenKey>()
            .await
            .expect("in parsing userDataStream response")
            .listen_key;
        key
    }

    async fn get_symbol_info(&self, sym: &str) -> Result<Symbol, Error> {
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

    async fn get_candles(&self, sym: &str, start: &NaiveDateTime) -> Vec<candles::Candle> {
        let start_str = format!("{}000", start.timestamp());
        let url = self.url.clone() + "/api/v3/klines";
        let request = self
            .client
            .get(url)
            .set_header("X-MBX-APIKEY", self.api_key.as_str())
            .query(&[("symbol", sym), ("interval", "1m"), ("startTime", &start_str), ("limit", "1000")])
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
            .map(to_candle)
            .collect()
    }
}

type WsConnection = actix_codec::Framed<BoxedSocket, Codec>;
pub struct Live {
    ticks: Vec<Tick>,
    url: String,
    hb: DateTime<Utc>,
    ws_conn: WsConnection,
}

impl Live {
    pub async fn new(ticks: &[Tick], listen_key: &str) -> Self {
        let base_url = String::from("wss://stream.binance.com:9443/stream?streams=");
        let stream_list = build_stream_list(ticks, listen_key);
        let url = base_url.clone() + &stream_list;
        println!("{:?} - {:?} - {:?}", base_url, stream_list, url);

        let (resp, conn) = Client::builder()
            .max_http_version(awc::http::Version::HTTP_11)
            .finish()
            .ws(url)
            .connect()
            .await
            .expect("on ws connecting to binance");

        println!("new response {:?}", resp);

        Self {
            ticks: ticks.to_vec(),
            url: base_url,
            ws_conn: conn,
            hb: Utc::now(),
        }
    }
}

#[async_trait(?Send)]
impl LiveFeed for Live {
    async fn next(&mut self) -> LiveEvent {
        loop {
            let nnext = self.ws_conn.next().await;
            if nnext.is_none() {
                return LiveEvent::ReconnectionRequired;
            }
            let msg = nnext.unwrap().expect("in next message");
            match msg {
                Frame::Text(text) => {
                    let mesg = serde_json::from_slice::<LiveMessage>(&text).expect("message not implemented");
                    match mesg.data {
                        LiveMessageType::LiveCandle(candle_msg) => {
                            if candle_msg.candle.kline_close {
                                let candle = to_candle_from_live(&candle_msg);
                                return LiveEvent::Candle(candle_msg.symbol, candle);
                            }
                        }
                    }
                }
                Frame::Ping(bytes) => {
                    self.ws_conn.send(awc::ws::Message::Pong(bytes)).await.expect("when ponging");
                }
                Frame::Close(reasons) => {
                    println!("connection closed {:?}", reasons);
                    return LiveEvent::ReconnectionRequired;
                }
                _ => {}
            };
        }
    }

    async fn submit(&self, order: &Order) {}
    async fn cancel(&self, order_reference: i32) {}
}

// --------------------------------
// helper functions
fn build_stream_list(ticks: &[Tick], listen_key: &str) -> String {
    let mut streams: Vec<_> = ticks
        .into_iter()
        .map(|tick| {
            let mut dur = humantime::format_duration(tick.interval.to_std().expect("duration to std")).to_string();
            dur.truncate(2);
            let tt = format!("{}@kline_{}", tick.sym.to_ascii_lowercase(), dur);
            tt
        })
        .collect();
    streams.push(String::from(listen_key));
    streams.join("/")
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
}
fn to_candle(cnd: &Candle) -> candles::Candle {
    candles::Candle {
        open: cnd.open.parse::<f64>().expect("in cnd.open"),
        low: cnd.low.parse::<f64>().expect("in cnd.low"),
        high: cnd.high.parse::<f64>().expect("in cnd.high"),
        close: cnd.close.parse::<f64>().expect("in cnd.close"),
        volume: cnd.volume.parse::<f64>().expect("in cnd.volume"),
        tstamp: NaiveDateTime::from_timestamp((cnd.open_time / 1000) as i64, 0),
        tframe: Duration::minutes(1),
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
struct ListenKey {
    #[serde(alias = "listenKey")]
    listen_key: String,
}

#[derive(Debug, serde::Deserialize)]
struct LiveCandle {
    //{"t":1620831000000,"T":1620831059999,"s":"BTCEUR","i":"1m","f":44491152,"L":44491273,"o":"46595.80000000","c":"46579.23000000","h":"46640.08000000","l":"46564.13000000","v":"2.23355400","n":122,"x":false,"q":"104071.79012531","V":"1.21994100","Q":"56842.26545704","B":"0"}
    #[serde(alias = "t")]
    tstamp_open: u64,
    #[serde(alias = "T")]
    tstamp_close: u64,
    #[serde(alias = "i")]
    interval: String,
    #[serde(alias = "s")]
    symbol: String,
    #[serde(alias = "o")]
    open: String,
    #[serde(alias = "l")]
    low: String,
    #[serde(alias = "h")]
    high: String,
    #[serde(alias = "c")]
    close: String,
    #[serde(alias = "x")]
    kline_close: bool,
    #[serde(alias = "v")]
    volume: String,
}
#[derive(Debug, serde::Deserialize)]
struct LiveCandleMsg {
    //{"e":"kline","E":1620831035519,"s":"BTCEUR","k":{}}
    #[serde(alias = "E")]
    tstamp_open: u64,
    #[serde(alias = "s")]
    symbol: String,
    #[serde(alias = "k")]
    candle: LiveCandle,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum LiveMessageType {
    LiveCandle(LiveCandleMsg),
}

#[derive(Debug, serde::Deserialize)]
struct LiveMessage {
    //{"stream":"btceur@kline_1m","data": {}}
    stream: String,
    data: LiveMessageType,
}

fn to_candle_from_live(msg: &LiveCandleMsg) -> candles::Candle {
    candles::Candle {
        open: msg.candle.open.parse::<f64>().expect("in cnd.open"),
        low: msg.candle.low.parse::<f64>().expect("in cnd.low"),
        high: msg.candle.high.parse::<f64>().expect("in cnd.high"),
        close: msg.candle.close.parse::<f64>().expect("in cnd.close"),
        volume: msg.candle.volume.parse::<f64>().expect("in cnd.volume"),
        tstamp: NaiveDateTime::from_timestamp((msg.candle.tstamp_open / 1000) as i64, 0),
        tframe: Duration::from_std(humantime::parse_duration(&msg.candle.interval).expect("could not parse interval in live candle"))
            .expect("to chrono"),
    }
}
