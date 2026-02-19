use crate::AppState;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs,
        ChatCompletionTool, ChatCompletionTools, CreateChatCompletionRequestArgs, FunctionCall,
        FunctionObject,
    },
};
use axum::{Json, extract::State, response::IntoResponse};
use futures::StreamExt;
use shared::models::{CompletionRequest, ROLE_ASSISTANT, ROLE_SYSTEM, ROLE_TOOL, ROLE_USER};
use std::io::Error;

const DEFAULT_API_BASE: &str = "https://openrouter.ai/api/v1";

/// Build a conversation from chat messages, optionally truncating at a specific message
fn build_conversation(
    messages: &[shared::models::ChatMessage],
    character: Option<&shared::models::Character>,
    truncate_at: Option<uuid::Uuid>,
) -> Vec<ChatCompletionRequestMessage> {
    let mut conversation: Vec<ChatCompletionRequestMessage> = Vec::new();

    // Add system prompt if character exists
    if let Some(char) = character {
        let mut system_prompt = String::new();
        system_prompt.push_str(&format!("Name: {}", char.name));
        if !char.description.is_empty() {
            system_prompt.push_str(&format!("\nDescription: {}", char.description));
        }
        if !char.personality.is_empty() {
            system_prompt.push_str(&format!("\nPersonality: {}", char.personality));
        }
        if !char.scenario.is_empty() {
            system_prompt.push_str(&format!("\nScenario: {}", char.scenario));
        }
        if !char.example_messages.is_empty() {
            system_prompt.push_str(&format!("\nExample messages: {}", char.example_messages));
        }
        if let Ok(msg) = ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()
        {
            conversation.push(ChatCompletionRequestMessage::System(msg));
        }
    }

    // Add messages, stopping before truncate_at if specified
    for msg in messages {
        if let Some(trunc_id) = truncate_at
            && msg.id == trunc_id
        {
            break;
        }

        let content = msg.active_content().to_string();
        let req_msg = if msg.role == ROLE_USER {
            let user_msg = ChatCompletionRequestUserMessageArgs::default()
                .content(content)
                .build()
                .unwrap_or_default();
            ChatCompletionRequestMessage::User(user_msg)
        } else if msg.role == ROLE_ASSISTANT {
            let mut assistant_msg_builder = ChatCompletionRequestAssistantMessageArgs::default();
            if !content.is_empty() {
                assistant_msg_builder
                    .content(ChatCompletionRequestAssistantMessageContent::Text(content));
            }

            if let Some(tool_calls) = &msg.tool_calls {
                let openai_tool_calls: Vec<ChatCompletionMessageToolCalls> = tool_calls
                    .iter()
                    .map(|tc| {
                        ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
                            id: tc.id.clone(),
                            function: FunctionCall {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            },
                        })
                    })
                    .collect();
                assistant_msg_builder.tool_calls(openai_tool_calls);
            }

            let assistant_msg = assistant_msg_builder.build().unwrap_or_default();
            ChatCompletionRequestMessage::Assistant(assistant_msg)
        } else if msg.role == ROLE_TOOL {
            let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
            let tool_msg = ChatCompletionRequestToolMessageArgs::default()
                .content(content)
                .tool_call_id(tool_call_id)
                .build()
                .unwrap_or_default();
            ChatCompletionRequestMessage::Tool(tool_msg)
        } else if msg.role == ROLE_SYSTEM {
            let system_msg = ChatCompletionRequestSystemMessageArgs::default()
                .content(content)
                .build()
                .unwrap_or_default();
            ChatCompletionRequestMessage::System(system_msg)
        } else {
            continue;
        };
        conversation.push(req_msg);
    }

    conversation
}

