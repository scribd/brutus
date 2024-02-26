//use core::slice::SlicePattern;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, ReloadPolicy, TantivyError};
use tempfile::TempDir;
use tracing::{event, info_span, Level};

use super::*;

//would be nice to understand how to do this in a more elegant way lots of members being passed around here lots of state which i dont like
pub struct TantivyTextIndex {
    index_path: TempDir,
    index_writer: tantivy::IndexWriter,
    index: tantivy::Index,
    id: Field,
    text: Field,
    schema: Schema,
    is_built: bool,
}

impl TantivyTextIndex {
    //todo map errors and use result types better instead of unwrap but need to enable flatten feature
    pub fn new() -> TantivyTextIndex {
        let index_path = TempDir::new().unwrap();
        let mut schema_builder = Schema::builder();

        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("en_stem")
                    .set_index_option(IndexRecordOption::Basic),
            )
            .set_stored();

        let id_options = NumericOptions::default().set_stored();

        schema_builder.add_text_field("text", text_options);
        schema_builder.add_i64_field("id", id_options);

        let schema = schema_builder.build();
        let index = tantivy::Index::create_in_dir(&index_path, schema.clone()).unwrap();
        let index_writer = index.writer(50_000_000).unwrap();

        TantivyTextIndex {
            index_path: index_path,
            index_writer: index_writer,
            index: index,
            id: schema.get_field("id").unwrap(),
            text: schema.get_field("text").unwrap(),
            schema: schema,
            is_built: false,
        }
    }
}

impl Index for TantivyTextIndex {
    type QueryType = String;
    type ErrorType = TantivyError;

    fn build(&mut self) -> Result<(), Self::ErrorType> {
        let build_result = self.index_writer.commit();
        match build_result {
            Err(e) => Err(e),
            Ok(_) => {
                self.is_built = true;
                Ok(())
            }
        }
    }

    fn add(&mut self, chunk: &Chunk) -> Result<(), Self::ErrorType> {
        //todo the clone here smells dirty ...
        self.index_writer
            .add_document(doc!(
            self.id => chunk.id,
            self.text => chunk.text.clone()))
            .map(|_| ())
    }

    fn search(&mut self, query: String, k: usize) -> Result<Vec<SearchResult>, Self::ErrorType> {
        if self.is_built {
            let span = info_span!("TantivyTextSearch::search");
            let _guard = span.enter();
            let start = std::time::Instant::now();

            let reader = self
                .index
                .reader_builder()
                .reload_policy(ReloadPolicy::OnCommit)
                .try_into()?;

            let searcher = reader.searcher();
            let query_parser = QueryParser::for_index(&self.index, vec![self.text]);
            let query = query_parser.parse_query(&query)?;
            event!(Level::INFO, elapsed=?start.elapsed(), "query parsed");

            let top_docs = searcher.search(&query, &TopDocs::with_limit(k))?;
            event!(Level::INFO, elapsed=?start.elapsed(), "search executed");

            let mut result = Vec::with_capacity(top_docs.len());
            //todo clean this up. Get rid of multiple unwraps map and throw error
            for (score, doc_address) in top_docs {
                let retrieved_doc = searcher.doc(doc_address)?;
                println!("{}", self.schema.to_json(&retrieved_doc));
                println!("wtf is this: {:?}", retrieved_doc.get_first(self.id));
                result.push(SearchResult {
                    chunk: retrieved_doc
                        .get_first(self.id)
                        .unwrap()
                        .as_i64()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                    score: score as f64,
                    data: SearchResultData::String(
                        retrieved_doc
                            .get_first(self.text)
                            .unwrap()
                            .as_text()
                            .unwrap()
                            .to_string(),
                    ),
                });
            }

            event!(Level::INFO, elapsed=?start.elapsed(), "results prepared");
            Ok(result)
        } else {
            Err(TantivyError::SchemaError(
                "Index not built yet, call build() first".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_search() {
        let mut text_search = TantivyTextIndex::new();

        let samples = vec![
            Chunk {
                id: 1,
                text: "abc".to_string(),
                ..Default::default()
            },
            Chunk {
                id: 2,
                text: "cde".to_string(),
                ..Default::default()
            },
            Chunk {
                id: 3,
                text: "abe".to_string(),
                ..Default::default()
            },
            Chunk {
                id: 4,
                text: "abx".to_string(),
                ..Default::default()
            },
        ];

        //todo is there a way to fail tests instead of unwrap ??
        for sample in samples.iter() {
            text_search.add(sample).expect("Failed to add sample");
        }

        let query = "abc".to_string();
        text_search.build().unwrap();
        let result = text_search.search(query, samples.len()).unwrap();
        println!("{:?} search result", result);
        assert!(!result.is_empty());
    }
}
