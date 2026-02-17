use crate::AppState;
use axum::{
    Json,
    extract::{Multipart, Path, State},
    http::StatusCode,
};
use shared::models::PluginManifest;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

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

pub async fn install_plugin(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<()>, StatusCode> {
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to get next field: {:?}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or_default().to_string();
        let file_name = field.file_name().unwrap_or_default().to_string();

        if name == "plugin" && !file_name.is_empty() {
            let data = field
                .bytes()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let mut path = PathBuf::from("./plugins");
            if !path.exists() {
                tokio::fs::create_dir_all(&path)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            path.push(&file_name);

            let mut file = File::create(&path).await.map_err(|e| {
                tracing::error!("Failed to create file: {:?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            file.write_all(&data)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = tokio::fs::metadata(&path)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .permissions();
                perms.set_mode(0o755);
                tokio::fs::set_permissions(&path, perms)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }

            // Immediately discover the new plugin
            state
                .plugins
                .load_plugin(path.to_str().unwrap())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            return Ok(Json(()));
        }
    }

    Err(StatusCode::BAD_REQUEST)
}