pub async fn generate_response(
    State(state): State<AppState>,
    Json(payload): Json<CompletionRequest>,
) -> axum::response::Response {
    let api_key = if payload.api_key.is_empty() {
        return (axum::http::StatusCode::UNAUTHORIZED, "Missing API Key").into_response();
    } else {
        payload.api_key.clone()
    };

    let api_base = payload
        .api_base
        .clone()
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

    let config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(api_base);

    let client = Client::with_config(config);

    // Fetch conversation history and character prompt
    let chat_res = state.db.get_chat(payload.chat_id).await;

    let chat = match chat_res {
        Ok(c) => c,
        Err(crate::dbs::DbError::NotFound(_)) => {
            return (axum::http::StatusCode::NOT_FOUND, "Chat not found").into_response();
        }
        Err(e) => {
            tracing::error!("Database error fetching chat: {:?}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
            )
                .into_response();
        }
    };

    // Determine if we need to truncate for regeneration
    let truncate_at = if payload.regenerate {
        if let Some(msg_id) = payload.message_id {
            // Check that the message exists
            if !chat.messages.iter().any(|m| m.id == msg_id) {
                return (axum::http::StatusCode::NOT_FOUND, "Message not found").into_response();
            }
            Some(msg_id)
        } else {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Missing message_id for regeneration",
            )
                .into_response();
        }
    } else {
        None
    };

    let character = state.db.get_character(chat.character_id).await.ok();

    // Initial conversation build
    // We might need to loop if tools are called, so we'll maintain a local conversation buffer
    let conversation = build_conversation(&chat.messages, character.as_ref(), truncate_at);

    // Fetch available tools
    let available_tools = state.plugins.get_all_tools().await;
    let openai_tools: Option<Vec<ChatCompletionTools>> = if !available_tools.is_empty() {
        Some(
            available_tools
                .into_iter()
                .map(|t| {
                    ChatCompletionTools::Function(ChatCompletionTool {
                        function: FunctionObject {
                            name: t.name,
                            description: Some(t.description),
                            parameters: Some(t.parameters),
                            strict: Some(false),
                        },
                    })
                })
                .collect(),
        )
    } else {
        None
    };

    let body = axum::body::Body::from_stream(async_stream::stream! {
        let mut current_conversation = conversation; // We work on a copy

        // Loop for tool calls (max 5 turns to prevent infinite loops)
        for _turn in 0..5 {
            let mut builder = CreateChatCompletionRequestArgs::default();
            builder
                .model(payload.model.clone())
                .messages(current_conversation.clone())
                .temperature(payload.temperature.unwrap_or(0.7))
                .max_tokens(payload.max_tokens.unwrap_or(4096));

            if let Some(tools) = &openai_tools {
                builder.tools(tools.clone());
            }

            let effort = match payload.reasoning_effort.as_str() {
                "low" => async_openai::types::chat::ReasoningEffort::Low,
                "medium" => async_openai::types::chat::ReasoningEffort::Medium,
                "high" => async_openai::types::chat::ReasoningEffort::High,
                "none" => async_openai::types::chat::ReasoningEffort::None,
                _ => async_openai::types::chat::ReasoningEffort::Medium,
            };
            builder.reasoning_effort(effort);

            let request = match builder.build() {
                Ok(req) => req,
                Err(e) => {
                    yield Ok::<String, Error>(format!("data: [ERROR] Failed to build completion request: {}\n\n", e));
                    return;
                }
            };

            let mut stream = match client.chat().create_stream(request).await {
                Ok(s) => s,
                Err(e) => {
                    yield Ok(format!("data: [ERROR] OpenAI Error: {}\n\n", e));
                    return;
                }
            };

            let mut full_response = String::new();

            // Temporary buffer for aggregating tool call chunks
            #[derive(Clone, Default)]
            struct ToolCallBuffer {
                id: String,
                name: String,
                arguments: String,
            }
            let mut tool_calls_map: std::collections::HashMap<u32, ToolCallBuffer> = std::collections::HashMap::new();

            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        if let Some(choice) = response.choices.first() {
                            // Handle content
                            if let Some(content) = &choice.delta.content {
                                full_response.push_str(content);
                                let encoded = serde_json::to_string(content).unwrap_or_else(|_| format!("\"{}\"", content.replace('"', "\\\"")));
                                yield Ok(format!("data: {}\n\n", encoded));
                            }

                            // Handle tool calls aggregation
                            if let Some(tcs) = &choice.delta.tool_calls {
                                for tc in tcs {
                                    let index = tc.index;
                                    let entry = tool_calls_map.entry(index).or_default();

                                    if let Some(id) = &tc.id { entry.id.push_str(id); }
                                    if let Some(function) = &tc.function {
                                        if let Some(name) = &function.name { entry.name.push_str(name); }
                                        if let Some(args) = &function.arguments { entry.arguments.push_str(args); }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                         yield Ok(format!("data: [ERROR] {}\n\n", e));
                    }
                }
            }

            // Stream finished. Check if we have tool calls.
            if !tool_calls_map.is_empty() {
                // Convert map to sorted vec
                let mut indices: Vec<u32> = tool_calls_map.keys().cloned().collect();
                indices.sort();

                let mut tool_calls_buffer: Vec<ChatCompletionMessageToolCall> = Vec::new();
                for i in indices {
                    if let Some(buf) = tool_calls_map.get(&i) {
                        tool_calls_buffer.push(ChatCompletionMessageToolCall {
                            id: buf.id.clone(),
                            function: FunctionCall {
                                name: buf.name.clone(),
                                arguments: buf.arguments.clone(),
                            },
                        });
                    }
                }

                // We have tool calls. Execute them.

                // 1. Append Assistant Message with Tool Calls to conversation
                // Wrap in enum
                let tool_calls_enum: Vec<ChatCompletionMessageToolCalls> = tool_calls_buffer.iter().map(|tc| {
                    ChatCompletionMessageToolCalls::Function(tc.clone())
                }).collect();

                let assistant_msg_req = ChatCompletionRequestAssistantMessageArgs::default()
                    .tool_calls(tool_calls_enum)
                    .build()
                    .unwrap();
                current_conversation.push(ChatCompletionRequestMessage::Assistant(assistant_msg_req));

                // Persist to DB (As assistant message but with tool_calls)
                let tool_calls_model: Vec<shared::models::ToolCall> = tool_calls_buffer.iter().map(|tc| {
                    shared::models::ToolCall {
                        id: tc.id.clone(),
                        r#type: "function".to_string(),
                        function: shared::models::FunctionCall {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    }
                }).collect();

                // Signal tool calls to frontend
                if let Ok(json) = serde_json::to_string(&tool_calls_model) {
                    yield Ok(format!("data: [TOOL_CALLS] {}\n\n", json));
                }

                let assistant_chat_msg = {
                    let mut m = shared::models::ChatMessage::new(ROLE_ASSISTANT, full_response.clone());
                    m.tool_calls = Some(tool_calls_model);
                    m
                };
                if let Err(e) = state.db.append_message(payload.chat_id, assistant_chat_msg).await {
                     yield Ok(format!("data: [ERROR] Failed to save tool calls: {}\n\n", e));
                }

                // 2. Execute Tools
                for tc in &tool_calls_buffer {

                    let args = match serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                        Ok(a) => a,
                        Err(e) => {
                             // Tool error
                             let tool_msg = ChatCompletionRequestToolMessageArgs::default()
                                .content(format!("Error parsing arguments: {}", e))
                                .tool_call_id(tc.id.clone())
                                .build()
                                .unwrap();
                             current_conversation.push(ChatCompletionRequestMessage::Tool(tool_msg));

                             let db_tool_msg = {
                                 let mut m = shared::models::ChatMessage::new(ROLE_TOOL, format!("Error parsing arguments: {}", e));
                                 m.tool_call_id = Some(tc.id.clone());
                                 m
                             };
                             let _ = state.db.append_message(payload.chat_id, db_tool_msg).await;
                             yield Ok(format!("data: [TOOL_RESULT] {}\n\n", serde_json::to_string(&serde_json::json!({"id": tc.id, "error": format!("Error parsing arguments: {}", e)})).unwrap()));
                             continue;
                        }
                    };

                    match state.plugins.call_tool(&tc.function.name, args).await {
                         Ok(result) => {
                             let content = result.to_string();
                             let tool_msg = ChatCompletionRequestToolMessageArgs::default()
                                .content(content.clone())
                                .tool_call_id(tc.id.clone())
                                .build()
                                .unwrap();
                             current_conversation.push(ChatCompletionRequestMessage::Tool(tool_msg));

                             let db_tool_msg = {
                                 let mut m = shared::models::ChatMessage::new(ROLE_TOOL, content.clone());
                                 m.tool_call_id = Some(tc.id.clone());
                                 m
                             };
                             let _ = state.db.append_message(payload.chat_id, db_tool_msg).await;
                             yield Ok(format!("data: [TOOL_RESULT] {}\n\n", serde_json::to_string(&serde_json::json!({"id": tc.id, "result": content})).unwrap()));
                         }
                         Err(e) => {
                             let content = format!("Error executing tool: {}", e);
                             let tool_msg = ChatCompletionRequestToolMessageArgs::default()
                                .content(content.clone())
                                .tool_call_id(tc.id.clone())
                                .build()
                                .unwrap();
                             current_conversation.push(ChatCompletionRequestMessage::Tool(tool_msg));

                             let db_tool_msg = {
                                 let mut m = shared::models::ChatMessage::new(ROLE_TOOL, content.clone());
                                 m.tool_call_id = Some(tc.id.clone());
                                 m
                             };
                             let _ = state.db.append_message(payload.chat_id, db_tool_msg).await;
                             yield Ok(format!("data: [TOOL_RESULT] {}\n\n", serde_json::to_string(&serde_json::json!({"id": tc.id, "error": content})).unwrap()));
                         }
                    }
                }

                // Loop continues to next turn
                continue;

            } else {
                // No tool calls, standard finish.
                if !full_response.is_empty() {
                    let res = if payload.regenerate && let Some(msg_id) = payload.message_id {
                         state.db.append_alternative(payload.chat_id, msg_id, full_response).await
                    } else {
                         state.db.append_message(payload.chat_id, shared::models::ChatMessage::new(ROLE_ASSISTANT, full_response)).await
                    };

                    if let Err(e) = res {
                         yield Ok(format!("data: [ERROR] Failed to save response: {}\n\n", e));
                    }
                }

                yield Ok("data: [DONE]\n\n".to_string());
                return;
            }
        }
    });

    axum::response::Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap()
}
