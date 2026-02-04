use gloo_storage::{LocalStorage, Storage};
use shared::models::*;
use std::rc::Rc;
use uuid::Uuid;
use yew::prelude::*;

const STORAGE_KEY: &str = "renoma.settings";

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub characters: Vec<Character>,
    pub active_character_id: Option<Uuid>,
    pub active_chat: Option<Chat>,
    pub settings: AppSettings,
    pub modal_open: Option<ModalType>,
    pub is_generating: bool,
    pub editing_message_id: Option<Uuid>,
    pub regenerating_message_id: Option<Uuid>,
}

impl Default for State {
    fn default() -> Self {
        let settings = LocalStorage::get(STORAGE_KEY).unwrap_or_else(|_| AppSettings::default());
        Self {
            characters: Vec::new(),
            active_character_id: None,
            active_chat: None,
            settings,
            modal_open: None,
            is_generating: false,
            editing_message_id: None,
            regenerating_message_id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModalType {
    Settings,
    CreateCharacter,
}

pub enum Action {
    SetCharacters(Vec<Character>),
    SelectCharacter(Uuid),
    SetChat(Chat),
    AppendMessage(ChatMessage),
    UpdateMessageContent { message_id: Uuid, content: String },
    UpdateSettings(AppSettings),
    OpenModal(ModalType),
    CloseModal,
    SetGenerating(bool),
    EditMessage { message_id: Uuid, content: String },
    DeleteMessage(Uuid),
    SetRegenerating(Option<Uuid>),
    AppendAlternative { message_id: Uuid, content: String },
    SwipeMessage { message_id: Uuid, direction: i32 }, // -1 = left, +1 = right
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
                // Chat loading is handled by a side effect in the component triggering this
                // or a use_effect in the main layout
                next.active_chat = None;
            }
            Action::SetChat(chat) => {
                next.active_chat = Some(chat);
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
                let _ = LocalStorage::set(STORAGE_KEY, settings);
            }
            Action::OpenModal(modal_type) => {
                next.modal_open = Some(modal_type);
            }
            Action::CloseModal => {
                next.modal_open = None;
            }
            Action::SetGenerating(is_gen) => {
                next.is_generating = is_gen;
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
            Action::SetRegenerating(message_id) => {
                next.regenerating_message_id = message_id;
                if let Some(id) = message_id
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
        }

        next.into()
    }
}

pub type StoreContext = UseReducerHandle<State>;
