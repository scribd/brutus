///
/// Data Access Layer
///
/// This module is responsible for all the interactions with S3
///
use arrow::array::cast::*;
use arrow::array::*;
use arrow::datatypes::*;
use async_std::stream::StreamExt;
use object_store::{aws::AmazonS3Builder, local::LocalFileSystem, path::Path, ObjectStore};
use parquet::arrow::async_reader::*;
use serde::{Deserialize, Serialize};
use tracing::log::*;
use url::Url;

use std::sync::Arc;

/// State is a struct to be used as state for web requests in the service.
///
/// This is being done instead of passing the `store` directly as state in case we need to expand
/// with more members of [State] in the future
#[derive(Debug, Clone)]
pub struct State {
    store: Arc<dyn ObjectStore>,
}

impl State {
    pub fn from_env() -> Result<Self, crate::error::Error> {
        let store: Arc<dyn ObjectStore> = match std::env::var("BRUTUS_DOCUMENTS_URL") {
            Ok(url) => {
                let store = AmazonS3Builder::from_env().with_url(url).build()?;
                Arc::new(store)
            }
            Err(_) => {
                warn!("Using the current working directory for loading documents");
                // If the environment variable hasn't been set, as in testing, just use the
                // current working directory
                Arc::new(LocalFileSystem::new_with_prefix(std::env::current_dir()?)?)
            }
        };

        Ok(Self {
            store: store.into(),
        })
    }

    pub async fn fetch_doc(
        &self,
        prefix: impl AsRef<str>,
    ) -> Result<Document, crate::error::Error> {
        let location = Path::from(prefix.as_ref());

        let meta = self.store.head(&location).await.unwrap();

        // Show Parquet metadata
        let reader = ParquetObjectReader::new(self.store.clone(), meta);
        let builder = ParquetRecordBatchStreamBuilder::new(reader).await.unwrap();

        let mut stream = builder.build()?;
        let mut document = Document::default();

        while let Some(Ok(batch)) = stream.next().await {
            let ids: &PrimitiveArray<Int64Type> = as_primitive_array(
                batch
                    .column_by_name("chunk_id")
                    .expect("Failed to get chunk_id from parquet file")
                    .as_ref(),
            );
            let texts = as_string_array(
                batch
                    .column_by_name("chunk_text")
                    .expect("Failed to get `chunk_text` from parquet file")
                    .as_ref(),
            );
            let pages: &PrimitiveArray<Int32Type> = as_primitive_array(
                batch
                    .column_by_name("page")
                    .expect("Failed to get `page` from parquet file")
                    .as_ref(),
            );
            let sequences: &PrimitiveArray<Int32Type> = as_primitive_array(
                batch
                    .column_by_name("chunk_sequence")
                    .expect("Failed to get `chunk_sequence` from parquet file")
                    .as_ref(),
            );
            let embeddings = as_list_array(
                batch
                    .column_by_name("chunk_embedding")
                    .expect("Failed to get `chunk_embedding` from parquet file")
                    .as_ref(),
            );

            for row in 0..batch.num_rows() {
                let embeddings = embeddings.value(row);
                let ems: &PrimitiveArray<Float32Type> = as_primitive_array(embeddings.as_ref());
                let chunk = Chunk {
                    id: ids.value(row),
                    page: pages.value(row),
                    text: texts.value(row).into(),
                    sequence: Some(sequences.value(row)),
                    embedding: ems.iter().map(|v| v.unwrap()).collect(),
                };
                document.chunks.push(chunk);
            }
        }

        Ok(document)
    }
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Document {
    id: u64,
    chunks: Vec<Chunk>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Chunk {
    id: i64,
    sequence: Option<i32>,
    page: i32,
    text: String,
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "integration")]
    fn setup() {
        use std::env;

        env::set_var("AWS_ACCESS_KEY_ID", "deltalake");
        env::set_var("AWS_SECRET_ACCESS_KEY", "weloverust");
        env::set_var("AWS_ENDPOINT", "http://localhost:4566");
        env::set_var("AWS_ALLOW_HTTP", "true");
        env::set_var("BRUTUS_DOCUMENTS_URL", "s3://brutus-data");
    }

    #[cfg(feature = "integration")]
    #[async_std::test]
    async fn test_load_static_file() -> Result<(), crate::error::Error> {
        setup();
        let state = State::from_env()?;

        let path = "doc_id_1106528470000.parquet";
        let doc = state
            .fetch_doc(&path)
            .await
            .expect("Failed to load document from storage");

        assert_ne!(doc.chunks.len(), 0);
        assert_eq!(doc.chunks.len(), 5);
        let chunk = doc.chunks.first().expect("We should have a first element");
        assert_eq!(chunk.id, -896282756710128915);
        assert_eq!(chunk.embedding[0], 0.026046248);
        Ok(())
    }
}
