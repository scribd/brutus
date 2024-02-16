///
/// Data Access Layer
///
/// This module is responsible for all the interactions with S3
///
use async_std::stream::StreamExt;
use object_store::{path::Path, ObjectStore};

use serde::{Deserialize, Serialize};

use std::sync::Arc;

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Document {
    id: u64,
    chunks: Vec<Chunk>,
}

#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Chunk {
    id: u64,
    sequence: Option<u64>,
    page: u64,
    text: String,
    embedding: Vec<f64>,
}

async fn doc_from_storage(
    prefix: impl AsRef<str>,
    store: Arc<dyn ObjectStore>,
) -> Result<Document, anyhow::Error> {
    use parquet::arrow::async_reader::*;
    use parquet::schema::printer::print_parquet_metadata;
    let location = Path::from(prefix.as_ref());

    let meta = store.head(&location).await.unwrap();

    // Show Parquet metadata
    let reader = ParquetObjectReader::new(store, meta);
    let builder = ParquetRecordBatchStreamBuilder::new(reader).await.unwrap();
    print_parquet_metadata(&mut std::io::stdout(), builder.metadata());

    let mut stream = builder.build()?;

    while let Some(batch) = stream.next().await {
        println!("BATCH: {batch:?}");
    }

    Ok(Document::default())
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
        Ok(())
    }
}
