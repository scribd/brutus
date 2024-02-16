///
/// Data Access Layer
///
/// This module is responsible for all the interactions with S3
///
use arrow::array::cast::*;
use arrow::array::*;
use arrow::datatypes::*;
use async_std::stream::StreamExt;
use object_store::{path::Path, ObjectStore};
use parquet::arrow::async_reader::*;
use serde::{Deserialize, Serialize};

use std::sync::Arc;

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

async fn doc_from_storage(
    prefix: impl AsRef<str>,
    store: Arc<dyn ObjectStore>,
) -> Result<Document, anyhow::Error> {
    let location = Path::from(prefix.as_ref());

    let meta = store.head(&location).await.unwrap();

    // Show Parquet metadata
    let reader = ParquetObjectReader::new(store, meta);
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

#[cfg(test)]
mod tests {
    use super::*;

    use object_store::local::LocalFileSystem;

    #[async_std::test]
    async fn test_load_static_file() -> Result<(), anyhow::Error> {
        let store = Arc::new(LocalFileSystem::new_with_prefix(std::env::current_dir()?)?);

        let path = "tests/data/doc_id_1106528470000.parquet";
        let doc = doc_from_storage(&path, store.clone())
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
