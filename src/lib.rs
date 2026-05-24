use serde::{Deserialize, Serialize};

pub mod proxy;
pub mod prompt;

#[derive(serde::Deserialize, Clone)]
pub struct Config {
    ragnar_port: u16,
    api: String,
    chat_completions_path: String
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