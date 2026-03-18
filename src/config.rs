use std::env;

#[derive(Debug, Clone, PartialEq)]
pub enum Provider {
    Claude,
    OpenAI,
    OpenRouter,
}

impl Provider {
    pub fn display_name(&self) -> &'static str {
        match self {
            Provider::Claude => "Claude",
            Provider::OpenAI => "OpenAI",
            Provider::OpenRouter => "OpenRouter",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub provider: Provider,
    pub api_key: String,
    pub model: String,
}

impl LlmConfig {
    pub fn display(&self) -> String {
        // Show a clean short name
        let short_model = match self.model.as_str() {
            "claude-haiku-4-5-20241022" => "haiku",
            "gpt-4.1-nano" => "4.1-nano",
            other => other,
        };
        format!("{} · {}", self.provider.display_name(), short_model)
    }
}

pub fn load_config() -> Option<LlmConfig> {
    let _ = dotenvy::dotenv();

    // Check explicit provider override
    let explicit_provider = env::var("LLM_PROVIDER").ok();
    let explicit_model = env::var("LLM_MODEL").ok();

    // Try providers in priority order, or use explicit if set
    if let Some(ref p) = explicit_provider {
        match p.to_lowercase().as_str() {
            "claude" | "anthropic" => try_claude(explicit_model),
            "openai" => try_openai(explicit_model),
            "openrouter" => try_openrouter(explicit_model),
            _ => auto_detect(explicit_model),
        }
    } else {
        auto_detect(explicit_model)
    }
}

fn auto_detect(explicit_model: Option<String>) -> Option<LlmConfig> {
    try_claude(explicit_model.clone())
        .or_else(|| try_openai(explicit_model.clone()))
        .or_else(|| try_openrouter(explicit_model))
}

fn try_claude(model: Option<String>) -> Option<LlmConfig> {
    env::var("ANTHROPIC_API_KEY").ok().map(|key| LlmConfig {
        provider: Provider::Claude,
        api_key: key,
        model: model.unwrap_or_else(|| "claude-haiku-4-5-20241022".to_string()),
    })
}

fn try_openai(model: Option<String>) -> Option<LlmConfig> {
    env::var("OPENAI_API_KEY").ok().map(|key| LlmConfig {
        provider: Provider::OpenAI,
        api_key: key,
        model: model.unwrap_or_else(|| "gpt-4.1-nano".to_string()),
    })
}

fn try_openrouter(model: Option<String>) -> Option<LlmConfig> {
    env::var("OPENROUTER_API_KEY").ok().map(|key| LlmConfig {
        provider: Provider::OpenRouter,
        api_key: key,
        model: model.unwrap_or_else(|| "anthropic/claude-haiku".to_string()),
    })
}
