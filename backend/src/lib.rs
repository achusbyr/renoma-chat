mod dbs;
mod handlers;
mod openai;
pub mod plugins;

use crate::dbs::Database;
use crate::dbs::local::LocalDatabase;
use crate::dbs::postgres::PostgresDatabase;
use crate::handlers::{
    append_message, create_character, create_chat, delete_character, delete_chat, delete_message,
    edit_message, get_chat, list_characters, list_chats, list_plugins, swipe_message,
    toggle_plugin,
};
use crate::openai::generate_response;
use crate::plugins::PluginManager;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
pub use dbs::DatabaseConfig;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<dyn Database>,
    pub plugins: PluginManager,
}

pub async fn init(router: Router<AppState>, config: DatabaseConfig) -> Router<()> {
    let db: Arc<dyn Database> = match config {
        DatabaseConfig::Local { url } => Arc::new(LocalDatabase::new(&url).await),
        DatabaseConfig::Postgres { url } => Arc::new(PostgresDatabase::new(&url).await),
    };

    let plugins = PluginManager::new();
    if let Err(e) = plugins.discover_plugins("./plugins").await {
        tracing::error!("Failed to discover plugins: {:?}", e);
    }

    let state = AppState { db, plugins };

    router
        .route("/api/health", get(|| async { "OK" }))
        .route(
            "/api/characters",
            get(list_characters).post(create_character),
        )
        .route("/api/characters/{character_id}", delete(delete_character))
        .route("/api/chats", get(list_chats).post(create_chat))
        .route("/api/chats/{chat_id}", get(get_chat).delete(delete_chat))
        .route("/api/chats/{chat_id}/message", post(append_message))
        .route(
            "/api/chats/{chat_id}/messages/{message_id}",
            put(edit_message).delete(delete_message),
        )
        .route(
            "/api/chats/{chat_id}/messages/{message_id}/swipe",
            post(swipe_message),
        )
        .route("/api/completion", post(generate_response))
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/install", post(handlers::install_plugin))
        .route("/api/plugins/{name}/toggle", post(toggle_plugin))
        .route(
            "/api/plugins/discover",
            post(|state: axum::extract::State<AppState>| async move {
                state
                    .plugins
                    .discover_plugins("./plugins")
                    .await
                    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
                Ok::<(), axum::http::StatusCode>(())
            }),
        )
        .route(
            "/favicon.ico",
            get(|| async {
                (
                    [
                        (axum::http::header::CONTENT_TYPE, "image/x-icon"),
                        (axum::http::header::CACHE_CONTROL, "public, max-age=604800"),
                    ],
                    include_bytes!("../../frontend/favicon.ico"),
                )
            }),
        )
        .layer(CorsLayer::permissive())
        .with_state(state)
}
