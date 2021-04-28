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
    fn on_new_candle(&mut self, history : &[candles::Candle]) -> Action {
        println!("at iteration {}", self.index);
        for c in history {
            println!("{:?}", c);
        }
        self.index += 1;
        Action::None
    }
    fn get_candles_history_size(&self) -> usize {
        3
    }
}
