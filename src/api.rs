use std::collections::HashMap;

use crate::dal::*;
use crate::index::{
    hora_vector_index::HoraVectorIndex, tantivy_text_index::TantivyTextIndex, Index, SearchResult,
};

///
/// The API module contains all the REST APIs which brutus provides
///
use serde::{Deserialize, Serialize};

use tide::{Body, Request, Result, Server};
use tracing::log::*;
use tracing::{event, info_span, Level};

/// Main handler for all the API routes
///
/// returns the API server which can be nsted under the main application
pub fn routes() -> Result<Server<State>> {
    let mut app = tide::with_state(State::from_env()?);

    app.at("/search/:doc_id/vector").post(vector_search);
    app.at("/search/:doc_id/relevance").post(relevance_search);
    app.at("/search/:doc_id/hybrid").post(hybrid_search);

    debug!("Registered API routes: {app:?}");
    Ok(app)
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct VectorSearchRequest {
    query: Vec<f64>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RelevanceSearchRequest {
    query: String,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct HybridSearchRequest {
    query: String,
    vector: Vec<f64>,
}

///
/// POST /relSearch
///
pub async fn hybrid_search(mut req: Request<State>) -> Result<Body> {
    let span = info_span!("relevance_search");
    let _guard = span.enter();
    let start = std::time::Instant::now();

    let request: HybridSearchRequest = req.body_json().await?;
    let doc_id = req.param("doc_id")?;

    let doc = req
        .state()
        .fetch_doc(format!("{doc_id}/v1.parquet"))
        .await?;
    event!(Level::INFO, elapsed=?start.elapsed(), "loaded parquet file");

    // todo infer number of dimensions from the first chunk
    const DIMENSION: usize = 1024;
    let mut vector_search = HoraVectorIndex::new(DIMENSION);
    let mut text_search = TantivyTextIndex::new();

    let _: Vec<_> = doc
        .chunks
        .iter()
        .map(|chunk| text_search.add(&chunk).map(|_| vector_search.add(&chunk)))
        .collect();

    event!(Level::INFO, elapsed=?start.elapsed(), "chunks added to text and vector index");

    text_search.build()?;
    vector_search.build()?;

    event!(Level::INFO, elapsed=?start.elapsed(), "additions committed");

    let txt_result = text_search.search(request.query, doc.chunks.len())?;
    event!(Level::INFO, elapsed=?start.elapsed(), "text search completed");

    let vector_result = vector_search.search(request.vector, doc.chunks.len())?;
    event!(Level::INFO, elapsed=?start.elapsed(), "vector search completed");

    //todo is there a more optimal way of combinging results ? look at initial performance
    let text_map = txt_result
        .iter()
        .map(|sr| (sr.chunk, sr))
        .collect::<HashMap<_, _>>();

    //todo probably lots of cleanup here initial POC
    //remove clone
    //whats the best way to combine scores ??
    //should individual scores also be returned ??
    let response = vector_result
        .iter()
        .map(|sr| SearchResult {
            chunk: sr.chunk,
            score: sr.score
                + text_map
                    .get(&sr.chunk)
                    .map(|sr| sr.score)
                    .unwrap_or(0.0_f64),
            // todo return text
            data: sr.data.clone(),
        })
        .collect::<Vec<_>>();

    event!(Level::INFO, elapsed=?start.elapsed(), "scores combined and returning");

    Body::from_json(&response)
}

///
/// POST /relSearch
///
pub async fn relevance_search(mut req: Request<State>) -> Result<Body> {
    let span = info_span!("relevance_search");
    let _guard = span.enter();
    let start = std::time::Instant::now();

    let request: RelevanceSearchRequest = req.body_json().await?;
    let doc_id = req.param("doc_id")?;

    let doc = req
        .state()
        .fetch_doc(format!("{doc_id}/v1.parquet"))
        .await?;
    let mut text_search = TantivyTextIndex::new();
    event!(Level::INFO, elapsed=?start.elapsed(), "loaded parquet file");

    let _: Vec<_> = doc
        .chunks
        .iter()
        .map(|chunk| text_search.add(&chunk))
        .collect();
    event!(Level::INFO, elapsed=?start.elapsed(), "chunks added to text search");

    text_search.build()?;
    event!(Level::INFO, elapsed=?start.elapsed(), "additions committed");

    let result = text_search.search(request.query, doc.chunks.len())?;
    event!(Level::INFO, elapsed=?start.elapsed(), "search completed and returning");
    Body::from_json(&result)
}

///
/// POST /vecSearch
///
pub async fn vector_search(mut req: Request<State>) -> Result<Body> {
    let span = info_span!("relevance_search");
    let _guard = span.enter();
    let start = std::time::Instant::now();

    let request: VectorSearchRequest = req.body_json().await?;
    let doc_id = req.param("doc_id")?;

    //todo move this to a common function
    let doc = req
        .state()
        .fetch_doc(format!("{doc_id}/v1.parquet"))
        .await?;
    event!(Level::INFO, elapsed=?start.elapsed(), "loaded parquet file");

    // todo infer number of dimensions from the first chunk
    const DIMENSION: usize = 1024;
    let mut vector_search = HoraVectorIndex::new(DIMENSION);

    let _: Vec<_> = doc
        .chunks
        .iter()
        .map(|chunk| vector_search.add(&chunk))
        .collect();
    event!(Level::INFO, elapsed=?start.elapsed(), "chunks added to vector index");

    vector_search.build()?;
    event!(Level::INFO, elapsed=?start.elapsed(), "index built");

    let response = vector_search.search(request.query, doc.chunks.len())?;
    event!(Level::INFO, elapsed=?start.elapsed(), "search completed and returning");

    Body::from_json(&response)
}
