
#[derive(Debug)]
pub enum Error {
    ErrNotFound(String),
    ErrTimeFrameNotSupported,
    Unexpected(Box<dyn std::error::Error>),
    Unknown,       // to be removed
    Unimplemented, // to be removed
    Done,
}
