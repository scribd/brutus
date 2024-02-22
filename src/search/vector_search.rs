use hora::core::{ann_index::ANNIndex, node::Node};
use serde::{Deserialize, Serialize};
use thiserror::Error;

//an index stores vector chunks

#[derive(Clone)]
pub struct VectorChunk {
    pub id: u64,
    pub vectors: Vec<f64>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct VectorSearchResult {
    // VectorChunk iod
    chunk: u64,
    // score
    score: f64,
}

pub trait VectorSearch<E> {
    // build the Vector Search index. This is done when all records are added
    fn build(&mut self) -> Result<(), E>;
    // add a new Vector Chunk to the index
    fn add(&mut self, chunk: VectorChunk) -> Result<(), E>;
    // search the index relies on items being added and the index being built
    fn search(&mut self, query: &Vec<f64>, k: usize) -> Result<Vec<VectorSearchResult>, E>;
}

#[derive(Debug, Clone, Error)]
pub enum HoraError {
    /// Generic error message todo add more specific types
    #[error("An Error Happened with the Hora Library '{0}'")]
    Error(String),
}

pub struct HoraVectorSearch {
    index: hora::index::bruteforce_idx::BruteForceIndex<f64, u64>,
}

impl HoraVectorSearch {
    pub fn new(dimension: usize) -> HoraVectorSearch {
        HoraVectorSearch {
            index: hora::index::bruteforce_idx::BruteForceIndex::<f64, u64>::new(
                dimension,
                &hora::index::bruteforce_params::BruteForceParams::default(),
            ),
        }
    }
}

impl VectorSearch<HoraError> for HoraVectorSearch {
    fn add(&mut self, chunk: VectorChunk) -> Result<(), HoraError> {
        // Probably a more optimal way to do this
        //for sample in chunk.vectors {
        //  self.index.add(sample.as_slice(), chunk.id);
        //}
        self.index
            .add(&chunk.vectors, chunk.id)
            .map_err(|err| HoraError::Error(err.to_string()))
    }

    fn build(&mut self) -> Result<(), HoraError> {
        self.index
            .build(hora::core::metrics::Metric::Euclidean)
            .map_err(|err| HoraError::Error(err.to_string()))
    }

    fn search(&mut self, query: &Vec<f64>, k: usize) -> Result<Vec<VectorSearchResult>, HoraError> {
        let nn = self.index.search_nodes(&query.as_slice(), k);
        let response: Vec<VectorSearchResult> = nn
            .iter()
            .map(|n| VectorSearchResult {
                chunk: n.0.idx().unwrap(), // todo throw error instead of unwrap .ok_or_else(|| HoraError("idx not found"))?,
                score: n.1,
            })
            .collect();

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use crate::search::vector_search::VectorSearch;

    use super::{HoraVectorSearch, VectorChunk};
    use rand::Rng;

    #[test]
    fn test_nn_search() {
        let mut rnd = rand::thread_rng();
        let n = 1000;
        let d = 1024;
        let mut hora_search = HoraVectorSearch::new(d);
        let mut seed: VectorChunk;
        for i in 0..n {
            let mut sample: Vec<f64> = Vec::with_capacity(d);
            for _ in 0..d {
                sample.push(rnd.gen());
            }
            let chunk = VectorChunk {
                id: rnd.gen(),
                vectors: sample,
            };
            hora_search.add(chunk).unwrap();
        }

        hora_search.build().unwrap();

        let mut seed: Vec<f64> = Vec::with_capacity(d);
        for _ in 0..d {
            seed.push(rnd.gen());
        }
        let target: usize = rnd.gen_range(0..n);
        let result = hora_search.search(&seed, n).unwrap();
        assert!(!result.is_empty());
    }
}
