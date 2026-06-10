use std::{
    fs,
    io::{Write, stdin, stdout},
    path::{Path, PathBuf},
    str::FromStr,
};

const CONFIG_NAME: &str = "ragnar.toml";

#[derive(serde::Deserialize, Clone)]
pub struct Config {
    pub ragnar_port: u16,
    pub top_k: usize,
    pub prepend_context: bool,
    pub db_file: String,
    pub docs_folder: String,
    pub docs_log_file: String,
    pub api: String,
    pub chat_model: String,
    pub embed_model: String,
    pub chat_completions_path: String,
    pub embed_path: String,
    pub embed_ndims: i32,
}

pub fn load_or_create() -> Config {
    // Config file in current directory overrides system config folder
    let config_path: PathBuf;
    if Path::new(".").join(CONFIG_NAME).is_file() {
        config_path = Path::new(".").to_path_buf();
    } else {
        config_path = Path::new(&dirs::config_dir().unwrap_or_else(|| {
            eprintln!(
                "failed to identify config dir for your system,
        falling back to program current directory for storing config file"
            );
            Path::new(".").to_path_buf()
        }))
        .join(CONFIG_NAME);
    }

    let default_data_dir = Path::new(&dirs::data_dir().unwrap_or_else(|| {
        eprintln!(
            "failed to identify data dir for your system,
        falling back to program current directory (./ragnar_data/ragnar/data)"
        );
        Path::new("./ragnar_data").to_path_buf()
    }))
    .join("ragnar")
    .join("data")
    .to_str()
    .unwrap()
    .to_owned();

    let default_docs_dir = Path::new(&dirs::data_dir().unwrap_or_else(|| {
        eprintln!(
            "failed to identify data dir for your system,
        falling back to program current directory (./ragnar_data/ragnar/docs/)"
        );
        Path::new("./ragnar_data").to_path_buf()
    }))
    .join("ragnar")
    .join("docs")
    .to_str()
    .unwrap()
    .to_owned();

    // New config initialization
    if !config_path.is_file() {
        println!("Config file not found, initializing new config file...");

        let data_dir = input(
            &format!(
                "Enter path to data folder for RAGnar, otherwise it sets to default ({default_data_dir}): "
            ),
            default_data_dir,
        );

        let docs_dir = input(
            &format!(
                "Enter path to docs folder for documents auto retrieval, otherwise it sets to default ({default_docs_dir}): "
            ),
            default_docs_dir,
        );

        let api = input(
            "Enter your LLM API (http://localhost:11434): ",
            String::from("http://localhost:11434"),
        );

        let chat_model = input(
            "Enter the exact chat model your api can use (tinyllama): ",
            String::from("tinyllama"),
        );

        let embed_model = input(
            "Enter the exact embedding model your api can use (nomic-embed-text): ",
            String::from("nomic-embed-text"),
        );

        let generated_config = generate_config(&data_dir, &docs_dir, &api, &chat_model, &embed_model);

        fs::write(&config_path, generated_config).expect(&format!(
            "failed to write to config file at {}",
            config_path.to_str().unwrap()
        ));
    }

    // Load config
    let config: Config = toml::from_str(&fs::read_to_string(&config_path).unwrap_or_else(|_| {
        panic!(
            "failed to read config file at: {}",
            config_path.to_str().unwrap()
        )
    }))
    .expect("failed to parse config toml, bad content");

    return config;
}

fn input<T: FromStr + Clone>(text: &str, default: T) -> T {
    let mut user_input = String::new();
    print!("{}", text);
    stdout().flush().unwrap();
    stdin()
        .read_line(&mut user_input)
        .expect("failed to read stdin");
    let to_return = user_input.trim().parse().unwrap_or_else(|_| {
        eprintln!("[!] Bad input, using default value instead.");
        default.clone()
    });

    if user_input.trim().len() <= 0 {
        println!("Default value set.");
        default
    } else {
        to_return
    }
}

fn generate_config(
    data_dir: &str,
    docs_dir: &str,
    api: &str,
    chat_model: &str,
    embed_model: &str,
) -> String {
    format!(
        r#"# Ragnar
docs_folder = "{docs_dir}" # folder to watch for new documents for RAG
ragnar_port = 11435
top_k = 5 # top k documents pulled
prepend_context = true # add context to paragraphs before embedding

# Database
db_file = "{data_dir}/lancedb"
docs_log_file = "{data_dir}/docs_seen.txt"

# LLM (OpenAI-compatible API)
api = "{api}"
chat_model = "{chat_model}"
embed_model = "{embed_model}"
chat_completions_path = "/v1/chat/completions"
embed_path = "/v1/embeddings"
embed_ndims = 768 # change if using a different embedding model
"#
    )
}
