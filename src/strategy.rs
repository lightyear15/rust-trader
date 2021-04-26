use super::*;

pub enum Action {
    None,
    NewOrder(order::Info),
    //ClosePosition,
}

// a 1-symbol strategy 
pub trait Strategy {
    fn symbol(&self) -> &Symbol;
    fn time_frame(&self) -> &chrono::Duration;
    fn on_new_candle<B>(&self, candles: &candles::CandleHistory, broker: &B) -> (Action, Self)
    where
        B: brokers::Broker;
}

pub fn create(strategy_name: &str, )
