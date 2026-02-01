use gloo_storage::{LocalStorage, Storage};
use shared::models::*;
use std::rc::Rc;
use yew::prelude::*;

const STORAGE_KEY: &str = "renoma.settings";

#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub characters: Vec<Character>,
    pub active_character_id: Option<uuid::Uuid>,
    pub active_chat: Option<Chat>,
    pub settings: AppSettings,
    pub modal_open: Option<ModalType>,
    pub is_generating: bool,
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
    SelectCharacter(uuid::Uuid),
    SetChat(Chat),
    AppendMessage(ChatMessage),
    UpdateLastMessage(String), // For streaming
    UpdateSettings(AppSettings),
    OpenModal(ModalType),
    CloseModal,
    SetGenerating(bool),
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
            Action::UpdateLastMessage(content) => {
                if let Some(chat) = &mut next.active_chat
                    && let Some(last) = chat.messages.last_mut()
                {
                    last.content = content;
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
        }

        next.into()
    }
}

pub type StoreContext = UseReducerHandle<State>;
