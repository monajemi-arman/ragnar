use std::{sync::Arc, time::Duration};

use arrow_array::{
    FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, RecordBatchReader,
    StringArray, UInt32Array,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::{
    Connection, Table,
    query::{ExecutableQuery, QueryBase},
};
use tokio::time::sleep;

pub struct ChunkRecord {
    pub id: String,
    pub chunk_index: u32,
    pub source: String,
    pub text: String,
    pub embedding: Vec<f32>,
}

pub struct Database {
    db_file: String,
    conn: Option<lancedb::Connection>,
    table_name: String,
    table: Option<Table>,
    ndims: i32,
}

impl Database {
    pub fn new(db_file: String, ndims: i32) -> Database {
        Database {
            db_file,
            conn: None,
            table_name: "chunks".to_owned(),
            table: None,
            ndims,
        }
    }

    async fn connect(&mut self) {
        let conn = lancedb::connect(&self.db_file).execute().await;
        if conn.is_ok() {
            self.conn = Some(conn.unwrap());
        } else {
            self.conn = None;
        }
    }

    async fn get_conn(&mut self) -> &Connection {
        for _ in 0..5 {
            if self.conn.is_none() {
                self.connect().await;
                sleep(Duration::from_secs(1)).await;
            } else {
                return self.conn.as_ref().unwrap();
            }
        }
        panic!("failed to connect to database after 5 tries")
    }

    /// Create or verify table exists
    pub async fn ensure_table(&mut self) {
        let table_name = self.table_name.clone();

        // Return if table exists
        if !self
            .get_conn()
            .await
            .table_names()
            .execute()
            .await
            .expect("failed to list tables")
            .contains(&table_name.to_string())
        {
            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new("chunk_index", DataType::UInt32, false),
                Field::new("source", DataType::Utf8, false),
                Field::new("text", DataType::Utf8, false),
                Field::new(
                    "embedding",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        self.ndims,
                    ),
                    false,
                ),
            ]));
            let empty: Box<dyn arrow_array::RecordBatchReader + Send> =
                Box::new(RecordBatchIterator::new(vec![], schema.clone()));

            self.get_conn()
                .await
                .create_table(&table_name, empty)
                .execute()
                .await
                .expect("failed to create table");
        }
        self.table = Some(
            self.get_conn()
                .await
                .open_table(table_name)
                .execute()
                .await
                .expect("failed to get table"),
        );
    }

    /// Insert a batch of chunk records
    pub async fn insert_chunks(&self, chunks: Vec<ChunkRecord>) -> anyhow::Result<()> {
        let ids = StringArray::from(chunks.iter().map(|c| c.id.as_str()).collect::<Vec<_>>());
        let sources =
            StringArray::from(chunks.iter().map(|c| c.source.as_str()).collect::<Vec<_>>());
        let indices: UInt32Array = chunks.iter().map(|c| c.chunk_index).collect();
        let texts = StringArray::from(chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>());

        // Flatten embeddings into one big Float32Array
        let flat: Float32Array = chunks.iter().flat_map(|c| c.embedding.clone()).collect();
        let embedding_col = FixedSizeListArray::try_new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            self.ndims,
            Arc::new(flat),
            None,
        )?;

        let schema = self
            .table
            .as_ref()
            .expect("table not loaded")
            .schema()
            .await?;
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(ids),
                Arc::new(indices),
                Arc::new(sources),
                Arc::new(texts),
                Arc::new(embedding_col),
            ],
        )?;

        let batch_schema = batch.schema().clone();
        let iter: Box<dyn RecordBatchReader + Send> =
            Box::new(RecordBatchIterator::new(vec![Ok(batch)], batch_schema));
        self.table
            .as_ref()
            .expect("table not loaded")
            .add(iter)
            .execute()
            .await?;

        Ok(())
    }

    /// Query: returns top-k chunks by vector similarity
    pub async fn search(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> anyhow::Result<Vec<(String, String)>> {
        // Returns vec of (text, source, distance)
        let mut results = self
            .table
            .as_ref()
            .expect("table not loaded")
            .vector_search(query_embedding)?
            .limit(top_k)
            .execute()
            .await?;

        // Parse the result RecordBatches
        let mut out = vec![];
        while let Some(batch) = results.try_next().await? {
            let texts = batch
                .column_by_name("text")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| anyhow::anyhow!("missing text column"))?;
            let sources = batch
                .column_by_name("source")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| anyhow::anyhow!("missing source column"))?;

            for i in 0..batch.num_rows() {
                out.push((texts.value(i).to_string(), sources.value(i).to_string()));
            }
        }

        Ok(out)
    }
}
