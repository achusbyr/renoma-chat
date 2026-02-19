use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_SYSTEM: &str = "system";
pub const ROLE_TOOL: &str = "tool";

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
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String, // usually "function"
    pub function: FunctionCall,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

impl ChatMessage {
    /// Create a new message with defaults for alternatives
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::now_v7(),
            role: role.into(),
            content: content.into(),
            sender_id: None,
            alternatives: Vec::new(),
            active_index: 0,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a new message from a specific sender (for group chats)
    pub fn new_from_sender(
        role: impl Into<String>,
        content: impl Into<String>,
        sender_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            role: role.into(),
            content: content.into(),
            sender_id: Some(sender_id),
            alternatives: Vec::new(),
            active_index: 0,
            tool_calls: None,
            tool_call_id: None,
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

    /// Total number of variants (1 primary and alternatives)
    pub fn variant_count(&self) -> usize {
        1 + self.alternatives.len()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EditMessageRequest {
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum SwipeDirection {
    /// Show previous alternative
    Left,
    /// Show the next alternative
    Right,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SwipeRequest {
    pub direction: SwipeDirection,
}
