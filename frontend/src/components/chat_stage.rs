use crate::api;
use crate::store::{Action, StoreContext};
use futures::StreamExt;
use gloo_net::http::Request;
use shared::models::{ChatMessage, CompletionRequest};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Element, HtmlTextAreaElement, js_sys};
use yew::prelude::*;

/// Props for individual message bubble component
#[derive(Properties, PartialEq)]
pub struct MessageBubbleProps {
    pub message: ChatMessage,
    pub char_name: String,
    pub is_last_assistant: bool,
    pub is_generating: bool,
}

/// Individual message bubble with actions
#[function_component(MessageBubble)]
pub fn message_bubble(props: &MessageBubbleProps) -> Html {
    let store = use_context::<StoreContext>().expect("Store context not found");
    let is_editing = use_state(|| false);
    let edit_content = use_state(|| props.message.content.clone());
    let is_hovered = use_state(|| false);

    let is_user = props.message.role == "user";
    let name = if is_user {
        "You".to_string()
    } else {
        props.char_name.clone()
    };

    // Get the currently displayed content (considering swipes)
    let display_content = props.message.active_content().to_string();
    let variant_count = props.message.variant_count();
    let active_index = props.message.active_index;

    let on_mouse_enter = {
        let is_hovered = is_hovered.clone();
        Callback::from(move |_: MouseEvent| is_hovered.set(true))
    };

    let on_mouse_leave = {
        let is_hovered = is_hovered.clone();
        Callback::from(move |_: MouseEvent| is_hovered.set(false))
    };

    // Edit handlers
    let on_edit_click = {
        let is_editing = is_editing.clone();
        let edit_content = edit_content.clone();
        let content = props.message.content.clone();
        Callback::from(move |_: MouseEvent| {
            edit_content.set(content.clone());
            is_editing.set(true);
        })
    };

    let on_edit_change = {
        let edit_content = edit_content.clone();
        Callback::from(move |e: InputEvent| {
            let target = e.target_dyn_into::<HtmlTextAreaElement>();
            if let Some(textarea) = target {
                edit_content.set(textarea.value());
            }
        })
    };

    let on_edit_save = {
        let is_editing = is_editing.clone();
        let edit_content = edit_content.clone();
        let store = store.clone();
        let message_id = props.message.id;
        Callback::from(move |_: MouseEvent| {
            let content = (*edit_content).clone();
            store.dispatch(Action::EditMessage {
                message_id,
                content: content.clone(),
            });
            is_editing.set(false);

            // Persist to backend
            let store = store.clone();
            let chat_id = store.active_chat.as_ref().map(|c| c.id);
            if let Some(chat_id) = chat_id {
                yew::platform::spawn_local(async move {
                    if let Err(e) = api::edit_message(chat_id, message_id, content).await {
                        tracing::error!("Failed to edit message: {:?}", e);
                    }
                });
            }
        })
    };

    let on_edit_cancel = {
        let is_editing = is_editing.clone();
        Callback::from(move |_: MouseEvent| {
            is_editing.set(false);
        })
    };

    let on_edit_keydown = {
        let is_editing = is_editing.clone();
        let edit_content = edit_content.clone();
        let store = store.clone();
        let message_id = props.message.id;
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Escape" {
                is_editing.set(false);
            } else if e.key() == "Enter" && e.ctrl_key() {
                e.prevent_default();
                let content = (*edit_content).clone();
                store.dispatch(Action::EditMessage {
                    message_id,
                    content: content.clone(),
                });
                is_editing.set(false);
            }
        })
    };

    // Delete handler
    let on_delete = {
        let store = store.clone();
        let message_id = props.message.id;
        Callback::from(move |_: MouseEvent| {
            store.dispatch(Action::DeleteMessage(message_id));

            // Persist to backend
            let store = store.clone();
            let chat_id = store.active_chat.as_ref().map(|c| c.id);
            if let Some(chat_id) = chat_id {
                yew::platform::spawn_local(async move {
                    if let Err(e) = api::delete_message(chat_id, message_id).await {
                        tracing::error!("Failed to delete message: {:?}", e);
                    }
                });
            }
        })
    };

    // Regenerate handler (for assistant messages)
    let on_regenerate = {
        let store = store.clone();
        let message_id = props.message.id;
        Callback::from(move |_: MouseEvent| {
            let store = store.clone();
            let chat = store.active_chat.clone();
            let settings = store.settings.clone();

            if let Some(chat) = chat {
                store.dispatch(Action::SetRegenerating(Some(message_id)));

                yew::platform::spawn_local(process_completion_stream(
                    store,
                    CompletionRequest {
                        chat_id: chat.id,
                        regenerate: true,
                        message_id: Some(message_id),
                        api_key: settings.api_key,
                        api_base: Some(settings.api_base),
                        model: settings.model,
                        temperature: Some(settings.temperature),
                        max_tokens: Some(settings.max_tokens),
                    },
                    message_id,
                ));
            }
        })
    };

    // Swipe handlers
    let on_swipe_left = {
        let store = store.clone();
        let message_id = props.message.id;
        Callback::from(move |_: MouseEvent| {
            store.dispatch(Action::SwipeMessage {
                message_id,
                direction: -1,
            });

            // Persist to backend
            let store = store.clone();
            let chat_id = store.active_chat.as_ref().map(|c| c.id);
            if let Some(chat_id) = chat_id {
                yew::platform::spawn_local(async move {
                    let _ = api::swipe_message(
                        chat_id,
                        message_id,
                        shared::models::SwipeDirection::Left,
                    )
                    .await;
                });
            }
        })
    };

    let on_swipe_right = {
        let store = store.clone();
        let message_id = props.message.id;
        Callback::from(move |_: MouseEvent| {
            store.dispatch(Action::SwipeMessage {
                message_id,
                direction: 1,
            });

            // Persist to backend
            let store = store.clone();
            let chat_id = store.active_chat.as_ref().map(|c| c.id);
            if let Some(chat_id) = chat_id {
                yew::platform::spawn_local(async move {
                    let _ = api::swipe_message(
                        chat_id,
                        message_id,
                        shared::models::SwipeDirection::Right,
                    )
                    .await;
                });
            }
        })
    };

    // Copy handler
    let on_copy = {
        let content = display_content.clone();
        Callback::from(move |_: MouseEvent| {
            let content = content.clone();
            yew::platform::spawn_local(async move {
                if let Some(window) = web_sys::window() {
                    let clipboard = window.navigator().clipboard();
                    let promise = clipboard.write_text(&content);
                    let _ = JsFuture::from(promise).await;
                }
            });
        })
    };

    let show_actions = *is_hovered && !*is_editing && !props.is_generating;
    let is_regenerating = store.regenerating_message_id == Some(props.message.id);

    html! {
        <div
            class={classes!("message", if is_user { "message-user" } else { "message-assistant" })}
            onmouseenter={on_mouse_enter}
            onmouseleave={on_mouse_leave}
        >
            if !is_user {
                <div class="avatar bot" title={name.clone()}>
                    {name.chars().next().unwrap_or('?')}
                </div>
            }
            <div class="message-content">
                <div class="message-role">{&name}</div>

                if *is_editing {
                    <div class="message-edit-container">
                        <textarea
                            class="message-edit-textarea"
                            value={(*edit_content).clone()}
                            oninput={on_edit_change}
                            onkeydown={on_edit_keydown}
                        />
                        <div class="message-edit-actions">
                            <button class="btn btn-primary btn-sm" onclick={on_edit_save}>{"Save"}</button>
                            <button class="btn btn-secondary btn-sm" onclick={on_edit_cancel}>{"Cancel"}</button>
                        </div>
                        <div class="message-edit-hint">{"Ctrl+Enter to save, Escape to cancel"}</div>
                    </div>
                } else {
                    <div class="message-text">
                        if is_regenerating && display_content.is_empty() {
                            <div class="regenerating-dots">{"..."}</div>
                        } else {
                            <super::markdown::Markdown content={display_content} />
                        }
                    </div>
                }

                // Swipe navigation (if alternatives exist)
                if variant_count > 1 && !*is_editing {
                    <div class="swipe-nav">
                        <button
                            class="swipe-btn"
                            onclick={on_swipe_left}
                            disabled={active_index == 0}
                        >
                            {"◀"}
                        </button>
                        <span class="swipe-indicator">
                            {format!("{}/{}", active_index + 1, variant_count)}
                        </span>
                        <button
                            class="swipe-btn"
                            onclick={on_swipe_right}
                            disabled={active_index >= variant_count - 1}
                        >
                            {"▶"}
                        </button>
                    </div>
                }

                // Action toolbar
                if show_actions {
                    <div class="message-actions">
                        <button class="message-action-btn" onclick={on_copy} title="Copy">
                            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                                <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                            </svg>
                        </button>
                        <button class="message-action-btn" onclick={on_edit_click} title="Edit">
                            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                                <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/>
                            </svg>
                        </button>
                        if !is_user {
                            <button
                                class="message-action-btn"
                                onclick={on_regenerate}
                                title="Regenerate"
                                disabled={props.is_generating}
                            >
                                <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                                    <path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/>
                                </svg>
                            </button>
                        }
                        <button class="message-action-btn message-action-btn-danger" onclick={on_delete} title="Delete">
                            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                                <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                            </svg>
                        </button>
                    </div>
                }
            </div>
        </div>
    }
}

