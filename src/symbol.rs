#[derive(Clone, PartialEq, Debug)]
pub struct Symbol {
    pub symbol: String,
    pub pretty: String,
    pub base: String,
    pub quote: String,
    pub base_decimals: usize,
    pub quote_decimals: usize,
    pub min_size: f64, // as multiple of 10^-base_decimals
    pub step_size : f64, // as multiple of 10^-base_decimals
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
            min_size: 0.0,
            step_size: 0.0,
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

