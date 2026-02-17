use crate::AppState;
use crate::dbs::DbError;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use shared::models::{Chat, ChatMessage, ChatParticipant, CreateChatRequest};
use uuid::Uuid;

pub async fn list_chats(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<Chat>>, StatusCode> {
    let char_id_str = params.get("character_id");
    let char_id = char_id_str.and_then(|s| Uuid::parse_str(s).ok());

    let result = state.db.get_chats(char_id).await.map_err(|e| {
        tracing::error!("Failed to get chats: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(result))
}

pub async fn create_chat(
    State(state): State<AppState>,
    Json(payload): Json<CreateChatRequest>,
) -> Result<Json<Chat>, StatusCode> {
    let id = Uuid::new_v4();
    let mut messages = Vec::new();

    let char_opt = state.db.get_character(payload.character_id).await;
    if let Ok(char) = char_opt
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

    state.db.create_chat(chat.clone()).await.map_err(|e| {
        tracing::error!("Failed to create chat: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(chat))
}

pub async fn delete_chat(
    State(state): State<AppState>,
    Path(chat_id): Path<Uuid>,
) -> Result<Json<()>, StatusCode> {
    let chat = state.db.get_chat(chat_id).await;
    if matches!(chat, Err(DbError::NotFound(_))) {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Err(e) = chat {
        tracing::error!("Failed to get chat: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    state.db.delete_chat(chat_id).await.map_err(|e| {
        tracing::error!("Failed to delete chat: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(()))
}

pub async fn get_chat(
    State(state): State<AppState>,
    Path(chat_id): Path<Uuid>,
) -> Result<Json<Chat>, StatusCode> {
    let chat = state.db.get_chat(chat_id).await.map_err(|e| {
        if matches!(e, DbError::NotFound(_)) {
            StatusCode::NOT_FOUND
        } else {
            tracing::error!("Failed to get chat: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(chat))
}
