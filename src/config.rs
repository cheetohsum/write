use std::env;
use std::fs;
use std::path::PathBuf;

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
            "gpt-5.4-nano" => "5.4-nano",
            other => other,
        };
        format!("{} · {}", self.provider.display_name(), short_model)
    }
}

pub fn load_config() -> Option<LlmConfig> {
    // Load from app config dir first (user settings take priority)
    let config_env = env_file_path();
    if config_env.exists() {
        let _ = dotenvy::from_path(&config_env);
    }

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
        model: model.unwrap_or_else(|| "gpt-5.4-nano".to_string()),
    })
}

fn try_openrouter(model: Option<String>) -> Option<LlmConfig> {
    env::var("OPENROUTER_API_KEY").ok().map(|key| LlmConfig {
        provider: Provider::OpenRouter,
        api_key: key,
        model: model.unwrap_or_else(|| "anthropic/claude-haiku".to_string()),
    })
}

// --- Settings persistence ---

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("write")
}

fn env_file_path() -> PathBuf {
    config_dir().join(".env")
}

pub const PROVIDER_NAMES: [&str; 3] = ["Anthropic", "OpenAI", "OpenRouter"];
pub const PROVIDER_URLS: [&str; 3] = [
    "console.anthropic.com/settings/keys",
    "platform.openai.com/api-keys",
    "openrouter.ai/keys",
];
const PROVIDER_FULL_URLS: [&str; 3] = [
    "https://console.anthropic.com/settings/keys",
    "https://platform.openai.com/api-keys",
    "https://openrouter.ai/keys",
];
const ENV_VAR_NAMES: [&str; 3] = ["ANTHROPIC_API_KEY", "OPENAI_API_KEY", "OPENROUTER_API_KEY"];

pub fn load_saved_keys() -> [String; 3] {
    let mut keys = [String::new(), String::new(), String::new()];

    if let Ok(content) = fs::read_to_string(env_file_path()) {
        for line in content.lines() {
            let line = line.trim();
            for (i, name) in ENV_VAR_NAMES.iter().enumerate() {
                if let Some(val) = line.strip_prefix(&format!("{}=", name)) {
                    keys[i] = val.to_string();
                }
            }
        }
    }

    for (i, name) in ENV_VAR_NAMES.iter().enumerate() {
        if keys[i].is_empty() {
            keys[i] = env::var(name).unwrap_or_default();
        }
    }

    keys
}

pub fn save_api_keys(keys: &[String; 3]) {
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);

    let mut content = String::new();
    for (i, name) in ENV_VAR_NAMES.iter().enumerate() {
        if !keys[i].is_empty() {
            content.push_str(&format!("{}={}\n", name, keys[i]));
        }
    }

    let _ = fs::write(env_file_path(), content);

    for (i, name) in ENV_VAR_NAMES.iter().enumerate() {
        if !keys[i].is_empty() {
            env::set_var(name, &keys[i]);
        } else {
            env::remove_var(name);
        }
    }
}

pub fn open_provider_url(index: usize) {
    if index >= PROVIDER_FULL_URLS.len() {
        return;
    }
    open_url(PROVIDER_FULL_URLS[index]);
}

fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
