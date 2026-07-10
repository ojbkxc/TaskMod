use axum::{extract::Path as AxumPath, extract::ws::WebSocketUpgrade, Json, response::IntoResponse};
use futures_util::{StreamExt, SinkExt};
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::fs;

use crate::config::{AI_CONF, SCRIPTS_DIR};
use crate::data::models::{AiProvider, AiProviderRequest, AiChatRequest};
use crate::data::response::ApiResponse;
use crate::tools::{adb_tools, script_tools, task_tools, ToolRegistry};
use crate::utils::adb;

/// 全局共享 HTTP Client（连接池复用，避免每次请求重建 TLS）
lazy_static::lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .pool_max_idle_per_host(4)
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("Failed to create HTTP client");
}

/// 全局共享 ToolRegistry JSON（26 个工具只注册一次）
fn shared_tools_json() -> &'static serde_json::Value {
    lazy_static::lazy_static! {
        static ref TOOLS_JSON: serde_json::Value = {
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
            registry.register(Box::new(adb_tools::AdbUnlockTool));
            registry.register(Box::new(script_tools::ListScriptsTool));
            registry.register(Box::new(script_tools::ReadScriptTool));
            registry.register(Box::new(script_tools::WriteScriptTool));
            registry.register(Box::new(script_tools::DeleteScriptTool));
            registry.register(Box::new(script_tools::RunScriptTool));
            registry.register(Box::new(script_tools::ViewLogsTool));
            registry.register(Box::new(task_tools::ListTasksTool));
            registry.register(Box::new(task_tools::AddTaskTool));
            registry.register(Box::new(task_tools::DeleteTaskTool));
            registry.register(Box::new(task_tools::ModifyTaskTool));
            registry.register(Box::new(task_tools::ListScriptsForTaskTool));
            registry.get_tools_json()
        };
    }
    &TOOLS_JSON
}

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
        const MAX_HISTORY: usize = 100; // 对话历史上限，防止 OOM

        while let Some(msg) = read.next().await {
            if let Ok(axum::extract::ws::Message::Text(text)) = msg {
                let req: Result<AiChatRequest, _> = serde_json::from_str(&text);
                if let Ok(req) = req {
                    let provider = match get_ai_provider(&req.provider_id) {
                        Some(p) => p,
                        None => {
                            let _ = write.send(axum::extract::ws::Message::Text(
                                serde_json::to_string(&json!({
                                    "type": "error",
                                    "message": "供应商不存在"
                                })).unwrap_or_default()
                            )).await;
                            continue;
                        }
                    };

                    if conversation_messages.is_empty() {
                        let screen_size = adb::get_screen_size().await;
                        let device_model = adb::get_device_info().await;
                        let scripts_list = match fs::read_dir(SCRIPTS_DIR) {
                            Ok(dir) => {
                                let files: Vec<String> = dir
                                    .filter_map(|e| e.ok())
                                    .filter(|e| e.path().is_file())
                                    .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                                    .collect();
                                if files.is_empty() { "无".to_string() } else { files.join(", ") }
                            }
                            Err(_) => "无".to_string(),
                        };

                        conversation_messages.push(json!({
                            "role": "system",
                            "content": format!(
                                "你是TaskMod Android设备控制助手，可以通过ADB命令操作手机。\n\n\
                                设备信息:\n{}\n屏幕分辨率: {}\n\n\
                                可用脚本: {}\n脚本目录: {}\n\n\
                                你可以:\n\
                                1. 用adb_command执行任意shell命令\n\
                                2. 用adb_tap/adb_swipe操作屏幕\n\
                                3. 用adb_input_text输入文本\n\
                                4. 用adb_keyevent模拟按键\n\
                                5. 用adb_screencap截图\n\
                                6. 用adb_start_app/adb_stop_app管理应用\n\
                                7. 用adb_tts语音播报\n\
                                8. 用get_device_info/get_battery_info/get_wifi_info查看设备状态\n\
                                9. 用run_script/list_scripts/read_script/write_script管理脚本\n\
                                10. 用view_logs查看系统日志\n\
                                11. 用list_tasks查看定时任务、add_task添加任务、delete_task删除任务、modify_task修改任务\n\
                                12. 用list_available_scripts查看可用脚本列表\n\n\
                                请根据用户请求调用工具完成任务。操作前先确认意图，危险操作需提醒用户。",
                                device_model.trim(),
                                screen_size,
                                scripts_list,
                                SCRIPTS_DIR
                            )
                        }));
                    }
                    conversation_messages.push(json!({
                        "role": "user",
                        "content": req.message
                    }));

                    // 注入记忆、预设、项目上下文（首次对话时）
                    if conversation_messages.len() <= 2 {
                        let settings = crate::api::ai_hub::get_prompt_settings_sync();

                        // 注入预设
                        if !settings.active_preset_id.is_empty() {
                            let presets = crate::api::ai_hub::get_active_presets();
                            if let Some(preset) = presets.iter().find(|p| p.id == settings.active_preset_id) {
                                conversation_messages.insert(1, json!({
                                    "role": "system",
                                    "content": format!("## 预设指令\n\n{}", preset.system_prompt)
                                }));
                            }
                        }

                        // 注入项目上下文
                        let projects = crate::api::ai_hub::get_active_projects_sync();
                        if !projects.is_empty() {
                            let project_ctx: String = projects.iter()
                                .map(|p| format!("- **{}**: {}", p.name, p.instructions))
                                .collect::<Vec<_>>()
                                .join("\n");
                            conversation_messages.insert(1, json!({
                                "role": "system",
                                "content": format!("## 项目上下文\n\n{}", project_ctx)
                            }));
                        }

                        // 智能注入相关记忆
                        let memories = crate::api::ai_hub::select_memories_for_prompt(&req.message, None);
                        let mem_ctx = crate::api::ai_hub::build_memory_context(&memories);
                        if !mem_ctx.is_empty() {
                            conversation_messages.insert(1, json!({
                                "role": "system",
                                "content": mem_ctx
                            }));
                            // 记录记忆访问计数
                            for mem in &memories {
                                let mid = mem.id.clone();
                                tokio::spawn(async move {
                                    crate::api::ai_hub::record_memory_access(&mid).await;
                                });
                            }
                        }

                        // 强制回复语言
                        if !settings.force_response_language.is_empty() && settings.force_response_language != "auto" {
                            let lang = match settings.force_response_language.as_str() {
                                "zh-CN" => "请用中文回复",
                                "en" => "Please respond in English",
                                _ => "",
                            };
                            if !lang.is_empty() {
                                conversation_messages.insert(1, json!({
                                    "role": "system",
                                    "content": lang
                                }));
                            }
                        }

                        // 注入已启用的Skill
                        let skills = crate::api::ai_hub::get_enabled_skills_sync();
                        if !skills.is_empty() {
                            let skill_ctx: String = skills.iter()
                                .map(|s| format!("## Skill: {}\n{}\n\n{}", s.name, s.description, s.prompt_template))
                                .collect::<Vec<_>>()
                                .join("\n\n---\n\n");
                            conversation_messages.insert(1, json!({
                                "role": "system",
                                "content": format!("## 可用技能\n\n{}", skill_ctx)
                            }));
                        }
                    }

                    let tools = shared_tools_json().clone();
                    let mut messages = conversation_messages.clone();

                    // 对话历史截断，保留 system prompt + 最近消息
                    if messages.len() > MAX_HISTORY {
                        let system_msgs: Vec<_> = messages.iter()
                            .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
                            .cloned()
                            .collect();
                        let recent: Vec<_> = messages.iter()
                            .filter(|m| m.get("role").and_then(|r| r.as_str()) != Some("system"))
                            .cloned()
                            .collect();
                        let keep = MAX_HISTORY.saturating_sub(system_msgs.len());
                        let recent_len = recent.len();
                        let truncated: Vec<_> = system_msgs.into_iter()
                            .chain(recent.into_iter().skip(recent_len.saturating_sub(keep)))
                            .collect();
                        messages = truncated;
                    }

                    loop {
                        let api_url = format!("{}/v1/chat/completions", provider.base_url);

                        let body = json!({
                            "model": provider.model,
                            "messages": messages,
                            "tools": tools,
                            "tool_choice": "auto",
                            "stream": true
                        });

                        let response = match HTTP_CLIENT.post(&api_url)
                            .header("Authorization", format!("Bearer {}", provider.api_key))
                            .header("Content-Type", "application/json")
                            .json(&body)
                            .send()
                            .await
                        {
                            Ok(r) => r,
                            Err(e) => {
                                let _ = write.send(axum::extract::ws::Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "error",
                                        "message": format!("API请求失败: {}", e)
                                    })).unwrap_or_default()
                                )).await;
                                break;
                            }
                        };

                        if !response.status().is_success() {
                            let _ = write.send(axum::extract::ws::Message::Text(
                                serde_json::to_string(&json!({
                                    "type": "error",
                                    "message": format!("API返回错误: {}", response.status())
                                })).unwrap_or_default()
                            )).await;
                            break;
                        }

                        let mut stream = response.bytes_stream();
                        let mut full_response = String::new();
                        let mut full_thinking = String::new();
                        let mut has_tool_call = false;
                        let mut streaming_msg_sent = false;

                        let mut tool_call_fragments: HashMap<usize, serde_json::Value> = HashMap::new();
                        let mut incomplete_line = String::new();

                        while let Some(chunk) = stream.next().await {
                            match chunk {
                                Ok(bytes) => {
                                    let text = String::from_utf8_lossy(&bytes);
                                    let mut combined = std::mem::take(&mut incomplete_line);
                                    combined.push_str(&text);

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
                                                            let content = delta.get("content").and_then(|c| c.as_str()).unwrap_or("");
                                                            // 思考链捕获 (DeepSeek reasoning_content)
                                                            let reasoning = delta.get("reasoning_content")
                                                                .and_then(|c| c.as_str())
                                                                .or_else(|| delta.get("reasoning").and_then(|c| c.as_str()))
                                                                .unwrap_or("");
                                                            if !reasoning.is_empty() {
                                                                full_thinking.push_str(reasoning);
                                                                let _ = write.send(axum::extract::ws::Message::Text(
                                                                    serde_json::to_string(&json!({
                                                                        "type": "thinking",
                                                                        "content": reasoning
                                                                    })).unwrap_or_default()
                                                                )).await;
                                                            }
                                                            if !content.is_empty() {
                                                                full_response.push_str(content);
                                                                let _ = write.send(axum::extract::ws::Message::Text(
                                                                    serde_json::to_string(&json!({
                                                                        "type": "chunk",
                                                                        "content": content
                                                                    })).unwrap_or_default()
                                                                )).await;
                                                                streaming_msg_sent = true;
                                                            }
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
                                                                                        let existing_args = existing_func.get("arguments").and_then(|v| v.as_str()).unwrap_or("");
                                                                                        existing_func["arguments"] = serde_json::Value::String(format!("{}{}", existing_args, tc_args));
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
                            // Send final message only if we didn't stream
                            if !streaming_msg_sent {
                                let _ = write.send(axum::extract::ws::Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "message",
                                        "content": full_response
                                    })).unwrap_or_default()
                                )).await;
                            }

                            let images: Vec<String> = extract_images(&full_response);
                            for img_url in images {
                                let _ = write.send(axum::extract::ws::Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "image",
                                        "url": img_url
                                    })).unwrap_or_default()
                                )).await;
                            }
                        }

                        // Signal stream end if we streamed
                        if streaming_msg_sent {
                            let _ = write.send(axum::extract::ws::Message::Text(
                                serde_json::to_string(&json!({
                                    "type": "stream_end"
                                })).unwrap_or_default()
                            )).await;
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

                            let _ = write.send(axum::extract::ws::Message::Text(
                                serde_json::to_string(&json!({
                                    "type": "tool_result",
                                    "results": tool_results
                                })).unwrap_or_default()
                            )).await;
                        } else {
                            if !full_response.is_empty() || !full_thinking.is_empty() {
                                let mut msg = json!({
                                    "role": "assistant",
                                    "content": full_response
                                });
                                if !full_thinking.is_empty() {
                                    msg["reasoning"] = serde_json::Value::String(full_thinking.clone());
                                }
                                conversation_messages.push(msg);
                            }
                            // 自动为首轮对话生成标题
                            if conversation_messages.len() <= 4 {
                                let first_user = conversation_messages.iter()
                                    .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
                                    .and_then(|m| m.get("content").and_then(|c| c.as_str()))
                                    .unwrap_or("");
                                let auto_title = crate::api::ai_hub::generate_title_from_message(first_user);
                                let _ = write.send(axum::extract::ws::Message::Text(
                                    serde_json::to_string(&json!({
                                        "type": "title",
                                        "title": auto_title
                                    })).unwrap_or_default()
                                )).await;
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

#[allow(dead_code)]
pub fn get_ai_providers() -> Vec<AiProvider> {
    load_ai_providers()
}

/// 获取所有已启用的Provider列表
pub fn get_enabled_providers() -> Vec<AiProvider> {
    load_ai_providers().into_iter().filter(|p| p.enabled).collect()
}

/// 带回退的AI调用：按provider_ids顺序尝试，直到成功
/// 如果provider_ids为空，则尝试所有已启用的Provider
pub async fn call_ai_with_fallback(prompt: &str, provider_ids: Option<&[String]>) -> Result<String, String> {
    let providers = if let Some(ids) = provider_ids {
        let all = load_ai_providers();
        ids.iter()
            .filter_map(|id| all.iter().find(|p| p.id == *id && p.enabled).cloned())
            .collect::<Vec<_>>()
    } else {
        get_enabled_providers()
    };

    if providers.is_empty() {
        return Err("没有可用的AI Provider".to_string());
    }

    let mut last_error = String::new();
    for provider in &providers {
        match call_ai(provider, prompt).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!("[AI回退] Provider '{}' ({}) 失败: {}", provider.name, provider.model, e);
                last_error = format!("{}: {}", provider.name, e);
            }
        }
    }

    Err(format!("所有Provider都失败，最后一个错误: {}", last_error))
}

