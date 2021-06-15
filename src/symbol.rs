#[derive(Clone, PartialEq, Debug)]
pub struct Symbol {
    pub symbol: String,
    pub pretty: String,
    pub base: String,
    pub base_decimals: usize,
    pub quote: String,
    pub quote_decimals: usize,
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
        Self {
            symbol: String::new(),
            pretty: String::new(),
            base: String::new(),
            base_decimals: 0,
            quote: String::new(),
            quote_decimals: 0,
        }
    }
}

