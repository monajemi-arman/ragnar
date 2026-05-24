use ragnar::{
    Config,
    proxy::start_server,
};
use std::fs;
use toml;

const CONFIG_FILE: &str = "config.toml";

#[tokio::main]
async fn main() {
    let config: Config = toml::from_str(
        &fs::read_to_string(CONFIG_FILE)
            .unwrap_or_else(|_| panic!("failed to read config file at: {}", CONFIG_FILE)),
    )
    .expect("failed to parse config toml, bad content");

    start_server(config).await;
}
