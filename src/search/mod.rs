pub mod text_search;
pub mod vector_search;

use serde::{Deserialize, Serialize};

/// Enum for determining what type of data, if any is carried along with a [SearchResult]
///
/// This is used instead of generics on [SearchResult] to keep interfaces less verbose and easier
/// to reason about given the small number of [Search] implementations at the moment.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum SearchResultData {
    Empty,
    String(String),
}

/// The search result to be returned by all [Search] implementations
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SearchResult {
    /// The chunk's ID
    chunk: u64,
    /// Result's search score
    score: f64,
    /// Optional data to return with the [SearchResult]
    data: SearchResultData,
}

/// All search implementations should implement this interface
pub trait Search {
    type Chunk;
    type QueryType;
    type ErrorType;

    /// Default implementation is a no-op since not all [Search] implementations require it
    fn commit(&mut self) -> Result<(), Self::ErrorType> {
        Ok(())
    }
    fn add(&mut self, chunk: &Self::Chunk) -> Result<(), Self::ErrorType>;

    /// Default implementation is a no-op since not all [Search] implementations require it
    fn build(&mut self) -> Result<(), Self::ErrorType> {
        Ok(())
    }
    fn search(
        &mut self,
        query: Self::QueryType,
        k: usize,
    ) -> Result<Vec<SearchResult>, Self::ErrorType>;
}
