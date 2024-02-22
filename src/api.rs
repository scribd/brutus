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

    app.at("/vecSearch").post(vector_search);
    app.at("/relSearch").post(relevance_search);
    app.at("/hybridSearch").post(hybrid_search);

    debug!("Registered API routes: {app:?}");
    Ok(app)
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct VectorSearchRequest {
    doc: u64,
    query: Vec<f64>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RelevanceSearchRequest {
    doc: u64,
    query: String,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct HybridSearchRequest {
    doc: u64,
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
    let _request: RelevanceSearchRequest = req.body_json().await?;

    let mut text_search = TantivyTextSearch::new();

    //todo replace sampels with data from S3
    let samples = vec![
        TextChunk {
            id: 1,
            text: "abc".to_string(),
        },
        TextChunk {
            id: 2,
            text: "cde".to_string(),
        },
        TextChunk {
            id: 3,
            text: "abe".to_string(),
        },
        TextChunk {
            id: 4,
            text: "abx".to_string(),
        },
    ];

    for sample in samples.iter() {
        text_search.add(sample)?;
    }

    text_search.commit()?;

    let result = text_search.search(_request.query, samples.len())?;
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
