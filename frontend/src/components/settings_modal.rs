use crate::store::{Action, StoreContext};
use yew::prelude::*;

#[function_component(SettingsModal)]
pub fn settings_modal() -> Html {
    let store = use_context::<StoreContext>().expect("Store context not found");

    // Local state for form fields to avoid global dispatch on every keystroke
    let local_state = use_state(|| store.settings.clone());

    let on_submit = {
        let store = store.clone();
        let local_state = local_state.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            store.dispatch(Action::UpdateSettings((*local_state).clone()));
            store.dispatch(Action::CloseModal);
        })
    };

    let on_cancel = {
        let store = store.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            store.dispatch(Action::CloseModal);
        })
    };

    let on_overlay_click = {
        let store = store.clone();
        Callback::from(move |_| store.dispatch(Action::CloseModal))
    };

    // Field update callbacks
    let on_api_key_input = {
        let local_state = local_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut s = (*local_state).clone();
            s.api_key = input.value();
            local_state.set(s);
        })
    };

    let on_api_base_input = {
        let local_state = local_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut s = (*local_state).clone();
            s.api_base = input.value();
            local_state.set(s);
        })
    };

    let on_model_input = {
        let local_state = local_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let mut s = (*local_state).clone();
            s.model = input.value();
            local_state.set(s);
        })
    };

    let on_temperature_input = {
        let local_state = local_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(val) = input.value().parse::<f32>() {
                let mut s = (*local_state).clone();
                s.temperature = val;
                local_state.set(s);
            }
        })
    };

    let on_max_tokens_input = {
        let local_state = local_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Ok(val) = input.value().parse::<u16>() {
                let mut s = (*local_state).clone();
                s.max_tokens = val;
                local_state.set(s);
            }
        })
    };

    let on_reasoning_change = {
        let local_state = local_state.clone();
        Callback::from(move |e: Event| {
            let select: web_sys::HtmlSelectElement = e.target_unchecked_into();
            let val = select.value();
            let mut s = (*local_state).clone();
            if val.is_empty() {
                s.reasoning_effort = None;
            } else {
                s.reasoning_effort = Some(val);
            }
            local_state.set(s);
        })
    };

    html! {
        <div class="modal-overlay" onclick={on_overlay_click}>
            <div class="modal-content" onclick={|e: MouseEvent| e.stop_propagation()}>
                <div class="modal-header">
                    <h2 class="modal-title">{"Settings"}</h2>
                    <button class="close-btn" onclick={on_cancel.clone()}>{"×"}</button>
                </div>

                <div class="modal-body">
                    <div class="form-group">
                        <label class="form-label">{"API Key"}</label>
                        <input type="password" class="form-input"
                            value={local_state.api_key.clone()}
                            oninput={on_api_key_input}
                            placeholder="sk-..."
                        />
                    </div>

                    <div class="form-group">
                        <label class="form-label">{"API Base URL"}</label>
                        <input type="text" class="form-input"
                            value={local_state.api_base.clone()}
                            oninput={on_api_base_input}
                            placeholder="https://openrouter.ai/api/v1"
                        />
                    </div>

                    <details class="model-config-section">
                        <summary>{"Model Configuration"}</summary>
                        <div class="model-config-content">
                            <div class="form-group">
                                <label class="form-label">{"Model"}</label>
                                <input type="text" class="form-input"
                                    value={local_state.model.clone()}
                                    oninput={on_model_input}
                                    placeholder="tngtech/deepseek-r1t2-chimera:free"
                                />
                            </div>

                            <div class="form-grid-2">
                                <div class="form-group">
                                    <label class="form-label">{"Temperature"}</label>
                                    <input type="number" class="form-input"
                                        step="0.1" min="0" max="2"
                                        value={local_state.temperature.to_string()}
                                        oninput={on_temperature_input}
                                    />
                                </div>
                                <div class="form-group">
                                    <label class="form-label">{"Max Tokens"}</label>
                                    <input type="number" class="form-input"
                                        min="1"
                                        value={local_state.max_tokens.to_string()}
                                        oninput={on_max_tokens_input}
                                    />
                                </div>
                            </div>

                            <div class="form-group">
                                <label class="form-label">{"Reasoning Effort"}</label>
                                <select class="form-select" onchange={on_reasoning_change}>
                                    <option value="" selected={local_state.reasoning_effort.is_none()}>{"Disabled"}</option>
                                    <option value="low" selected={local_state.reasoning_effort.as_deref() == Some("low")}>{"Low"}</option>
                                    <option value="medium" selected={local_state.reasoning_effort.as_deref() == Some("medium")}>{"Medium"}</option>
                                    <option value="high" selected={local_state.reasoning_effort.as_deref() == Some("high")}>{"High"}</option>
                                </select>
                            </div>
                        </div>
                    </details>

                    <div class="form-actions">
                        <button class="btn btn-secondary" onclick={on_cancel}>{"Cancel"}</button>
                        <button class="btn btn-primary" onclick={on_submit}>{"Save Settings"}</button>
                    </div>
                </div>
            </div>
        </div>
    }
}
