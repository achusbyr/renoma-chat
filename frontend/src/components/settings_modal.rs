use crate::api;
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
            s.reasoning_effort = val;
            local_state.set(s);
        })
    };

    // Plugin effects and callbacks
    {
        let store = store.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(plugins) = api::fetch_plugins().await {
                    store.dispatch(Action::SetPlugins(plugins));
                }
            });
            || ()
        });
    }

    let on_toggle_plugin = {
        let store = store.clone();
        Callback::from(move |name: String| {
            let store = store.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if api::toggle_plugin(&name).await.is_ok()
                    && let Ok(plugins) = api::fetch_plugins().await
                {
                    store.dispatch(Action::SetPlugins(plugins));
                }
            });
        })
    };

    let on_discover = {
        let store = store.clone();
        Callback::from(move |_| {
            let store = store.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if api::discover_plugins().await.is_ok()
                    && let Ok(plugins) = api::fetch_plugins().await
                {
                    store.dispatch(Action::SetPlugins(plugins));
                }
            });
        })
    };

    let on_install = {
        let store = store.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files()
                && let Some(file) = files.get(0)
            {
                let store = store.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if api::install_plugin(file).await.is_ok()
                        && let Ok(plugins) = api::fetch_plugins().await
                    {
                        store.dispatch(Action::SetPlugins(plugins));
                    }
                });
            }
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
                                    <option value="none" selected={local_state.reasoning_effort == "none"}>{"None"}</option>
                                    <option value="low" selected={local_state.reasoning_effort == "low"}>{"Low"}</option>
                                    <option value="medium" selected={local_state.reasoning_effort == "medium"}>{"Medium"}</option>
                                    <option value="high" selected={local_state.reasoning_effort == "high"}>{"High"}</option>
                                </select>
                            </div>
                        </div>
                    </details>

                    <details class="plugins-section">
                        <summary>{"Plugins"}</summary>
                        <div class="plugins-content">
                            <div class="plugin-actions">
                                <button class="btn btn-secondary btn-sm" onclick={on_discover}>{"Discover Plugins"}</button>
                                <label class="btn btn-primary btn-sm">
                                    {"Install Plugin"}
                                    <input type="file" style="display: none;" onchange={on_install} />
                                </label>
                            </div>
                            <div class="plugin-list">
                                {for store.plugins.iter().map(|plugin| {
                                    let name = plugin.name.clone();
                                    let on_toggle = on_toggle_plugin.clone();
                                    html! {
                                        <div class="plugin-item">
                                            <div class="plugin-info">
                                                <div class="plugin-name">{&plugin.name} <span class="plugin-version">{&plugin.version}</span></div>
                                                <div class="plugin-desc">{&plugin.description}</div>
                                                <div class="tool-list">
                                                    {for plugin.tools.iter().map(|tool| html! {
                                                        <span class="tool-tag" title={tool.description.clone()}>{&tool.name}</span>
                                                    })}
                                                </div>
                                            </div>
                                            <label class="switch">
                                                <input type="checkbox" checked={plugin.enabled} onclick={move |_| on_toggle.emit(name.clone())} />
                                                <span class="slider round"></span>
                                            </label>
                                        </div>
                                    }
                                })}
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
