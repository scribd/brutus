//use core::slice::SlicePattern;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::ReloadPolicy;
use tantivy::TantivyError;
use tempfile::TempDir;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct TextChunk {
    pub id: u64,
    pub text: String,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TextSearchResult {
    // VectorChunk iod
    chunk: u64,
    // score
    score: f64,
    text: String,
}

pub trait TextSearch<E> {
    fn commit(&mut self) -> Result<(), E>;
    fn add(&mut self, chunk: &TextChunk) -> Result<(), E>;
    fn search(&mut self, query: String, k: usize) -> Result<Vec<TextSearchResult>, E>;
}

//would be nice to understand how to do this in a more elegant way lots of members being passed around here lots of state which i dont like
pub struct TantivyTextSearch {
    index_path: TempDir,
    index_writer: tantivy::IndexWriter,
    index: tantivy::Index,
    id: Field,
    text: Field,
    schema: Schema,
}

impl TantivyTextSearch {
    //todo map errors and use result types better instead of unwrap but need to enable flatten feature
    pub fn new() -> TantivyTextSearch {
        let index_path = TempDir::new().unwrap();
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("text", TEXT | STORED);
        schema_builder.add_u64_field("id", STORED);

        let schema = schema_builder.build();
        let index = Index::create_in_dir(&index_path, schema.clone()).unwrap();
        let index_writer = index.writer(50_000_000).unwrap();

        TantivyTextSearch {
            index_path: index_path,
            index_writer: index_writer,
            index: index,
            id: schema.get_field("id").unwrap(),
            text: schema.get_field("text").unwrap(),
            schema: schema,
        }
    }
}

impl TextSearch<TantivyError> for TantivyTextSearch {
    fn commit(&mut self) -> Result<(), TantivyError> {
        self.index_writer.commit().map(|op| ())
    }

    fn add(&mut self, chunk: &TextChunk) -> Result<(), TantivyError> {
        //todo the clone here smells dirty ...
        self.index_writer
            .add_document(doc!(
            self.id => chunk.id,
            self.text => chunk.text.clone()))
            .map(|_| ())
    }

    fn search(&mut self, query: String, k: usize) -> Result<Vec<TextSearchResult>, TantivyError> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.text]);

        let query = query_parser.parse_query(&query)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(k))?;

        let mut result = Vec::with_capacity(top_docs.len());
        //todo clean this up. Get rid of multiple unwraps map and throw error
        for (score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            println!("{}", self.schema.to_json(&retrieved_doc));
            result.push(TextSearchResult {
                chunk: retrieved_doc.get_first(self.id).unwrap().as_u64().unwrap(),
                score: score as f64,
                text: retrieved_doc
                    .get_first(self.text)
                    .unwrap()
                    .as_text()
                    .unwrap()
                    .to_string(),
            });
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{TantivyTextSearch, TextChunk, TextSearch};

    #[test]

    fn test_text_search() {
        let mut text_search = TantivyTextSearch::new();

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

        //todo is there a way to fail tests instead of unwrap ??
        for sample in samples.iter() {
            text_search.add(sample).unwrap();
        }

        let query = "abc".to_string();
        text_search.commit().unwrap();
        let result = text_search.search(query, samples.len()).unwrap();
        println!("{:?} search result", result);
        assert!(!result.is_empty());
    }
}