/// 带回退的图像生成
#[allow(dead_code)]
pub async fn call_ai_image_with_fallback(prompt: &str, size: &str, provider_ids: Option<&[String]>) -> Result<String, String> {
    let providers = if let Some(ids) = provider_ids {
        let all = load_ai_providers();
        ids.iter()
            .filter_map(|id| all.iter().find(|p| p.id == *id && p.enabled).cloned())
            .collect::<Vec<_>>()
    } else {
        get_enabled_providers()
    };

    if providers.is_empty() {
        return Err("没有可用的AI Provider".to_string());
    }

    let mut last_error = String::new();
    for provider in &providers {
        match call_ai_image(provider, prompt, size).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                tracing::warn!("[AI图像回退] Provider '{}' 失败: {}", provider.name, e);
                last_error = format!("{}: {}", provider.name, e);
            }
        }
    }

    Err(format!("所有Provider都失败，最后一个错误: {}", last_error))
}

pub async fn call_ai(provider: &AiProvider, prompt: &str) -> Result<String, String> {
    let api_url = format!("{}/v1/chat/completions", provider.base_url);
    
    let body = json!({
        "model": provider.model,
        "messages": [{
            "role": "user",
            "content": prompt
        }],
        "stream": false
    });
    
    let response = HTTP_CLIENT.post(&api_url)
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

/// 调用AI生成图像（兼容DALL-E API）
#[allow(dead_code)]
pub async fn call_ai_image(provider: &AiProvider, prompt: &str, size: &str) -> Result<String, String> {
    let api_url = format!("{}/v1/images/generations", provider.base_url);
    
    let body = json!({
        "model": provider.model,
        "prompt": prompt,
        "n": 1,
        "size": size,
        "response_format": "url"
    });
    
    let response = HTTP_CLIENT.post(&api_url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("API请求失败: {}", e))?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API返回错误: {} - {}", status, error_text));
    }
    
    let json_response: serde_json::Value = response.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    
    let url = json_response
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|d| d.get("url"))
        .and_then(|u| u.as_str())
        .ok_or("无法提取图像URL".to_string())?;
    
    Ok(url.to_string())
}

