use crate::db::AppState;
use axum::{Json, extract::State};
use shared::models::*;

pub async fn list_characters(State(state): State<AppState>) -> Json<Vec<Character>> {
    let db = state.db.read().unwrap();
    Json(db.characters.clone())
}

pub async fn create_character(
    State(state): State<AppState>,
    Json(payload): Json<CreateCharacterRequest>,
) -> Json<Character> {
    let id = uuid::Uuid::new_v4();
    let char = Character {
        id,
        name: payload.name,
        description: payload.description,
        personality: payload.personality,
        scenario: payload.scenario,
        first_message: payload.first_message,
        example_messages: payload.example_messages,
    };

    {
        let mut db = state.db.write().unwrap();
        db.characters.push(char.clone());
        db.save();
    }

    Json(char)
}

pub async fn list_chats(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Vec<Chat>> {
    let db = state.db.read().unwrap();
    let char_id_str = params.get("character_id");

    let result = if let Some(cid_str) = char_id_str {
        if let Ok(cid) = uuid::Uuid::parse_str(cid_str) {
            db.chats
                .iter()
                .filter(|c| c.character_id == cid)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    } else {
        db.chats.clone()
    };

    Json(result)
}

pub async fn create_chat(
    State(state): State<AppState>,
    Json(payload): Json<CreateChatRequest>,
) -> Json<Chat> {
    let id = uuid::Uuid::new_v4();
    let mut messages = Vec::new();

    {
        let db_read = state.db.read().unwrap();
        if let Some(char) = db_read
            .characters
            .iter()
            .find(|c| c.id == payload.character_id)
            && !char.first_message.is_empty()
        {
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: char.first_message.clone(),
            });
        }
    }

    let chat = Chat {
        id,
        character_id: payload.character_id,
        messages,
    };

    {
        let mut db = state.db.write().unwrap();
        db.chats.push(chat.clone());
        db.save();
    }

    Json(chat)
}

pub async fn append_message(
    State(state): State<AppState>,
    axum::extract::Path(chat_id): axum::extract::Path<uuid::Uuid>,
    Json(payload): Json<ChatMessage>,
) -> Json<()> {
    let mut db = state.db.write().unwrap();
    if let Some(chat) = db.chats.iter_mut().find(|c| c.id == chat_id) {
        chat.messages.push(payload);
        db.save();
    }

    Json(())
}
