pub mod claude;
pub mod openai;
pub mod openrouter;
pub mod prompt;

use anyhow::Result;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use crate::config::{LlmConfig, Provider};

#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub text: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub cleaned_text: String,
    pub original_hash: String,
    pub rate_limited: bool,
}

pub fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn spawn_llm_task(
    config: LlmConfig,
) -> (mpsc::Sender<LlmRequest>, mpsc::Receiver<LlmResponse>) {
    let (req_tx, mut req_rx) = mpsc::channel::<LlmRequest>(4);
    let (resp_tx, resp_rx) = mpsc::channel::<LlmResponse>(4);

    tokio::spawn(async move {
        let client = reqwest::Client::new();

        while let Some(request) = req_rx.recv().await {
            let result = call_llm(&client, &config, &request.text).await;

            match result {
                Ok(cleaned) => {
                    let _ = resp_tx
                        .send(LlmResponse {
                            cleaned_text: cleaned,
                            original_hash: request.hash,
                            rate_limited: false,
                        })
                        .await;
                }
                Err(e) => {
                    let is_rate_limit = e.to_string().contains("rate_limited");
                    let _ = resp_tx
                        .send(LlmResponse {
                            cleaned_text: String::new(),
                            original_hash: request.hash,
                            rate_limited: is_rate_limit,
                        })
                        .await;
                }
            }
        }
    });

    (req_tx, resp_rx)
}

async fn call_llm(client: &reqwest::Client, config: &LlmConfig, text: &str) -> Result<String> {
    let sys_prompt = prompt::system_prompt(config.writing_mode);
    match config.provider {
        Provider::Claude => claude::cleanup(client, &config.api_key, &config.model, text, &sys_prompt).await,
        Provider::OpenAI => {
            openai::cleanup(
                client,
                &config.api_key,
                &config.model,
                text,
                "https://api.openai.com/v1/chat/completions",
                &sys_prompt,
            )
            .await
        }
        Provider::OpenRouter => {
            openrouter::cleanup(client, &config.api_key, &config.model, text, &sys_prompt).await
        }
    }
}
