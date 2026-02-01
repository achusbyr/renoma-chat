use gloo_net::http::Request;
use shared::models::*;

const API_BASE: &str = "/api";

pub async fn fetch_characters() -> Result<Vec<Character>, gloo_net::Error> {
    Request::get(&format!("{}/characters", API_BASE))
        .send()
        .await?
        .json()
        .await
}

pub async fn create_character(char: CreateCharacterRequest) -> Result<Character, gloo_net::Error> {
    Request::post(&format!("{}/characters", API_BASE))
        .json(&char)?
        .send()
        .await?
        .json()
        .await
}

pub async fn fetch_chats(char_id: uuid::Uuid) -> Result<Vec<Chat>, gloo_net::Error> {
    Request::get(&format!("{}/chats?character_id={}", API_BASE, char_id))
        .send()
        .await?
        .json()
        .await
}

pub async fn create_chat(char_id: uuid::Uuid) -> Result<Chat, gloo_net::Error> {
    Request::post(&format!("{}/chats", API_BASE))
        .json(&CreateChatRequest {
            character_id: char_id,
        })?
        .send()
        .await?
        .json()
        .await
}

pub async fn send_message(chat_id: uuid::Uuid, content: String) -> Result<(), gloo_net::Error> {
    let msg = ChatMessage {
        role: "user".to_string(),
        content,
    };

    Request::post(&format!("{}/chats/{}/message", API_BASE, chat_id))
        .json(&msg)?
        .send()
        .await?;
    Ok(())
}
