use axum::{extract::Path as AxumPath, extract::ws::WebSocketUpgrade, Json, response::IntoResponse};
use futures_util::{StreamExt, SinkExt};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::fs;

use crate::config::{AI_CONF, SCRIPTS_DIR, LOG_FILE};
use crate::data::models::{AiProvider, AiProviderRequest, AiChatRequest};
use crate::data::response::ApiResponse;
use crate::tools::{adb_tools, script_tools, ToolRegistry};
use crate::utils::adb;

pub async fn list_ai_providers() -> Json<ApiResponse<Vec<AiProvider>>> {
    let providers = load_ai_providers();
    Json(ApiResponse::ok(providers))
}

pub async fn get_ai_provider_api(AxumPath(id): AxumPath<String>) -> Json<ApiResponse<AiProvider>> {
    if let Some(provider) = get_ai_provider(&id) {
        Json(ApiResponse::ok(provider))
    } else {
        Json(ApiResponse::err("供应商不存在"))
    }
}

pub async fn add_ai_provider(Json(req): Json<AiProviderRequest>) -> Json<ApiResponse<String>> {
    let mut providers = load_ai_providers();
    let id = format!("{}", providers.len() + 1);
    providers.push(AiProvider {
        id,
        name: req.name,
        base_url: req.base_url,
        api_key: req.api_key,
        model: req.model,
        enabled: req.enabled,
    });
    match save_ai_providers(&providers) {
        Ok(_) => Json(ApiResponse::ok("添加成功".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_ai_provider(
    AxumPath(id): AxumPath<String>,
    Json(req): Json<AiProviderRequest>,
) -> Json<ApiResponse<String>> {
    let mut providers = load_ai_providers();
    if let Some(provider) = providers.iter_mut().find(|p| p.id == id) {
        provider.name = req.name;
        provider.base_url = req.base_url;
        provider.api_key = req.api_key;
        provider.model = req.model;
        provider.enabled = req.enabled;
        match save_ai_providers(&providers) {
            Ok(_) => Json(ApiResponse::ok("更新成功".to_string())),
            Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
        }
    } else {
        Json(ApiResponse::err("供应商不存在"))
    }
}

pub async fn delete_ai_provider(AxumPath(id): AxumPath<String>) -> Json<ApiResponse<String>> {
    let mut providers = load_ai_providers();
    let original_len = providers.len();
    providers.retain(|p| p.id != id);
    if providers.len() == original_len {
        return Json(ApiResponse::err("供应商不存在"));
    }
    match save_ai_providers(&providers) {
        Ok(_) => Json(ApiResponse::ok("删除成功".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}

pub async fn ai_chat_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let (mut write, mut read) = socket.split();
        let mut conversation_messages: Vec<serde_json::Value> = Vec::new();

        while let Some(msg) = read.next().await {
            if let Ok(tokio_tungstenite::tungstenite::protocol::Message::Text(text)) = msg {
                let req: Result<AiChatRequest, _> = serde_json::from_str(&text);
                if let Ok(req) = req {
                    let provider = get_ai_provider(&req.provider_id);
                    if provider.is_none() {
                        let _ = write.send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                            serde_json::to_string(&json!({
                                "type": "error",
                                "message": "供应商不存在"
                            })).unwrap_or_default()
                        )).await;
                        continue;
                    }
                    let provider = provider.unwrap();

                    let client = match Client::builder().build() {
                        Ok(c) => c,
                        Err(_) => {
                            let _ = write.send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                                serde_json::to_string(&json!({
                                    "type": "error",
                                    "message": "创建客户端失败"
                                })).unwrap_or_default()
                            )).await;
                            continue;
                        }
                    };

                    if conversation_messages.is_empty() {
                        let screen_size = adb::get_screen_size().await;
                        let scripts_list = match fs::read_dir(SCRIPTS_DIR) {
                            Ok(dir) => {
                                let files: Vec<String> = dir
                                    .filter_map(|e| e.ok())
                                    .filter(|e| e.path().is_file())
                                    .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                                    .collect();
                                files.join(", ")
                            }
                            Err(_) => "".to_string(),
                        };

                        conversation_messages.push(json!({
                            "role": "system",
                            "content": format!("你是一个Android设备控制助手，可以通过ADB命令操作手机。\n\n设备信息:\n- 屏幕分辨率: {}\n\n可用脚本: {}\n\n脚本目录: {}\n\n请根据用户的请求，调用适当的工具来完成任务。", 
                                screen_size, 
                                if scripts_list.is_empty() { "无" } else { &scripts_list },
                                SCRIPTS_DIR
                            )
                        }));
                    }
                    conversation_messages.push(json!({
                        "role": "user",
                        "content": req.message
                    }));

                    let mut registry = ToolRegistry::new();
                    registry.register(Box::new(adb_tools::AdbTapTool));
                    registry.register(Box::new(adb_tools::AdbSwipeTool));
                    registry.register(Box::new(adb_tools::AdbKeyeventTool));
                    registry.register(Box::new(adb_tools::AdbInputTextTool));
                    registry.register(Box::new(adb_tools::AdbScreencapTool));
                    registry.register(Box::new(adb_tools::AdbCommandTool));
                    registry.register(Box::new(adb_tools::AdbStartAppTool));
                    registry.register(Box::new(adb_tools::AdbStopAppTool));
                    registry.register(Box::new(adb_tools::AdbClearAppDataTool));
                    registry.register(Box::new(adb_tools::GetWifiInfoTool));
                    registry.register(Box::new(adb_tools::GetDeviceInfoTool));
                    registry.register(Box::new(adb_tools::GetBatteryInfoTool));
                    registry.register(Box::new(adb_tools::GetRunningAppsTool));
                    registry.register(Box::new(adb_tools::AdbRebootTool));
                    registry.register(Box::new(adb_tools::AdbShutdownTool));
                    registry.register(Box::new(adb_tools::AdbTtsTool));
                    registry.register(Box::new(script_tools::ListScriptsTool));
                    registry.register(Box::new(script_tools::ReadScriptTool));
                    registry.register(Box::new(script_tools::WriteScriptTool));
                    registry.register(Box::new(script_tools::DeleteScriptTool));
                    registry.register(Box::new(script_tools::RunScriptTool));
                    registry.register(Box::new(script_tools::ViewLogsTool));

                    let tools = registry.get_tools_json();
                    let mut messages = conversation_messages.clone();

                    loop {
                        let api_url = format!("{}/v1/chat/completions", provider.base_url);

                        let body = json!({
                            "model": provider.model,
                            "messages": messages,
                            "tools": tools,
                            "tool_choice": "auto",
                            "stream": true
                        });

                        let response = match client.post(&api_url)
                            .header("Authorization", format!("Bearer {}", provider.api_key))
                            .header("Content-Type", "application/json")
                            .json(&body)
                            .send()
                            .await
                        {
                            Ok(r) => r,
                            Err(e) => {
                                let _ = write.send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "error",
                                        "message": format!("API请求失败: {}", e)
                                    })).unwrap_or_default()
                                )).await;
                                break;
                            }
                        };

                        if !response.status().is_success() {
                            let _ = write.send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                                serde_json::to_string(&json!({
                                    "type": "error",
                                    "message": format!("API返回错误: {}", response.status())
                                })).unwrap_or_default()
                            )).await;
                            break;
                        }

                        let mut stream = response.bytes_stream();
                        let mut full_response = String::new();
                        let mut has_tool_call = false;

                        let mut tool_call_fragments: HashMap<usize, serde_json::Value> = HashMap::new();
                        let mut incomplete_line = String::new();

                        while let Some(chunk) = stream.next().await {
                            match chunk {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    let mut combined = incomplete_line + &text;
                                    incomplete_line.clear();

                                    let lines: Vec<&str> = combined.split('\n').collect();
                                    for i in 0..lines.len() {
                                        let line = lines[i].trim();
                                        if i == lines.len() - 1 && !combined.ends_with('\n') {
                                            incomplete_line = line.to_string();
                                            continue;
                                        }
                                        if line.starts_with("data: ") {
                                            let data = &line[6..];
                                            if data == "[DONE]" {
                                                break;
                                            }
                                            if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(data) {
                                                if let Some(choices) = json_data.get("choices") {
                                                    if let Some(first) = choices.as_array().and_then(|a| a.first()) {
                                                        if let Some(delta) = first.get("delta") {
                                                            full_response.push_str(delta.get("content").and_then(|c| c.as_str()).unwrap_or(""));
                                                            if let Some(tc) = delta.get("tool_calls") {
                                                                if let Some(tc_array) = tc.as_array() {
                                                                    for tc_item in tc_array {
                                                                        has_tool_call = true;
                                                                        let index = tc_item.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                                                                        if let Some(existing) = tool_call_fragments.get_mut(&index) {
                                                                            if let Some(existing_func) = existing.get_mut("function") {
                                                                                if let Some(tc_func) = tc_item.get("function") {
                                                                                    if let Some(tc_name) = tc_func.get("name").and_then(|v| v.as_str()) {
                                                                                        existing_func["name"] = serde_json::Value::String(tc_name.to_string());
                                                                                    }
                                                                                    if let Some(tc_args) = tc_func.get("arguments").and_then(|v| v.as_str()) {
                                                                                        let existing_args = existing_func.get_mut("arguments").and_then(|v| v.as_str_mut()).unwrap_or(&mut String::new());
                                                                                        existing_args.push_str(tc_args);
                                                                                    }
                                                                                }
                                                                            }
                                                                            if let Some(tc_id) = tc_item.get("id").and_then(|v| v.as_str()) {
                                                                                existing["id"] = serde_json::Value::String(tc_id.to_string());
                                                                            }
                                                                        } else {
                                                                            tool_call_fragments.insert(index, tc_item.clone());
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
                                }
                                Err(_) => break,
                            }
                        }

                        let mut tool_calls: Vec<serde_json::Value> = tool_call_fragments.into_values().collect();
                        tool_calls.sort_by_key(|tc| tc.get("index").and_then(|v| v.as_u64()).unwrap_or(0));

                        if !full_response.is_empty() {
                            let _ = write.send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                                serde_json::to_string(&json!({
                                    "type": "message",
                                    "content": full_response
                                })).unwrap_or_default()
                            )).await;

                            let images: Vec<String> = extract_images(&full_response);
                            for img_url in images {
                                let _ = write.send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "image",
                                        "url": img_url
                                    })).unwrap_or_default()
                                )).await;
                            }
                        }

                        if has_tool_call {
                            let assistant_msg = json!({
                                "role": "assistant",
                                "content": full_response,
                                "tool_calls": tool_calls
                            });
                            messages.push(assistant_msg.clone());
                            conversation_messages.push(assistant_msg);

                            let mut tool_results: Vec<serde_json::Value> = Vec::new();

                            for tc in &tool_calls {
                                if let Some(func) = tc.get("function") {
                                    let name = func.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                    let args = func.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}");

                                    let result = match registry.execute(name, args).await {
                                        Some(r) => r,
                                        None => format!("未知工具: {}", name),
                                    };

                                    tool_results.push(json!({
                                        "role": "tool",
                                        "tool_call_id": tc.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                                        "content": result
                                    }));
                                }
                            }

                            for tr in &tool_results {
                                messages.push(tr.clone());
                                conversation_messages.push(tr.clone());
                            }

                            let _ = write.send(tokio_tungstenite::tungstenite::protocol::Message::Text(
                                serde_json::to_string(&json!({
                                    "type": "tool_result",
                                    "results": tool_results
                                })).unwrap_or_default()
                            )).await;
                        } else {
                            if !full_response.is_empty() {
                                conversation_messages.push(json!({
                                    "role": "assistant",
                                    "content": full_response
                                }));
                            }
                            break;
                        }
                    }
                }
            }
        }
    })
}

