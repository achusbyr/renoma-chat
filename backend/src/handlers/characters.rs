use crate::AppState;
use crate::dbs::DbError;
use axum::{Json, extract::Path, extract::State, http::StatusCode};
use shared::models::{Character, CreateCharacterRequest};
use uuid::Uuid;

pub async fn list_characters(
    State(state): State<AppState>,
) -> Result<Json<Vec<Character>>, StatusCode> {
    let characters = state.db.get_characters().await.map_err(|e| {
        tracing::error!("Failed to list characters: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(characters))
}

pub async fn create_character(
    State(state): State<AppState>,
    Json(payload): Json<CreateCharacterRequest>,
) -> Result<Json<Character>, StatusCode> {
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

    state.db.create_character(char.clone()).await.map_err(|e| {
        tracing::error!("Failed to create character: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(char))
}

pub async fn delete_character(
    State(state): State<AppState>,
    Path(character_id): Path<Uuid>,
) -> Result<Json<()>, StatusCode> {
    let char = state.db.get_character(character_id).await;
    if matches!(char, Err(DbError::NotFound(_))) {
        return Err(StatusCode::NOT_FOUND);
    }
    if let Err(e) = char {
        tracing::error!("Failed to get character: {:?}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    state.db.delete_character(character_id).await.map_err(|e| {
        tracing::error!("Failed to delete character: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(()))
}
