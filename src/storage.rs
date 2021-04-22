use super::*;
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

    async fn store(&self, exchange: &str, symbol: &str, candles: &[candles::Candle]) -> Result<u64, Error> {
        let insert_statement = format!(
            "INSERT INTO {} (symbol, tstamp, open, low, high, close, volume) VALUES",
            exchange
        );
        let mut value_statements: String = candles.iter().fold(String::new(), |statement, cnd| {
            format!(
                "{},({},{},{},{},{},{})",
                statement, symbol, cnd.tstamp, cnd.open, cnd.low, cnd.high, cnd.volume
            )
        });
        value_statements.remove(0);
        let statement = format!("{} {}", insert_statement, &value_statements);
        self.client.execute(statement.as_str(), &[]).await
    }
}
