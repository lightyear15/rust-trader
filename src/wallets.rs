use std::collections::HashMap;

#[derive(Debug)]
pub struct SpotWallet {
    pub assets: HashMap<String, f64>,
}

impl Default for SpotWallet {
    fn default() -> Self {
        Self { assets: HashMap::new() }
    }
}
