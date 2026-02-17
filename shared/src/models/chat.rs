use super::message::ChatMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    pub participants: Vec<ChatParticipant>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreateChatRequest {
    pub character_id: Uuid,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompletionRequest {
    pub chat_id: Uuid,
    pub regenerate: bool,
    pub message_id: Option<Uuid>,
    pub api_key: String,
    pub api_base: Option<String>,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u16>,
    pub reasoning_effort: String,
}
