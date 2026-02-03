use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shared::models::*;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

const DB_PATH: &str = "db.json";

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct LocalDatabase {
    pub characters: Vec<Character>,
    pub chats: Vec<Chat>,
}

#[async_trait]
pub trait Database: Send + Sync {
    async fn get_characters(&self) -> Vec<Character>;
    async fn create_character(&self, character: Character);
    async fn get_chats(&self, character_id: Option<Uuid>) -> Vec<Chat>;
    async fn create_chat(&self, chat: Chat);
    async fn get_chat(&self, chat_id: Uuid) -> Option<Chat>;
    async fn append_message(&self, chat_id: Uuid, message: ChatMessage);
    // Needed for handlers that might need to check character details from a chat
    async fn get_character(&self, character_id: Uuid) -> Option<Character>;
}

impl LocalDatabase {
    pub fn load() -> Self {
        if let Ok(content) = std::fs::read_to_string(DB_PATH) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(DB_PATH, content);
        }
    }
}

#[async_trait]
impl Database for RwLock<LocalDatabase> {
    async fn get_characters(&self) -> Vec<Character> {
        let db = self.read().unwrap();
        db.characters.clone()
    }

    async fn create_character(&self, character: Character) {
        let mut db = self.write().unwrap();
        db.characters.push(character);
        db.save();
    }

    async fn get_chats(&self, character_id: Option<Uuid>) -> Vec<Chat> {
        let db = self.read().unwrap();
        if let Some(cid) = character_id {
            db.chats
                .iter()
                .filter(|c| c.character_id == cid)
                .cloned()
                .collect()
        } else {
            db.chats.clone()
        }
    }

    async fn create_chat(&self, chat: Chat) {
        let mut db = self.write().unwrap();
        db.chats.push(chat);
        db.save();
    }

    async fn get_chat(&self, chat_id: Uuid) -> Option<Chat> {
        let db = self.read().unwrap();
        db.chats.iter().find(|c| c.id == chat_id).cloned()
    }

    async fn append_message(&self, chat_id: Uuid, message: ChatMessage) {
        let mut db = self.write().unwrap();
        if let Some(chat) = db.chats.iter_mut().find(|c| c.id == chat_id) {
            chat.messages.push(message);
            db.save();
        }
    }

    async fn get_character(&self, character_id: Uuid) -> Option<Character> {
        let db = self.read().unwrap();
        db.characters.iter().find(|c| c.id == character_id).cloned()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<dyn Database>,
}
