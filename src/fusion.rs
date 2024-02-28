use crate::index::{SearchResult, SearchResultData};
use std::collections::{HashMap, HashSet};

pub trait Fusion {
    fn merge(r1: &mut Vec<SearchResult>, r2: &mut Vec<SearchResult>) -> Vec<SearchResult>;
}

pub struct RankedFusion {}

impl Fusion for RankedFusion {
    fn merge(r1: &mut Vec<SearchResult>, r2: &mut Vec<SearchResult>) -> Vec<SearchResult> {
        //todo probably lots of cleanup here initial POC
        // ... whats the best way to combine scores computationally
        // remove repeated code i.e. compares score to a func

        fn rank(m: &HashMap<i64, (f64, &SearchResult)>, id: &i64) -> f64 {
            m.get(&id).map(|(r, _)| r.clone()).unwrap_or(0.0_f64)
        }

        fn data(
            r1: &HashMap<i64, (f64, &SearchResult)>,
            r2: &HashMap<i64, (f64, &SearchResult)>,
            id: &i64,
        ) -> SearchResultData {
            let d1 = r1
                .get(&id)
                .map(|(_, r)| r.data.clone())
                .unwrap_or(SearchResultData::Empty);
            let d2 = r2
                .get(&id)
                .map(|(_, r)| r.data.clone())
                .unwrap_or(SearchResultData::Empty);

            if d1 == SearchResultData::Empty {
                d2
            } else {
                d1
            }
        }

        // add position (rank) in sorted vec and convert to hashmap of ID -> (rank, SearchResult)
        r1.sort_by(|a, b| a.score.total_cmp(&b.score));
        let r1_ranked = r1
            .iter()
            .enumerate()
            .map(|(i, sr)| (sr.chunk, ((i as f64 + 1.0)/r1.len() as f64, sr)))
            .collect::<HashMap<_, _>>();


        r2.sort_by(|a, b| a.score.total_cmp(&b.score));
        let r2_ranked = r2
            .iter()
            .enumerate()
            .map(|(i, sr)| (sr.chunk, ((i as f64 + 1.0)/r2.len() as f64, sr)))
            .collect::<HashMap<_, _>>();

        //generate unique IDs for which a score exists
        let unique_ids = r1_ranked
            .keys()
            .chain(r2_ranked.keys())
            .cloned()
            .collect::<HashSet<_>>();

        //for each id generate score as rank of id in r1 + rank of id in r2
        let mut result = unique_ids
            .iter()
            .map(|id| SearchResult {
                chunk: id.clone(),
                score: (rank(&r1_ranked, &id) + rank(&r2_ranked, &id))/2.0,
                data: data(&r1_ranked, &r2_ranked, &id),
            })
            .collect::<Vec<_>>();

        result.sort_by(|a, b| a.score.total_cmp(&b.score));
        result.reverse();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranked_fusion(){
        let mut r1 = vec![
            SearchResult {
                chunk: 1,
                score: 0.1,
                data: SearchResultData::Empty,
            },
            SearchResult {
                chunk: 2,
                score: 0.2,
                data: SearchResultData::Empty,
            },
            SearchResult {
                chunk: 3,
                score: 0.3,
                data: SearchResultData::Empty,
            },
            SearchResult {
                chunk: 4,
                score: 0.4,
                data: SearchResultData::Empty,
            },
            SearchResult {
                chunk: 5,
                score: 0.5,
                data: SearchResultData::Empty,
            },
        ];

        let mut r2 = vec![
            SearchResult {
                chunk: 1,
                score: 0.11,
                data: SearchResultData::Empty,
            },
            SearchResult {
                chunk: 2,
                score: 0.12,
                data: SearchResultData::Empty,
            },
            SearchResult {
                chunk: 3,
                score: 0.3,
                data: SearchResultData::Empty,
            },
            SearchResult {
                chunk: 4,
                score: 0.6,
                data: SearchResultData::Empty,
            },
            SearchResult {
                chunk: 5,
                score: 0.13,
                data: SearchResultData::Empty,
            },
        ];

        let result = RankedFusion::merge(&mut r1, &mut r2);

        println!("{:?} r1", result);
        //println!("{:?} search result", result);
        assert_eq!(
            result[0], SearchResult {
                chunk: 4,
                score: 0.9,
                data: SearchResultData::Empty,
            }
        );
        
    }   
}
 