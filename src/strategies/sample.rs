use super::*;

#[derive(Clone)]
pub struct Sample {
    index: u32,
}

impl Sample {
    pub fn new() -> Self {
        Sample { index: 0 }
    }
}

impl Strategy for Sample {
    fn on_new_candle(&mut self) -> Action {
        println!("{}", self.index);
        self.index += 1;
        Action::None
    }
}
