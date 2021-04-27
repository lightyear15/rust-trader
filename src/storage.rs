use super::candles::Candle;
use chrono::{Duration, NaiveDateTime};
use tokio_postgres::{Client, Error, NoTls};

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

    pub async fn store(&self, exchange: &str, symbol: &str, candles: &[Candle]) -> Result<u64, Error> {
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

    pub async fn get(&self, exc: &str, sym: &str, start: &NaiveDateTime, num: u32, interval: Duration) -> Vec<Candle> {
        /*
         * 1 layer  -- enough is interval < 1 hour
         * SELECT tstamp, open, low, high, close, volume 
         * FROM <exchange>
         * WHERE symbol = <sym> AND tstamp >= <start>
         * ORDER BY 1
         * LIMIT <num>
         */

        /* 2 layer -- if interval > 1 hour  -- if interval is > 1 day s/'hour'/'day'/
         * SELECT tstamp_trunc AS tstamp, open, close,
         *      MIN(low) AS low, MAX(high) AS high, SUM(volume) AS volume
         * FROM (
         *      SELECT tstamp, tstamp_trunc, low, high, volume
         *      FIRST_VALUE(open) OVER(PARTITION BY tstamp_trunc ORDER BY tstamp) AS open ,
         *      LAST_VALUE(close) OVER(PARTITION BY tstamp_trunc ORDER BY tstamp) AS close,
         *      FROM (
         *          SELECT tstamp, DATE_TRUNC('hour', tstamp) AS tstamp_trunc,
         *                  open, low, high, close, volume
         *          FROM <exchange>
         *          WHERE symbol = <sym> AND tstamp >= <start>
         *      )
         * )
         * GROUP BY 1, 2, 3
         * ORDER BY 1
         * LIMIT <num> * <chunk_size>
         */
        let select_st = "SELECT tstamp, open, low, high, close, volume";
        let from_st = format!("FROM {}", exc);
        let where_st = format!("symbol = '{}' AND tstamp > '{}'", sym, start);
        Vec::new()
    }
}
