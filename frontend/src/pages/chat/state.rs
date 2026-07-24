use crate::api::client::{AiProvider, ChatSession};

#[derive(Debug, Clone, PartialEq)]
pub enum ChatMessage {
    User { content: String },
    Assistant { content: String, thinking: String, tool_results: Vec<ToolResult> },
    System { content: String },
    Tool { name: String, args: String, result: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolResult {
    pub tool_name: String,
    pub result: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub current_message: String,
    pub selected_provider: Option<AiProvider>,
    pub providers: Vec<AiProvider>,
    pub sessions: Vec<ChatSession>,
    pub current_session: Option<ChatSession>,
    pub is_typing: bool,
    pub current_thinking: String,
    pub error: Option<String>,
    pub ws_connected: bool,
    pub loading_providers: bool,
    pub loading_sessions: bool,
    pub reconnect_attempts: usize,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            current_message: String::new(),
            selected_provider: None,
            providers: Vec::new(),
            sessions: Vec::new(),
            current_session: Option::None,
            is_typing: false,
            current_thinking: String::new(),
            error: None,
            ws_connected: false,
            loading_providers: true,
            loading_sessions: true,
            reconnect_attempts: 0,
        }
    }
}