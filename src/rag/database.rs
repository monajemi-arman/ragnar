use std::{sync::Arc, time::Duration};

use arrow_array::{RecordBatch, RecordBatchIterator};
use arrow_schema::{DataType, Field, Schema};
use lancedb::{Connection, Table};
use tokio::time::sleep;

struct ChunkRecord {
    id: String,
    chunk_index: u32,
    text: String,
    embedding: Vec<f32>,
}

pub struct Database {
    db_file: String,
    conn: Option<lancedb::Connection>,
    table_name: String,
}

impl Database {
    pub fn new(db_file: String) -> Database {
        Database {
            db_file,
            conn: None,
            table_name: "chunks".to_owned(),
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
        if self
            .get_conn()
            .await
            .table_names()
            .execute()
            .await
            .expect("failed to list tables")
            .contains(&table_name.to_string())
        {
            return ();
        }

        let ndims = 128;
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("chunk_index", DataType::UInt32, false),
            Field::new("text", DataType::Utf8, false),
            Field::new(
                "embedding",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    ndims,
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

        let names = self
            .get_conn()
            .await
            .table_names()
            .execute()
            .await
            .expect("failed to list tables");

        if !names.contains(&table_name) {
            panic!("Table does not exist after creation!");
        }
    }
}
