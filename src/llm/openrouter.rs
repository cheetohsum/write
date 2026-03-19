use anyhow::Result;

pub async fn cleanup(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    text: &str,
    system_prompt: &str,
) -> Result<String> {
    super::openai::cleanup(
        client,
        api_key,
        model,
        text,
        "https://openrouter.ai/api/v1/chat/completions",
        system_prompt,
    )
    .await
}