pub fn load_ai_providers() -> Vec<AiProvider> {
    fs::read_to_string(AI_CONF)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn get_ai_providers() -> Vec<AiProvider> {
    load_ai_providers()
}

pub async fn call_ai(provider: &AiProvider, prompt: &str) -> Result<String, String> {
    let client = Client::builder().build().map_err(|e| format!("创建客户端失败: {}", e))?;
    
    let api_url = format!("{}/v1/chat/completions", provider.base_url);
    
    let body = json!({
        "model": provider.model,
        "messages": [{
            "role": "user",
            "content": prompt
        }],
        "stream": false
    });
    
    let response = client.post(&api_url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("API请求失败: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("API返回错误: {}", response.status()));
    }
    
    let json_response: serde_json::Value = response.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    
    let content = json_response
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or("无法提取响应内容".to_string())?;
    
    Ok(content.to_string())
}

fn save_ai_providers(providers: &[AiProvider]) -> Result<(), std::io::Error> {
    let content = serde_json::to_string_pretty(providers).unwrap_or_default();
    fs::write(AI_CONF, content)
}

pub fn get_ai_provider(id: &str) -> Option<AiProvider> {
    load_ai_providers().into_iter().find(|p| p.id == id)
}

fn extract_images(text: &str) -> Vec<String> {
    let mut images = Vec::new();

    let re = regex::Regex::new(r"!\[.*?\]\((https?://[^\)]+)\)").unwrap();
    for cap in re.captures_iter(text) {
        if let Some(url) = cap.get(1) {
            images.push(url.as_str().to_string());
        }
    }

    let re_base64 = regex::Regex::new(r#"data:image/[a-zA-Z]+;base64,[^\s"'"]+"#).unwrap();
    for cap in re_base64.find_iter(text) {
        images.push(cap.as_str().to_string());
    }

    images
}