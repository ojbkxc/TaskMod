use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API 基础 URL（相对路径，因为前端和后端在同一服务器）
const API_BASE: &str = "/api";

/// 通用 API 响应（字段名必须匹配后端: success, 不是 ok）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

/// 系统状态（匹配后端 system_status 返回格式）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemStatus {
    pub uptime: Option<String>,
    pub disk: Option<String>,
    pub tasks_count: Option<usize>,
    pub screenshots_count: Option<usize>,
    pub battery: Option<BatteryInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatteryInfo {
    pub capacity: String,
    pub temperature: String,
    pub status: String,
}

/// 任务（匹配后端 Task 模型: id, time, weeks, script, task_type, interval）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: usize,
    pub time: String,
    pub weeks: String,
    pub script: String,
    pub task_type: String,
    #[serde(default)]
    pub interval: Option<u32>,
}

/// AI 提供商
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub enabled: bool,
}

/// 邮件配置（匹配后端 EmailConfig）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmailConfig {
    #[serde(default)]
    pub enable_notify: bool,
    #[serde(default)]
    pub smtp_server: String,
    #[serde(default)]
    pub smtp_port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub timeout_secs: u64,
    #[serde(default)]
    pub max_retries: u32,
    #[serde(default)]
    pub retry_interval: u64,
}

/// MQTT 配置（匹配后端 MqttConfig）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MqttConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub broker: String,
    #[serde(default)]
    pub topic_prefix: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub client_id: String,
}

