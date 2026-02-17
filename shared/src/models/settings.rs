use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub api_key: String,
    pub api_base: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u16,
    pub reasoning_effort: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_base: "https://openrouter.ai/api/v1".to_string(),
            model: "tngtech/deepseek-r1t2-chimera:free".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            reasoning_effort: "medium".to_string(),
        }
    }
}
