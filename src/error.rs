use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Parquet error: {0}")]
    ParquetError(#[from] parquet::errors::ParquetError),

    #[error("Object Store: {0}")]
    ObjectStoreError(#[from] object_store::Error),

    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Generic error: {0}")]
    Generic(#[from] Box<dyn std::error::Error + Send + Sync + 'static>),

    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),
}
