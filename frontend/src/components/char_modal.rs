use crate::api;
use crate::store::{Action, StoreContext};
use shared::models::CreateCharacterRequest;
use yew::prelude::*;

#[function_component(CharModal)]
pub fn char_modal() -> Html {
    let store = use_context::<StoreContext>().expect("Store context not found");

    // State for inputs
    let name = use_state(String::new);
    let desc = use_state(String::new);
    let personality = use_state(String::new);
    let scenario = use_state(String::new);
    let first_message = use_state(String::new);
    let example_messages = use_state(String::new);

    let on_save = {
        let store = store.clone();
        let name = name.clone();
        let desc = desc.clone();
        let personality = personality.clone();
        let scenario = scenario.clone();
        let first_message = first_message.clone();
        let example_messages = example_messages.clone();

        Callback::from(move |_| {
            let req = CreateCharacterRequest {
                name: (*name).clone(),
                description: (*desc).clone(),
                personality: (*personality).clone(),
                scenario: (*scenario).clone(),
                first_message: (*first_message).clone(),
                example_messages: (*example_messages).clone(),
            };

            let store = store.clone();
            yew::platform::spawn_local(async move {
                if let Ok(new_char) = api::create_character(req).await {
                    if let Ok(chars) = api::fetch_characters().await {
                        store.dispatch(Action::SetCharacters(chars));
                    }
                    store.dispatch(Action::SelectCharacter(new_char.id));
                    store.dispatch(Action::CloseModal);
                }
            });
        })
    };

    let on_close = {
        let store = store.clone();
        Callback::from(move |_| store.dispatch(Action::CloseModal))
    };

    let on_cancel = {
        let store = store.clone();
        Callback::from(move |_| store.dispatch(Action::CloseModal))
    };

    html! {
        <div class="modal-overlay" onclick={on_close}>
            <div class="modal-content" onclick={|e: MouseEvent| e.stop_propagation()}>
                <div class="modal-header">
                    <h2 class="modal-title">{"Create New Character"}</h2>
                    <button class="close-btn" onclick={on_cancel.clone()}>{"×"}</button>
                </div>

                <div class="modal-body">
                    <div class="form-group">
                        <label class="form-label">{"Name"}</label>
                        <input class="form-input" type="text" placeholder="e.g. Seraphina" oninput={Callback::from(move |e: InputEvent| {
                            let i: web_sys::HtmlInputElement = e.target_unchecked_into();
                            name.set(i.value());
                        })} />
                    </div>

                    <div class="form-group">
                        <label class="form-label">{"Description"}</label>
                        <textarea class="form-textarea" rows="2" placeholder="A brief summary of who they are..." oninput={Callback::from(move |e: InputEvent| {
                            let i: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                            desc.set(i.value());
                        })} />
                    </div>

                    <div class="form-group">
                        <label class="form-label">{"Personality"}</label>
                        <textarea class="form-textarea" rows="3" placeholder="Detailed personality traits, likes, dislikes..." oninput={Callback::from(move |e: InputEvent| {
                            let i: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                            personality.set(i.value());
                        })} />
                    </div>

                    <div class="form-group">
                        <label class="form-label">{"Scenario"}</label>
                        <textarea class="form-textarea" rows="2" placeholder="The setting or current situation..." oninput={Callback::from(move |e: InputEvent| {
                            let i: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                            scenario.set(i.value());
                        })} />
                    </div>

                    <div class="form-group">
                        <label class="form-label">{"First Message"}</label>
                        <textarea class="form-textarea" rows="2" placeholder="The very first thing the character says..." oninput={Callback::from(move |e: InputEvent| {
                            let i: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                            first_message.set(i.value());
                        })} />
                    </div>

                    <div class="form-group">
                        <label class="form-label">{"Example Messages"}</label>
                        <textarea class="form-textarea" rows="5" placeholder={"<START>\nUser: Hello!\nChar: Hi there! How can I help you today?"} oninput={Callback::from(move |e: InputEvent| {
                            let i: web_sys::HtmlTextAreaElement = e.target_unchecked_into();
                            example_messages.set(i.value());
                        })} />
                    </div>

                    <div class="form-actions">
                        <button class="btn btn-secondary" onclick={on_cancel}>{"Cancel"}</button>
                        <button class="btn btn-primary" onclick={on_save}>{"Create Character"}</button>
                    </div>
                </div>
            </div>
        </div>
    }
}
