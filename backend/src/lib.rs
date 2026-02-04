mod dbs;
mod handlers;
mod openai;

use crate::dbs::local::{AppState, LocalDatabase};
use crate::handlers::{
    append_message, create_character, create_chat, delete_character, delete_message, edit_message,
    list_characters, list_chats, swipe_message,
};
use crate::openai::{generate_response, regenerate_response};
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;

pub fn init(router: Router<AppState>) -> Router<()> {
    let db = LocalDatabase::load();
    let state = AppState {
        // Wrap LocalDatabase in RwLock, then in Arc, so it matches the trait implementation
        // impl Database for RwLock<LocalDatabase>
        db: Arc::new(RwLock::new(db)),
    };

    router
        .route("/api/health", get(|| async { "OK" }))
        .route(
            "/api/characters",
            get(list_characters).post(create_character),
        )
        .route("/api/characters/{character_id}", delete(delete_character))
        .route("/api/chats", get(list_chats).post(create_chat))
        .route("/api/chats/{chat_id}/message", post(append_message))
        .route(
            "/api/chats/{chat_id}/messages/{message_id}",
            put(edit_message).delete(delete_message),
        )
        .route(
            "/api/chats/{chat_id}/messages/{message_id}/swipe",
            post(swipe_message),
        )
        .route("/api/generate", post(generate_response))
        .route("/api/regenerate", post(regenerate_response))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
