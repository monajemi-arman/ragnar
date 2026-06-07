use reqwest::Client;
use tokio::sync::Mutex;
use std::sync::{Arc};

use crate::rag::database::Database;

pub mod app;
pub mod rag;

#[derive(serde::Deserialize, Clone)]
pub struct Config {
    ragnar_port: u16,
    top_k: usize,
    prepend_context: bool,
    db_file: String,
    docs_folder: String,
    docs_log_file: String,
    api: String,
    chat_completions_path: String,
    embed_path: String,
    embed_model: String,
    embed_ndims: i32,
}

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub config: Config,
    pub database: Arc<Mutex<Database>>,
}

impl AppState {
    fn new(config: Config) -> AppState {
        AppState {
            database: Arc::new(Mutex::new(Database::new(
                config.db_file.clone(),
                config.embed_ndims,
            ))),
            config,
            client: Client::default(),
        }
    }
}
