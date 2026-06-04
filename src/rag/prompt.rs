use serde::{Deserialize, Serialize};

use crate::{AppState, rag::embed::generate_embedding};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct PromptBody {
    model: String,
    stream: Option<bool>,
    context: Option<Vec<i64>>,
    messages: Vec<PromptMessage>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PromptMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
pub struct PromptResponse {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Deserialize)]
pub struct Message {
    pub content: String,
}

pub async fn prompt(
    state: &AppState,
    prompt_body: &mut PromptBody,
) -> Result<String, anyhow::Error> {
    prompt_body.stream = Some(false);
    let body = serde_json::to_string(&prompt_body)?;

    let resp = state
        .client
        .post(state.config.api.clone() + &state.config.chat_completions_path)
        .body(body)
        .send()
        .await?
        .text()
        .await?;

    let mut parsed: PromptResponse = serde_json::from_str(&resp)?;
    match parsed.choices.pop() {
        Some(x) => Ok(x.message.content),
        None => return Err(anyhow::anyhow!("Empty prompt response from llm api")),
    }
}

pub async fn manipulate(state: &AppState, prompt_body: &mut PromptBody) {
    let top_k = state.config.top_k;
    let Some(last_message) = prompt_body.messages.pop() else {
        println!("no messages so no prompt manipulation");
        return;
    };
    let conversation_history: Vec<_> = prompt_body
        .messages
        .iter()
        .map(|msg| msg.content.as_str())
        .collect();

    let query_text;
    if conversation_history.len() > 0 {
        query_text = format!(
        "Create a standalone query from the following query based on additional conversation history.
        Query: {}
        Convesation history: {}
        ",
        last_message.content,
        conversation_history.join("\n")
    );
    }
    else {
        query_text = last_message.content
    }

    let Ok(query_embedding) = generate_embedding(&state, &query_text).await else {
        println!("no embedding came so no prompt manipulation");
        return;
    };

    let results = state
        .database
        .lock()
        .await
        .search(query_embedding, top_k)
        .await
        .expect("failed to get search result from database");

    let prompt_augment_vec: Vec<_> = results
        .iter()
        .map(|x| format!("{}: {}", x.0, x.1))
        .collect();
    let prompt_augment = prompt_augment_vec.join(", ");

    prompt_body.messages.append(&mut vec![PromptMessage {
        role: "user".to_owned(),
        content: format!(
            "Answer to prompt using this context.
        =START OF CONTEXT=
        {}
        =END OF CONTEXT=",
            prompt_augment
        ),
    }]);
}
