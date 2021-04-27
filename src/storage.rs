use super::candles;
use chrono::{Duration, NaiveDateTime};
use tokio_postgres::{row, Client, Error, NoTls};

pub struct Candles {
    client: Client,
}

impl Candles {
    pub async fn new(host: &str) -> Candles {
        let (client, connection) = tokio_postgres::connect(host, NoTls)
            .await
            .expect("when connecting to postgres");
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });
        Candles { client }
    }

    pub async fn store(&self, exchange: &str, symbol: &str, candles: &[candles::Candle]) -> Result<u64, Error> {
        let insert_statement = format!(
            "INSERT INTO {} (symbol, tstamp, open, low, high, close, volume) VALUES",
            exchange
        );
        let mut value_statements: String = candles.iter().fold(String::new(), |statement, cnd| {
            format!(
                "{},('{}','{}',{},{},{},{},{})",
                statement, symbol, cnd.tstamp, cnd.open, cnd.low, cnd.high, cnd.close, cnd.volume
            )
        });
        value_statements.remove(0);
        let statement = format!(
            "{} {} ON CONFLICT(symbol, tstamp) DO NOTHING;",
            insert_statement, &value_statements
        );
        self.client.execute(statement.as_str(), &[]).await
    }

    //TODO: add a filter on end time
    pub async fn get(&self, exc: &str, sym: &str, start: &NaiveDateTime, num: usize, interval: Duration) -> Vec<candles::Candle> {
        let statement: String;
        let mut chunk_size: usize;
        if interval.num_hours() == 0 {
            chunk_size = interval.num_minutes() as usize;
            statement = format!(
                "SELECT tstamp, open, low, high, close, volume
FROM {exchange}
WHERE symbol = '{symbol}' AND tstamp >= '{start_time}'
ORDER BY 1
LIMIT {num}",
                exchange = exc,
                symbol = sym,
                start_time = start.format("%Y-%m-%d %H:%M:%S"),
                num = num * chunk_size
            );
        } else {
            chunk_size = interval.num_hours() as usize;
            let mut date_part = "hour";
            if interval.num_days() > 0 {
                chunk_size = interval.num_days() as usize;
                date_part = "day";
            }
            statement = format!(
                "SELECT tstamp_trunc AS tstamp, open, close, MIN(low) AS low, MAX(high) AS high, SUM(volume) AS volume
FROM (
     SELECT tstamp, tstamp_trunc, low, high, volume,
     FIRST_VALUE(open) OVER(PARTITION BY tstamp_trunc ORDER BY tstamp) AS open,
     FIRST_VALUE(close) OVER(PARTITION BY tstamp_trunc ORDER BY tstamp DESC) AS close
     FROM (
         SELECT tstamp, DATE_TRUNC('{date_part}', tstamp) AS tstamp_trunc, open, low, high, close, volume
         FROM {exchange}
         WHERE symbol = '{symbol}' AND tstamp >= '{start_time}'
     ) AS t1
) AS t2
GROUP BY 1, 2, 3
ORDER BY 1
LIMIT {num}",
                exchange = exc,
                symbol = sym,
                start_time = start.format("%Y-%m-%d %H:%M:%S"),
                num = num * chunk_size,
                date_part = date_part,
            );
        }
        let mut rows = self
            .client
            .query(statement.as_str(), &[])
            .await
            .expect("in querying for candles");
        let candles: Vec<candles::Candle> = rows.drain(0..).map(|row| row.into()).collect();
        candles.chunks(chunk_size).map(group_candles).collect()
    }
}

impl std::convert::From<row::Row> for candles::Candle {
    fn from(row: row::Row) -> Self {
        let mut cnd = candles::Candle {
            tstamp: NaiveDateTime::from_timestamp(0, 0),
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
}

fn group_candles(cnds: &[candles::Candle]) -> candles::Candle {
    let cnd = candles::Candle {
        tstamp: cnds.first().expect("can't be size 0").tstamp,
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
