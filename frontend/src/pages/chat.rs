use dioxus::prelude::*;
use eq_ui::prelude::*;
use serde_json::json;
use crate::api::client::{get_ai_providers, list_chat_sessions, delete_chat_session, screenshot_analyze, AiProvider, ChatSession};

#[derive(Debug, Clone, PartialEq)]
enum ChatMessage {
    User { content: String },
    Assistant { content: String, thinking: String, tool_results: Vec<ToolResult> },
    System { content: String },
    Tool { name: String, args: String, result: String },
}

#[derive(Debug, Clone, PartialEq)]
struct ToolResult {
    tool_name: String,
    result: String,
}

#[derive(Debug, Clone, PartialEq)]
struct ChatState {
    messages: Vec<ChatMessage>,
    current_message: String,
    selected_provider: Option<AiProvider>,
    providers: Vec<AiProvider>,
    sessions: Vec<ChatSession>,
    current_session: Option<ChatSession>,
    is_typing: bool,
    current_thinking: String,
    error: Option<String>,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            current_message: String::new(),
            selected_provider: None,
            providers: Vec::new(),
            sessions: Vec::new(),
            current_session: None,
            is_typing: false,
            current_thinking: String::new(),
            error: None,
        }
    }
}

#[component]
pub fn ChatPage() -> Element {
    let state = use_signal(ChatState::default);
    let ws = use_signal(|| Option::<dioxus_websocket::WebSocket>::None);
    let container_ref = use_ref(|| None::<std::rc::Rc<dyn dioxus_core::ElementRef>>);

    use_effect(move || {
        let state = state.clone();
        async move {
            match get_ai_providers().await {
                Ok(providers) => {
                    state.write().providers = providers;
                    if let Some(p) = state.read().providers.iter().find(|p| p.enabled).cloned() {
                        state.write().selected_provider = Some(p);
                    }
                }
                Err(e) => {
                    state.write().error = Some(format!("加载提供商失败: {}", e));
                }
            }
            match list_chat_sessions().await {
                Ok(sessions) => {
                    state.write().sessions = sessions;
                }
                Err(e) => {
                    state.write().error = Some(format!("加载会话失败: {}", e));
                }
            }
        }
    });

    let scroll_to_bottom = move || {
        if let Some(container) = container_ref.read().as_ref() {
            let element = container.get_element().unwrap();
            let height = element.scroll_height();
            element.set_scroll_top(height);
        }
    };

    let connect_ws = move || {
        let state = state.clone();
        let ws = ws.clone();
        async move {
            if ws.read().is_some() {
                return;
            }
            let ws_url = format!("ws://{}/ws/ai-chat", window().location().host().unwrap_or_default());
            let socket = dioxus_websocket::WebSocket::new(&ws_url, {
                let state = state.clone();
                move |msg| {
                    let mut state = state.write();
                    match msg {
                        dioxus_websocket::WsMessage::Text(text) => {
                            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                                match data.get("type").and_then(|t| t.as_str()) {
                                    Some("chunk") => {
                                        if let Some(content) = data.get("content").and_then(|c| c.as_str()) {
                                            if let Some(ChatMessage::Assistant { content: ref mut c, .. }) = state.messages.last_mut() {
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
                                            if let Some(ChatMessage::Assistant { thinking: ref mut t, .. }) = state.messages.last_mut() {
                                                t.push_str(content);
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
                                                        if let Some(tcid) = result.get("tool_call_id").and_then(|id| id.as_str()) {
                                                            let tool_name = extract_tool_name_from_id(tcid);
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
                                            if let Some(ChatMessage::Assistant { content: ref mut c, .. }) = state.messages.last_mut() {
                                                c.push_str(&format!("\n\n![image]({})", url));
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        dioxus_websocket::WsMessage::Binary(_) => {}
                        dioxus_websocket::WsMessage::Close(_) => {
                            state.is_typing = false;
                            ws.write().take();
                        }
                        dioxus_websocket::WsMessage::Error(_) => {
                            state.is_typing = false;
                        }
                        dioxus_websocket::WsMessage::Open => {}
                    }
                    scroll_to_bottom();
                }
            }).unwrap();
            ws.write().replace(socket);
        }
    };

    let send_message = move |message: String| {
        let state = state.clone();
        let ws = ws.clone();
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

            state.write().messages.push(ChatMessage::User { content: message.clone() });
            state.write().current_message = String::new();
            state.write().is_typing = true;
            state.write().error = None;

            if ws.read().is_none() {
                connect_ws().await;
            }

            if let Some(socket) = ws.read().as_ref() {
                let req = json!({
                    "provider_id": provider.id,
                    "message": message,
                });
                let _ = socket.send_text(req.to_string());
            }
            scroll_to_bottom();
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
        state.write().current_session = Some(session.clone());
        state.write().messages = session.messages.iter().filter_map(|msg| {
            match msg.get("role").and_then(|r| r.as_str()) {
                Some("user") => msg.get("content").and_then(|c| c.as_str()).map(|c| ChatMessage::User { content: c.to_string() }),
                Some("assistant") => {
                    let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string();
                    let thinking = msg.get("reasoning").and_then(|r| r.as_str()).unwrap_or("").to_string();
                    Some(ChatMessage::Assistant { content, thinking, tool_results: Vec::new() })
                }
                Some("tool") => msg.get("content").and_then(|c| c.as_str()).map(|result| ChatMessage::Tool {
                    name: "tool".to_string(),
                    args: String::new(),
                    result: result.to_string(),
                }),
                Some("system") => msg.get("content").and_then(|c| c.as_str()).map(|c| ChatMessage::System { content: c.to_string() }),
                _ => None,
            }
        }).collect();
        scroll_to_bottom();
    };

    let new_session = move || {
        state.write().current_session = None;
        state.write().messages = Vec::new();
    };

    let delete_session = move |session_id: String| {
        let state = state.clone();
        async move {
            let _ = delete_chat_session(&session_id).await;
            state.write().sessions.retain(|s| s.id != session_id);
            if let Some(s) = &state.read().current_session {
                if s.id == session_id {
                    state.write().current_session = None;
                    state.write().messages = Vec::new();
                }
            }
        }
    };

    let handle_screenshot_analyze = move || {
        let state = state.clone();
        async move {
            state.write().is_typing = true;
            state.write().messages.push(ChatMessage::User { content: "截图分析".to_string() });
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
            scroll_to_bottom();
        }
    };

    let provider_label = state.read().selected_provider.as_ref().map_or("未选择".to_string(), |p| p.name.clone());

    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex items-center justify-between p-3 border-b border-[var(--ds-border)]",
                div { class: "flex items-center gap-2",
                    span { class: "text-sm font-semibold text-[var(--ds-text)]", "AI 助手" }
                    span { class: "px-1.5 py-0.5 rounded-full bg-[var(--ds-surface)] text-[10px] text-[var(--ds-text-tertiary)]",
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

            div { class: "flex flex-1 overflow-hidden",
                div { class: "w-64 border-r border-[var(--ds-border)] overflow-y-auto",
                    div { class: "p-2",
                        div { class: "text-xs font-semibold text-[var(--ds-text-tertiary)] mb-2 px-1", "对话列表" }
                        if state.read().sessions.is_empty() {
                            div { class: "text-xs text-[var(--ds-text-tertiary)] text-center py-4", "暂无对话" }
                        } else {
                            for session in &state.read().sessions {
                                if !session.archived {
                                    div {
                                        class: "flex items-center justify-between p-2 rounded-md cursor-pointer mb-1 transition-colors",
                                        class: if state.read().current_session.as_ref().map(|s| s.id == session.id).unwrap_or(false) {
                                            "bg-[var(--ds-blue-light)] border border-[var(--ds-blue)]"
                                        } else {
                                            "hover:bg-[var(--ds-surface)] border border-transparent"
                                        },
                                        onclick: move |_| select_session(session.clone()),
                                        div { class: "flex-1 min-w-0",
                                            div { class: "text-sm text-[var(--ds-text)] truncate", "{session.title}" }
                                            div { class: "text-[10px] text-[var(--ds-text-tertiary)]", "{session.model}" }
                                        }
                                        button {
                                            class: "p-1 hover:bg-[var(--ds-border)] rounded",
                                            onclick: move |ev| {
                                                ev.stop_propagation();
                                                spawn(async move { delete_session(session.id.clone()).await; });
                                            },
                                            svg { class: "w-3 h-3 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "p-2 border-t border-[var(--ds-border)]",
                        div { class: "text-xs font-semibold text-[var(--ds-text-tertiary)] mb-2 px-1", "AI 提供商" }
                        if state.read().providers.is_empty() {
                            div { class: "text-xs text-[var(--ds-text-tertiary)] text-center py-2", "暂无提供商" }
                        } else {
                            for provider in &state.read().providers {
                                div {
                                    class: "p-2 rounded-md cursor-pointer mb-1 transition-colors",
                                    class: if state.read().selected_provider.as_ref().map(|p| p.id == provider.id).unwrap_or(false) {
                                        "bg-[var(--ds-blue-light)] border border-[var(--ds-blue)]"
                                    } else {
                                        "hover:bg-[var(--ds-surface)] border border-transparent"
                                    },
                                    onclick: move |_| select_provider(provider.clone()),
                                    div { class: "flex items-center justify-between",
                                        div { class: "flex-1 min-w-0",
                                            div { class: "text-sm text-[var(--ds-text)] truncate", "{provider.name}" }
                                            div { class: "text-[10px] text-[var(--ds-text-tertiary)]", "{provider.model}" }
                                        }
                                        if provider.enabled {
                                            div { class: "w-2 h-2 rounded-full bg-green-500" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    class: "flex-1 flex flex-col overflow-hidden",
                    if state.read().messages.is_empty() {
                        div { class: "flex-1 flex flex-col items-center justify-center p-8 text-center",
                            div { class: "w-16 h-16 rounded-full bg-[var(--ds-surface)] flex items-center justify-center mb-4",
                                svg { class: "w-8 h-8 text-[var(--ds-text-secondary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" }
                                }
                            }
                            h3 { class: "text-lg font-semibold text-[var(--ds-text)] mb-2", "欢迎使用 AI 助手" }
                            p { class: "text-sm text-[var(--ds-text-tertiary)] max-w-md",
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
                        div { ref: container_ref, class: "flex-1 overflow-y-auto p-4 space-y-4",
                            if let Some(error) = &state.read().error {
                                div { class: "p-3 rounded-lg bg-red-50 border border-red-200 text-red-700 text-sm", "{error}" }
                            }
                            for (idx, msg) in state.read().messages.iter().enumerate() {
                                match msg {
                                    ChatMessage::User { content } => {
                                        div { class: "flex justify-end", key: "{idx}",
                                            div { class: "max-w-[80%] p-3 rounded-lg bg-[var(--ds-blue)] text-white text-sm", "{content}" }
                                        }
                                    }
                                    ChatMessage::Assistant { content, thinking, .. } => {
                                        div { class: "flex justify-start", key: "{idx}",
                                            div { class: "max-w-[80%] space-y-2",
                                                if !thinking.is_empty() {
                                                    div { class: "p-3 rounded-lg bg-amber-50 border border-amber-200 text-amber-800 text-sm italic",
                                                        "思考中: {thinking}"
                                                    }
                                                }
                                                div { class: "p-3 rounded-lg bg-[var(--ds-card)] border border-[var(--ds-border)] text-[var(--ds-text)] text-sm whitespace-pre-wrap",
                                                    "{content}"
                                                }
                                            }
                                        }
                                    }
                                    ChatMessage::System { content } => {
                                        div { class: "flex justify-center", key: "{idx}",
                                            div { class: "px-3 py-1 rounded-full bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)] text-xs", "{content}" }
                                        }
                                    }
                                    ChatMessage::Tool { name, args, result } => {
                                        div { class: "flex justify-start", key: "{idx}",
                                            div { class: "max-w-[80%] p-3 rounded-lg bg-green-50 border border-green-200",
                                                div { class: "text-xs font-semibold text-green-700 mb-1", "工具调用: {name}" }
                                                if !args.is_empty() {
                                                    div { class: "text-xs text-green-600 mb-1", "参数: {args}" }
                                                }
                                                div { class: "text-sm text-green-800", "{result}" }
                                            }
                                        }
                                    }
                                }
                            }
                            if state.read().is_typing {
                                div { class: "flex justify-start",
                                    div { class: "p-3 rounded-lg bg-[var(--ds-card)] border border-[var(--ds-border)]",
                                        div { class: "flex gap-1",
                                            div { class: "w-2 h-2 rounded-full bg-[var(--ds-text-tertiary)] animate-bounce", style: "animation-delay: 0ms" }
                                            div { class: "w-2 h-2 rounded-full bg-[var(--ds-text-tertiary)] animate-bounce", style: "animation-delay: 150ms" }
                                            div { class: "w-2 h-2 rounded-full bg-[var(--ds-text-tertiary)] animate-bounce", style: "animation-delay: 300ms" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "border-t border-[var(--ds-border)] p-3 bg-[color-mix(in_srgb,var(--ds-bg)_94%,var(--ds-surface))]",
                        div { class: "border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] shadow-sm p-2",
                            textarea {
                                class: "w-full min-h-[42px] max-h-[150px] resize-none bg-transparent text-sm text-[var(--ds-text)] outline-none",
                                placeholder: "输入消息...",
                                value: state.read().current_message.clone(),
                                oninput: move |e| state.write().current_message = e.value(),
                                onkeydown: handle_keydown,
                            }
                            div { class: "flex items-center justify-between pt-1",
                                span { class: "text-[10px] text-[var(--ds-text-tertiary)] px-2 py-0.5 rounded-full bg-[var(--ds-surface)]",
                                    "--"
                                }
                                EqButton {
                                    variant: EqButtonVariant::Primary,
                                    size: EqButtonSize::Sm,
                                    disabled: state.read().is_typing || state.read().current_message.trim().is_empty(),
                                    onclick: move |_| {
                                        let msg = state.read().current_message.clone();
                                        spawn(async move { send_message(msg).await; });
                                    },
                                    "发送"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct QuickPromptCardProps {
    label: &'static str,
    on_click: EventHandler<()>,
}

#[component]
fn QuickPromptCard(props: QuickPromptCardProps) -> Element {
    rsx! {
        button {
            class: "flex items-center gap-2 p-2.5 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-[var(--ds-text-secondary)] cursor-pointer text-xs font-medium transition-all hover:border-[var(--ds-blue)] hover:bg-[var(--ds-blue-light)] hover:text-[var(--ds-blue)]",
            onclick: props.on_click,
            "{props.label}"
        }
    }
}

fn extract_tool_name_from_id(_id: &str) -> String {
    "tool".to_string()
}