use crate::dbs::{Database, DbError, DbResult};
use async_trait::async_trait;
use serde_json::Value;
use shared::models::{Character, Chat, ChatMessage, ChatParticipant};
use sqlx::{Pool, Row, Sqlite, sqlite::SqlitePoolOptions};
use uuid::Uuid;

#[derive(Clone)]
pub struct LocalDatabase {
    pool: Pool<Sqlite>,
}

impl LocalDatabase {
    pub async fn new(database_url: &str) -> Self {
        let pool = SqlitePoolOptions::new()
            .connect(database_url)
            .await
            .expect("Failed to connect to database");

        let db = Self { pool };
        db.init().await;
        db
    }

    async fn init(&self) {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS characters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                personality TEXT NOT NULL,
                scenario TEXT NOT NULL,
                first_message TEXT NOT NULL,
                example_messages TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await
        .expect("Failed to create characters table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS chats (
                id TEXT PRIMARY KEY,
                character_id TEXT NOT NULL,
                participants JSON NOT NULL,
                FOREIGN KEY(character_id) REFERENCES characters(id)
            )",
        )
        .execute(&self.pool)
        .await
        .expect("Failed to create chats table");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                chat_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                sender_id TEXT,
                alternatives JSON NOT NULL,
                active_index INTEGER NOT NULL,
                FOREIGN KEY(chat_id) REFERENCES chats(id)
            )",
        )
        .execute(&self.pool)
        .await
        .expect("Failed to create messages table");
    }
}

