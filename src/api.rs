use crate::dal::*;
use crate::search::{
    text_search::{TantivyTextSearch, TextChunk},
    vector_search::{HoraVectorSearch, VectorChunk},
    Search, SearchResult,
};

use rand::Rng;
///
/// The API module contains all the REST APIs which brutus provides
///
use serde::{Deserialize, Serialize};

use tide::{Body, Request, Result, Server};
use tracing::log::*;

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
    let _request: HybridSearchRequest = req.body_json().await?;
    let response: Vec<SearchResult> = Vec::with_capacity(100);
    Body::from_json(&response)
}

///
/// POST /relSearch
///
pub async fn relevance_search(mut req: Request<State>) -> Result<Body> {
    let request: RelevanceSearchRequest = req.body_json().await?;
    let doc_id = req.param("doc_id")?;

    let doc = req
        .state()
        .fetch_doc(format!("{doc_id}/v1.parquet"))
        .await?;
    let mut text_search = TantivyTextSearch::new();

    let _: Vec<_> = doc
        .chunks
        .iter()
        .map(|chunk| {
            // TODO: The clone is unnecessary here and we should really be using a uniform chunk object
            text_search.add(&TextChunk {
                id: chunk.id,
                text: chunk.text.clone(),
            })
        })
        .collect();
    text_search.commit()?;

    let result = text_search.search(request.query, doc.chunks.len())?;
    Body::from_json(&result)
}

///
/// POST /vecSearch
///
pub async fn vector_search(mut req: Request<State>) -> Result<Body> {
    let request: VectorSearchRequest = req.body_json().await?;

    const DIMENSION: usize = 1024;
    let mut vector_search = HoraVectorSearch::new(DIMENSION);

    // \todo read samples from s3 instead of making these up
    let mut rnd = rand::thread_rng();
    let n: usize = 1000;
    for _i in 0..n {
        let mut sample: Vec<f64> = Vec::with_capacity(DIMENSION);
        for _j in 0..DIMENSION {
            sample.push(rnd.gen());
        }
        let chunk = VectorChunk {
            id: rnd.gen(),
            vectors: sample,
        };
        vector_search.add(&chunk)?;
    }

    let response = vector_search.search(request.query, n)?;
    Body::from_json(&response)
}
