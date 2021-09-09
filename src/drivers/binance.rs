use crate::candles;
use crate::drivers::{LiveEvent, LiveFeed, RestApi, Tick};
use crate::error::Error;
use crate::orders;
use crate::orders::Transaction;
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use async_trait::async_trait;
use awc::ws::Message;
use awc::ws::{Codec, Frame};
use awc::{BoxedSocket, Client};
use bytes::Bytes;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use log::{debug, info, warn};
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private};
use openssl::sign::Signer;
use std::collections::HashMap;

use super::binance_types::*;

#[derive(Clone)]
pub struct Rest {
    url: String,
    api_key: String,
    secret: PKey<Private>,
    client: Client,
}

impl Rest {
    pub fn new(api_key: &str, secret_word: &str) -> Rest {
        let secret = PKey::hmac(secret_word.as_bytes()).expect("cannot create private key from secret");
        let client = Client::builder()
            .header("User-Agent", "trader/0.0.1")
            .header("Host", "api.binance.com")
            .header("X-MBX-APIKEY", api_key)
            .finish();
        Rest {
            url: String::from("https://api.binance.com"),
            //url: String::from("http://localhost:8080"),
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
        if let Some(token) = old_token {
            let resp = self
                .client
                .put(url)
                .query(&[("listenKey", &token)])
                .expect("in building put ws token")
                .send()
                .await
                .expect("in sending listen_key");
            if !resp.status().is_success() {
                panic!("token refresh failed {:?}", resp)
            }
            token
        } else {
            self.client
                .post(url)
                .send()
                .await
                .expect("in sending listen_key request")
                .json::<ListenKey>()
                .await
                .expect("in parsing userDataStream response")
                .listen_key
        }
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
            .map(|info| info.into())
            .ok_or_else(|| Error::ErrNotFound(format!("can't find symbol {}", sym)));
        x
    }

    async fn get_candles(
        &self,
        sym: &str,
        maybe_interval: Option<&Duration>,
        start: Option<&NaiveDateTime>,
        limit: Option<usize>,
    ) -> Vec<candles::Candle> {
        let interval = *maybe_interval.unwrap_or(&Duration::minutes(1));
        let mut queries: Vec<(String, String)> =
            start.map_or(Vec::new(), |st| vec![(String::from("startTime"), format!("{}000", st.timestamp()))]);
        queries.push((String::from("symbol"), String::from(sym)));
        queries.push((String::from("interval"), to_interval(&interval)));
        queries.push((String::from("limit"), limit.unwrap_or(1000).to_string()));
        let url = self.url.clone() + "/api/v3/klines";
        let request = self.client.get(url).query(&queries).expect("in adding queries");
        let mut response = request.send().await.expect("in send binance klines request");
        response
            .json::<Vec<Candle>>()
            .limit(128_000_000)
            .await
            .expect("in json<Vec<Candle>>")
            .drain(0..)
            .filter_map(|cnd| {
                let candle = candles::Candle::from(cnd);
                if candle.tframe < interval {
                    None
                } else {
                    Some(candle)
                }
            })
            .collect()
    }

    async fn get_wallet(&self) -> Result<SpotWallet, Error> {
        let url = self.url.clone() + "/sapi/v1/accountSnapshot";
        let tstamp_str = format!("{}", Utc::now().timestamp_millis());
        let mut queries: Vec<(&str, &str)> = vec![("type", "SPOT"), ("timestamp", &tstamp_str)];
        let mut request = self.client.get(url).query(&queries).expect("in adding queries");
        let query_str = request.get_uri().query().expect("no query?");
        let signature = Signer::new(MessageDigest::sha256(), &self.secret)
            .expect("in creating the signer")
            .sign_oneshot_to_vec(query_str.as_bytes())
            .expect("in digesting body")
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("");
        queries.push(("signature", &signature));
        request = request.query(&queries).expect("in setting queries with signature");
        let mut wl = request
            .send()
            .await
            .expect("in send binance account snapshot request")
            .json::<AccountStatus>()
            .limit(128_000_000)
            .await
            .expect("in json::<AccountStatus>")
            .snapshot;
        wl.sort_by_key(|shot| shot.tstamp);
        Ok(wl.remove(wl.len() - 1).data.balances.into())
    }

    async fn send_order(&self, order: orders::Order) -> orders::OrderStatus {
        let url = self.url.clone() + "/api/v3/order";
        let mut queries = order_to_query(&order);
        let mut request = self.client.post(url).query(&queries).expect("in adding queries");
        let query_str = request.get_uri().query().expect("no query?");
        let signature = Signer::new(MessageDigest::sha256(), &self.secret)
            .expect("in creating the signer")
            .sign_oneshot_to_vec(query_str.as_bytes())
            .expect("in digesting body")
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("");
        queries.push((String::from("signature"), signature));
        request = request.query(&queries).expect("in setting queries with signature");
        let mut response = request.send().await.expect("in receiving server response");
        let resp_code = response.status();
        if resp_code.is_success() {
            orders::OrderStatus::Accepted
        } else {
            let bd = response
                .body()
                .await
                .map_or_else(|err| format!("Error {:?}", err), |body| format!("body {:?}", body));
            orders::OrderStatus::Rejected(bd)
        }
    }

    async fn cancel_order(&self, symbol: String, order_id: u32) -> orders::OrderStatus {
        let url = self.url.clone() + "/api/v3/order";
        let mut queries = cancel_query(symbol, order_id);
        let mut request = self.client.delete(url).query(&queries).expect("in adding queries");
        let query_str = request.get_uri().query().expect("no query?");
        let signature = Signer::new(MessageDigest::sha256(), &self.secret)
            .expect("in creating the signer")
            .sign_oneshot_to_vec(query_str.as_bytes())
            .expect("in digesting body")
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("");
        queries.push((String::from("signature"), signature));
        request = request.query(&queries).expect("in setting queries with signature");
        let resp_code = request.send().await.expect("in receiving server response").status();
        if resp_code.is_success() {
            orders::OrderStatus::Canceled
        } else {
            orders::OrderStatus::Rejected(String::new())
        }
    }
}

type WsConnection = actix_codec::Framed<BoxedSocket, Codec>;

pub struct Live {
    ticks: Vec<Tick>,
    token: String,
    url: String,
    ws_conn: WsConnection,
    heartbeat: chrono::NaiveDateTime,
    refresh: chrono::NaiveDateTime,
    reconnect: chrono::NaiveDateTime,
}

impl Live {
    pub async fn new(ticks: Vec<Tick>, listen_key: String) -> Self {
        let base_url = String::from("wss://stream.binance.com:9443/stream?streams=");
        let stream_list = build_stream_list(ticks.as_slice(), &listen_key);
        let url = base_url.clone() + &stream_list;
        let (resp, conn) = Client::builder()
            .max_http_version(awc::http::Version::HTTP_11)
            .finish()
            .ws(url)
            .connect()
            .await
            .expect("on ws connecting to binance");
        debug!("new response {:?}", resp);
        let now = Utc::now().naive_utc();
        Self {
            ticks,
            token: listen_key,
            url: base_url,
            ws_conn: conn,
            heartbeat: now,
            refresh: now,
            reconnect: now,
        }
    }
}

#[async_trait(?Send)]
impl LiveFeed for Live {
    fn token(&self) -> String {
        self.token.clone()
    }

