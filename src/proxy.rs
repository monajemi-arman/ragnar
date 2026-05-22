use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{StatusCode, method},
    response::Response,
    routing::any,
};
use http_body_util::BodyExt;
use reqwest::Client;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::Config;

#[derive(Clone)]
struct AppState {
    client: Client,
    config: Config,
}

pub async fn start_server(config: Config) {
    let state = AppState {
        config,
        client: Client::default(),
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], state.config.ragnar_port));
    let listener = TcpListener::bind(addr)
        .await
        .unwrap_or_else(|_| panic!("failed to bind to port {}", addr));

    let router = Router::new()
        .route("/{*path}", any(handler))
        .route("/", any(handler))
        .with_state(state);

    axum::serve(listener, router)
        .await
        .expect("failed to serve axum");
}

async fn handler(State(state): State<AppState>, req: Request) -> Result<Response, StatusCode> {
    let path = req.uri().path().to_owned();
    let req_method = req.method().clone();
    let req_body = req
        .into_body()
        .collect()
        .await
        .expect("failed to get request body")
        .to_bytes();

    // Modify prompt before sending to api
    if path == state.config.chat_completions_path && req_method == method::Method::POST {}

    let resp_api = state
        .client
        .request(req_method, state.config.api + &path)
        .body(req_body)
        .send()
        .await
        .expect("failed to send requeset to api");

    Ok(Response::new(Body::from_stream(resp_api.bytes_stream())))
}
