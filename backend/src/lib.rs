mod db;
mod handlers;
mod openai;

use crate::db::{AppState, Database};
use crate::handlers::{append_message, create_character, create_chat, list_characters, list_chats};
use crate::openai::generate_response;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::{Arc, RwLock};
use tower_http::cors::CorsLayer;

pub fn init(router: Router<AppState>) -> Router<()> {
    let db = Database::load();
    let state = AppState {
        db: Arc::new(RwLock::new(db)),
    };

    router
        .route("/api/health", get(|| async { "OK" }))
        .route(
            "/api/characters",
            get(list_characters).post(create_character),
        )
        .route("/api/chats", get(list_chats).post(create_chat))
        .route("/api/chats/{chat_id}/message", post(append_message))
        .route("/api/generate", post(generate_response))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
