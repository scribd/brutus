//use core::slice::SlicePattern;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::ReloadPolicy;
use tempfile::TempDir;

use hora::core::{ann_index::ANNIndex, node::Node};

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
pub fn routes() -> Result<Server<()>> {
    let mut app = tide::new();

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

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ChunkResponse {
    /// VectorChunk iod
    chunk: u64,
    /// score
    score: f64,
    /// chunk text
    text: String,
}

pub struct VectorChunk {
    id: u64,
    vectors: Vec<f64>,
}

const DIMENSION: usize = 1024;
///
/// POST /relSearch
///
pub async fn hybrid_search(mut req: Request<()>) -> Result<Body> {
    let _request: HybridSearchRequest = req.body_json().await?;
    let response: Vec<ChunkResponse> = Vec::with_capacity(100);
    Body::from_json(&response)
}

///
/// POST /relSearch
///
pub async fn relevance_search(mut req: Request<()>) -> Result<Body> {
    let _request: RelevanceSearchRequest = req.body_json().await?;

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

    let result = bm25_search(&samples, _request.query).unwrap();
    Body::from_json(&result)
}

///
/// POST /vecSearch
///
pub async fn vector_search(mut req: Request<()>) -> Result<Body> {
    let _request: VectorSearchRequest = req.body_json().await?;

    // \todo read samples from s3 instead of making these up
    let mut rnd = rand::thread_rng();
    let n = 1000;
    let mut samples = Vec::with_capacity(n);
    for _i in 0..n {
        let mut sample: Vec<f64> = Vec::with_capacity(DIMENSION);
        for _j in 0..DIMENSION {
            sample.push(rnd.gen());
        }
        let chunk = VectorChunk {
            id: rnd.gen(),
            vectors: sample,
        };
        samples.push(chunk);
    }

    let nn = nn_search(&samples, _request.query.as_slice());

    let response: Vec<ChunkResponse> = nn
        .iter()
        .map(|n| ChunkResponse {
            chunk: n.0.idx().unwrap(),
            score: n.1,
            text: String::from("temp"),
        })
        .collect::<Vec<_>>();

    Body::from_json(&response)
}

fn nn_search(samples: &Vec<VectorChunk>, query: &[f64]) -> Vec<(Node<f64, u64>, f64)> {
    let mut index = hora::index::bruteforce_idx::BruteForceIndex::<f64, u64>::new(
        DIMENSION,
        &hora::index::bruteforce_params::BruteForceParams::default(),
    );

    // Probably a more optimal way to do this
    for sample in samples.iter() {
        index.add(sample.vectors.as_slice(), sample.id).unwrap();
    }

    index.build(hora::core::metrics::Metric::Euclidean).unwrap();

    let seed = query;
    index.search_nodes(seed, samples.len())
}

pub struct TextChunk {
    id: u64,
    text: String,
}

fn bm25_search(samples: &Vec<TextChunk>, query: String) -> Result<Vec<ChunkResponse>> {
    let index_path = TempDir::new()?;
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("text", TEXT | STORED);
    schema_builder.add_u64_field("id", STORED);

    let schema = schema_builder.build();
    let index = Index::create_in_dir(&index_path, schema.clone())?;
    let mut index_writer = index.writer(50_000_000)?;

    let id = schema.get_field("id").unwrap();
    let text = schema.get_field("text").unwrap();
    //todo the clone here smells dirty ...
    for sample in samples.iter() {
        index_writer
            .add_document(doc!(
            id => sample.id,
            text => sample.text.clone()))
            .unwrap();
    }

    index_writer.commit()?;

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into()?;

    let searcher = reader.searcher();

    let query_parser = QueryParser::for_index(&index, vec![text]);

    let query = query_parser.parse_query(&query)?;

    let k = samples.len();
    let top_docs = searcher.search(&query, &TopDocs::with_limit(k))?;

    let mut result = Vec::with_capacity(top_docs.len());
    for (score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address)?;
        println!("{}", schema.to_json(&retrieved_doc));
        result.push(ChunkResponse {
            chunk: retrieved_doc.get_first(id).unwrap().as_u64().unwrap(),
            score: score as f64,
            text: retrieved_doc
                .get_first(text)
                .unwrap()
                .as_text()
                .unwrap()
                .to_string(),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::{bm25_search, nn_search, TextChunk, VectorChunk, DIMENSION};
    use rand::Rng;

    #[test]
    fn test_nn_search() {
        let mut rnd = rand::thread_rng();
        let n = 1000;
        let mut samples = Vec::with_capacity(n);
        for _ in 0..n {
            let mut sample: Vec<f64> = Vec::with_capacity(DIMENSION);
            for _ in 0..DIMENSION {
                sample.push(rnd.gen());
            }
            let chunk = VectorChunk {
                id: rnd.gen(),
                vectors: sample,
            };
            samples.push(chunk);
        }
        let target: usize = rnd.gen_range(0..n);
        let seed = samples[target].vectors.as_slice();

        assert!(!nn_search(&samples, seed).is_empty());
    }

    #[test]
    fn test_bm25_search() {
        let mut samples = Vec::with_capacity(4);
        samples.push(TextChunk {
            id: 1,
            text: "abc".to_string(),
        });
        samples.push(TextChunk {
            id: 2,
            text: "cde".to_string(),
        });
        samples.push(TextChunk {
            id: 3,
            text: "abe".to_string(),
        });
        samples.push(TextChunk {
            id: 4,
            text: "abx".to_string(),
        });
        let query = "abc".to_string();

        let result = bm25_search(&samples, query);
        println!("{:?} bm25 search result", result);
        assert!(!result.unwrap().is_empty());
    }
}
