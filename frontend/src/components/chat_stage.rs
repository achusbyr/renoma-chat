use crate::api;
use crate::store::{Action, StoreContext};
use futures::StreamExt;
use gloo_net::http::Request;
use shared::models::{ChatMessage, GenerateRequest};
use web_sys::{Element, HtmlTextAreaElement};
use yew::prelude::*;

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
            store.dispatch(Action::AppendMessage(ChatMessage {
                role: "user".to_string(),
                content: text.clone(),
            }));

            // 2. Add a placeholder assistant message
            store.dispatch(Action::AppendMessage(ChatMessage {
                role: "assistant".to_string(),
                content: "".to_string(),
            }));

            store.dispatch(Action::SetGenerating(true));

            let store = store.clone();
            yew::platform::spawn_local(async move {
                // Save user message to backend
                let _ = api::send_message(chat_id, text).await;

                // Start Stream
                let payload = GenerateRequest {
                    chat_id,
                    api_key: settings.api_key,
                    api_base: Some(settings.api_base),
                    model: settings.model,
                    temperature: Some(settings.temperature),
                    max_tokens: Some(settings.max_tokens),
                };

                // Use gloo-net raw request for body streaming (feature might vary)
                // or plain fetch. Here using a custom loop for SSE parsing manually.
                let req = Request::post("/api/generate").json(&payload).unwrap();

                // Note: In a real prod app you might wrap this better,
                // but here is raw logic to read the body stream
                let resp = req.send().await.unwrap();

                if let Some(body) = resp.body() {
                    let mut stream = wasm_streams::ReadableStream::from_raw(body).into_stream();
                    let mut full_response = String::new();
                    let mut buffer = String::new();

                    while let Some(Ok(chunk)) = stream.next().await {
                        let chunk_text = chunk.as_string().unwrap_or_default();
                        // This is a naive SSE parser for the specific backend format
                        // Backend sends: "data: token\n"
                        buffer.push_str(&chunk_text);

                        let lines: Vec<&str> = buffer.split('\n').collect();
                        // Keep the last incomplete part
                        let last_idx = lines.len() - 1;

                        for (i, line) in lines.iter().enumerate() {
                            if i == last_idx {
                                buffer = line.to_string();
                                break;
                            }
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    break;
                                }
                                full_response.push_str(data);
                                store.dispatch(Action::UpdateLastMessage(full_response.clone()));
                            }
                        }
                    }
                }

                store.dispatch(Action::SetGenerating(false));
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

    html! {
    html! {
        <div class="main-stage">
            // Header
            if let Some(_) = store.active_chat {
                 <div class="chat-header" style="position: sticky; top: 0; background: var(--bg-sec); padding: 16px 24px; border-bottom: 1px solid var(--border); display: flex; align-items: center; z-index: 10;">
                    <div style="font-weight: 600; font-size: 1rem; color: var(--text-main);">{&char_name}</div>
                </div>
            }

            <div class={classes!("chat-message-list")} ref={container_ref}>
                if store.active_chat.is_none() {
                    <div class="chat-placeholder">
                        <div style="font-size: 3rem;">{"✨"}</div>
                        <div>{"Select a character to start chatting"}</div>
                    </div>
                } else {
                    { for store.active_chat.as_ref().unwrap().messages.iter().map(|msg| {
                        let is_user = msg.role == "user";
                        let name = if is_user { "You".to_string() } else { char_name.clone() };

                        html! {
                            <div class="message">
                                if !is_user {
                                    <div class="avatar bot" title={name.clone()}>{name.chars().next().unwrap_or('?')}</div>
                                }
                                <div class="message-content">
                                    <div class="message-role">{name}</div>
                                    <div style="white-space: pre-wrap;">{ &msg.content }</div>
                                </div>
                            </div>
                        }
                    })}

                    if store.is_generating {
                         <div class="message">
                            <div class="avatar bot">{char_name.chars().next().unwrap_or('?')}</div>
                             <div class="message-content">
                                <div class="message-role">{&char_name}</div>
                                <div>{"..."}</div>
                            </div>
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
}
