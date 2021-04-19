
#[derive(Debug, serde::Deserialize)]
pub struct ExchangeSettings {
    pub api_key: String,
    pub secret_key: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Settings {
    pub binance: ExchangeSettings,
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
