use crate::AppState;
use crate::dbs::DbError;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use shared::models::{ChatMessage, EditMessageRequest, SwipeDirection, SwipeRequest};
use uuid::Uuid;

pub async fn append_message(
    State(state): State<AppState>,
    Path(chat_id): Path<Uuid>,
    Json(payload): Json<ChatMessage>,
) -> Result<Json<()>, StatusCode> {
    state
        .db
        .append_message(chat_id, payload)
        .await
        .map_err(|e| {
            tracing::error!("Failed to append message: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(()))
}

pub async fn edit_message(
    State(state): State<AppState>,
    Path((chat_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<EditMessageRequest>,
) -> Result<Json<()>, StatusCode> {
    if let Err(e) = state.db.get_message(chat_id, message_id).await {
        if matches!(e, DbError::NotFound(_)) {
            return Err(StatusCode::NOT_FOUND);
        }
        tracing::error!("Failed to get message for edit: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    state
        .db
        .update_message(chat_id, message_id, payload.content)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update message: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(()))
}

pub async fn delete_message(
    State(state): State<AppState>,
    Path((chat_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, StatusCode> {
    if let Err(e) = state.db.get_message(chat_id, message_id).await {
        if matches!(e, DbError::NotFound(_)) {
            return Err(StatusCode::NOT_FOUND);
        }
        tracing::error!("Failed to get message for delete: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    state
        .db
        .delete_message(chat_id, message_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete message: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(()))
}

pub async fn swipe_message(
    State(state): State<AppState>,
    Path((chat_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<SwipeRequest>,
) -> Result<Json<()>, StatusCode> {
    let message = state.db.get_message(chat_id, message_id).await;
    if matches!(message, Err(DbError::NotFound(_))) {
        return Err(StatusCode::NOT_FOUND);
    }
    let message = message.map_err(|e| {
        tracing::error!("Failed to get message for swipe: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let total = message.variant_count();
    let new_index = match payload.direction {
        SwipeDirection::Left => message.active_index.saturating_sub(1),
        SwipeDirection::Right => (message.active_index + 1).min(total - 1),
    };

    state
        .db
        .set_active_alternative(chat_id, message_id, new_index)
        .await
        .map_err(|e| {
            tracing::error!("Failed to set active alternative: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(()))
}
