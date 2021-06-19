use super::candles;
use super::orders::Transaction;
use chrono::{Duration, NaiveDateTime};
use futures_util::TryFutureExt;
use tokio_postgres::{row, Client, Error, NoTls};

pub struct Candles {
    client: Client,
}

/*
CREATE TABLE binance (
symbol varchar(16) NOT NULL,
tstamp timestamp NOT NULL,
"open" float4 NULL,
low float4 NULL,
high float4 NULL,
"close" float4 NULL,
volume float4 NULL,
CONSTRAINT binance_pkey PRIMARY KEY (symbol, tstamp)
);
 */
impl Candles {
    pub async fn new(host: &str) -> Self {
        let (client, connection) = tokio_postgres::connect(host, NoTls).await.expect("when connecting to postgres");
        actix_rt::Arbiter::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        Self { client }
    }

    pub async fn store(&self, exchange: &str, symbol: &str, candles: &[candles::Candle]) -> Result<u64, Error> {
        let insert_statement = format!("INSERT INTO {} (symbol, tstamp, open, low, high, close, volume) VALUES", exchange);
        let mut value_statements: String = candles.iter().fold(String::new(), |statement, cnd| {
            format!(
                "{},('{}','{}',{},{},{},{},{})",
                statement, symbol, cnd.tstamp, cnd.open, cnd.low, cnd.high, cnd.close, cnd.volume
            )
        });
        value_statements.remove(0);
        let statement = format!("{} {} ON CONFLICT(symbol, tstamp) DO NOTHING;", insert_statement, &value_statements);
        self.client.execute(statement.as_str(), &[]).await
    }

    pub async fn get(
        &self,
        exc: &str,
        sym: &str,
        start: &NaiveDateTime,
        end: &NaiveDateTime,
        interval: &Duration,
        num: usize,
    ) -> Vec<candles::Candle> {
        let statement: String;
        let mut chunk_size: usize;
        let mut tframe: Duration;
        if interval.num_hours() == 0 {
            tframe = Duration::minutes(1);
            chunk_size = interval.num_minutes() as usize;
            statement = format!(
                "SELECT tstamp, open, low, high, close, volume
                FROM {exchange} WHERE symbol = '{symbol}' AND tstamp BETWEEN '{start_time}' AND '{end_time}'
                ORDER BY 1
                LIMIT {num}",
                exchange = exc,
                symbol = sym,
                start_time = start.format("%Y-%m-%d %H:%M:%S"),
                end_time = end.format("%Y-%m-%d %H:%M:%S"),
                num = num * chunk_size
            );
        } else {
            tframe = Duration::hours(1);
            chunk_size = interval.num_hours() as usize;
            let mut date_part = "hour";
            if interval.num_days() > 0 {
                tframe = Duration::days(1);
                chunk_size = interval.num_days() as usize;
                date_part = "day";
            }
            statement = format!(
                "SELECT tstamp_trunc AS tstamp, open, close, MIN(low) AS low, MAX(high) AS high, SUM(volume) AS volume
                FROM ( SELECT tstamp, tstamp_trunc, low, high, volume,
                    FIRST_VALUE(open) OVER(PARTITION BY tstamp_trunc ORDER BY tstamp) AS open,
                    FIRST_VALUE(close) OVER(PARTITION BY tstamp_trunc ORDER BY tstamp DESC) AS close
                    FROM (SELECT tstamp, DATE_TRUNC('{date_part}', tstamp) AS tstamp_trunc, open, low, high, close, volume
                        FROM {exchange}
                        WHERE symbol = '{symbol}' AND tstamp BETWEEN '{start_time}' AND '{end_time}'
                    ) AS t1
                ) AS t2
                GROUP BY 1, 2, 3
                ORDER BY 1
                LIMIT {num}",
                exchange = exc,
                symbol = sym,
                start_time = start.format("%Y-%m-%d %H:%M:%S"),
                end_time = end.format("%Y-%m-%d %H:%M:%S"),
                num = num * chunk_size,
                date_part = date_part,
            );
        }
        self.client
            .query(statement.as_str(), &[])
            .await
            .expect("in querying for candles")
            .drain(0..)
            .map(|row| row_to_candle(row, &tframe))
            .collect::<Vec<candles::Candle>>()
            .chunks(chunk_size)
            .map(group_candles)
            .collect()
    }
    pub async fn find_lower(
        &self,
        exc: &str,
        sym: &str,
        start: &NaiveDateTime,
        end: &NaiveDateTime,
        price: f64,
    ) -> Option<chrono::NaiveDateTime> {
        let statement = format!("SELECT tstamp FROM {exchange} WHERE symbol = '{symbol}' AND low <= {price} AND tstamp BETWEEN '{start_time}' AND '{end_time}' ORDER BY tstamp LIMIT 1",
                                exchange = exc,
                                symbol = sym,
                                start_time = start.format("%Y-%m-%d %H:%M:%S"),
                                end_time = end.format("%Y-%m-%d %H:%M:%S"),
                                price = price
                               );
        self.client
            .query(statement.as_str(), &[])
            .await
            .expect("in querying for lower")
            .first()
            .map(|row| row.get(0))
    }

