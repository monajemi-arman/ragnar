use reqwest::Client;
use serde::{Deserialize, Serialize};

pub mod database;
pub mod prompt;
pub mod proxy;

#[derive(serde::Deserialize, Clone)]
pub struct Config {
    ragnar_port: u16,
    db_file: String,
    api: String,
    chat_completions_path: String,
    embed_path: String,
    embed_model: String,
}

#[derive(Clone)]
pub struct AppState {
    client: Client,
    config: Config,
}

impl AppState {
    fn new(config: Config) -> AppState {
        AppState {
            config,
            client: Client::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct PromptBody {
    model: String,
    stream: Option<bool>,
    context: Option<Vec<i64>>,
    messages: Vec<PromptMessage>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PromptMessage {
    role: String,
    content: String,
}
