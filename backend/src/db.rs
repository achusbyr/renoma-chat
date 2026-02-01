use serde::{Deserialize, Serialize};
use shared::models::*;
use std::sync::{Arc, RwLock};

const DB_PATH: &str = "db.json";

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Database {
    pub characters: Vec<Character>,
    pub chats: Vec<Chat>,
}

impl Database {
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

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<RwLock<Database>>,
}
