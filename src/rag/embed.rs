use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::{app::AppState, rag::database::ChunkRecord};

#[derive(Serialize, Deserialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}
#[derive(Serialize, Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}
#[derive(Serialize, Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

pub async fn generate_embedding(state: &AppState, text: &str) -> Result<Vec<f32>> {
    let body = serde_json::to_string(&EmbedRequest {
        model: &state.config.embed_model,
        input: text,
    })?;

    let client = Client::new();
    let resp = client
        .post(state.config.api.clone() + &state.config.embed_path)
        .body(body)
        .send()
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "Failed to send request to OpenAI-compatible API for embedding, is it even running?"
            )
        })?
        .text()
        .await?;

    let mut parsed_resp: EmbedResponse = serde_json::from_str(&resp)?;

    match parsed_resp.data.pop() {
        Some(embed_resp) => Ok(embed_resp.embedding),
        None => Err(anyhow::anyhow!("bad response from embed api")),
    }
}

pub async fn embed_and_save(state: &AppState, source: String, text: String) -> Result<()> {
    let embedding = generate_embedding(&state, &text).await?;
    let chunk: ChunkRecord = ChunkRecord {
        source,
        text,
        embedding,
    };
    state
        .database
        .lock()
        .await
        .insert_chunks(vec![chunk])
        .await?;
    Ok(())
}