    pub async fn find_higher(
        &self,
        exc: &str,
        sym: &str,
        start: &NaiveDateTime,
        end: &NaiveDateTime,
        price: f64,
    ) -> Option<chrono::NaiveDateTime> {
        let statement = format!("SELECT tstamp FROM {exchange} WHERE symbol = '{symbol}' AND high >= {price} AND tstamp BETWEEN '{start_time}' AND '{end_time}' ORDER BY tstamp LIMIT 1",
                                exchange = exc,
                                symbol = sym,
                                start_time = start.format("%Y-%m-%d %H:%M:%S"),
                                end_time = end.format("%Y-%m-%d %H:%M:%S"),
                                price = price
                               );
        self.client
            .query(statement.as_str(), &[])
            .await
            .expect("in querying for lower")
            .first()
            .map(|row| row.get(0))
    }
}

fn row_to_candle(row: row::Row, tframe: &chrono::Duration) -> candles::Candle {
    let mut cnd = candles::Candle {
        tstamp: NaiveDateTime::from_timestamp(0, 0),
        tframe: *tframe,
        open: 0.0,
        low: 0.0,
        high: 0.0,
        close: 0.0,
        volume: 0.0,
    };
    for (idx, col) in row.columns().iter().enumerate() {
        match col.name() {
            "tstamp" => {
                cnd.tstamp = row.get(idx);
            }
            "open" => {
                cnd.open = row.get::<usize, f32>(idx) as f64;
            }
            "low" => {
                cnd.low = row.get::<usize, f32>(idx) as f64;
            }
            "high" => {
                cnd.high = row.get::<usize, f32>(idx) as f64;
            }
            "close" => {
                cnd.close = row.get::<usize, f32>(idx) as f64;
            }
            "volume" => {
                cnd.volume = row.get::<usize, f32>(idx) as f64;
            }
            _ => {}
        };
    }
    cnd
}

fn group_candles(cnds: &[candles::Candle]) -> candles::Candle {
    let cnd = candles::Candle {
        tstamp: cnds.first().expect("can't be size 0").tstamp,
        tframe: cnds.first().expect("can't be size 0").tframe * cnds.len() as i32,
        low: std::f64::MAX,
        high: std::f64::MIN,
        open: cnds.first().expect("can't be size 0").open,
        close: cnds.last().expect("can't be size 0").close,
        volume: 0.0,
    };
    cnds.iter().fold(cnd, |mut folded, cnd| {
        folded.low = folded.low.min(cnd.low);
        folded.high = folded.high.max(cnd.high);
        folded.volume += cnd.volume;
        folded
    })
}

pub struct Transactions {
    client: Client,
}

/*
create table transactions (
exchange varchar(32) not null,
symbol varchar(16) not null,
tstamp timestamp not null,
side varchar(16) not null,
price float4 not null,
volume float4 not null,
reference int not null,
constraint transactions_pkey primary key (exchange, symbol, tstamp, reference)
)
*/
impl Transactions {
    pub async fn new(host: &str, arbiter: &mut actix_rt::Arbiter) -> Self {
        let (client, connection) = tokio_postgres::connect(host, NoTls).await.expect("when connecting to postgres");
        let f_ = connection.map_ok(|_x| ()).unwrap_or_else(|e| {
            println!("error is {:?}", e);
        });
        arbiter.send(f_);
        Self { client }
    }

    pub async fn store(&self, exchange: &str, tx: &Transaction) -> Result<u64, Error> {
        let statement = format!(
            "INSERT INTO transactions (exchange, symbol, tstamp, side, price, volume, reference)
                                VALUES ('{}', '{}', '{}', '{}', {}, {}, {})",
            exchange,
            tx.symbol,
            tx.tstamp,
            tx.side.to_string(),
            tx.avg_price,
            tx.volume,
            tx.order.id        );
        self.client.execute(statement.as_str(), &[]).await
    }
}
