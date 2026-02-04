use crate::dbs::local::AppState;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
};
use axum::{Json, extract::State, response::IntoResponse};
use futures::StreamExt;
use shared::models::{GenerateRequest, RegenerateRequest};
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
        let system_prompt = format!(
            "You are {}, description: {}\nPersonality: {}\nScenario: {}\nExample messages: {}",
            char.name, char.description, char.personality, char.scenario, char.example_messages
        );
        conversation.push(ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()
                .unwrap(),
        ));
    }

    // Add messages, stopping before truncate_at if specified
    for msg in messages {
        if let Some(trunc_id) = truncate_at
            && msg.id == trunc_id
        {
            break;
        }

        let content = msg.active_content().to_string();
        let req_msg = if msg.role == "user" {
            let user_msg = ChatCompletionRequestUserMessageArgs::default()
                .content(content)
                .build()
                .unwrap_or_default();
            ChatCompletionRequestMessage::User(user_msg)
        } else {
            let assistant_msg = ChatCompletionRequestAssistantMessageArgs::default()
                .content(ChatCompletionRequestAssistantMessageContent::Text(content))
                .build()
                .unwrap_or_default();
            ChatCompletionRequestMessage::Assistant(assistant_msg)
        };
        conversation.push(req_msg);
    }

    conversation
}

pub async fn generate_response(
    State(state): State<AppState>,
    Json(payload): Json<GenerateRequest>,
) -> axum::response::Response {
    let api_key = if payload.api_key.is_empty() {
        return (axum::http::StatusCode::UNAUTHORIZED, "Missing API Key").into_response();
    } else {
        payload.api_key.clone()
    };

    let api_base = payload
        .api_base
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

    let config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(api_base);

    let client = Client::with_config(config);

    // Fetch conversation history and character prompt
    let chat = state.db.get_chat(payload.chat_id).await;

    if chat.is_none() {
        return (axum::http::StatusCode::NOT_FOUND, "Chat not found").into_response();
    }
    let chat = chat.unwrap();

    let character = state.db.get_character(chat.character_id).await;
    let conversation = build_conversation(&chat.messages, character.as_ref(), None);

    let request = CreateChatCompletionRequestArgs::default()
        .model(payload.model)
        .messages(conversation)
        .temperature(payload.temperature.unwrap_or(0.7))
        .max_tokens(payload.max_tokens.unwrap_or(4096))
        .build()
        .unwrap();

    let response_stream = client.chat().create_stream(request).await;

    match response_stream {
        Ok(mut stream) => {
            let body = axum::body::Body::from_stream(async_stream::stream! {
                let mut full_response = String::new();

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(response) => {
                            if let Some(choice) = response.choices.first()
                                && let Some(content) = &choice.delta.content {
                                    full_response.push_str(content);
                                    yield Ok::<String, Error>(format!("data: {}\n\n", content));
                                }
                        }
                        Err(e) => {
                            yield Ok(format!("data: [ERROR] {}\n\n", e));
                        }
                    }
                }

                // Persist the full response to the database
                if !full_response.is_empty() {
                    state.db.append_message(payload.chat_id, shared::models::ChatMessage::new("assistant", full_response)).await;
                }

                yield Ok("data: [DONE]\n\n".to_string());
            });

            let response = axum::response::Response::builder()
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .body(body);

            match response {
                Ok(resp) => resp,
                Err(e) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Response Builder Error: {}", e),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("OpenAI Error: {}", e),
        )
            .into_response(),
    }
}

/// Regenerate a specific message - adds the result as an alternative to an existing message
pub async fn regenerate_response(
    State(state): State<AppState>,
    Json(payload): Json<RegenerateRequest>,
) -> axum::response::Response {
    let api_key = if payload.api_key.is_empty() {
        return (axum::http::StatusCode::UNAUTHORIZED, "Missing API Key").into_response();
    } else {
        payload.api_key.clone()
    };

    let api_base = payload
        .api_base
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

    let config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(api_base);

    let client = Client::with_config(config);

    // Fetch conversation history and character prompt
    let chat = state.db.get_chat(payload.chat_id).await;

    if chat.is_none() {
        return (axum::http::StatusCode::NOT_FOUND, "Chat not found").into_response();
    }
    let chat = chat.unwrap();

    // Check that the message exists
    let target_message = chat.messages.iter().find(|m| m.id == payload.message_id);
    if target_message.is_none() {
        return (axum::http::StatusCode::NOT_FOUND, "Message not found").into_response();
    }

    let character = state.db.get_character(chat.character_id).await;

    // Build conversation up to (but not including) the message to regenerate
    let conversation =
        build_conversation(&chat.messages, character.as_ref(), Some(payload.message_id));

    let request = CreateChatCompletionRequestArgs::default()
        .model(payload.model)
        .messages(conversation)
        .temperature(payload.temperature.unwrap_or(0.7))
        .max_tokens(payload.max_tokens.unwrap_or(4096))
        .build()
        .unwrap();

    let response_stream = client.chat().create_stream(request).await;

    match response_stream {
        Ok(mut stream) => {
            let body = axum::body::Body::from_stream(async_stream::stream! {
                let mut full_response = String::new();

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(response) => {
                            if let Some(choice) = response.choices.first()
                                && let Some(content) = &choice.delta.content {
                                    full_response.push_str(content);
                                    yield Ok::<String, Error>(format!("data: {}\n\n", content));
                                }
                        }
                        Err(e) => {
                            yield Ok(format!("data: [ERROR] {}\n\n", e));
                        }
                    }
                }

                // Add as alternative to the existing message instead of creating new
                if !full_response.is_empty() {
                    state.db.append_alternative(payload.chat_id, payload.message_id, full_response).await;
                }

                yield Ok("data: [DONE]\n\n".to_string());
            });

            let response = axum::response::Response::builder()
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .body(body);

            match response {
                Ok(resp) => resp,
                Err(e) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Response Builder Error: {}", e),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("OpenAI Error: {}", e),
        )
            .into_response(),
    }
}
