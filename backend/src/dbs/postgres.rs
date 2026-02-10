use crate::dbs::{Database, DbError, DbResult};
use async_trait::async_trait;
use serde_json::Value;
use shared::models::{Character, Chat, ChatMessage, ChatParticipant};
use sqlx::{Pool, Postgres, Row, postgres::PgPoolOptions};
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: Pool<Postgres>,
}

impl PostgresDatabase {
    pub async fn new(database_url: &str) -> Self {
        let pool = PgPoolOptions::new()
            .connect(database_url)
            .await
            .expect("Failed to connect to database");

        let db = Self { pool };
        db.init().await;
        db
    }

    async fn init(&self) {
        // Create tables compatible with PostgreSQL/CockroachDB
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS characters (
                id UUID PRIMARY KEY,
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
                id UUID PRIMARY KEY,
                character_id UUID NOT NULL,
                participants JSONB NOT NULL,
                FOREIGN KEY(character_id) REFERENCES characters(id)
            )",
        )
        .execute(&self.pool)
        .await
        .expect("Failed to create chats table");

        // Note: active_index is INTEGER. alternatives is JSONB.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id UUID PRIMARY KEY,
                chat_id UUID NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                sender_id UUID,
                alternatives JSONB NOT NULL,
                active_index INTEGER NOT NULL,
                FOREIGN KEY(chat_id) REFERENCES chats(id)
            )",
        )
        .execute(&self.pool)
        .await
        .expect("Failed to create messages table");
    }

    async fn get_messages_for_chat(&self, chat_id: Uuid) -> DbResult<Vec<ChatMessage>> {
        let rows = sqlx::query(
            "SELECT id, role, content, sender_id, alternatives, active_index FROM messages WHERE chat_id = $1 ORDER BY id", // Assuming insertion order or ID sort. CockroachDB UUIDs aren't sequential by default, so we might need a timestamp. But local.rs doesn't sort explicitly either.
        )
        .bind(chat_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let alts_val: Value = row.get("alternatives");
                let alternatives: Vec<String> =
                    serde_json::from_value(alts_val).unwrap_or_default();

                ChatMessage {
                    id: row.get("id"),
                    role: row.get("role"),
                    content: row.get("content"),
                    sender_id: row.get("sender_id"),
                    alternatives,
                    active_index: row.get::<i32, _>("active_index") as usize,
                }
            })
            .collect())
    }

    async fn get_message_by_id(&self, message_id: Uuid) -> DbResult<Option<ChatMessage>> {
        let row = sqlx::query(
            "SELECT id, role, content, sender_id, alternatives, active_index FROM messages WHERE id = $1",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;

        let row = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        let alts_val: Value = row.get("alternatives");
        let alternatives: Vec<String> = serde_json::from_value(alts_val).unwrap_or_default();

        Ok(Some(ChatMessage {
            id: row.get("id"),
            role: row.get("role"),
            content: row.get("content"),
            sender_id: row.get("sender_id"),
            alternatives,
            active_index: row.get::<i32, _>("active_index") as usize,
        }))
    }

    async fn save_message(&self, message_id: Uuid, msg: ChatMessage) -> DbResult<()> {
        let alts_json = serde_json::to_value(&msg.alternatives)?;
        sqlx::query(
            "UPDATE messages SET content = $1, alternatives = $2, active_index = $3 WHERE id = $4",
        )
        .bind(msg.content)
        .bind(alts_json)
        .bind(msg.active_index as i32)
        .bind(message_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[async_trait]
impl Database for PostgresDatabase {
    async fn get_characters(&self) -> DbResult<Vec<Character>> {
        let rows = sqlx::query(
            "SELECT id, name, description, personality, scenario, first_message, example_messages FROM characters"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Character {
                id: row.get("id"),
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
            "INSERT INTO characters (id, name, description, personality, scenario, first_message, example_messages) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(character.id)
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
        let text_id = character_id; // Postgres driver handles UUIDs natively

        // First get all chats
        let chats = self.get_chats(Some(character_id)).await?;
        for chat in chats {
            // Delete messages for chat
            sqlx::query("DELETE FROM messages WHERE chat_id = $1")
                .bind(chat.id)
                .execute(&self.pool)
                .await?;
            // Delete chat
            sqlx::query("DELETE FROM chats WHERE id = $1")
                .bind(chat.id)
                .execute(&self.pool)
                .await?;
        }

        sqlx::query("DELETE FROM characters WHERE id = $1")
            .bind(text_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_character(&self, character_id: Uuid) -> DbResult<Character> {
        let row = sqlx::query(
            "SELECT id, name, description, personality, scenario, first_message, example_messages FROM characters WHERE id = $1",
        )
        .bind(character_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Character {
                id: row.get("id"),
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
            sqlx::query("SELECT id, character_id, participants FROM chats WHERE character_id = $1")
                .bind(cid)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query("SELECT id, character_id, participants FROM chats")
                .fetch_all(&self.pool)
                .await?
        };

        // Collect chat IDs to fetch messages in batch
        let chat_ids: Vec<Uuid> = rows.iter().map(|r| r.get("id")).collect();

        // Fetch all messages for these chats in one query if there are any chats
        let mut messages_map: std::collections::HashMap<Uuid, Vec<ChatMessage>> =
            std::collections::HashMap::new();

        if !chat_ids.is_empty() {
            let placeholders: Vec<String> = chat_ids
                .iter()
                .enumerate()
                .map(|(i, _)| format!("${}", i + 1))
                .collect();
            let query = format!(
                "SELECT id, chat_id, role, content, sender_id, alternatives, active_index FROM messages WHERE chat_id IN ({})",
                placeholders.join(",")
            );

            let mut query_builder = sqlx::query(&query);
            for id in &chat_ids {
                query_builder = query_builder.bind(id);
            }

            let msg_rows = query_builder.fetch_all(&self.pool).await?;

            for row in msg_rows {
                let chat_id: Uuid = row.get("chat_id");
                let alts_val: Value = row.get("alternatives");
                let alternatives: Vec<String> =
                    serde_json::from_value(alts_val).unwrap_or_default();

                let msg = ChatMessage {
                    id: row.get("id"),
                    role: row.get("role"),
                    content: row.get("content"),
                    sender_id: row.get("sender_id"),
                    alternatives,
                    active_index: row.get::<i32, _>("active_index") as usize,
                };

                messages_map.entry(chat_id).or_default().push(msg);
            }
        }

        let mut chats = Vec::new();
        for row in rows {
            let participants_val: Value = row.get("participants");
            let participants: Vec<ChatParticipant> =
                serde_json::from_value(participants_val).map_err(DbError::Serde)?;
            let chat_id: Uuid = row.get("id");

            let messages = messages_map.remove(&chat_id).unwrap_or_default(); // Needs Uuid key

            chats.push(Chat {
                id: chat_id,
                character_id: row.get("character_id"),
                messages,
                participants,
            });
        }
        Ok(chats)
    }

    async fn create_chat(&self, chat: Chat) -> DbResult<()> {
        let participants_json = serde_json::to_value(&chat.participants)?;
        sqlx::query("INSERT INTO chats (id, character_id, participants) VALUES ($1, $2, $3)")
            .bind(chat.id)
            .bind(chat.character_id)
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
        let row = sqlx::query("SELECT id, character_id, participants FROM chats WHERE id = $1")
            .bind(chat_id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let participants_val: Value = row.get("participants");
                let participants: Vec<ChatParticipant> =
                    serde_json::from_value(participants_val).map_err(DbError::Serde)?;
                let messages = self.get_messages_for_chat(chat_id).await?;

                Ok(Chat {
                    id: chat_id,
                    character_id: row.get("character_id"),
                    messages,
                    participants,
                })
            }
            None => Err(DbError::NotFound(format!("Chat {} not found", chat_id))),
        }
    }

    async fn append_message(&self, chat_id: Uuid, message: ChatMessage) -> DbResult<()> {
        let alts_json = serde_json::to_value(&message.alternatives)?;
        let sender_id = message.sender_id;

        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, sender_id, alternatives, active_index) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(message.id)
        .bind(chat_id)
        .bind(message.role)
        .bind(message.content)
        .bind(sender_id)
        .bind(alts_json)
        .bind(message.active_index as i32)
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
        sqlx::query("DELETE FROM messages WHERE id = $1")
            .bind(message_id)
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
