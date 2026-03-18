use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::prompt::SYSTEM_PROMPT;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    max_completion_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

pub async fn cleanup(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    text: &str,
    endpoint: &str,
) -> Result<String> {
    let body = ChatRequest {
        model: model.to_string(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            },
            Message {
                role: "user".to_string(),
                content: text.to_string(),
            },
        ],
        max_completion_tokens: 8192,
    };

    let resp = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if resp.status() == 429 {
        anyhow::bail!("rate_limited");
    }

    let resp = resp.error_for_status()?;
    let parsed: ChatResponse = resp.json().await?;

    parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or_else(|| anyhow::anyhow!("Empty response"))
}
