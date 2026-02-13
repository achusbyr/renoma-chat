use crate::AppState;
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
use shared::models::{CompletionRequest, ROLE_ASSISTANT, ROLE_USER};
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
        system_prompt.push_str(&format!("You are {}", char.name));
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
    let chat = state.db.get_chat(payload.chat_id).await;

    let chat = match chat {
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
    let conversation = build_conversation(&chat.messages, character.as_ref(), truncate_at);

    let request = match CreateChatCompletionRequestArgs::default()
        .model(payload.model.clone())
        .messages(conversation)
        .temperature(payload.temperature.unwrap_or(0.7))
        .max_tokens(payload.max_tokens.unwrap_or(4096))
        .build()
    {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to build completion request: {:?}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build completion request",
            )
                .into_response();
        }
    };

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
                                    let encoded = serde_json::to_string(content).unwrap_or_else(|_| format!("\"{}\"", content.replace('"', "\\\"")));
                                    yield Ok::<String, Error>(format!("data: {}\n\n", encoded));
                                }
                        }
                        Err(e) => {
                            yield Ok(format!("data: [ERROR] {}\n\n", e));
                        }
                    }
                }

                // Persist the full response to the database
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
