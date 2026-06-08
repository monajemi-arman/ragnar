use ragnar::{app::start_server, config};

#[tokio::main]
async fn main() {
    let config = config::load_or_create();
    start_server(config).await;
}
