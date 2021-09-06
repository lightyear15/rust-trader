use std::collections::HashMap;

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ExchangeSettings {
    pub api_key: String,
    pub secret_key: String,
    pub backtest: BacktestSettings,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct BacktestSettings {
    fees_perc : f64,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct StrategySettings {
    pub name: String,
    pub exchange: String,
    pub symbol: String,
    #[serde(deserialize_with = "chrono_duration_de")]
    pub time_frame: chrono::Duration,
    pub settings: HashMap<String,String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Settings {
    pub exchanges : HashMap<String, ExchangeSettings>,
    pub candle_storage: String,
    pub transaction_storage: String,
    pub strategies: Vec<StrategySettings>,
}

impl Settings {
    pub fn get_configuration(config_file: &str) -> Result<Self, config::ConfigError> {
        println!("loading config from {}", config_file);
        let mut config_reader = config::Config::default();
        config_reader.merge(config::File::with_name(config_file).required(false))?;
        let settings = config_reader.try_into()?;
        Ok(settings)
    }
}

fn chrono_duration_de<'de, D>(des: D) -> Result<chrono::Duration, D::Error>
where
D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    chrono::Duration::from_std(humantime_serde::deserialize(des)?).map_err(|_| D::Error::custom("out of range"))
}
