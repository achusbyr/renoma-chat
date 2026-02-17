use crate::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use shared::models::PluginManifest;

pub async fn list_plugins(
    State(state): State<AppState>,
) -> Result<Json<Vec<PluginManifest>>, StatusCode> {
    let plugins = state.plugins.get_plugins().await;
    Ok(Json(plugins))
}

pub async fn toggle_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<()>, StatusCode> {
    state
        .plugins
        .toggle_plugin(&name)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(()))
}
