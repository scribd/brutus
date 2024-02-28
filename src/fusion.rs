use crate::index::{SearchResult, SearchResultData};
use std::collections::HashMap;
pub trait Fusion {
    fn merge(r1: &mut Vec<SearchResult>, r2: &mut Vec<SearchResult>) -> Vec<SearchResult>;
}

pub struct RankedFusion {}

impl Fusion for RankedFusion {
    fn merge(r1: &mut Vec<SearchResult>, r2: &mut Vec<SearchResult>) -> Vec<SearchResult> {
        //todo probably lots of cleanup here initial POC
        // ... whats the best way to combine scores computationally
        // remove repeated code i.e. compares score to a func

        fn rank(r: &HashMap<i64, (f64, &SearchResult)>, id: &i64) -> f64 {
            r.get(&id).map(|(r, _)| r.clone()).unwrap_or(0.0_f64)
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

        // add position (rank) in sorted vec and convert to hashmap of ID -> rank
        r1.sort_by(|a, b| a.score.total_cmp(&b.score));
        //.sort_unstable_by_key(|sr| sr.score);
        //.sort_by(|a, b| a.cmp(b).unwrap());
        let r1_ranked = r1
            .iter()
            .enumerate()
            .map(|(i, sr)| (sr.chunk, (i as f64, sr)))
            .collect::<HashMap<_, _>>();

        r2.sort_by(|a, b| a.score.total_cmp(&b.score));
        let r2_ranked = r2
            .iter()
            .enumerate()
            .map(|(i, sr)| (sr.chunk, (i as f64, sr)))
            .collect::<HashMap<_, _>>();

        //generate unique vec of ID for which a score exists
        let unique_ids = r1_ranked
            .keys()
            .chain(r2_ranked.keys())
            .cloned()
            .collect::<Vec<_>>();

        //for each id generate score as rank of id in r1 + rank of id in r2
        let mut result = unique_ids
            .iter()
            .map(|id| SearchResult {
                chunk: id.clone(),
                score: rank(&r1_ranked, &id) + rank(&r2_ranked, &id),
                data: data(&r1_ranked, &r2_ranked, &id),
            })
            .collect::<Vec<_>>();

        result.sort_by(|a, b| a.score.total_cmp(&b.score));
        result
    }
}
