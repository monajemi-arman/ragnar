use axum::{
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::Response,
    routing::any,
};
use http_body_util::BodyExt;
use reqwest::Client;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::{join, net::TcpListener};

use crate::rag::{database::Database, docs::watch_folder};
use crate::{config::Config, rag::prompt};

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

pub async fn start_server(config: Config) {
    let state = AppState::new(config);

    // Database check
    state.database.lock().await.ensure_table().await;

    let addr = SocketAddr::from(([127, 0, 0, 1], state.config.ragnar_port));
    let listener = TcpListener::bind(addr)
        .await
        .unwrap_or_else(|_| panic!("failed to bind to port {}", addr));

    let router = Router::new()
        .route("/{*path}", any(handler))
        .route("/", any(handler))
        .with_state(state.clone());

    let (_, result) = join!(watch_folder(&state), axum::serve(listener, router));
    result.expect("failed to start server");
}

async fn handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
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
    if path == state.config.chat_completions_path && req_method == Method::POST {
        match Json::from_bytes(&req_body) {
            Ok(Json(mut prompt_body)) => {
                prompt::manipulate(&state, &mut prompt_body).await;
                req_body = serde_json::to_vec(&prompt_body).expect("failed to do json to vec");
            }
            Err(e) => eprintln!("Json::from_bytes failed: {:?}", e),
        }
    }

    let resp_api = state
        .client
        .request(req_method, state.config.api.clone() + &path)
        .body(req_body)
        .send()
        .await
        .expect("failed to send requeset to api");

    let mut response_builder = Response::builder().status(resp_api.status());
    for (key, value) in resp_api.headers() {
        response_builder = response_builder.header(key, value);
    }
    Ok(response_builder
        .body(Body::from_stream(resp_api.bytes_stream()))
        .unwrap())
}