/// 获取系统状态（兼容旧接口）
pub async fn get_status() -> Result<SystemStatus, reqwest::Error> {
    let url = format!("{}/status", API_BASE);
    let resp: ApiResponse<SystemStatus> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 应用状态（任务数、截图数）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppStatus {
    pub tasks_count: usize,
    pub screenshots_count: usize,
}

/// 获取应用状态
pub async fn get_app_status() -> Result<AppStatus, reqwest::Error> {
    let url = format!("{}/app/status", API_BASE);
    let resp: ApiResponse<AppStatus> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 获取任务列表
pub async fn get_tasks() -> Result<Vec<Task>, reqwest::Error> {
    let url = format!("{}/tasks", API_BASE);
    let resp: ApiResponse<Vec<Task>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 添加任务
pub async fn add_task(time: &str, weeks: &str, script: &str, task_type: &str, interval: Option<u32>) -> Result<String, reqwest::Error> {
    let url = format!("{}/tasks", API_BASE);
    let mut body = serde_json::json!({
        "time": time,
        "weeks": weeks,
        "script": script,
        "task_type": task_type,
    });
    if let Some(iv) = interval {
        body["interval"] = serde_json::json!(iv);
    }
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 删除任务
pub async fn delete_task(id: usize) -> Result<String, reqwest::Error> {
    let url = format!("{}/tasks/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 获取脚本列表（后端返回 Vec<String> 文件名列表）
pub async fn get_scripts() -> Result<Vec<String>, reqwest::Error> {
    let url = format!("{}/scripts", API_BASE);
    let resp: ApiResponse<Vec<String>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 获取脚本内容
pub async fn get_script_content(name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/scripts/{}", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 保存脚本
pub async fn save_script(name: &str, content: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/scripts/{}", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .put(&url)
        .json(&serde_json::json!({"content": content}))
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 删除脚本
pub async fn delete_script(name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/scripts/{}", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 获取日志（后端返回 Vec<String>）
pub async fn get_logs(limit: usize) -> Result<Vec<String>, reqwest::Error> {
    let url = format!("{}/logs?limit={}", API_BASE, limit);
    let resp: ApiResponse<Vec<String>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 清空日志
pub async fn clear_logs() -> Result<String, reqwest::Error> {
    let url = format!("{}/logs/clear", API_BASE);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 获取邮件配置
pub async fn get_email_config() -> Result<EmailConfig, reqwest::Error> {
    let url = format!("{}/email/config", API_BASE);
    let resp: ApiResponse<EmailConfig> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 保存邮件配置
pub async fn save_email_config(config: &EmailConfig) -> Result<String, reqwest::Error> {
    let url = format!("{}/email/config", API_BASE);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .put(&url)
        .json(config)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 获取 MQTT 配置
pub async fn get_mqtt_config() -> Result<MqttConfig, reqwest::Error> {
    let url = format!("{}/mqtt/config", API_BASE);
    let resp: ApiResponse<MqttConfig> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 保存 MQTT 配置
pub async fn save_mqtt_config(config: &MqttConfig) -> Result<String, reqwest::Error> {
    let url = format!("{}/mqtt/config", API_BASE);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .put(&url)
        .json(config)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 执行系统命令
pub async fn execute_command(command: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/command", API_BASE);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({"command": command}))
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.or(resp.message).unwrap_or_else(|| "无输出".into()))
}

/// 获取 TTS 引擎列表
pub async fn get_tts_engines() -> Result<Vec<serde_json::Value>, reqwest::Error> {
    let url = format!("{}/tts/engines", API_BASE);
    let resp: ApiResponse<Vec<serde_json::Value>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// TTS 语音播放
pub async fn tts_speak(text: &str, engine: Option<&str>) -> Result<String, reqwest::Error> {
    let url = format!("{}/tts/speak", API_BASE);
    let mut body = serde_json::json!({"text": text});
    if let Some(e) = engine {
        body["engine"] = serde_json::json!(e);
    }
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// TTS 停止
pub async fn tts_stop() -> Result<String, reqwest::Error> {
    let url = format!("{}/tts/stop", API_BASE);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 获取 AI 提供商列表
pub async fn get_ai_providers() -> Result<Vec<AiProvider>, reqwest::Error> {
    let url = format!("{}/ai/providers", API_BASE);
    let resp: ApiResponse<Vec<AiProvider>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 保存 AI 提供商
pub async fn save_ai_provider(provider: &AiProvider) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/providers/{}", API_BASE, provider.id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .put(&url)
        .json(provider)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 添加 AI 提供商
pub async fn add_ai_provider(provider: &AiProvider) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/providers", API_BASE);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .json(provider)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 删除 AI 提供商
pub async fn delete_ai_provider(id: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/providers/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 测试 AI 连接
pub async fn test_ai_connection(provider: &AiProvider) -> Result<u64, reqwest::Error> {
    let url = format!("{}/ai/test-connection", API_BASE);
    let mut body = serde_json::json!({
        "base_url": provider.base_url,
        "api_key": provider.api_key,
    });
    if !provider.model.is_empty() {
        body["model"] = serde_json::json!(&provider.model);
    }
    let resp: ApiResponse<serde_json::Value> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if resp.success {
        let latency = resp.data.and_then(|d| d.get("latency").and_then(|l| l.as_u64())).unwrap_or(0);
        Ok(latency)
    } else {
        Err(reqwest::Error::new(
            reqwest::error::Kind::Other,
            std::io::Error::new(std::io::ErrorKind::Other, resp.message.unwrap_or_else(|| "连接失败".to_string()))
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
    pub pinned: bool,
    pub archived: bool,
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub name: String,
    pub content: String,
    pub category: String,
    pub memory_type: String,
    pub scope: String,
    pub project_id: String,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub pinned: bool,
    pub access_count: i32,
    pub last_accessed_at: i64,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt_template: String,
    pub variables: Vec<SkillVariable>,
    pub enabled: bool,
    pub category: String,
    pub source: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVariable {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedItem {
    pub id: String,
    pub title: String,
    pub content: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub source_url: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub enabled: bool,
    pub auto_inject: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub label: String,
    pub template: String,
    pub enabled: bool,
    pub built_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSettings {
    pub memory_enabled: bool,
    pub system_prompt_enabled: bool,
    pub preset_cadence: String,
    pub force_response_language: String,
    pub active_preset_id: String,
}

pub async fn list_chat_sessions() -> Result<Vec<ChatSession>, reqwest::Error> {
    let url = format!("{}/ai/sessions", API_BASE);
    let resp: ApiResponse<Vec<ChatSession>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn get_chat_session(id: &str) -> Result<ChatSession, reqwest::Error> {
    let url = format!("{}/ai/sessions/{}", API_BASE, id);
    let resp: ApiResponse<ChatSession> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn create_chat_session(title: &str, provider_id: &str, provider_name: &str, model: &str) -> Result<ChatSession, reqwest::Error> {
    let url = format!("{}/ai/sessions", API_BASE);
    let body = serde_json::json!({
        "title": title,
        "provider_id": provider_id,
        "provider_name": provider_name,
        "model": model,
    });
    let resp: ApiResponse<ChatSession> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn update_chat_session(id: &str, title: Option<&str>, pinned: Option<bool>, archived: Option<bool>) -> Result<ChatSession, reqwest::Error> {
    let url = format!("{}/ai/sessions/{}", API_BASE, id);
    let mut body = serde_json::json!({});
    if let Some(t) = title { body["title"] = serde_json::json!(t); }
    if let Some(p) = pinned { body["pinned"] = serde_json::json!(p); }
    if let Some(a) = archived { body["archived"] = serde_json::json!(a); }
    let resp: ApiResponse<ChatSession> = reqwest::Client::new()
        .put(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn delete_chat_session(id: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/sessions/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn list_presets() -> Result<Vec<Preset>, reqwest::Error> {
    let url = format!("{}/ai/presets", API_BASE);
    let resp: ApiResponse<Vec<Preset>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn save_preset(name: &str, description: &str, system_prompt: &str, enabled: bool) -> Result<Preset, reqwest::Error> {
    let url = format!("{}/ai/presets", API_BASE);
    let body = serde_json::json!({
        "name": name,
        "description": description,
        "system_prompt": system_prompt,
        "enabled": enabled,
    });
    let resp: ApiResponse<Preset> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn update_preset(id: &str, name: &str, description: &str, system_prompt: &str, enabled: bool) -> Result<Preset, reqwest::Error> {
    let url = format!("{}/ai/presets/{}", API_BASE, id);
    let body = serde_json::json!({
        "name": name,
        "description": description,
        "system_prompt": system_prompt,
        "enabled": enabled,
    });
    let resp: ApiResponse<Preset> = reqwest::Client::new()
        .put(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn delete_preset(id: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/presets/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn list_memories(query: Option<&str>, category: Option<&str>) -> Result<Vec<Memory>, reqwest::Error> {
    let mut url = format!("{}/ai/memories", API_BASE);
    let mut params = Vec::new();
    if let Some(q) = query { params.push(format!("q={}", q)); }
    if let Some(c) = category { params.push(format!("category={}", c)); }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }
    let resp: ApiResponse<Vec<Memory>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn create_memory(content: &str, name: Option<&str>, category: Option<&str>, tags: Option<&[String]>) -> Result<Memory, reqwest::Error> {
    let url = format!("{}/ai/memories", API_BASE);
    let mut body = serde_json::json!({
        "content": content,
    });
    if let Some(n) = name { body["name"] = serde_json::json!(n); }
    if let Some(c) = category { body["category"] = serde_json::json!(c); }
    if let Some(t) = tags { body["tags"] = serde_json::json!(t); }
    let resp: ApiResponse<Memory> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn update_memory(id: &str, content: &str, name: Option<&str>, category: Option<&str>, tags: Option<&[String]>) -> Result<Memory, reqwest::Error> {
    let url = format!("{}/ai/memories/{}", API_BASE, id);
    let mut body = serde_json::json!({
        "content": content,
    });
    if let Some(n) = name { body["name"] = serde_json::json!(n); }
    if let Some(c) = category { body["category"] = serde_json::json!(c); }
    if let Some(t) = tags { body["tags"] = serde_json::json!(t); }
    let resp: ApiResponse<Memory> = reqwest::Client::new()
        .put(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn delete_memory(id: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/memories/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn list_skills() -> Result<Vec<Skill>, reqwest::Error> {
    let url = format!("{}/ai/skills", API_BASE);
    let resp: ApiResponse<Vec<Skill>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn create_skill(name: &str, description: &str, prompt_template: &str, enabled: bool) -> Result<Skill, reqwest::Error> {
    let url = format!("{}/ai/skills", API_BASE);
    let body = serde_json::json!({
        "name": name,
        "description": description,
        "prompt_template": prompt_template,
        "enabled": enabled,
    });
    let resp: ApiResponse<Skill> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn update_skill(id: &str, name: &str, description: &str, prompt_template: &str, enabled: bool) -> Result<Skill, reqwest::Error> {
    let url = format!("{}/ai/skills/{}", API_BASE, id);
    let body = serde_json::json!({
        "name": name,
        "description": description,
        "prompt_template": prompt_template,
        "enabled": enabled,
    });
    let resp: ApiResponse<Skill> = reqwest::Client::new()
        .put(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn delete_skill(id: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/skills/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn list_projects() -> Result<Vec<Project>, reqwest::Error> {
    let url = format!("{}/ai/projects", API_BASE);
    let resp: ApiResponse<Vec<Project>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn create_project(name: &str, description: &str, instructions: &str, enabled: bool, auto_inject: bool) -> Result<Project, reqwest::Error> {
    let url = format!("{}/ai/projects", API_BASE);
    let body = serde_json::json!({
        "name": name,
        "description": description,
        "instructions": instructions,
        "enabled": enabled,
        "auto_inject": auto_inject,
    });
    let resp: ApiResponse<Project> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn update_project(id: &str, name: &str, description: &str, instructions: &str, enabled: bool, auto_inject: bool) -> Result<Project, reqwest::Error> {
    let url = format!("{}/ai/projects/{}", API_BASE, id);
    let body = serde_json::json!({
        "name": name,
        "description": description,
        "instructions": instructions,
        "enabled": enabled,
        "auto_inject": auto_inject,
    });
    let resp: ApiResponse<Project> = reqwest::Client::new()
        .put(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn delete_project(id: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/projects/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn list_scenarios() -> Result<Vec<Scenario>, reqwest::Error> {
    let url = format!("{}/ai/scenarios", API_BASE);
    let resp: ApiResponse<Vec<Scenario>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn screenshot_analyze(prompt: Option<&str>) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/screenshot", API_BASE);
    let mut body = serde_json::json!({});
    if let Some(p) = prompt { body["prompt"] = serde_json::json!(p); }
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn get_prompt_settings() -> Result<PromptSettings, reqwest::Error> {
    let url = format!("{}/ai/prompt-settings", API_BASE);
    let resp: ApiResponse<PromptSettings> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn update_prompt_settings(settings: &PromptSettings) -> Result<PromptSettings, reqwest::Error> {
    let url = format!("{}/ai/prompt-settings", API_BASE);
    let resp: ApiResponse<PromptSettings> = reqwest::Client::new()
        .put(&url)
        .json(settings)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

/// 获取文件列表
pub async fn list_files(path: &str) -> Result<Vec<serde_json::Value>, reqwest::Error> {
    let url = format!("{}/files?path={}", API_BASE, path);
    let resp: ApiResponse<Vec<serde_json::Value>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn list_saved_items() -> Result<Vec<SavedItem>, reqwest::Error> {
    let url = format!("{}/ai/saved", API_BASE);
    let resp: ApiResponse<Vec<SavedItem>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn create_saved_item(title: &str, content: &str, kind: Option<&str>, tags: Option<&[String]>, source_url: Option<&str>) -> Result<SavedItem, reqwest::Error> {
    let url = format!("{}/ai/saved", API_BASE);
    let mut body = serde_json::json!({
        "title": title,
        "content": content,
    });
    if let Some(k) = kind { body["kind"] = serde_json::json!(k); }
    if let Some(t) = tags { body["tags"] = serde_json::json!(t); }
    if let Some(s) = source_url { body["source_url"] = serde_json::json!(s); }
    let resp: ApiResponse<SavedItem> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn update_saved_item(id: &str, title: &str, content: &str, kind: Option<&str>, tags: Option<&[String]>, source_url: Option<&str>) -> Result<SavedItem, reqwest::Error> {
    let url = format!("{}/ai/saved/{}", API_BASE, id);
    let mut body = serde_json::json!({
        "title": title,
        "content": content,
    });
    if let Some(k) = kind { body["kind"] = serde_json::json!(k); }
    if let Some(t) = tags { body["tags"] = serde_json::json!(t); }
    if let Some(s) = source_url { body["source_url"] = serde_json::json!(s); }
    let resp: ApiResponse<SavedItem> = reqwest::Client::new()
        .put(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn delete_saved_item(id: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/saved/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: String,
    pub enabled: bool,
    pub auto_connect: bool,
    pub allowed_tools: Vec<String>,
    pub cached_tools: Vec<serde_json::Value>,
    pub created_at: i64,
}

pub async fn list_mcp_servers() -> Result<Vec<McpServer>, reqwest::Error> {
    let url = format!("{}/ai/mcp", API_BASE);
    let resp: ApiResponse<Vec<McpServer>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn create_mcp_server(name: &str, transport: Option<&str>, command: Option<&str>, args: Option<&[String]>, url: Option<&str>, enabled: Option<bool>, auto_connect: Option<bool>) -> Result<McpServer, reqwest::Error> {
    let url_path = format!("{}/ai/mcp", API_BASE);
    let mut body = serde_json::json!({ "name": name });
    if let Some(t) = transport { body["transport"] = serde_json::json!(t); }
    if let Some(c) = command { body["command"] = serde_json::json!(c); }
    if let Some(a) = args { body["args"] = serde_json::json!(a); }
    if let Some(u) = url { body["url"] = serde_json::json!(u); }
    if let Some(e) = enabled { body["enabled"] = serde_json::json!(e); }
    if let Some(ac) = auto_connect { body["auto_connect"] = serde_json::json!(ac); }
    let resp: ApiResponse<McpServer> = reqwest::Client::new()
        .post(&url_path)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn update_mcp_server(id: &str, name: Option<&str>, transport: Option<&str>, command: Option<&str>, args: Option<&[String]>, url: Option<&str>, enabled: Option<bool>, auto_connect: Option<bool>) -> Result<McpServer, reqwest::Error> {
    let url_path = format!("{}/ai/mcp/{}", API_BASE, id);
    let mut body = serde_json::json!({});
    if let Some(n) = name { body["name"] = serde_json::json!(n); }
    if let Some(t) = transport { body["transport"] = serde_json::json!(t); }
    if let Some(c) = command { body["command"] = serde_json::json!(c); }
    if let Some(a) = args { body["args"] = serde_json::json!(a); }
    if let Some(u) = url { body["url"] = serde_json::json!(u); }
    if let Some(e) = enabled { body["enabled"] = serde_json::json!(e); }
    if let Some(ac) = auto_connect { body["auto_connect"] = serde_json::json!(ac); }
    let resp: ApiResponse<McpServer> = reqwest::Client::new()
        .put(&url_path)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn delete_mcp_server(id: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/ai/mcp/{}", API_BASE, id);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

/// 获取截图列表
pub async fn list_screenshots() -> Result<Vec<String>, reqwest::Error> {
    let url = format!("{}/screenshots", API_BASE);
    let resp: ApiResponse<Vec<String>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub name: String,
    pub token: String,
    pub enabled: bool,
    pub services: Vec<ServiceInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub tunnel_name: String,
    pub pid: u32,
    pub uptime_secs: u64,
    pub is_alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceInfo {
    pub model: String,
    pub android_version: String,
    pub screen_size: String,
    pub battery: String,
    pub ip: String,
    pub storage: String,
    pub cpu: String,
    pub memory: String,
    pub wifi: String,
}

pub async fn get_device_info() -> Result<DeviceInfo, reqwest::Error> {
    let url = format!("{}/device/info", API_BASE);
    let resp: ApiResponse<DeviceInfo> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn list_tunnels() -> Result<Vec<TunnelInfo>, reqwest::Error> {
    let url = format!("{}/daemon/tunnels", API_BASE);
    let resp: ApiResponse<Vec<TunnelInfo>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn list_processes() -> Result<Vec<ProcessStatus>, reqwest::Error> {
    let url = format!("{}/daemon/processes", API_BASE);
    let resp: ApiResponse<Vec<ProcessStatus>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn add_tunnel(name: &str, token: &str, enabled: bool) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels", API_BASE);
    let body = serde_json::json!({
        "name": name,
        "token": token,
        "enabled": enabled,
    });
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn enable_tunnel(name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/enable", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn disable_tunnel(name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/disable", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn start_tunnel(name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/start", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn stop_tunnel(name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/stop", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn restart_tunnel(name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/restart", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn delete_tunnel(name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}", API_BASE, name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn list_services(tunnel_name: &str) -> Result<Vec<ServiceInfo>, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/services", API_BASE, tunnel_name);
    let resp: ApiResponse<Vec<ServiceInfo>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn add_service(tunnel_name: &str, name: &str, url: &str, enabled: bool) -> Result<String, reqwest::Error> {
    let url_path = format!("{}/daemon/tunnels/{}/services", API_BASE, tunnel_name);
    let body = serde_json::json!({
        "name": name,
        "url": url,
        "enabled": enabled,
    });
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url_path)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn enable_service(tunnel_name: &str, service_name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/services/{}/enable", API_BASE, tunnel_name, service_name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn disable_service(tunnel_name: &str, service_name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/services/{}/disable", API_BASE, tunnel_name, service_name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn delete_service(tunnel_name: &str, service_name: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/tunnels/{}/services/{}", API_BASE, tunnel_name, service_name);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .delete(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn get_daemon_status() -> Result<serde_json::Value, reqwest::Error> {
    let url = format!("{}/daemon/status", API_BASE);
    let resp: ApiResponse<serde_json::Value> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn stop_daemon() -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/stop", API_BASE);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn restart_daemon() -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/restart", API_BASE);
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .body("")
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn get_cloudflared_status() -> Result<serde_json::Value, reqwest::Error> {
    let url = format!("{}/daemon/cloudflared/status", API_BASE);
    let resp: ApiResponse<serde_json::Value> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

pub async fn download_cloudflared(version: &str) -> Result<String, reqwest::Error> {
    let url = format!("{}/daemon/cloudflared/download", API_BASE);
    let body = serde_json::json!({ "version": version });
    let resp: ApiResponse<String> = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp.message.unwrap_or_else(|| if resp.success { "ok".into() } else { "失败".into() }))
}

pub async fn list_cloudflared_versions() -> Result<Vec<String>, reqwest::Error> {
    let url = format!("{}/daemon/cloudflared/versions", API_BASE);
    let resp: ApiResponse<serde_json::Value> = reqwest::get(&url).await?.json().await?;
    if let Some(data) = resp.data {
        if let Some(arr) = data.as_array() {
            let result: Vec<String> = arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    } else {
        Ok(Vec::new())
    }
}
