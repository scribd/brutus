use hora::core::ann_index::ANNIndex;
use thiserror::Error;

use super::*;

#[derive(Debug, Clone, Error)]
pub enum HoraError {
    /// Generic error message todo add more specific types
    #[error("An Error Happened with the Hora Library '{0}'")]
    Error(String),
}

pub struct HoraVectorIndex {
    index: hora::index::bruteforce_idx::BruteForceIndex<f64, i64>,
    is_built: bool,
}

impl HoraVectorIndex {
    pub fn new(dimension: usize) -> HoraVectorIndex {
        HoraVectorIndex {
            index: hora::index::bruteforce_idx::BruteForceIndex::<f64, i64>::new(
                dimension,
                &hora::index::bruteforce_params::BruteForceParams::default(),
            ),
            // TODO this is the same across all index types, should be moved to a common place
            is_built: false,
        }
    }
}
impl Index for HoraVectorIndex {
    // TODO: I think this can turn into a slice
    type QueryType = Vec<f64>;
    type ErrorType = HoraError;

    fn add(&mut self, chunk: &Chunk) -> Result<(), Self::ErrorType> {
        self.index
            .add(chunk.embedding.as_slice(), chunk.id)
            .map_err(|err| HoraError::Error(err.to_string()))
    }

    fn build(&mut self) -> Result<(), HoraError> {
        let build_result = self.index.build(hora::core::metrics::Metric::Euclidean);

        match build_result {
            Err(e) => Err(HoraError::Error(e.to_string())),
            Ok(_) => {
                self.is_built = true;
                Ok(())
            }
        }
    }

    fn search(
        &mut self,
        query: Self::QueryType,
        k: usize,
    ) -> Result<Vec<SearchResult>, Self::ErrorType> {
        if self.is_built {
            let nn = self.index.search_nodes(&query.as_slice(), k);
            let response: Vec<SearchResult> = nn
                .iter()
                .map(|n| SearchResult {
                    chunk: n.0.idx().unwrap(), // todo throw error instead of unwrap .ok_or_else(|| HoraError("idx not found"))?,
                    score: n.1,
                    data: SearchResultData::Empty,
                })
                .collect();

            Ok(response)
        } else {
            Err(HoraError::Error("Index not built".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_nn_search() {
        let mut rnd = rand::thread_rng();
        let n = 1000;
        let d = 1024;
        let mut hora_search = HoraVectorIndex::new(d);
        for i in 0..n {
            let mut sample: Vec<f64> = Vec::with_capacity(d);
            for _ in 0..d {
                sample.push(rnd.gen());
            }
            let chunk = Chunk {
                id: rnd.gen(),
                embedding: sample,
                ..Default::default()
            };
            hora_search.add(&chunk).unwrap();
        }

        hora_search.build().unwrap();

        let mut seed: Vec<f64> = Vec::with_capacity(d);
        for _ in 0..d {
            seed.push(rnd.gen());
        }
        let target: usize = rnd.gen_range(0..n);
        let result = hora_search.search(seed, n).unwrap();
        assert!(!result.is_empty());
    }
}
