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
    chat_model: String,
    embed_model: String,
    chat_completions_path: String,
    embed_path: String,
    embed_ndims: i32,
}