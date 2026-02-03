use crate::dbs::local::AppState;
use axum::{Json, extract::State};
use shared::models::*;

pub async fn list_characters(State(state): State<AppState>) -> Json<Vec<Character>> {
    let characters = state.db.get_characters().await;
    Json(characters)
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

    state.db.create_character(char.clone()).await;

    Json(char)
}

pub async fn list_chats(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<Vec<Chat>> {
    let char_id_str = params.get("character_id");
    let char_id = char_id_str.and_then(|s| uuid::Uuid::parse_str(s).ok());

    let result = state.db.get_chats(char_id).await;

    Json(result)
}

pub async fn create_chat(
    State(state): State<AppState>,
    Json(payload): Json<CreateChatRequest>,
) -> Json<Chat> {
    let id = uuid::Uuid::new_v4();
    let mut messages = Vec::new();

    if let Some(char) = state.db.get_character(payload.character_id).await
        && !char.first_message.is_empty()
    {
        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: char.first_message,
        });
    }

    let chat = Chat {
        id,
        character_id: payload.character_id,
        messages,
    };

    state.db.create_chat(chat.clone()).await;

    Json(chat)
}

pub async fn append_message(
    State(state): State<AppState>,
    axum::extract::Path(chat_id): axum::extract::Path<uuid::Uuid>,
    Json(payload): Json<ChatMessage>,
) -> Json<()> {
    state.db.append_message(chat_id, payload).await;
    Json(())
}
