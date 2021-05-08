#[derive(Clone, PartialEq, Debug)]
pub struct Symbol {
    pub symbol: String,
    pub pretty: String,
    pub base: String,
    pub base_decimals: usize,
    pub quote: String,
    pub quote_decimals: usize,
}

impl std::string::ToString for Symbol {
    fn to_string(&self) -> String {
        self.pretty.clone()
    }
}
