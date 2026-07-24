mod components;
mod state;
mod websocket;

use dioxus::prelude::*;
use eq_ui::prelude::*;
use gloo_timers::future::sleep;
use serde_json::json;
use std::time::Duration;
use web_sys::{KeyboardEvent, WebSocket};

use crate::api::client::{
    create_chat_session, delete_chat_session, get_ai_providers, get_chat_session,
    list_chat_sessions, screenshot_analyze, AiProvider, ChatSession,
};

use self::components::{render_markdown, scroll_to_bottom, QuickPromptCard};
use self::state::{ChatMessage, ChatState};
use self::websocket::{connect_ws, schedule_reconnect};

#[component]
pub fn ChatPage() -> Element {
    let state = use_signal(ChatState::default);
    let ws = use_signal(|| Option::<WebSocket>::None);
    let message_container = use_signal(|| None::<MountedData>);

    // 初始化加载
    use_effect(move || {
        let state = state.clone();
        async move {
            let (providers_res, sessions_res) =
                join(get_ai_providers(), list_chat_sessions()).await;

            let mut new_state = state.write();
            new_state.loading_providers = false;
            new_state.loading_sessions = false;

            match providers_res {
                Ok(providers) => {
                    new_state.providers = providers;
                    if let Some(p) = new_state.providers.iter().find(|p| p.enabled).cloned() {
                        new_state.selected_provider = Some(p);
                    }
                }
                Err(e) => {
                    new_state.error = Some(format!("加载提供商失败: {}", e));
                }
            }
            match sessions_res {
                Ok(sessions) => {
                    new_state.sessions = sessions;
                }
                Err(e) => {
                    new_state.error = Some(format!("加载会话失败: {}", e));
                }
            }
        }
    });

    // 自动连接 WebSocket
    use_effect(move || {
        let ws = ws.clone();
        let state = state.clone();
        let message_container = message_container.clone();
        async move {
            if ws.read().is_none() {
                connect_ws(state, ws, message_container).await;
            }
        }
    });

    let send_message = move |message: String| {
        let state = state.clone();
        let ws = ws.clone();
        let message_container = message_container.clone();
        async move {
            let provider = match state.read().selected_provider.clone() {
                Some(p) => p,
                None => {
                    state.write().error = Some("请先选择AI提供商".to_string());
                    return;
                }
            };

            if message.trim().is_empty() {
                return;
            }

            let mut state_mut = state.write();
            state_mut.messages.push(ChatMessage::User {
                content: message.clone(),
            });
            state_mut.current_message = String::new();
            state_mut.is_typing = true;
            state_mut.error = None;
            state_mut.current_thinking = String::new();

            if ws.read().is_none() {
                drop(state_mut);
                connect_ws(state, ws, message_container).await;
                for _ in 0..20 {
                    sleep(Duration::from_millis(100)).await;
                    if ws.read().is_some()
                        && ws
                            .read()
                            .as_ref()
                            .map(|s| s.ready_state() == 1)
                            .unwrap_or(false)
                    {
                        break;
                    }
                }
            }

            let session_id = match state.read().current_session.as_ref() {
                Some(s) => s.id.clone(),
                None => {
                    let new_session = match create_chat_session(
                        "新对话",
                        &provider.id,
                        &provider.name,
                        &provider.model,
                    )
                    .await
                    {
                        Ok(s) => s,
                        Err(e) => {
                            state.write().error = Some(format!("创建会话失败: {}", e));
                            state.write().is_typing = false;
                            return;
                        }
                    };
                    state.write().current_session = Some(new_session.clone());
                    state.write().sessions.insert(0, new_session.clone());
                    new_session.id
                }
            };

            if let Some(socket) = ws.read().as_ref() {
                if socket.ready_state() != 1 {
                    state.write().error = Some("WebSocket连接未建立，请重试".to_string());
                    state.write().is_typing = false;
                    scroll_to_bottom(&message_container);
                    return;
                }
                let req = json!({
                    "provider_id": provider.id,
                    "message": message,
                    "session_id": session_id,
                });
                if let Err(e) = socket.send_with_str(&req.to_string()) {
                    state.write().error = Some(format!(
                        "发送消息失败: {}",
                        e.as_string().unwrap_or_default()
                    ));
                    state.write().is_typing = false;
                }
            } else {
                state.write().error = Some("WebSocket连接未建立".to_string());
                state.write().is_typing = false;
            }
            scroll_to_bottom(&message_container);
        }
    };

    let handle_keydown = move |ev: Event<KeyboardEvent>| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            let msg = state.read().current_message.clone();
            spawn(async move {
                send_message(msg).await;
            });
        }
    };

    let select_provider = move |provider: AiProvider| {
        state.write().selected_provider = Some(provider);
    };

    let select_session = move |session: ChatSession| {
        let state = state.clone();
        let message_container = message_container.clone();
        spawn(async move {
            let mut state_mut = state.write();
            state_mut.current_thinking = String::new();
            drop(state_mut);

            match get_chat_session(&session.id).await {
                Ok(full_session) => {
                    let messages: Vec<ChatMessage> = full_session
                        .messages
                        .iter()
                        .filter_map(|msg| match msg.get("role").and_then(|r| r.as_str()) {
                            Some("user") => msg
                                .get("content")
                                .and_then(|c| c.as_str())
                                .map(|c| ChatMessage::User {
                                    content: c.to_string(),
                                }),
                            Some("assistant") => {
                                let content = msg
                                    .get("content")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let thinking = msg
                                    .get("reasoning")
                                    .and_then(|r| r.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                Some(ChatMessage::Assistant {
                                    content,
                                    thinking,
                                    tool_results: Vec::new(),
                                })
                            }
                            Some("tool") => {
                                let name = msg
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("tool")
                                    .to_string();
                                msg.get("content")
                                    .and_then(|c| c.as_str())
                                    .map(|result| ChatMessage::Tool {
                                        name,
                                        args: String::new(),
                                        result: result.to_string(),
                                    })
                            }
                            Some("system") => msg
                                .get("content")
                                .and_then(|c| c.as_str())
                                .map(|c| ChatMessage::System {
                                    content: c.to_string(),
                                }),
                            _ => None,
                        })
                        .collect();
                    state.write().current_session = Some(full_session.clone());
                    state.write().messages = messages;
                    scroll_to_bottom(&message_container);
                }
                Err(e) => {
                    state.write().error = Some(format!("加载会话失败: {}", e));
                    state.write().current_session = Some(session);
                    state.write().messages = session
                        .messages
                        .iter()
                        .filter_map(|msg| match msg.get("role").and_then(|r| r.as_str()) {
                            Some("user") => msg
                                .get("content")
                                .and_then(|c| c.as_str())
                                .map(|c| ChatMessage::User {
                                    content: c.to_string(),
                                }),
                            Some("assistant") => {
                                let content = msg
                                    .get("content")
                                    .and_then(|c| c.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let thinking = msg
                                    .get("reasoning")
                                    .and_then(|r| r.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                Some(ChatMessage::Assistant {
                                    content,
                                    thinking,
                                    tool_results: Vec::new(),
                                })
                            }
                            _ => None,
                        })
                        .collect();
                }
            }
        });
    };

    let new_session = move || {
        let mut state_mut = state.write();
        state_mut.current_session = None;
        state_mut.messages = Vec::new();
        state_mut.current_thinking = String::new();
        state_mut.error = None;
    };

    let delete_session = move |session_id: String| {
        let state = state.clone();
        async move {
            let _ = delete_chat_session(&session_id).await;
            let mut state_mut = state.write();
            state_mut.sessions.retain(|s| s.id != session_id);
            if let Some(s) = &state_mut.current_session {
                if s.id == session_id {
                    state_mut.current_session = None;
                    state_mut.messages = Vec::new();
                    state_mut.current_thinking = String::new();
                }
            }
        }
    };

    let handle_screenshot_analyze = move || {
        let state = state.clone();
        let message_container = message_container.clone();
        async move {
            if state.read().selected_provider.is_none() {
                state.write().error = Some("请先选择AI提供商".to_string());
                return;
            }

            state.write().is_typing = true;
            state.write().current_thinking = String::new();
            state.write().messages.push(ChatMessage::User {
                content: "截图分析".to_string(),
            });

            match screenshot_analyze(None).await {
                Ok(result) => {
                    state.write().messages.push(ChatMessage::Assistant {
                        content: result,
                        thinking: String::new(),
                        tool_results: Vec::new(),
                    });
                }
                Err(e) => {
                    state.write().error = Some(format!("截图分析失败: {}", e));
                }
            }
            state.write().is_typing = false;
            scroll_to_bottom(&message_container);
        }
    };

    let provider_label = state
        .read()
        .selected_provider
        .as_ref()
        .map_or("未选择".to_string(), |p| p.name.clone());

    rsx! {
        div { class: "flex flex-col h-full",
            // Header
            div { class: "flex items-center justify-between px-4 py-3 border-b border-[var(--ds-border)]",
                div { class: "flex items-center gap-3",
                    span { class: "text-sm font-semibold text-[var(--ds-text)]", "AI 助手" }
                    span { class: "px-2 py-0.5 rounded-full bg-[var(--ds-surface)] text-[10px] text-[var(--ds-text-tertiary)]",
                        "{provider_label}"
                    }
                }
                div { class: "flex items-center gap-2",
                    EqButton {
                        variant: EqButtonVariant::Ghost,
                        size: EqButtonSize::Sm,
                        onclick: move |_| spawn(async move { handle_screenshot_analyze().await; }),
                        "截图分析"
                    }
                    EqButton {
                        variant: EqButtonVariant::Ghost,
                        size: EqButtonSize::Sm,
                        onclick: move |_| new_session(),
                        "新对话"
                    }
                    EqButton {
                        variant: EqButtonVariant::Ghost,
                        size: EqButtonSize::Sm,
                        "管理"
                    }
                }
            }

            // Sidebar + Chat area
            div { class: "flex flex-1 overflow-hidden",
                // Sidebar
                div { class: "w-60 border-r border-[var(--ds-border)] overflow-y-auto flex flex-col",
                    div { class: "p-3 border-b border-[var(--ds-border)]",
                        div { class: "text-xs font-semibold text-[var(--ds-text-tertiary)] mb-2", "对话列表" }
                        if state.read().loading_sessions {
                            div { class: "space-y-2",
                                for _ in 0..3 {
                                    div { class: "h-12 bg-[var(--ds-surface)] rounded-md animate-pulse" }
                                }
                            }
                        } else if state.read().sessions.is_empty() {
                            div { class: "text-xs text-[var(--ds-text-tertiary)] text-center py-6", "暂无对话" }
                        } else {
                            div { class: "space-y-1 max-h-[200px] overflow-y-auto",
                                for session in &state.read().sessions {
                                    if !session.archived {
                                        div {
                                            class: "flex items-center justify-between px-3 py-2.5 rounded-md cursor-pointer transition-all",
                                            class: if state.read().current_session.as_ref().map(|s| s.id == session.id).unwrap_or(false) {
                                                "bg-[var(--ds-blue-light)] border border-[var(--ds-blue)]"
                                            } else {
                                                "hover:bg-[var(--ds-surface)] border border-transparent"
                                            },
                                            onclick: move |_| select_session(session.clone()),
                                            div { class: "flex-1 min-w-0 mr-2",
                                                div { class: "text-sm text-[var(--ds-text)] truncate", "{session.title}" }
                                                div { class: "text-[10px] text-[var(--ds-text-tertiary)] mt-0.5", "{session.model}" }
                                            }
                                            button {
                                                class: "p-1 rounded opacity-0 hover:opacity-100 transition-opacity",
                                                class: if state.read().current_session.as_ref().map(|s| s.id == session.id).unwrap_or(false) {
                                                    "hover:bg-blue-100"
                                                } else {
                                                    "hover:bg-[var(--ds-border)]"
                                                },
                                                onclick: move |ev| {
                                                    ev.stop_propagation();
                                                    spawn(async move { delete_session(session.id.clone()).await; });
                                                },
                                                svg { class: "w-3.5 h-3.5 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex-1 p-3",
                        div { class: "text-xs font-semibold text-[var(--ds-text-tertiary)] mb-2", "AI 提供商" }
                        if state.read().loading_providers {
                            div { class: "space-y-2",
                                for _ in 0..2 {
                                    div { class: "h-10 bg-[var(--ds-surface)] rounded-md animate-pulse" }
                                }
                            }
                        } else if state.read().providers.is_empty() {
                            div { class: "text-xs text-[var(--ds-text-tertiary)] text-center py-4", "暂无提供商" }
                        } else {
                            div { class: "space-y-1",
                                for provider in &state.read().providers {
                                    div {
                                        class: "flex items-center justify-between px-3 py-2 rounded-md cursor-pointer transition-all",
                                        class: if state.read().selected_provider.as_ref().map(|p| p.id == provider.id).unwrap_or(false) {
                                            "bg-[var(--ds-blue-light)] border border-[var(--ds-blue)]"
                                        } else {
                                            "hover:bg-[var(--ds-surface)] border border-transparent"
                                        },
                                        onclick: move |_| select_provider(provider.clone()),
                                        div { class: "flex-1 min-w-0",
                                            div { class: "text-sm text-[var(--ds-text)] truncate", "{provider.name}" }
                                            div { class: "text-[10px] text-[var(--ds-text-tertiary)] mt-0.5", "{provider.model}" }
                                        }
                                        if provider.enabled {
                                            div { class: "w-1.5 h-1.5 rounded-full bg-green-500" }
                                        } else {
                                            div { class: "w-1.5 h-1.5 rounded-full bg-gray-400" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Main chat area
                div { class: "flex-1 flex flex-col overflow-hidden",
                    if state.read().messages.is_empty() {
                        div { class: "flex-1 flex flex-col items-center justify-center p-8 text-center",
                            div { class: "w-14 h-14 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center mb-4 shadow-lg",
                                svg { class: "w-7 h-7 text-white", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" }
                                }
                            }
                            h3 { class: "text-base font-semibold text-[var(--ds-text)] mb-2", "欢迎使用 AI 助手" }
                            p { class: "text-xs text-[var(--ds-text-tertiary)] max-w-sm",
                                "选择一个AI提供商，然后输入消息控制设备。支持截图分析、设备控制等功能。"
                            }
                            div { class: "grid grid-cols-2 gap-2 mt-6 max-w-sm",
                                QuickPromptCard { label: "查看设备状态", on_click: move |_| spawn(async move { send_message("查看设备状态".to_string()).await; }) }
                                QuickPromptCard { label: "截图分析", on_click: move |_| spawn(async move { handle_screenshot_analyze().await; }) }
                                QuickPromptCard { label: "打开设置", on_click: move |_| spawn(async move { send_message("打开设置".to_string()).await; }) }
                                QuickPromptCard { label: "列出应用", on_click: move |_| spawn(async move { send_message("列出应用".to_string()).await; }) }
                            }
                        }
                    } else {
                        div {
                            class: "flex-1 overflow-y-auto p-4 space-y-4",
                            onmounted: move |md| {
                                *message_container.write() = Some(md);
                                scroll_to_bottom(&message_container);
                            },
                            if let Some(error) = &state.read().error {
                                div { class: "flex items-center gap-2 p-3 rounded-lg bg-red-50 border border-red-200 text-red-700 text-xs animate-in fade-in slide-in-from-top-2",
                                    svg { class: "w-4 h-4 flex-shrink-0", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                                    }
                                    "{error}"
                                    button {
                                        class: "ml-auto p-0.5 hover:bg-red-100 rounded",
                                        onclick: move |_| state.write().error = None,
                                        svg { class: "w-3 h-3", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                                        }
                                    }
                                }
                            }
                            if !state.read().ws_connected && state.read().messages.is_empty() {
                                div { class: "flex flex-col items-center justify-center py-8",
                                    div { class: "w-12 h-12 rounded-full bg-gray-100 flex items-center justify-center mb-3",
                                        svg { class: "w-6 h-6 text-gray-400", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" }
                                        }
                                    }
                                    p { class: "text-sm text-[var(--ds-text-tertiary)] mb-3", "WebSocket连接已断开" }
                                    EqButton {
                                        variant: EqButtonVariant::Primary,
                                        size: EqButtonSize::Sm,
                                        onclick: move |_| {
                                            let state = state.clone();
                                            let ws = ws.clone();
                                            let message_container = message_container.clone();
                                            state.write().ws_connected = false;
                                            state.write().reconnect_attempts = 0;
                                            ws.write().take();
                                            spawn(async move {
                                                connect_ws(state, ws, message_container).await;
                                            });
                                        },
                                        "重新连接"
                                    }
                                }
                            }
                            for (idx, msg) in state.read().messages.iter().enumerate() {
                                match msg {
                                    ChatMessage::User { content } => {
                                        div { class: "flex justify-end", key: "{idx}",
                                            div { class: "max-w-[75%] px-4 py-2.5 rounded-2xl rounded-tr-sm bg-[var(--ds-blue)] text-white text-sm shadow-sm", "{content}" }
                                        }
                                    }
                                    ChatMessage::Assistant { content, thinking, .. } => {
                                        div { class: "flex justify-start", key: "{idx}",
                                            div { class: "max-w-[75%] space-y-2",
                                                if !thinking.is_empty() {
                                                    div { class: "px-3 py-2 rounded-xl bg-amber-50 border border-amber-200 text-amber-800 text-xs italic",
                                                        "思考中: {thinking}"
                                                    }
                                                }
                                                div {
                                                    class: "px-4 py-2.5 rounded-2xl rounded-tl-sm bg-[var(--ds-card)] border border-[var(--ds-border)] text-[var(--ds-text)] text-sm shadow-sm",
                                                    inner_html: "{render_markdown(content)}"
                                                }
                                            }
                                        }
                                    }
                                    ChatMessage::System { content } => {
                                        div { class: "flex justify-center", key: "{idx}",
                                            div { class: "px-3 py-1 rounded-full bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)] text-[10px]", "{content}" }
                                        }
                                    }
                                    ChatMessage::Tool { name, args, result } => {
                                        div { class: "flex justify-start", key: "{idx}",
                                            div { class: "max-w-[75%] rounded-xl border border-green-200 overflow-hidden",
                                                div { class: "px-3 py-1.5 bg-green-50 border-b border-green-200",
                                                    div { class: "flex items-center gap-2",
                                                        svg { class: "w-3 h-3 text-green-600", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" }
                                                        }
                                                        span { class: "text-xs font-medium text-green-700", "工具调用: {name}" }
                                                    }
                                                }
                                                if !args.is_empty() {
                                                    div { class: "px-3 py-1.5 text-xs text-green-600 bg-green-50/50", "参数: {args}" }
                                                }
                                                div { class: "px-3 py-2 text-xs text-green-800", "{result}" }
                                            }
                                        }
                                    }
                                }
                            }
                            if state.read().is_typing {
                                div { class: "flex justify-start",
                                    div { class: "px-4 py-3 rounded-2xl rounded-tl-sm bg-[var(--ds-card)] border border-[var(--ds-border)] shadow-sm",
                                        div { class: "flex gap-1.5",
                                            div { class: "w-1.5 h-1.5 rounded-full bg-[var(--ds-text-tertiary)] animate-bounce", style: "animation-delay: 0ms" }
                                            div { class: "w-1.5 h-1.5 rounded-full bg-[var(--ds-text-tertiary)] animate-bounce", style: "animation-delay: 150ms" }
                                            div { class: "w-1.5 h-1.5 rounded-full bg-[var(--ds-text-tertiary)] animate-bounce", style: "animation-delay: 300ms" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Input area
                    div { class: "border-t border-[var(--ds-border)] px-4 py-3 bg-[color-mix(in_srgb,var(--ds-bg)_94%,var(--ds-surface))]",
                        div { class: "flex items-end gap-2",
                            div { class: "flex-1 border border-[var(--ds-border)] rounded-xl bg-[var(--ds-card)] shadow-sm overflow-hidden",
                                textarea {
                                    class: "w-full min-h-[44px] max-h-[160px] px-4 py-3 resize-none bg-transparent text-sm text-[var(--ds-text)] outline-none placeholder:text-[var(--ds-text-tertiary)]",
                                    placeholder: "输入消息... (Shift+Enter换行)",
                                    value: state.read().current_message.clone(),
                                    oninput: move |e| state.write().current_message = e.value(),
                                    onkeydown: handle_keydown,
                                }
                            }
                            if state.read().is_typing {
                                EqButton {
                                    variant: EqButtonVariant::Danger,
                                    size: EqButtonSize::Md,
                                    onclick: move |_| {
                                        let ws = ws.clone();
                                        let state = state.clone();
                                        let message_container = message_container.clone();
                                        state.write().is_typing = false;
                                        state.write().current_thinking = String::new();
                                        ws.write().take();
                                        spawn(async move {
                                            connect_ws(state, ws, message_container).await;
                                        });
                                    },
                                    "停止"
                                }
                            } else {
                                EqButton {
                                    variant: EqButtonVariant::Primary,
                                    size: EqButtonSize::Md,
                                    disabled: state.read().current_message.trim().is_empty(),
                                    onclick: move |_| {
                                        let msg = state.read().current_message.clone();
                                        spawn(async move { send_message(msg).await; });
                                    },
                                    svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 19l9 2-9-18-9 18 9-2zm0 0v-8" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}