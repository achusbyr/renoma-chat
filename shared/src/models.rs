use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_message: String,
    pub example_messages: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
    // Timestamp simplified for frontend display if needed
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chat {
    pub id: Uuid,
    pub character_id: Uuid,
    pub messages: Vec<ChatMessage>,
}

// Request payloads
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateCharacterRequest {
    pub name: String,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_message: String,
    pub example_messages: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateChatRequest {
    pub character_id: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GenerateRequest {
    pub chat_id: Uuid,
    pub api_key: String,
    pub api_base: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub api_key: String,
    pub api_base: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            api_base: "https://openrouter.ai/api/v1".to_string(),
            model: "tngtech/tng-r1t-chimera:free".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
        }
    }
}
