use crate::api;
use crate::store::{Action, ModalType, StoreContext};
use yew::prelude::*;

#[function_component(CharSidebar)]
pub fn char_sidebar() -> Html {
    let store = use_context::<StoreContext>().expect("Store context not found");

    // Load characters on mount
    {
        let store = store.clone();
        use_effect_with((), move |_| {
            yew::platform::spawn_local(async move {
                if let Ok(chars) = api::fetch_characters().await {
                    store.dispatch(Action::SetCharacters(chars));
                }
            });
            || {}
        });
    }

    let on_select = {
        let store = store.clone();
        Callback::from(move |id: uuid::Uuid| {
            store.dispatch(Action::SelectCharacter(id));

            // Side effect: Load chat for character
            let store = store.clone();
            yew::platform::spawn_local(async move {
                let chats = api::fetch_chats(id).await.unwrap_or_default();
                if let Some(chat) = chats.into_iter().next() {
                    store.dispatch(Action::SetChat(chat));
                } else if let Ok(new_chat) = api::create_chat(id).await {
                    store.dispatch(Action::SetChat(new_chat));
                }
            });
        })
    };

    let open_create = {
        let store = store.clone();
        Callback::from(move |_| store.dispatch(Action::OpenModal(ModalType::CreateCharacter)))
    };

    let open_settings = {
        let store = store.clone();
        Callback::from(move |_| store.dispatch(Action::OpenModal(ModalType::Settings)))
    };

    let on_delete = {
        let store = store.clone();
        Callback::from(move |id: uuid::Uuid| {
            let store = store.clone();
            yew::platform::spawn_local(async move {
                if web_sys::window()
                    .and_then(|w| w.confirm_with_message("Are you sure you want to delete this character? This will also delete all associated chats.").ok())
                    == Some(true)
                &&api::delete_character(id).await.is_ok() {
                        store.dispatch(Action::DeleteCharacter(id));
                    }
            });
        })
    };

    html! {
        <div class="sidebar">
            <header>
                <div class="sidebar-header-content">
                    <h1 class="app-title">{"Renoma"}</h1>
                </div>
                <div class="sidebar-toolbar">
                    <button class="icon-btn" onclick={open_create} title="Create Character">
                        <svg viewBox="0 0 24 24"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"></path></svg>
                    </button>
                    <button class="icon-btn" onclick={open_settings} title="Settings">
                        <svg viewBox="0 0 24 24"><path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L3.16 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"></path></svg>
                    </button>
                </div>
            </header>

            <div class="section-label">
                {"Characters"}
            </div>

            <div class="char-list">
                if store.characters.is_empty() {
                    <div class="sidebar-empty-state">
                        {"No characters found."}
                    </div>
                }
                { for store.characters.iter().map(|char| {
                    let id = char.id;
                    let on_click = on_select.clone();
                    let on_delete_click = on_delete.clone();
                    let is_active = Some(id) == store.active_character_id;

                    html! {
                        <div class={classes!("char-item", if is_active { "active" } else { "" })} onclick={move |_| on_click.emit(id)}>
                            <div class="avatar bot">{char.name.chars().next().unwrap_or('?')}</div>
                            <div class="char-info">
                                <div class="char-name">{&char.name}</div>
                                <div class="char-desc">{&char.description}</div>
                            </div>
                            <button class="delete-btn" onclick={move |e: MouseEvent| { e.stop_propagation(); on_delete_click.emit(id); }} title="Delete character">
                                <svg viewBox="0 0 24 24"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"></path></svg>
                            </button>
                        </div>
                    }
                })}
            </div>

            <div class="sidebar-footer">
                {"Renoma v0.1.0"}
            </div>
        </div>
    }
}
