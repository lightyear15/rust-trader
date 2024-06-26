#[derive(Clone, PartialEq, Debug)]
pub struct Symbol {
    pub symbol: String,
    pub pretty: String,
    pub base: String,
    pub quote: String,
    pub base_decimals: usize,
    pub quote_decimals: usize,
    pub min_volume: f64,
    pub volume_step: f64,
    pub min_price: f64,
    pub price_tick: f64,
}

impl Symbol {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            pretty: String::new(),
            base: String::new(),
            base_decimals: 0,
            quote: String::new(),
            quote_decimals: 0,
            min_volume: 0.0,
            volume_step: 0.0,
            min_price: 0.0,
            price_tick: 0.0,
        }
    }
}

impl std::string::ToString for Symbol {
    fn to_string(&self) -> String {
        self.pretty.clone()
    }
}

impl Default for Symbol {
    fn default() -> Self {
        Self::new(String::new())
    }
}