    async fn reconnect(&mut self, new_key: String) {
        let stream_list = build_stream_list(self.ticks.as_slice(), &new_key);
        let url = self.url.clone() + &stream_list;
        let (resp, conn) = Client::builder()
            .max_http_version(awc::http::Version::HTTP_11)
            .finish()
            .ws(url)
            .connect()
            .await
            .expect("on ws connecting to binance");
        self.ws_conn = conn;
        debug!("new response {:?}", resp);
        self.token = new_key;
        let now = Utc::now().naive_utc();
        self.heartbeat = now;
        self.refresh = now;
        self.reconnect = now;
    }

    async fn next(&mut self) -> LiveEvent {
        let hb_interval = Duration::minutes(30);
        let refr_interval = Duration::minutes(60);
        let refr_interval_grace = Duration::minutes(45);
        let recon_interval = Duration::hours(24);
        loop {
            let now = Utc::now().naive_utc();
            // reconnection required
            if now - self.reconnect > recon_interval {
                return LiveEvent::ReconnectionRequired;
            }
            if now - self.refresh > refr_interval {
                return LiveEvent::ReconnectionRequired;
            }
            // refresh required
            if now - self.refresh > refr_interval_grace {
                self.refresh = now;
                return LiveEvent::TokenRefreshRequired;
            }
            // ping required
            if now - self.heartbeat > hb_interval {
                self.ws_conn
                    .send(Message::Ping(Bytes::from("hello")))
                    .await
                    .expect("in sending ping");
                self.heartbeat = now;
            }

            let nnext = self.ws_conn.next().await;
            debug!("[{}] received a next {:?}", Utc::now(), nnext);
            /*
            if nnext.is_none() {
                return LiveEvent::ReconnectionRequired;
            }
            */
            let msg = nnext.unwrap().expect("in next message");
            match msg {
                Frame::Text(text) => {
                    let panic_msg = format!("original text {:?}", text);
                    let mesg = serde_json::from_slice::<LiveMessage>(&text).expect(&panic_msg);
                    let m_event = interpret_message(mesg);
                    if let Some(event) = m_event {
                        return event;
                    }
                }
                Frame::Ping(bytes) => {
                    self.ws_conn.send(awc::ws::Message::Pong(bytes)).await.expect("when ponging");
                }
                Frame::Close(reasons) => {
                    warn!("connection closed {:?}", reasons);
                    return LiveEvent::ReconnectionRequired;
                }
                _ => {}
            };
        }
    }
}

// --------------------------------
// helper functions
fn to_interval(interval: &Duration) -> String {
    if *interval == Duration::minutes(1) {
        String::from("1m")
    } else if *interval == Duration::minutes(15) {
        String::from("15m")
    } else if *interval == Duration::hours(1) {
        String::from("1h")
    } else if *interval == Duration::days(1) {
        String::from("1d")
    } else {
        panic!("duration unknown")
    }
}

fn build_stream_list(ticks: &[Tick], listen_key: &str) -> String {
    let mut streams: Vec<_> = ticks
        .iter()
        .map(|tick| format!("{}@kline_{}", tick.sym.to_ascii_lowercase(), to_interval(&tick.interval)))
        .collect();
    streams.push(String::from(listen_key));
    streams.join("/")
}

fn interpret_message(mesg: LiveMessage) -> Option<LiveEvent> {
    match mesg.data {
        LiveMessageType::LiveCandle(candle_msg) => {
            if candle_msg.is_closed() {
                let symbol_name = candle_msg.name();
                return Some(LiveEvent::Candle(symbol_name, candle_msg.into()));
            }
        }
        LiveMessageType::OrderUpdate(tx_msg) => {
            match tx_msg.order_status {
                OrderStatus::Filled => {
                    return Some(LiveEvent::Transaction(tx_msg.into()));
                }
                OrderStatus::New => {
                    let tx: orders::Transaction = tx_msg.into();
                    return Some(LiveEvent::NewOrder(tx.order));
                }
                _ => {}
            };
        }
        LiveMessageType::AccountUpdate(account_msg) => {
            return Some(LiveEvent::BalanceUpdate(account_msg.into()));
        }
        LiveMessageType::BalanceUpdate(balance_update) => {
            let delta = balance_update.delta.parse::<f64>().expect("not a delta");
            return Some(LiveEvent::AssetUpdate {
                asset: balance_update.asset,
                delta,
            });
        }
    }
    None
}

fn order_to_query(order: &orders::Order) -> Vec<(String, String)> {
    let tstamp = Utc::now().timestamp_millis() as u64;
    let side: Side = order.side.clone().into();
    let qty = normalize_it(order.volume, order.symbol.min_volume, order.symbol.volume_step);
    let order_id = format!("{}_{}", order.id.to_string(), order.tx_ref);
    let mut queries: Vec<(String, String)> = vec![
        (String::from("symbol"), order.symbol.symbol.clone()),
        (String::from("side"), side.to_string()),
        (
            String::from("quantity"),
            format!("{:.prec$}", qty, prec = order.symbol.base_decimals),
        ),
        (String::from("newClientOrderId"), order_id),
        (String::from("newOrderRespType"), String::from("ACK")),
        (String::from("timestamp"), tstamp.to_string()),
    ];
    match order.o_type {
        orders::Type::Market => {
            queries.push((String::from("type"), String::from("MARKET")));
        }
        orders::Type::Limit(price) => {
            let norm_pr = normalize_it(price, order.symbol.min_price, order.symbol.price_tick);
            queries.push((String::from("type"), String::from("LIMIT")));
            queries.push((
                String::from("price"),
                format!("{:.prec$}", norm_pr, prec = order.symbol.base_decimals),
            ));
            queries.push((String::from("timeInForce"), String::from("GTC")))
        }
        orders::Type::StopLoss(stop) => {
            let norm_stop = normalize_it(stop, order.symbol.min_price, order.symbol.price_tick);
            queries.push((String::from("type"), String::from("STOP_LOSS")));
            queries.push((
                String::from("stopPrice"),
                format!("{:.prec$}", norm_stop, prec = order.symbol.base_decimals),
            ));
            queries.push((String::from("timeInForce"), String::from("GTC")))
        }
    }
    queries
}

fn cancel_query(symbol: String, id: u32) -> Vec<(String, String)> {
    let tstamp = Utc::now().timestamp_millis() as u64;
    vec![
        (String::from("symbol"), symbol),
        (String::from("origClientOrderId"), id.to_string()),
        (String::from("timestamp"), tstamp.to_string()),
    ]
}

fn normalize_it(value: f64, min: f64, tick: f64) -> f64 {
    let mut norm = min.max(value);
    if tick != 0.0 {
        let mult = ((norm - min) / tick) as i32;
        norm = mult as f64 * tick + min;
    }
    norm
}