#[async_trait]
impl Database for LocalDatabase {
    async fn get_characters(&self) -> DbResult<Vec<Character>> {
        let rows = sqlx::query(
            "SELECT id, name, description, personality, scenario, first_message, example_messages FROM characters"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Character {
                id: Uuid::parse_str(row.get("id")).unwrap_or_default(),
                name: row.get("name"),
                description: row.get("description"),
                personality: row.get("personality"),
                scenario: row.get("scenario"),
                first_message: row.get("first_message"),
                example_messages: row.get("example_messages"),
            })
            .collect())
    }

    async fn create_character(&self, character: Character) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO characters (id, name, description, personality, scenario, first_message, example_messages) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(character.id.to_string())
        .bind(character.name)
        .bind(character.description)
        .bind(character.personality)
        .bind(character.scenario)
        .bind(character.first_message)
        .bind(character.example_messages)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_character(&self, character_id: Uuid) -> DbResult<()> {
        // Cascading delete would be nice, but for now manual
        // First get all chats
        let chats = self.get_chats(Some(character_id)).await?;
        for chat in chats {
            // Delete messages for chat
            sqlx::query("DELETE FROM messages WHERE chat_id = ?")
                .bind(chat.id.to_string())
                .execute(&self.pool)
                .await?;
            // Delete chat
            sqlx::query("DELETE FROM chats WHERE id = ?")
                .bind(chat.id.to_string())
                .execute(&self.pool)
                .await?;
        }

        sqlx::query("DELETE FROM characters WHERE id = ?")
            .bind(character_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_chat(&self, chat_id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM messages WHERE chat_id = ?")
            .bind(chat_id.to_string())
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM chats WHERE id = ?")
            .bind(chat_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_character(&self, character_id: Uuid) -> DbResult<Character> {
        let row = sqlx::query(
            "SELECT id, name, description, personality, scenario, first_message, example_messages FROM characters WHERE id = ?",
        )
        .bind(character_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Character {
                id: Uuid::parse_str(row.get("id")).unwrap_or_default(),
                name: row.get("name"),
                description: row.get("description"),
                personality: row.get("personality"),
                scenario: row.get("scenario"),
                first_message: row.get("first_message"),
                example_messages: row.get("example_messages"),
            }),
            None => Err(DbError::NotFound(format!(
                "Character {} not found",
                character_id
            ))),
        }
    }

    async fn get_chats(&self, character_id: Option<Uuid>) -> DbResult<Vec<Chat>> {
        let rows = if let Some(cid) = character_id {
            sqlx::query("SELECT id, character_id, participants FROM chats WHERE character_id = ?")
                .bind(cid.to_string())
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query("SELECT id, character_id, participants FROM chats")
                .fetch_all(&self.pool)
                .await?
        };

        let mut chats = Vec::new();
        for row in rows {
            let participants_val: Value = row.get("participants");
            let participants: Vec<ChatParticipant> =
                serde_json::from_value(participants_val).map_err(DbError::Serde)?;
            let chat_id_str: String = row.get("id");
            let char_id_str: String = row.get("character_id");

            chats.push(Chat {
                id: Uuid::parse_str(&chat_id_str).unwrap_or_default(),
                character_id: Uuid::parse_str(&char_id_str).unwrap_or_default(),
                messages: Vec::new(),
                participants,
            });
        }
        Ok(chats)
    }

    async fn create_chat(&self, chat: Chat) -> DbResult<()> {
        let participants_json = serde_json::to_value(&chat.participants)?;
        sqlx::query("INSERT INTO chats (id, character_id, participants) VALUES (?, ?, ?)")
            .bind(chat.id.to_string())
            .bind(chat.character_id.to_string())
            .bind(participants_json)
            .execute(&self.pool)
            .await?;

        // Also insert initial messages if any
        for msg in chat.messages {
            self.append_message(chat.id, msg).await?;
        }
        Ok(())
    }

    async fn get_chat(&self, chat_id: Uuid) -> DbResult<Chat> {
        let row = sqlx::query("SELECT id, character_id, participants FROM chats WHERE id = ?")
            .bind(chat_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let participants_val: Value = row.get("participants");
                let participants: Vec<ChatParticipant> =
                    serde_json::from_value(participants_val).map_err(DbError::Serde)?;
                let messages = self.get_messages_for_chat(chat_id).await?;

                let char_id_str: String = row.get("character_id");

                Ok(Chat {
                    id: chat_id,
                    character_id: Uuid::parse_str(&char_id_str).unwrap_or_default(),
                    messages,
                    participants,
                })
            }
            None => Err(DbError::NotFound(format!("Chat {} not found", chat_id))),
        }
    }

    async fn append_message(&self, chat_id: Uuid, message: ChatMessage) -> DbResult<()> {
        // Ensure chat exists? Optional but good practice.
        // For now, raw insert.
        let alts_json = serde_json::to_value(&message.alternatives)?;
        let sender_id = message.sender_id.map(|u| u.to_string());

        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, sender_id, alternatives, active_index) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(message.id.to_string())
        .bind(chat_id.to_string())
        .bind(message.role)
        .bind(message.content)
        .bind(sender_id)
        .bind(alts_json)
        .bind(message.active_index as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_message(
        &self,
        _chat_id: Uuid,
        message_id: Uuid,
        content: String,
    ) -> DbResult<()> {
        if let Some(mut msg) = self.get_message_by_id(message_id).await? {
            if msg.active_index == 0 {
                msg.content = content;
            } else if let Some(alt) = msg.alternatives.get_mut(msg.active_index - 1) {
                *alt = content;
            }
            self.save_message(message_id, msg).await?;
            Ok(())
        } else {
            Err(DbError::NotFound(format!(
                "Message {} not found",
                message_id
            )))
        }
    }

    async fn delete_message(&self, _chat_id: Uuid, message_id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM messages WHERE id = ?")
            .bind(message_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn append_alternative(
        &self,
        _chat_id: Uuid,
        message_id: Uuid,
        content: String,
    ) -> DbResult<()> {
        if let Some(mut msg) = self.get_message_by_id(message_id).await? {
            msg.alternatives.push(content);
            msg.active_index = msg.alternatives.len();
            self.save_message(message_id, msg).await?;
            Ok(())
        } else {
            Err(DbError::NotFound(format!(
                "Message {} not found",
                message_id
            )))
        }
    }

    async fn set_active_alternative(
        &self,
        _chat_id: Uuid,
        message_id: Uuid,
        index: usize,
    ) -> DbResult<()> {
        if let Some(mut msg) = self.get_message_by_id(message_id).await? {
            if index < msg.variant_count() {
                msg.active_index = index;
                self.save_message(message_id, msg).await?;
            }
            Ok(())
        } else {
            Err(DbError::NotFound(format!(
                "Message {} not found",
                message_id
            )))
        }
    }
    async fn get_message(&self, _chat_id: Uuid, message_id: Uuid) -> DbResult<ChatMessage> {
        self.get_message_by_id(message_id)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("Message {} not found", message_id)))
    }
}

impl LocalDatabase {
    async fn get_messages_for_chat(&self, chat_id: Uuid) -> DbResult<Vec<ChatMessage>> {
        let rows = sqlx::query(
            "SELECT id, role, content, sender_id, alternatives, active_index FROM messages WHERE chat_id = ? ORDER BY id",
        )
        .bind(chat_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let alts_val: Value = row.get("alternatives");
                let alternatives: Vec<String> =
                    serde_json::from_value(alts_val).unwrap_or_default();
                let id_str: String = row.get("id");
                let sender_id_str: Option<String> = row.get("sender_id");

                ChatMessage {
                    id: Uuid::parse_str(&id_str).unwrap_or_default(),
                    role: row.get("role"),
                    content: row.get("content"),
                    sender_id: sender_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    alternatives,
                    active_index: row.get::<i64, _>("active_index") as usize,
                    tool_calls: None,
                    tool_call_id: None,
                }
            })
            .collect())
    }

    async fn get_message_by_id(&self, message_id: Uuid) -> DbResult<Option<ChatMessage>> {
        let row = sqlx::query(
            "SELECT id, role, content, sender_id, alternatives, active_index FROM messages WHERE id = ?",
        )
        .bind(message_id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        let row = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        let alts_val: Value = row.get("alternatives");
        let alternatives: Vec<String> = serde_json::from_value(alts_val).unwrap_or_default();
        let id_str: String = row.get("id");
        let sender_id_str: Option<String> = row.get("sender_id");

        Ok(Some(ChatMessage {
            id: Uuid::parse_str(&id_str).unwrap_or_default(),
            role: row.get("role"),
            content: row.get("content"),
            sender_id: sender_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
            alternatives,
            active_index: row.get::<i64, _>("active_index") as usize,
            tool_calls: None,
            tool_call_id: None,
        }))
    }

    async fn save_message(&self, message_id: Uuid, msg: ChatMessage) -> DbResult<()> {
        let alts_json = serde_json::to_value(&msg.alternatives)?;
        sqlx::query(
            "UPDATE messages SET content = ?, alternatives = ?, active_index = ? WHERE id = ?",
        )
        .bind(msg.content)
        .bind(alts_json)
        .bind(msg.active_index as i64)
        .bind(message_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
