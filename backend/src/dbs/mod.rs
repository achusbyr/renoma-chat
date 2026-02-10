use async_trait::async_trait;
use shared::models::{Character, Chat, ChatMessage};
use thiserror::Error;
use uuid::Uuid;

pub mod local;
pub mod postgres;

pub type DbResult<T> = Result<T, DbError>;

#[derive(Clone, Debug)]
pub enum DatabaseConfig {
    Local { url: String },
    Postgres { url: String },
}

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Entity not found: {0}")]
    NotFound(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait Database: Send + Sync {
    async fn get_characters(&self) -> DbResult<Vec<Character>>;
    async fn get_character(&self, character_id: Uuid) -> DbResult<Character>;
    async fn get_chats(&self, character_id: Option<Uuid>) -> DbResult<Vec<Chat>>;
    async fn get_chat(&self, chat_id: Uuid) -> DbResult<Chat>;
    async fn get_message(&self, chat_id: Uuid, message_id: Uuid) -> DbResult<ChatMessage>;
    async fn create_character(&self, character: Character) -> DbResult<()>;
    async fn create_chat(&self, chat: Chat) -> DbResult<()>;
    async fn delete_character(&self, character_id: Uuid) -> DbResult<()>;
    async fn delete_message(&self, chat_id: Uuid, message_id: Uuid) -> DbResult<()>;
    async fn append_message(&self, chat_id: Uuid, message: ChatMessage) -> DbResult<()>;
    async fn append_alternative(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
        content: String,
    ) -> DbResult<()>;
    async fn update_message(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
        content: String,
    ) -> DbResult<()>;
    async fn set_active_alternative(
        &self,
        chat_id: Uuid,
        message_id: Uuid,
        index: usize,
    ) -> DbResult<()>;
}
