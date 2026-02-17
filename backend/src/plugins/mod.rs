use shared::models::Tool;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock, oneshot};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

mod protocol;
use protocol::*;

#[derive(Clone)]
pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<String, Arc<PluginInstance>>>>,
    tools: Arc<RwLock<HashMap<String, String>>>, // Tool Name -> Plugin Name
}

struct PluginInstance {
    name: RwLock<String>,
    version: RwLock<String>,
    description: RwLock<String>,
    enabled: Arc<RwLock<bool>>,
    #[allow(dead_code)]
    process: Mutex<Child>,
    stdin: Mutex<tokio::process::ChildStdin>,
    tools: RwLock<Vec<Tool>>,
    pending_requests: Arc<Mutex<HashMap<PluginRequestId, oneshot::Sender<JsonRpcResponse>>>>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn load_plugin(
        &self,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut command = Command::new(path);
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let mut child = command.spawn()?;
        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stdout_reader = BufReader::new(stdout);

        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let pending_requests_clone = pending_requests.clone();

        let instance = Arc::new(PluginInstance {
            name: RwLock::new(String::new()),
            version: RwLock::new(String::new()),
            description: RwLock::new(String::new()),
            enabled: Arc::new(RwLock::new(true)),
            process: Mutex::new(child),
            stdin: Mutex::new(stdin),
            tools: RwLock::new(Vec::new()),
            pending_requests,
        });

        // Start background listener
        let instance_name_for_task = path.to_string();
        tokio::spawn(async move {
            let mut reader = stdout_reader;
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        info!("Plugin process exited: {}", instance_name_for_task);
                        break;
                    }
                    Ok(_) => {
                        if let Ok(message) = serde_json::from_str::<PluginMessage>(&line) {
                            match message {
                                PluginMessage::Response(resp) => {
                                    if let Some(id) = resp.id.clone() {
                                        let mut pending = pending_requests_clone.lock().await;
                                        if let Some(tx) = pending.remove(&id) {
                                            let _ = tx.send(resp);
                                        }
                                    }
                                }
                                PluginMessage::Notification(notif) => {
                                    debug!("Received notification from plugin: {:?}", notif);
                                }
                                PluginMessage::Request(req) => {
                                    warn!(
                                        "Received request from plugin (not supported yet): {:?}",
                                        req
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading from plugin stdout: {:?}", e);
                        break;
                    }
                }
            }
        });

        // Initialize handshake
        let init_req = JsonRpcRequest {
            json_rpc: "2.0".to_string(),
            method: "initialize".to_string(),
            params: Some(serde_json::to_value(InitializeParams {
                host: "renoma".to_string(),
                version: "0.1.0".to_string(),
            })?),
            id: Some(PluginRequestId::Number(1)),
        };

        let response = instance.send_request(init_req).await?;

        if let Some(result) = response.result {
            let init_result: InitializeResult = serde_json::from_value(result)?;

            {
                let mut name = instance.name.write().await;
                let mut version = instance.version.write().await;
                let mut description = instance.description.write().await;
                let mut tools_list = instance.tools.write().await;

                *name = init_result.name.clone();
                *version = init_result.version.clone();
                *description = init_result.description.clone();
                *tools_list = init_result.tools.clone();
            }

            let plugin_name = init_result.name.clone();
            info!("Loaded plugin: {} ({})", plugin_name, init_result.version);

            // Register instance
            {
                let mut plugins = self.plugins.write().await;
                plugins.insert(plugin_name.clone(), instance);
            }

            // Register tools with collision detection
            {
                let mut tools = self.tools.write().await;
                for tool in init_result.tools {
                    if let Some(existing_plugin) = tools.get(&tool.name) {
                        warn!(
                            "Tool collision: {} already registered by {}. Overwriting with {}.",
                            tool.name, existing_plugin, plugin_name
                        );
                    }
                    tools.insert(tool.name, plugin_name.clone());
                }
            }
        } else if let Some(err) = response.error {
            return Err(format!("Plugin initialization failed: {}", err.message).into());
        } else {
            return Err("Plugin initialization failed: Unknown error".into());
        }

        Ok(())
    }

    pub async fn get_all_tools(&self) -> Vec<Tool> {
        let plugins = self.plugins.read().await;
        let mut all_tools = Vec::new();
        for plugin in plugins.values() {
            let tools = plugin.tools.read().await;
            all_tools.extend(tools.clone());
        }
        all_tools
    }

    pub async fn get_plugins(&self) -> Vec<shared::models::PluginManifest> {
        let plugins = self.plugins.read().await;
        let mut results = Vec::new();
        for p in plugins.values() {
            results.push(shared::models::PluginManifest {
                name: p.name.read().await.clone(),
                description: p.description.read().await.clone(),
                version: p.version.read().await.clone(),
                enabled: *p.enabled.read().await,
                tools: p.tools.read().await.clone(),
            });
        }
        results
    }

    pub async fn call_tool(
        &self,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let plugin_name = {
            let tools = self.tools.read().await;
            tools.get(tool_name).cloned()
        };

        if let Some(plugin_name) = plugin_name {
            let plugins = self.plugins.read().await;
            if let Some(plugin) = plugins.get(&plugin_name) {
                if !*plugin.enabled.read().await {
                    return Err(format!("Plugin {} is disabled", plugin_name).into());
                }
                let req = JsonRpcRequest {
                    json_rpc: "2.0".to_string(),
                    method: "call_tool".to_string(),
                    params: Some(serde_json::to_value(CallToolParams {
                        name: tool_name.to_string(),
                        arguments: args,
                    })?),
                    id: Some(PluginRequestId::Number(Uuid::now_v7().as_u128() as i64)),
                };
                let response = plugin.send_request(req).await?;
                if let Some(result) = response.result {
                    return Ok(result);
                } else if let Some(err) = response.error {
                    return Err(format!("Tool execution error: {}", err.message).into());
                }
            }
        }

        Err(format!("Tool not found: {}", tool_name).into())
    }

    pub async fn toggle_plugin(
        &self,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let plugins = self.plugins.read().await;
        if let Some(plugin) = plugins.get(name) {
            let mut enabled = plugin.enabled.write().await;
            *enabled = !*enabled;
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", name).into())
        }
    }

    pub async fn discover_plugins(
        &self,
        dir: impl AsRef<Path>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let dir = dir.as_ref();
        if !dir.exists() {
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file()
                && is_executable::is_executable(&path)
                && let Err(e) = self.load_plugin(path.to_str().unwrap()).await
            {
                error!("Failed to load plugin from {:?}: {:?}", path, e);
            }
        }

        Ok(())
    }

    pub async fn unload_plugin(
        &self,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut plugins = self.plugins.write().await;
        if let Some(plugin) = plugins.remove(name) {
            let mut child = plugin.process.lock().await;
            child.kill().await?;
            info!("Unloaded plugin: {}", name);
            Ok(())
        } else {
            Err(format!("Plugin not found: {}", name).into())
        }
    }
}

impl PluginInstance {
    async fn send_request(
        &self,
        req: JsonRpcRequest,
    ) -> Result<JsonRpcResponse, Box<dyn std::error::Error + Send + Sync>> {
        let id = req.id.clone().ok_or("Request must have an ID")?;
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(id, tx);
        }

        let mut req_json = serde_json::to_string(&req)?;
        req_json.push('\n');

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(req_json.as_bytes()).await?;
            stdin.flush().await?;
        }

        Ok(rx.await?)
    }
}
