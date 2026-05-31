use axum::{
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{StatusCode, method},
    response::Response,
    routing::any,
};
use http_body_util::BodyExt;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use crate::{AppState, Config, PromptBody, rag};

pub async fn start_server(config: Config) {
    let state = AppState::new(config);

    // Database check
    state
        .database
        .lock()
        .expect("failed to get database lock")
        .ensure_table()
        .await;

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
    let mut req_body = req
        .into_body()
        .collect()
        .await
        .expect("failed to get request body")
        .to_bytes()
        .to_vec();

    // Modify prompt before sending to api
    if path == state.config.chat_completions_path && req_method == method::Method::POST {
        if let Ok(Json(mut prompt_body)) = Json::<PromptBody>::from_bytes(&req_body) {
            rag::prompt::manipulate(&mut prompt_body);
            req_body = serde_json::to_vec(&prompt_body).expect("failed to do json to vec");
        }
    }

    let resp_api = state
        .client
        .request(req_method, state.config.api + &path)
        .body(req_body)
        .send()
        .await
        .expect("failed to send requeset to api");

    Ok(Response::new(Body::from_stream(resp_api.bytes_stream())))
}
