#[derive(Clone, Copy, Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("non canonical encoding")]
    NonCanonicalEncoding,
    #[error("malformed encoding")]
    MalformedEncoding,
    #[error("too large allocation")]
    TooLargeAlloc,
    #[error("value overflow")]
    OverFlowError,
    #[error("custom error")]
    Custom(&'static str),
}
