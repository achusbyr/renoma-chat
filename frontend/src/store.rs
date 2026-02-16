use gloo_storage::{LocalStorage, Storage};
use shared::models::*;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

const LOCAL_STORAGE_KEY: &str = "renoma.settings";

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub characters: Vec<Character>,
    pub active_character_id: Option<Uuid>,
    pub chats: Vec<Chat>,
    pub active_chat: Option<Chat>,
    pub settings: AppSettings,
    pub modal_open: Option<ModalType>,
    pub active_stream: Option<StreamingContext>,
    pub editing_message_id: Option<Uuid>,
}

impl Default for State {
    fn default() -> Self {
        let settings =
            LocalStorage::get(LOCAL_STORAGE_KEY).unwrap_or_else(|_| AppSettings::default());
        Self {
            characters: Vec::new(),
            active_character_id: None,
            chats: Vec::new(),
            active_chat: None,
            settings,
            modal_open: None,
            active_stream: None,
            editing_message_id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StreamingContext {
    Generation(Uuid),
    Regeneration(Uuid),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModalType {
    Settings,
    CreateCharacter,
}

pub enum Action {
    SetCharacters(Vec<Character>),
    SetChats(Vec<Chat>),
    SelectChat(Uuid),
    SetActiveChat(Chat),
    AddChat(Chat),
    DeleteChat(Uuid),
    SetStream(Option<StreamingContext>),
    SelectCharacter(Uuid),
    DeleteCharacter(Uuid),
    DeleteMessage(Uuid),
    EditMessage {
        message_id: Uuid,
        content: String,
    },
    AppendMessage(ChatMessage),
    AppendAlternative {
        message_id: Uuid,
        content: String,
    },
    UpdateMessageContent {
        message_id: Uuid,
        content: String,
    },
    UpdateSettings(AppSettings),
    OpenModal(ModalType),
    CloseModal,
    CloseChat,
    /// -1 = left, +1 = right
    SwipeMessage {
        message_id: Uuid,
        direction: i32,
    },
}

impl Reducible for State {
    type Action = Action;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut next = (*self).clone();

        match action {
            Action::SetCharacters(chars) => {
                next.characters = chars;
            }
            Action::SelectCharacter(id) => {
                next.active_character_id = Some(id);
                next.chats = Vec::new();
                next.active_chat = None;
            }
            Action::DeleteCharacter(id) => {
                next.characters.retain(|c| c.id != id);
                if next.active_character_id == Some(id) {
                    next.active_character_id = None;
                    next.chats = Vec::new();
                    next.active_chat = None;
                }
            }
            Action::SetChats(chats) => {
                next.chats = chats;
            }
            Action::SelectChat(id) => {
                if let Some(chat) = next.chats.iter().find(|c| c.id == id) {
                    next.active_chat = Some(chat.clone());
                }
            }
            Action::SetActiveChat(chat) => {
                // Update in list if present
                if let Some(c) = next.chats.iter_mut().find(|c| c.id == chat.id) {
                    *c = chat.clone();
                }
                next.active_chat = Some(chat);
            }
            Action::AddChat(chat) => {
                next.chats.push(chat.clone());
                next.active_chat = Some(chat);
            }
            Action::DeleteChat(id) => {
                next.chats.retain(|c| c.id != id);
                if next.active_chat.as_ref().map(|c| c.id) == Some(id) {
                    next.active_chat = None;
                }
            }
            Action::AppendMessage(msg) => {
                if let Some(chat) = &mut next.active_chat {
                    chat.messages.push(msg);
                }
            }
            Action::UpdateMessageContent {
                message_id,
                content,
            } => {
                if let Some(chat) = &mut next.active_chat
                    && let Some(msg) = chat.messages.iter_mut().find(|m| m.id == message_id)
                {
                    msg.content = content;
                }
            }
            Action::UpdateSettings(settings) => {
                next.settings = settings.clone();
                let _ = LocalStorage::set(LOCAL_STORAGE_KEY, settings);
            }
            Action::OpenModal(modal_type) => {
                next.modal_open = Some(modal_type);
            }
            Action::CloseModal => {
                next.modal_open = None;
            }
            Action::SetStream(context) => {
                next.active_stream = context.clone();

                if let Some(StreamingContext::Regeneration(id)) = context
                    && let Some(chat) = &mut next.active_chat
                    && let Some(msg) = chat.messages.iter_mut().find(|m| m.id == id)
                    && msg.alternatives.is_empty()
                {
                    // Move current content to alternatives before it gets overwritten by stream
                    let original = msg.content.clone();
                    msg.alternatives.push(original);
                    msg.content = "".to_string();
                }
            }
            Action::EditMessage {
                message_id,
                content,
            } => {
                if let Some(chat) = &mut next.active_chat
                    && let Some(msg) = chat.messages.iter_mut().find(|m| m.id == message_id)
                {
                    // If we're editing an alternative, update the appropriate one
                    if msg.active_index == 0 {
                        msg.content = content;
                    } else if let Some(alt) = msg.alternatives.get_mut(msg.active_index - 1) {
                        *alt = content;
                    }
                }
                next.editing_message_id = None;
            }
            Action::DeleteMessage(message_id) => {
                if let Some(chat) = &mut next.active_chat {
                    chat.messages.retain(|m| m.id != message_id);
                }
            }
            Action::AppendAlternative {
                message_id,
                content,
            } => {
                if let Some(chat) = &mut next.active_chat
                    && let Some(msg) = chat.messages.iter_mut().find(|m| m.id == message_id)
                {
                    // If we already moved the original to alternatives in SetRegenerating,
                    // msg.alternatives will not be empty.
                    // We just need to ensure the final content is set, and we're on the right index.

                    // Add the new content as another alternative
                    msg.alternatives.push(content.clone());
                    // Switch to show the new alternative
                    msg.active_index = msg.alternatives.len();
                    msg.content = content;
                }
            }
            Action::SwipeMessage {
                message_id,
                direction,
            } => {
                if let Some(chat) = &mut next.active_chat
                    && let Some(msg) = chat.messages.iter_mut().find(|m| m.id == message_id)
                {
                    let total = msg.variant_count();
                    let new_index = if direction < 0 {
                        msg.active_index.saturating_sub(1)
                    } else {
                        (msg.active_index + 1).min(total - 1)
                    };

                    if new_index != msg.active_index {
                        msg.active_index = new_index;
                    }
                }
            }
            Action::CloseChat => {
                next.active_chat = None;
            }
        }

        next.into()
    }
}

pub type StoreContext = UseReducerHandle<State>;
