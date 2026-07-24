use dioxus::prelude::*;
use gloo_timers::future::sleep;
use serde_json::Value;
use std::time::Duration;
use wasm_bindgen::JsCast;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use super::state::{ChatMessage, ChatState};

/// WebSocket 连接管理
pub async fn connect_ws(
    state: Signal<ChatState>,
    ws: Signal<Option<WebSocket>>,
    message_container: Signal<Option<MountedData>>,
) {
    if ws.read().is_some() {
        return;
    }

    let window = web_sys::window().expect("window should be available in browser context");
    let protocol = window.location().protocol().unwrap_or_default();
    let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
    let host = window.location().host().unwrap_or_default();
    let ws_url = format!("{}//{}/ws/ai-chat", ws_protocol, host);

    let socket = match WebSocket::new(&ws_url) {
        Ok(s) => s,
        Err(e) => {
            state.write().error = Some(format!(
                "WebSocket连接失败: {}",
                e.as_string().unwrap_or_default()
            ));
            schedule_reconnect(state, ws, message_container).await;
            return;
        }
    };
    socket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // on_message
    let state_clone = state.clone();
    let ws_clone = ws.clone();
    let message_container_clone = message_container.clone();
    let on_message = Closure::wrap(Box::new(move |event: MessageEvent| {
        if let Ok(text) = event.data().as_string() {
            if text == "ping" {
                if let Some(s) = ws_clone.read().as_ref() {
                    let _ = s.send_with_str("pong");
                }
                return;
            }
            let mut state = state_clone.write();
            if let Ok(data) = serde_json::from_str::<Value>(&text) {
                match data.get("type").and_then(|t| t.as_str()) {
                    Some("chunk") => {
                        if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
                            if let Some(ChatMessage::Assistant {
                                content: ref mut c, ..
                            }) = state.messages.last_mut()
                            {
                                c.push_str(content);
                            } else {
                                state.messages.push(ChatMessage::Assistant {
                                    content: content.to_string(),
                                    thinking: String::new(),
                                    tool_results: Vec::new(),
                                });
                            }
                        }
                    }
                    Some("thinking") => {
                        if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
                            state.current_thinking.push_str(content);
                            if let Some(ChatMessage::Assistant {
                                thinking: ref mut t, ..
                            }) = state.messages.last_mut()
                            {
                                t.push_str(content);
                            } else {
                                state.messages.push(ChatMessage::Assistant {
                                    content: String::new(),
                                    thinking: content.to_string(),
                                    tool_results: Vec::new(),
                                });
                            }
                        }
                    }
                    Some("tool_result") => {
                        if let Some(results) = data.get("results").and_then(|r| r.as_array()) {
                            for result in results {
                                if let (Some(role), Some(content)) = (
                                    result.get("role").and_then(|r| r.as_str()),
                                    result.get("content").and_then(|c| c.as_str()),
                                ) {
                                    if role == "tool" {
                                        let tool_name = result
                                            .get("name")
                                            .and_then(|n| n.as_str())
                                            .unwrap_or("unknown_tool")
                                            .to_string();
                                        state.messages.push(ChatMessage::Tool {
                                            name: tool_name,
                                            args: String::new(),
                                            result: content.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Some("title") => {
                        if let Some(title) = data.get("title").and_then(|t| t.as_str()) {
                            if let Some(session) = state.current_session.as_mut() {
                                session.title = title.to_string();
                            }
                        }
                    }
                    Some("error") => {
                        if let Some(message) = data.get("message").and_then(|m| m.as_str()) {
                            state.error = Some(message.to_string());
                            state.is_typing = false;
                        }
                    }
                    Some("stream_end") => {
                        state.is_typing = false;
                        state.current_thinking = String::new();
                    }
                    Some("message") => {
                        if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
                            state.messages.push(ChatMessage::Assistant {
                                content: content.to_string(),
                                thinking: String::new(),
                                tool_results: Vec::new(),
                            });
                            state.is_typing = false;
                        }
                    }
                    Some("image") => {
                        if let Some(url) = data.get("url").and_then(|u| u.as_str()) {
                            if let Some(ChatMessage::Assistant {
                                content: ref mut c, ..
                            }) = state.messages.last_mut()
                            {
                                c.push_str(&format!("\n\n![image]({})", url));
                            }
                        }
                    }
                    _ => {}
                }
            }
            scroll_to_bottom(&message_container_clone);
        }
    }) as Box<dyn FnMut(_)>);

    socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    // on_close
    let state_clone2 = state.clone();
    let ws_clone2 = ws.clone();
    let message_container_clone2 = message_container.clone();
    let on_close = Closure::wrap(Box::new(move |_event: CloseEvent| {
        let mut state = state_clone2.write();
        state.is_typing = false;
        state.ws_connected = false;
        state.reconnect_attempts += 1;
        ws_clone2.write().take();
        spawn(async move {
            schedule_reconnect(state_clone2, ws_clone2, message_container_clone2).await;
        });
    }) as Box<dyn FnMut(_)>);
    socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    on_close.forget();

    // on_error
    let state_clone3 = state.clone();
    let ws_clone3 = ws.clone();
    let message_container_clone3 = message_container.clone();
    let on_error = Closure::wrap(Box::new(move |_event: ErrorEvent| {
        let mut state = state_clone3.write();
        state.is_typing = false;
        state.error = Some("WebSocket连接失败".to_string());
        state.ws_connected = false;
        state.reconnect_attempts += 1;
        ws_clone3.write().take();
        spawn(async move {
            schedule_reconnect(state_clone3, ws_clone3, message_container_clone3).await;
        });
    }) as Box<dyn FnMut(_)>);
    socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();

    // on_open
    let state_clone4 = state.clone();
    let on_open = Closure::wrap(Box::new(move || {
        let mut state = state_clone4.write();
        state.ws_connected = true;
        state.reconnect_attempts = 0;
        state.error = None;
    }) as Box<dyn FnMut()>);
    socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    ws.write().replace(socket);

    start_heartbeat(ws, state);
}

/// 重连调度（指数退避）
pub async fn schedule_reconnect(
    state: Signal<ChatState>,
    ws: Signal<Option<WebSocket>>,
    message_container: Signal<Option<MountedData>>,
) {
    let attempts = state.read().reconnect_attempts;
    let delay = std::cmp::min(attempts * 2, 16);
    sleep(Duration::from_secs(delay)).await;
    if ws.read().is_none() {
        connect_ws(state, ws, message_container).await;
    }
}

/// 心跳检测（30秒间隔）
pub fn start_heartbeat(ws: Signal<Option<WebSocket>>, state: Signal<ChatState>) {
    spawn(async move {
        loop {
            sleep(Duration::from_secs(30)).await;
            if let Some(socket) = ws.read().as_ref() {
                if socket.ready_state() == 1 {
                    let _ = socket.send_with_str("ping");
                } else {
                    state.write().ws_connected = false;
                    ws.write().take();
                    break;
                }
            } else {
                break;
            }
        }
    });
}

/// 滚动到消息容器底部
fn scroll_to_bottom(container: &Signal<Option<MountedData>>) {
    if let Some(md) = container.read().as_ref() {
        if let Ok(element) = md.get() {
            if let Some(elem) = element.dyn_ref::<web_sys::HtmlDivElement>() {
                elem.set_scroll_top(elem.scroll_height());
            }
        }
    }
}