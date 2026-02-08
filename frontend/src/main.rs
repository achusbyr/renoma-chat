mod api;
mod components;
mod store;

use components::char_modal::CharModal;
use components::chat_stage::ChatStage;
use components::settings_modal::SettingsModal;
use components::sidebar::CharSidebar;
use store::{ModalType, State, StoreContext};
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    let store = use_reducer(State::default);

    let app_class = classes!(
        "app-container",
        store.active_chat.as_ref().map(|_| "chat-active")
    );

    html! {
        <ContextProvider<StoreContext> context={store.clone()}>
            <div class={app_class}>
                <div class="sidebar-container">
                    <CharSidebar />
                </div>
                <div class="main-stage">
                    <ChatStage />
                </div>

                {
                    match store.modal_open {
                        Some(ModalType::Settings) => html! { <SettingsModal /> },
                        Some(ModalType::CreateCharacter) => html! { <CharModal /> },
                        None => html! {},
                    }
                }
            </div>
        </ContextProvider<StoreContext>>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
