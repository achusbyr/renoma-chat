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
    pub id: Uuid,
    /// "user" or "assistant"
    pub role: String,
    pub content: String,
    #[serde(default)]
    /// For group chats: which character sent this
    pub sender_id: Option<Uuid>,
    #[serde(default)]
    /// Swipe alternatives (content variants)
    pub alternatives: Vec<String>,
    #[serde(default)]
    /// Which alternative is currently shown (0 = primary content)
    pub active_index: usize,
}

impl ChatMessage {
    /// Create a new message with defaults for alternatives
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: role.into(),
            content: content.into(),
            sender_id: None,
            alternatives: Vec::new(),
            active_index: 0,
        }
    }

    /// Create a new message from a specific sender (for group chats)
    pub fn new_from_sender(
        role: impl Into<String>,
        content: impl Into<String>,
        sender_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: role.into(),
            content: content.into(),
            sender_id: Some(sender_id),
            alternatives: Vec::new(),
            active_index: 0,
        }
    }

    /// Get the currently active content (considering alternatives)
    pub fn active_content(&self) -> &str {
        if self.active_index == 0 || self.alternatives.is_empty() {
            &self.content
        } else {
            self.alternatives
                .get(self.active_index - 1)
                .map(|s| s.as_str())
                .unwrap_or(&self.content)
        }
    }

    /// Total number of variants (1 primary + alternatives)
    pub fn variant_count(&self) -> usize {
        1 + self.alternatives.len()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatParticipant {
    pub character_id: Uuid,
    pub is_active: bool, // Can take turns in group chat
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chat {
    pub id: Uuid,
    pub character_id: Uuid,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub participants: Vec<ChatParticipant>, // Group chat participants
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditMessageRequest {
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegenerateRequest {
    pub chat_id: Uuid,
    pub message_id: Uuid,
    pub api_key: String,
    pub api_base: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SwipeDirection {
    Left,  // Show previous alternative
    Right, // Show next alternative
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwipeRequest {
    pub direction: SwipeDirection,
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
