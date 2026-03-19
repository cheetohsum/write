use anyhow::Result;
use serde::{Deserialize, Serialize};


#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

pub async fn cleanup(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    text: &str,
    system_prompt: &str,
) -> Result<String> {
    let body = ClaudeRequest {
        model: model.to_string(),
        max_tokens: 8192,
        system: system_prompt.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: text.to_string(),
        }],
    };

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    if resp.status() == 429 {
        anyhow::bail!("rate_limited");
    }

    let resp = resp.error_for_status()?;
    let parsed: ClaudeResponse = resp.json().await?;

    parsed
        .content
        .into_iter()
        .find_map(|b| b.text)
        .ok_or_else(|| anyhow::anyhow!("Empty response from Claude"))
}
