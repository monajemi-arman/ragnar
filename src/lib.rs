pub mod proxy;

#[derive(serde::Deserialize, Clone)]
pub struct Config {
    ragnar_port: u16,
    api: String,
    chat_completions_path: String
}