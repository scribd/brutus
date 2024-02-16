///
/// The API module contains all the REST APIs which brutus provides
///
use serde::{Deserialize, Serialize};
use tide::{Body, Request, Result, Server};
use tracing::log::*;

/// Main handler for all the API routes
///
/// returns the API server which can be nsted under the main application
pub fn routes() -> Result<Server<()>> {
    let mut app = tide::new();

    app.at("/vecSearch").post(vector_search);

    debug!("Registered API routes: {app:?}");
    Ok(app)
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct VectorSearchRequest {
    doc: u64,
    query: Vec<f64>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ChunkResponse {
    /// Chunk iod
    chunk: u64,
    /// score
    score: f64,
    /// chunk text
    text: String,
}

///
/// POST /vecSearch
///
pub async fn vector_search(mut req: Request<()>) -> Result<Body> {
    let _request: VectorSearchRequest = req.body_json().await?;
    let response: Vec<ChunkResponse> = vec![];

    Body::from_json(&response)
}

#[cfg(test)]
mod tests {}
