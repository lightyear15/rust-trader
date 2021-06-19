use crate::candles;
use crate::drivers::{LiveEvent, LiveFeed, RestApi, Tick};
use crate::error::Error;
use crate::orders;
use crate::orders::Transaction;
use crate::symbol::Symbol;
use crate::wallets::SpotWallet;
use async_trait::async_trait;
use awc::ws::{Codec, Frame};
use awc::{BoxedSocket, Client};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
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
        let request = if let Some(token) = old_token {
            self.client
                .put(url)
                .query(&["listen_key", &token])
                .expect("in building put ws token")
        } else {
            self.client.post(url)
        };
        request
            .send()
            .await
            .expect("in sending listen_key request")
            .json::<ListenKey>()
            .await
            .expect("in parsing userDataStream response")
            .listen_key
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
        interval: Option<&Duration>,
        start: Option<&NaiveDateTime>,
        limit: Option<usize>,
    ) -> Vec<candles::Candle> {
        let mut queries: Vec<(String, String)> =
            start.map_or(Vec::new(), |st| vec![(String::from("startTime"), format!("{}000", st.timestamp()))]);
        queries.push((String::from("symbol"), String::from(sym)));
        queries.push((String::from("interval"), to_interval(interval.unwrap_or(&Duration::minutes(1)))));
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
            .map(|cnd| cnd.into())
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
        let mut queries = to_query(&order);
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
        println!("new response {:?}", resp);
        Self {
            ticks,
            token: listen_key,
            url: base_url,
            ws_conn: conn,
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
        println!("new response {:?}", resp);
        self.token = new_key;
    }

    async fn next(&mut self) -> LiveEvent {
        loop {
            let nnext = self.ws_conn.next().await;
            if nnext.is_none() {
                return LiveEvent::ReconnectionRequired;
            }
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
                    println!("connection closed {:?}", reasons);
                    return LiveEvent::ReconnectionRequired;
                }
                _ => {}
            };
        }
    }
}

