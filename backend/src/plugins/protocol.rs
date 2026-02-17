use serde::{Deserialize, Serialize};
use shared::models::Tool;

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub json_rpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<PluginRequestId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub json_rpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum PluginRequestId {
    String(String),
    Number(i64),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub json_rpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    pub id: Option<PluginRequestId>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeParams {
    pub host: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tools: Vec<Tool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: serde_json::Value,
}
