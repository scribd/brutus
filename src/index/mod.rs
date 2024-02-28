pub mod hora_vector_index;
pub mod tantivy_text_index;
use crate::dal::Chunk;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Enum for determining what type of data, if any is carried along with a [SearchResult]
///
/// This is used instead of generics on [SearchResult] to keep interfaces less verbose and easier
/// to reason about given the small number of [Search] implementations at the moment.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum SearchResultData {
    Empty,
    String(String),
}

///
/// Error wrapper type to allow callers to treat searches identically
#[derive(Debug, Error)]
pub enum SearchError {
    /// Generic error message todo add more specific types
    #[error("An untyped error has occured: {0}")]
    Generic(String),
    #[error("Full text search error: {0}")]
    Fulltext(#[from] tantivy::TantivyError),
    #[error("Query parsing error: {0}")]
    Query(#[from] tantivy::query::QueryParserError),
}

/// The search result to be returned by all [Search] implementations
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SearchResult {
    /// The chunk's ID
    pub chunk: i64,
    /// Result's search score
    pub score: f64,
    /// Optional data to return with the [SearchResult]
    pub data: SearchResultData,
}

/// All search implementations should implement this interface
pub trait Index {
    type QueryType;

    fn add(&mut self, chunk: &Chunk) -> Result<(), SearchError>;

    /// Default implementation is a no-op since not all [Search] implementations require it
    fn build(&mut self) -> Result<(), SearchError>;

    fn search(
        &mut self,
        query: Self::QueryType,
        k: usize,
    ) -> Result<Vec<SearchResult>, SearchError>;
}
