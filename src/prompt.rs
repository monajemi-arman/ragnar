use serde::{Deserialize, Serialize};

use crate::{AppState, PromptBody};

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

pub async fn generate_embedding(state: &AppState, text: &str) -> Result<Vec<f32>, anyhow::Error> {
    let body = serde_json::to_string(&EmbedRequest {
        model: &state.config.embed_model,
        input: text,
    })?;

    let resp = state
        .client
        .post(state.config.api.clone() + &state.config.embed_path)
        .body(body)
        .send()
        .await?
        .text()
        .await?;

    let mut parsed_resp: EmbedResponse = serde_json::from_str(&resp)?;

    match parsed_resp.data.pop() {
        Some(embed_resp) => Ok(embed_resp.embedding),
        None => Err(anyhow::anyhow!("bad response from embed api")),
    }
}

pub fn manipulate(prompt_body: &mut PromptBody) {}