//---------------------------------
// binance messages and objects
#[derive(Debug, serde::Deserialize, Clone)]
struct ExchangeInfo {
    symbols: Vec<SymbolInfo>,
}
#[derive(Debug, serde::Deserialize, Clone)]
struct ListenKey {
    #[serde(alias = "listenKey")]
    listen_key: String,
}
#[derive(Debug, serde::Deserialize)]
struct AccountStatusData {
    balances: Vec<Balance>,
    #[serde(alias = "totalAssetOfBtc")]
    total: String,
}
#[derive(Debug, serde::Deserialize)]
struct AccountStatusSnapshot {
    data: AccountStatusData,
    #[serde(alias = "updateTime")]
    tstamp: u64,
}
#[derive(Debug, serde::Deserialize)]
struct AccountStatus {
    #[serde(alias = "snapshotVos")]
    snapshot: Vec<AccountStatusSnapshot>,
    msg: String,
    code: i16,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum Side {
    #[serde(alias = "BUY")]
    Buy,
    #[serde(alias = "SELL")]
    Sell,
}
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum Type {
    #[serde(alias = "MARKET")]
    Market,
    #[serde(alias = "LIMIT")]
    Limit,
    #[serde(alias = "STOP_LOSS")]
    StopLoss,
}
#[derive(Debug, serde::Deserialize)]
enum OrderStatus {
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

#[derive(Debug, serde::Deserialize)]
struct LiveOrderUpdate {
    #[serde(alias = "E")]
    tstamp: u64,
    #[serde(alias = "s")]
    symbol: String,
    #[serde(alias = "c")]
    order_id: String,
    #[serde(alias = "X")]
    order_status: OrderStatus,
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
}
#[derive(Debug, serde::Deserialize)]
struct Balance {
    #[serde(alias = "a")]
    asset: String,
    #[serde(alias = "f")]
    free: String,
    #[serde(alias = "l")]
    locked: String,
}
#[derive(Debug, serde::Deserialize)]
struct LiveAccountUpdate {
    #[serde(alias = "E")]
    tstamp: u64,
    #[serde(alias = "B")]
    balances: Vec<Balance>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum LiveMessageType {
    LiveCandle(LiveCandleMsg),
    OrderUpdate(LiveOrderUpdate),
    AccountUpdate(LiveAccountUpdate),
}

#[derive(Debug, serde::Deserialize)]
struct LiveMessage {
    //{"stream":"btceur@kline_1m","data": {}}
    stream: String,
    data: LiveMessageType,
}

// --------------------------------
// helper functions
fn to_interval(interval: &Duration) -> String {
    if *interval == Duration::minutes(1) {
        String::from("1m")
    } else if *interval == Duration::days(1) {
        String::from("1d")
    } else if *interval == Duration::hours(1) {
        String::from("1h")
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

impl From<Side> for orders::Side {
    fn from(side: Side) -> Self {
        match side {
            Side::Buy => orders::Side::Buy,
            Side::Sell => orders::Side::Sell,
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
impl From<LiveOrderUpdate> for Transaction {
    fn from(msg: LiveOrderUpdate) -> Self {
        let tot_quantity = msg.cumulative_quantity.parse::<f64>().expect("in cumulative_quantity");
        let tot_price = msg.cumulative_price.parse::<f64>().expect("in cumulative_price");
        Self {
            tstamp: NaiveDateTime::from_timestamp((msg.tstamp / 1000) as i64, 0),
            symbol: msg.symbol.clone(),
            side: msg.side.clone().into(),
            avg_price: tot_price / tot_quantity,
            volume: tot_quantity,
            order: orders::Order {
                volume: msg.order_quantity.parse::<f64>().expect("in msg.order_quantity"),
                exchange: String::from("binance"),
                expire: None,
                side: msg.side.into(),
                symbol: Symbol::new(msg.symbol),
                id: msg.order_id.parse::<u32>().unwrap_or(0),
                o_type: to_type(&msg.order_type, msg.order_price.parse::<f64>().expect("in msg.order_price")),
            },
        }
    }
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
fn to_type(msg_o_type: &Type, o_price: f64) -> orders::Type {
    match msg_o_type {
        Type::Limit => orders::Type::Limit(o_price),
        Type::Market => orders::Type::Market,
        _ => panic!("unknown type"),
    }
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
            return Some(LiveEvent::Balance(account_msg.balances.into()));
        }
    }
    None
}

impl From<orders::Side> for Side {
    fn from(side: orders::Side) -> Self {
        match side {
            orders::Side::Buy => Side::Buy,
            orders::Side::Sell => Side::Sell,
        }
    }
}

fn to_query(order: &orders::Order) -> Vec<(String, String)> {
    let tstamp = Utc::now().timestamp_millis() as u64;
    let side: Side = order.side.clone().into();
    let mut qty = order.symbol.min_size.max(order.volume);
    if order.symbol.step_size != 0.0 {
        let mult = ((qty - order.symbol.min_size) / order.symbol.step_size) as i32;
        qty = mult as f64 * order.symbol.step_size + order.symbol.min_size;
    }
    let mut queries: Vec<(String, String)> = vec![
        (String::from("symbol"), order.symbol.symbol.clone()),
        (String::from("side"), side.to_string()),
        (
            String::from("quantity"),
            format!("{:.prec$}", qty, prec = order.symbol.base_decimals),
        ),
        (String::from("newClientOrderId"), order.id.to_string()),
        (String::from("newOrderRespType"), String::from("ACK")),
        (String::from("timestamp"), tstamp.to_string()),
    ];
    match order.o_type {
        orders::Type::Market => {
            queries.push((String::from("type"), String::from("MARKET")));
        }
        orders::Type::Limit(price) => {
            queries.push((String::from("type"), String::from("LIMIT")));
            (
                String::from("price"),
                format!("{:.prec$}", price, prec = order.symbol.base_decimals),
            );
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
