use crate::dbs::local::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use shared::models::{
    Character, Chat, ChatMessage, ChatParticipant, CreateCharacterRequest, CreateChatRequest,
    EditMessageRequest, SwipeDirection, SwipeRequest,
};
use uuid::Uuid;

pub async fn list_characters(State(state): State<AppState>) -> Json<Vec<Character>> {
    let characters = state.db.get_characters().await;
    Json(characters)
}

pub async fn create_character(
    State(state): State<AppState>,
    Json(payload): Json<CreateCharacterRequest>,
) -> Json<Character> {
    let id = Uuid::new_v4();
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
    let char_id = char_id_str.and_then(|s| Uuid::parse_str(s).ok());

    let result = state.db.get_chats(char_id).await;

    Json(result)
}

pub async fn create_chat(
    State(state): State<AppState>,
    Json(payload): Json<CreateChatRequest>,
) -> Json<Chat> {
    let id = Uuid::new_v4();
    let mut messages = Vec::new();

    if let Some(char) = state.db.get_character(payload.character_id).await
        && !char.first_message.is_empty()
    {
        messages.push(ChatMessage::new("assistant", char.first_message));
    }

    let chat = Chat {
        id,
        character_id: payload.character_id,
        messages,
        participants: vec![ChatParticipant {
            character_id: payload.character_id,
            is_active: true,
        }],
    };

    state.db.create_chat(chat.clone()).await;

    Json(chat)
}

pub async fn append_message(
    State(state): State<AppState>,
    Path(chat_id): Path<Uuid>,
    Json(payload): Json<ChatMessage>,
) -> Json<()> {
    state.db.append_message(chat_id, payload).await;
    Json(())
}

pub async fn edit_message(
    State(state): State<AppState>,
    Path((chat_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<EditMessageRequest>,
) -> Result<Json<()>, StatusCode> {
    // Check if message exists
    if state.db.get_message(chat_id, message_id).await.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    state
        .db
        .update_message(chat_id, message_id, payload.content)
        .await;
    Ok(Json(()))
}

pub async fn delete_message(
    State(state): State<AppState>,
    Path((chat_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, StatusCode> {
    // Check if message exists
    if state.db.get_message(chat_id, message_id).await.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    state.db.delete_message(chat_id, message_id).await;
    Ok(Json(()))
}

pub async fn swipe_message(
    State(state): State<AppState>,
    Path((chat_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<SwipeRequest>,
) -> Result<Json<()>, StatusCode> {
    let message = state.db.get_message(chat_id, message_id).await;
    if message.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let message = message.unwrap();

    let total = message.variant_count();
    let new_index = match payload.direction {
        SwipeDirection::Left => message.active_index.saturating_sub(1),
        SwipeDirection::Right => (message.active_index + 1).min(total - 1),
    };

    state
        .db
        .set_active_alternative(chat_id, message_id, new_index)
        .await;
    Ok(Json(()))
}
