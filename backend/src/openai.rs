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
use shared::models::GenerateRequest;
use std::io::Error;

const DEFAULT_API_BASE: &str = "https://openrouter.ai/api/v1";

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
    let messages = chat.messages.clone();

    let mut conversation: Vec<ChatCompletionRequestMessage> = messages
        .into_iter()
        .map(|msg| {
            if msg.role == "user" {
                ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(msg.content)
                        .build()
                        .unwrap(),
                )
            } else {
                ChatCompletionRequestMessage::Assistant(
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(ChatCompletionRequestAssistantMessageContent::Text(
                            msg.content,
                        ))
                        .build()
                        .unwrap(),
                )
            }
        })
        .collect();

    if let Some(char) = character {
        let system_prompt = format!(
            "You are {}. {}\nPersonality: {}\nScenario: {}",
            char.name, char.description, char.personality, char.scenario
        );
        conversation.insert(
            0,
            ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(system_prompt)
                    .build()
                    .unwrap(),
            ),
        );
    }

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
                    state.db.append_message(payload.chat_id, shared::models::ChatMessage {
                         role: "assistant".to_string(),
                         content: full_response,
                    }).await;
                }

                yield Ok("data: [DONE]\n\n".to_string());
            });

            axum::response::Response::builder()
                .header("Content-Type", "text/event-stream")
                .header("Cache-Control", "no-cache")
                .header("Connection", "keep-alive")
                .body(body)
                .unwrap()
                .into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("OpenAI Error: {}", e),
        )
            .into_response(),
    }
}
