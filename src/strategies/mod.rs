use super::{ orders, Error};

pub mod sample;
pub use sample::Sample;

#[derive(Debug)]
pub enum Action {
    None,
    NewOrder(orders::Info),
}

// a 1-symbol strategy
pub trait Strategy {
    fn on_new_candle(&mut self) -> Action;
}

pub fn create(strategy: &str ) -> Result<Box<dyn Strategy>, Error> {
    match strategy {
        "sample" => Ok(Box::new(Sample::new())),
        _ => Err(Error::ErrNotFound),
    }
}