#[function_component(ChatStage)]
pub fn chat_stage() -> Html {
    let store = use_context::<StoreContext>().expect("Store context not found");
    let input_ref = use_node_ref();
    let container_ref = use_node_ref();

    // Auto-scroll on message change
    {
        let container_ref = container_ref.clone();
        let messages_len = store
            .active_chat
            .as_ref()
            .map(|c| c.messages.len())
            .unwrap_or(0);
        use_effect_with(messages_len, move |_| {
            if let Some(div) = container_ref.cast::<Element>() {
                div.set_scroll_top(div.scroll_height());
            }
            || {}
        });
    }

    let on_send = {
        let store = store.clone();
        let input_ref = input_ref.clone();

        Callback::from(move |_| {
            let input = input_ref.cast::<HtmlTextAreaElement>().unwrap();
            let text = input.value().trim().to_string();

            if text.is_empty() || store.is_generating || store.active_chat.is_none() {
                return;
            }

            input.set_value("");

            let chat_id = store.active_chat.as_ref().unwrap().id;
            let settings = store.settings.clone();

            // 1. Update UI with a user message
            store.dispatch(Action::AppendMessage(ChatMessage::new(
                "user",
                text.clone(),
            )));

            // 2. Add a placeholder assistant message
            let assistant_msg = ChatMessage::new("assistant", "");
            let assistant_msg_id = assistant_msg.id;
            store.dispatch(Action::AppendMessage(assistant_msg));

            store.dispatch(Action::SetGenerating(true));

            let store = store.clone();
            yew::platform::spawn_local(async move {
                // Save user message to backend
                if let Err(e) = api::send_message(chat_id, text).await {
                    tracing::error!("Failed to send message: {:?}", e);
                    store.dispatch(Action::SetGenerating(false));
                    return;
                }

                // Start Stream
                process_completion_stream(
                    store,
                    CompletionRequest {
                        chat_id,
                        regenerate: false,
                        message_id: None,
                        api_key: settings.api_key,
                        api_base: Some(settings.api_base),
                        model: settings.model,
                        temperature: Some(settings.temperature),
                        max_tokens: Some(settings.max_tokens),
                    },
                    assistant_msg_id,
                )
                .await;
            });
        })
    };

    let on_keydown = {
        let on_send = on_send.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" && !e.shift_key() {
                e.prevent_default();
                on_send.emit(());
            }
        })
    };

    let char_name = store
        .characters
        .iter()
        .find(|c| Some(c.id) == store.active_character_id)
        .map(|c| c.name.clone())
        .unwrap_or("AI".to_string());

    // Find the last assistant message index for regenerate button visibility
    let last_assistant_idx = store.active_chat.as_ref().and_then(|chat| {
        chat.messages
            .iter()
            .enumerate()
            .rev()
            .find(|(_, m)| m.role == "assistant")
            .map(|(i, _)| i)
    });

    html! {
        <div class="main-stage">
            // Header
            if let Some(_) = store.active_chat {
                <div class="chat-header">
                    <div class="chat-title">{&char_name}</div>
                </div>
            }

            <div class={classes!("chat-message-list")} ref={container_ref}>
                if store.active_chat.is_none() {
                    <div class="chat-placeholder">
                        <div class="chat-placeholder-icon">{"✨"}</div>
                        <div>{"Select a character to start chatting"}</div>
                    </div>
                } else {
                    { for store.active_chat.as_ref().unwrap().messages.iter().enumerate().map(|(idx, msg)| {
                        let is_last_assistant = Some(idx) == last_assistant_idx;
                        html! {
                            <MessageBubble
                                message={msg.clone()}
                                char_name={char_name.clone()}
                                is_last_assistant={is_last_assistant}
                                is_generating={store.is_generating}
                            />
                        }
                    })}

                    if store.is_generating {
                        <div class="typing-indicator">
                            <span></span>
                            <span></span>
                            <span></span>
                        </div>
                    }
                }
            </div>

            <div class="input-area">
                <div class="input-box">
                    <textarea
                        class="chat-input"
                        ref={input_ref}
                        placeholder={"Type a message..."}
                        onkeydown={on_keydown}
                    />
                    <button class="send-btn" onclick={move |_| on_send.emit(())} disabled={store.is_generating}>
                         <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor"><path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"></path></svg>
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Helper to process the completion stream and update the store
async fn process_completion_stream(
    store: StoreContext,
    payload: CompletionRequest,
    message_id: uuid::Uuid,
) {
    let req = match Request::post("/api/completion").json(&payload) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to create request: {:?}", e);
            if payload.regenerate {
                store.dispatch(Action::SetRegenerating(None));
            } else {
                store.dispatch(Action::SetGenerating(false));
            }
            return;
        }
    };

    let resp = match req.send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Failed to send request: {:?}", e);
            store.dispatch(Action::UpdateMessageContent {
                message_id,
                content: format!("[Error: {}]", e),
            });
            if payload.regenerate {
                store.dispatch(Action::SetRegenerating(None));
            } else {
                store.dispatch(Action::SetGenerating(false));
            }
            return;
        }
    };

    if let Some(body) = resp.body() {
        let mut stream = wasm_streams::ReadableStream::from_raw(body).into_stream();
        let mut full_response = String::new();
        let mut buffer = Vec::new();

        while let Some(result) = stream.next().await {
            let chunk = match result {
                Ok(chunk) => chunk,
                Err(e) => {
                    tracing::error!("Stream error: {:?}", e);
                    break;
                }
            };

            let bytes = js_sys::Uint8Array::new(&chunk).to_vec();
            buffer.extend_from_slice(&bytes);

            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                let line_bytes = buffer.drain(..pos + 1).collect::<Vec<u8>>();
                let line = String::from_utf8_lossy(&line_bytes);
                let line = line.trim_end_matches(['\n', '\r']);

                if line.is_empty() {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }
                    if data.starts_with("[ERROR]") {
                        tracing::error!("Backend error in stream: {}", data);
                        full_response.push_str(data);
                        store.dispatch(Action::UpdateMessageContent {
                            message_id,
                            content: full_response.clone(),
                        });
                        break;
                    }
                    full_response.push_str(data);
                    store.dispatch(Action::UpdateMessageContent {
                        message_id,
                        content: full_response.clone(),
                    });
                }
            }
        }

        if payload.regenerate && !full_response.is_empty() {
            store.dispatch(Action::AppendAlternative {
                message_id,
                content: full_response,
            });
        }
    }

    if payload.regenerate {
        store.dispatch(Action::SetRegenerating(None));
    } else {
        store.dispatch(Action::SetGenerating(false));
    }
}
