use crate::Config;

struct Database {
    db_file: String,
    conn: Option<lancedb::Connection>,
}

impl Database {
    fn new(db_file: String) -> Database {
        Database { db_file, conn: None }
    }

    async fn connect(&mut self) {
        let conn = lancedb::connect(&self.db_file).execute().await;
        if conn.is_ok() {
            self.conn = Some(conn.unwrap());
        }
        else {
            self.conn = None;
        }
    }
}

pub async fn create_or_load_db(config: &Config) {
    let mut db = Database::new(config.db_file.clone());
    db.connect().await;
}
