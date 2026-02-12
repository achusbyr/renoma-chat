use gloo_net::http::Request;
use shared::models::*;
use uuid::Uuid;

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

pub async fn delete_character(id: Uuid) -> Result<(), gloo_net::Error> {
    Request::delete(&format!("{}/characters/{}", API_BASE, id))
        .send()
        .await?;
    Ok(())
}

pub async fn fetch_chats(char_id: Uuid) -> Result<Vec<Chat>, gloo_net::Error> {
    Request::get(&format!("{}/chats?character_id={}", API_BASE, char_id))
        .send()
        .await?
        .json()
        .await
}

pub async fn create_chat(char_id: Uuid) -> Result<Chat, gloo_net::Error> {
    Request::post(&format!("{}/chats", API_BASE))
        .json(&CreateChatRequest {
            character_id: char_id,
        })?
        .send()
        .await?
        .json()
        .await
}

pub async fn delete_chat(chat_id: Uuid) -> Result<(), gloo_net::Error> {
    Request::delete(&format!("{}/chats/{}", API_BASE, chat_id))
        .send()
        .await?;
    Ok(())
}

pub async fn send_message(chat_id: Uuid, content: String) -> Result<(), gloo_net::Error> {
    let msg = ChatMessage::new(ROLE_USER, content);

    Request::post(&format!("{}/chats/{}/message", API_BASE, chat_id))
        .json(&msg)?
        .send()
        .await?;
    Ok(())
}

pub async fn edit_message(
    chat_id: Uuid,
    message_id: Uuid,
    content: String,
) -> Result<(), gloo_net::Error> {
    Request::put(&format!(
        "{}/chats/{}/messages/{}",
        API_BASE, chat_id, message_id
    ))
    .json(&EditMessageRequest { content })?
    .send()
    .await?;
    Ok(())
}

pub async fn delete_message(chat_id: Uuid, message_id: Uuid) -> Result<(), gloo_net::Error> {
    Request::delete(&format!(
        "{}/chats/{}/messages/{}",
        API_BASE, chat_id, message_id
    ))
    .send()
    .await?;
    Ok(())
}

pub async fn swipe_message(
    chat_id: Uuid,
    message_id: Uuid,
    direction: SwipeDirection,
) -> Result<(), gloo_net::Error> {
    Request::post(&format!(
        "{}/chats/{}/messages/{}/swipe",
        API_BASE, chat_id, message_id
    ))
    .json(&SwipeRequest { direction })?
    .send()
    .await?;
    Ok(())
}