/// 调用AI生成嵌入向量
#[allow(dead_code)]
pub async fn call_ai_embedding(provider: &AiProvider, input: &str) -> Result<Vec<f64>, String> {
    let api_url = format!("{}/v1/embeddings", provider.base_url);
    
    let body = json!({
        "model": provider.model,
        "input": input
    });
    
    let response = HTTP_CLIENT.post(&api_url)
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
    
    let embedding = json_response
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|d| d.get("embedding"))
        .and_then(|e| e.as_array())
        .ok_or("无法提取嵌入向量".to_string())?;
    
    let result: Vec<f64> = embedding.iter()
        .filter_map(|v| v.as_f64())
        .collect();
    
    if result.is_empty() {
        return Err("嵌入向量为空".to_string());
    }
    
    Ok(result)
}

fn save_ai_providers(providers: &[AiProvider]) -> Result<(), std::io::Error> {
    let content = serde_json::to_string_pretty(providers).unwrap_or_default();
    if let Some(parent) = std::path::Path::new(AI_CONF).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(AI_CONF, content)
}

/// 调用支持视觉的AI分析截图
pub async fn call_ai_image_analyze(provider: &AiProvider, prompt: &str, img_base64: &str) -> Result<String, String> {
    let api_url = format!("{}/chat/completions", provider.base_url.trim_end_matches('/'));
    let body = json!({
        "model": provider.model,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": prompt},
                {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{}", img_base64)}}
            ]
        }],
        "max_tokens": 2000
    });

    let resp = HTTP_CLIENT.post(&api_url)
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("API请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("API返回错误: {}", resp.status()));
    }

    let json_resp: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    json_resp.get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "无法提取AI回复".to_string())
}

pub fn get_ai_provider(id: &str) -> Option<AiProvider> {
    load_ai_providers().into_iter().find(|p| p.id == id)
}

fn extract_images(text: &str) -> Vec<String> {
    let mut images = Vec::new();

    let re = match regex::Regex::new(r"!\[.*?\]\((https?://[^\)]+)\)") {
        Ok(r) => r,
        Err(_) => return images,
    };
    for cap in re.captures_iter(text) {
        if let Some(url) = cap.get(1) {
            images.push(url.as_str().to_string());
        }
    }

    let re_base64 = match regex::Regex::new(r#"data:image/[a-zA-Z]+;base64,[^\s"'"]+"#) {
        Ok(r) => r,
        Err(_) => return images,
    };
    for cap in re_base64.find_iter(text) {
        images.push(cap.as_str().to_string());
    }

    images
}