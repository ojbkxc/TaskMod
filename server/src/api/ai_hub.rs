use axum::{Json, extract::Path, extract::Query};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

use crate::config::*;
use crate::data::response::ApiResponse;

// ==================== 通用工具函数 ====================

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn gen_id() -> String {
    format!("{:x}", now_ms() as u64)
}

async fn ensure_dir(path: &str) {
    let _ = fs::create_dir_all(path).await;
}

async fn read_json_file<T: serde::de::DeserializeOwned>(path: &str) -> Option<T> {
    let data = fs::read(path).await.ok()?;
    serde_json::from_slice(&data).ok()
}

async fn write_json_file<T: Serialize>(path: &str, data: &T) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
    fs::write(path, json).await.map_err(|e| e.to_string())
}

async fn list_json_dir<T: serde::de::DeserializeOwned>(dir: &str) -> Vec<T> {
    let mut items = Vec::new();
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Some(item) = read_json_file(path.to_str().unwrap_or("")).await {
                    items.push(item);
                }
            }
        }
    }
    items
}

// ==================== 对话历史 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionReq {
    pub title: Option<String>,
    pub provider_id: String,
    pub provider_name: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionReq {
    pub title: Option<String>,
    pub messages: Option<Vec<serde_json::Value>>,
    pub pinned: Option<bool>,
    pub archived: Option<bool>,
    pub project_id: Option<String>,
}

fn session_path(id: &str) -> String {
    format!("{}/{}.json", CHAT_HISTORY_DIR, id)
}

pub async fn list_sessions() -> Json<ApiResponse<Vec<ChatSession>>> {
    let mut sessions: Vec<ChatSession> = list_json_dir(CHAT_HISTORY_DIR).await;
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Json(ApiResponse::ok(sessions))
}

pub async fn get_session(Path(id): Path<String>) -> Json<ApiResponse<ChatSession>> {
    match read_json_file::<ChatSession>(&session_path(&id)).await {
        Some(s) => Json(ApiResponse::ok(s)),
        None => Json(ApiResponse::err("对话不存在")),
    }
}

pub async fn create_session(Json(req): Json<CreateSessionReq>) -> Json<ApiResponse<ChatSession>> {
    ensure_dir(CHAT_HISTORY_DIR).await;
    let now = now_ms();
    let session = ChatSession {
        id: gen_id(),
        title: req.title.unwrap_or_else(|| "新对话".to_string()),
        provider_id: req.provider_id,
        provider_name: req.provider_name.unwrap_or_default(),
        model: req.model.unwrap_or_default(),
        messages: Vec::new(),
        created_at: now,
        updated_at: now,
        pinned: false,
        archived: false,
        project_id: req.project_id.unwrap_or_default(),
    };
    match write_json_file(&session_path(&session.id), &session).await {
        Ok(()) => Json(ApiResponse::ok(session)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_session(
    Path(id): Path<String>,
    Json(req): Json<UpdateSessionReq>,
) -> Json<ApiResponse<ChatSession>> {
    let path = session_path(&id);
    let mut session: ChatSession = match read_json_file(&path).await {
        Some(s) => s,
        None => return Json(ApiResponse::err("对话不存在")),
    };
    if let Some(title) = req.title { session.title = title; }
    if let Some(msgs) = req.messages { session.messages = msgs; }
    if let Some(pinned) = req.pinned { session.pinned = pinned; }
    if let Some(archived) = req.archived { session.archived = archived; }
    if let Some(pid) = req.project_id { session.project_id = pid; }
    session.updated_at = now_ms();
    match write_json_file(&path, &session).await {
        Ok(()) => Json(ApiResponse::ok(session)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn delete_session(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    match fs::remove_file(&session_path(&id)).await {
        Ok(()) => Json(ApiResponse::ok("已删除".to_string())),
        Err(_) => Json(ApiResponse::err("删除失败")),
    }
}

// ==================== Prompt 预设 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct PresetReq {
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub enabled: Option<bool>,
}

fn presets_data() -> Vec<Preset> {
    // 同步读取，供 AI 模块调用
    std::fs::read(PRESETS_FILE)
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or_default()
}

pub fn get_active_presets() -> Vec<Preset> {
    presets_data().into_iter().filter(|p| p.enabled).collect()
}

pub async fn list_presets() -> Json<ApiResponse<Vec<Preset>>> {
    let presets: Vec<Preset> = read_json_file(PRESETS_FILE).await.unwrap_or_default();
    Json(ApiResponse::ok(presets))
}

pub async fn save_preset(Json(req): Json<PresetReq>) -> Json<ApiResponse<Preset>> {
    let mut presets: Vec<Preset> = read_json_file(PRESETS_FILE).await.unwrap_or_default();
    let preset = Preset {
        id: gen_id(),
        name: req.name,
        description: req.description.unwrap_or_default(),
        system_prompt: req.system_prompt,
        enabled: req.enabled.unwrap_or(false),
    };
    presets.push(preset.clone());
    match write_json_file(PRESETS_FILE, &presets).await {
        Ok(()) => Json(ApiResponse::ok(preset)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_preset(
    Path(id): Path<String>,
    Json(req): Json<PresetReq>,
) -> Json<ApiResponse<Preset>> {
    let mut presets: Vec<Preset> = read_json_file(PRESETS_FILE).await.unwrap_or_default();
    if let Some(p) = presets.iter_mut().find(|p| p.id == id) {
        p.name = req.name;
        p.description = req.description.unwrap_or_default();
        p.system_prompt = req.system_prompt;
        if let Some(enabled) = req.enabled { p.enabled = enabled; }
        let updated = p.clone();
        match write_json_file(PRESETS_FILE, &presets).await {
            Ok(()) => return Json(ApiResponse::ok(updated)),
            Err(e) => return Json(ApiResponse::err(&format!("保存失败: {}", e))),
        }
    }
    Json(ApiResponse::err("预设不存在"))
}

pub async fn delete_preset(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    let mut presets: Vec<Preset> = read_json_file(PRESETS_FILE).await.unwrap_or_default();
    let before = presets.len();
    presets.retain(|p| p.id != id);
    if presets.len() == before {
        return Json(ApiResponse::err("预设不存在"));
    }
    match write_json_file(PRESETS_FILE, &presets).await {
        Ok(()) => Json(ApiResponse::ok("已删除".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}

// ==================== 记忆系统 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Memory {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub memory_type: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Deserialize)]
pub struct MemoryReq {
    pub content: String,
    pub name: Option<String>,
    pub category: Option<String>,
    pub memory_type: Option<String>,
    pub scope: Option<String>,
    pub project_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pinned: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct MemoryQuery {
    pub q: Option<String>,
    pub category: Option<String>,
}

fn memory_path(id: &str) -> String {
    format!("{}/{}.json", MEMORY_DIR, id)
}

/// 获取所有记忆（同步，供AI模块调用）
pub fn get_all_memories_sync() -> Vec<Memory> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(MEMORY_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(data) = std::fs::read(&path) {
                    if let Ok(item) = serde_json::from_slice::<Memory>(&data) {
                        items.push(item);
                    }
                }
            }
        }
    }
    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    items
}

pub async fn list_memories(Query(q): Query<MemoryQuery>) -> Json<ApiResponse<Vec<Memory>>> {
    let mut items: Vec<Memory> = list_json_dir(MEMORY_DIR).await;

    if let Some(ref keyword) = q.q {
        let kw = keyword.to_lowercase();
        items.retain(|m| m.content.to_lowercase().contains(&kw) || m.tags.iter().any(|t| t.to_lowercase().contains(&kw)));
    }
    if let Some(ref cat) = q.category {
        items.retain(|m| m.category == *cat);
    }

    items.sort_by(|a, b| {
        a.pinned.cmp(&b.pinned).reverse()
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
    Json(ApiResponse::ok(items))
}

pub async fn create_memory(Json(req): Json<MemoryReq>) -> Json<ApiResponse<Memory>> {
    ensure_dir(MEMORY_DIR).await;
    let now = now_ms();
    let mem = Memory {
        id: gen_id(),
        name: req.name.unwrap_or_default(),
        content: req.content,
        category: req.category.unwrap_or_default(),
        memory_type: req.memory_type.unwrap_or_else(|| "user".to_string()),
        scope: req.scope.unwrap_or_else(|| "global".to_string()),
        project_id: req.project_id.unwrap_or_default(),
        tags: req.tags.unwrap_or_default(),
        created_at: now,
        updated_at: now,
        pinned: req.pinned.unwrap_or(false),
    };
    match write_json_file(&memory_path(&mem.id), &mem).await {
        Ok(()) => Json(ApiResponse::ok(mem)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_memory(
    Path(id): Path<String>,
    Json(req): Json<MemoryReq>,
) -> Json<ApiResponse<Memory>> {
    let path = memory_path(&id);
    let mut mem: Memory = match read_json_file(&path).await {
        Some(m) => m,
        None => return Json(ApiResponse::err("记忆不存在")),
    };
    mem.content = req.content;
    if let Some(name) = req.name { mem.name = name; }
    if let Some(cat) = req.category { mem.category = cat; }
    if let Some(mt) = req.memory_type { mem.memory_type = mt; }
    if let Some(scope) = req.scope { mem.scope = scope; }
    if let Some(pid) = req.project_id { mem.project_id = pid; }
    if let Some(tags) = req.tags { mem.tags = tags; }
    if let Some(pinned) = req.pinned { mem.pinned = pinned; }
    mem.updated_at = now_ms();
    match write_json_file(&path, &mem).await {
        Ok(()) => Json(ApiResponse::ok(mem)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn delete_memory(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    match fs::remove_file(&memory_path(&id)).await {
        Ok(()) => Json(ApiResponse::ok("已删除".to_string())),
        Err(_) => Json(ApiResponse::err("删除失败")),
    }
}

// ==================== Skill 系统（热加载） ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt_template: String,
    #[serde(default)]
    pub variables: Vec<SkillVariable>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub source: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillVariable {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: String,
}

#[derive(Debug, Deserialize)]
pub struct SkillReq {
    pub name: String,
    pub description: Option<String>,
    pub prompt_template: Option<String>,
    pub variables: Option<Vec<SkillVariable>>,
    pub enabled: Option<bool>,
    pub category: Option<String>,
    pub source: Option<String>,
}

fn skill_path(id: &str) -> String {
    format!("{}/{}.json", SKILLS_DIR, id)
}

/// 获取所有已启用的Skill（同步，供AI模块调用）
#[allow(dead_code)]
pub fn get_enabled_skills_sync() -> Vec<Skill> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(SKILLS_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(data) = std::fs::read(&path) {
                    if let Ok(item) = serde_json::from_slice::<Skill>(&data) {
                        if item.enabled {
                            items.push(item);
                        }
                    }
                }
            }
        }
    }
    items
}

pub async fn list_skills() -> Json<ApiResponse<Vec<Skill>>> {
    let mut items: Vec<Skill> = list_json_dir(SKILLS_DIR).await;
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Json(ApiResponse::ok(items))
}

pub async fn create_skill(Json(req): Json<SkillReq>) -> Json<ApiResponse<Skill>> {
    ensure_dir(SKILLS_DIR).await;
    let skill = Skill {
        id: gen_id(),
        name: req.name,
        description: req.description.unwrap_or_default(),
        prompt_template: req.prompt_template.unwrap_or_default(),
        variables: req.variables.unwrap_or_default(),
        enabled: req.enabled.unwrap_or(true),
        category: req.category.unwrap_or_default(),
        source: req.source.unwrap_or_else(|| "custom".to_string()),
        created_at: now_ms(),
    };
    match write_json_file(&skill_path(&skill.id), &skill).await {
        Ok(()) => Json(ApiResponse::ok(skill)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_skill(
    Path(id): Path<String>,
    Json(req): Json<SkillReq>,
) -> Json<ApiResponse<Skill>> {
    let path = skill_path(&id);
    let mut skill: Skill = match read_json_file(&path).await {
        Some(s) => s,
        None => return Json(ApiResponse::err("Skill不存在")),
    };
    skill.name = req.name;
    skill.description = req.description.unwrap_or_default();
    skill.prompt_template = req.prompt_template.unwrap_or(skill.prompt_template);
    if let Some(vars) = req.variables { skill.variables = vars; }
    if let Some(enabled) = req.enabled { skill.enabled = enabled; }
    if let Some(cat) = req.category { skill.category = cat; }
    if let Some(source) = req.source { skill.source = source; }
    match write_json_file(&path, &skill).await {
        Ok(()) => Json(ApiResponse::ok(skill)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn delete_skill(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    match fs::remove_file(&skill_path(&id)).await {
        Ok(()) => Json(ApiResponse::ok("已删除".to_string())),
        Err(_) => Json(ApiResponse::err("删除失败")),
    }
}

// ==================== 保存项（Saved Items） ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedItem {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_url: String,
    pub created_at: i64,
    pub updated_at: i64,
}

fn default_kind() -> String { "snippet".to_string() }

#[derive(Debug, Deserialize)]
pub struct SavedItemReq {
    pub title: String,
    pub content: String,
    pub kind: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source_url: Option<String>,
}

fn saved_item_path(id: &str) -> String {
    format!("{}/{}.json", SAVED_ITEMS_DIR, id)
}

pub async fn list_saved_items() -> Json<ApiResponse<Vec<SavedItem>>> {
    let mut items: Vec<SavedItem> = list_json_dir(SAVED_ITEMS_DIR).await;
    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Json(ApiResponse::ok(items))
}

pub async fn create_saved_item(Json(req): Json<SavedItemReq>) -> Json<ApiResponse<SavedItem>> {
    ensure_dir(SAVED_ITEMS_DIR).await;
    let now = now_ms();
    let item = SavedItem {
        id: gen_id(),
        title: req.title,
        content: req.content,
        kind: req.kind.unwrap_or_else(|| "snippet".to_string()),
        tags: req.tags.unwrap_or_default(),
        source_url: req.source_url.unwrap_or_default(),
        created_at: now,
        updated_at: now,
    };
    match write_json_file(&saved_item_path(&item.id), &item).await {
        Ok(()) => Json(ApiResponse::ok(item)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_saved_item(
    Path(id): Path<String>,
    Json(req): Json<SavedItemReq>,
) -> Json<ApiResponse<SavedItem>> {
    let path = saved_item_path(&id);
    let mut item: SavedItem = match read_json_file(&path).await {
        Some(i) => i,
        None => return Json(ApiResponse::err("保存项不存在")),
    };
    item.title = req.title;
    item.content = req.content;
    if let Some(kind) = req.kind { item.kind = kind; }
    if let Some(tags) = req.tags { item.tags = tags; }
    if let Some(url) = req.source_url { item.source_url = url; }
    item.updated_at = now_ms();
    match write_json_file(&path, &item).await {
        Ok(()) => Json(ApiResponse::ok(item)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn delete_saved_item(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    match fs::remove_file(&saved_item_path(&id)).await {
        Ok(()) => Json(ApiResponse::ok("已删除".to_string())),
        Err(_) => Json(ApiResponse::err("删除失败")),
    }
}

// ==================== 项目上下文 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_inject: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ProjectReq {
    pub name: String,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub enabled: Option<bool>,
    pub auto_inject: Option<bool>,
}

fn project_path(id: &str) -> String {
    format!("{}/{}.json", PROJECTS_DIR, id)
}

/// 获取所有已启用且自动注入的项目（同步，供AI模块调用）
pub fn get_active_projects_sync() -> Vec<Project> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(PROJECTS_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(data) = std::fs::read(&path) {
                    if let Ok(item) = serde_json::from_slice::<Project>(&data) {
                        if item.enabled && item.auto_inject {
                            items.push(item);
                        }
                    }
                }
            }
        }
    }
    items
}

pub async fn list_projects() -> Json<ApiResponse<Vec<Project>>> {
    let mut items: Vec<Project> = list_json_dir(PROJECTS_DIR).await;
    items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Json(ApiResponse::ok(items))
}

pub async fn create_project(Json(req): Json<ProjectReq>) -> Json<ApiResponse<Project>> {
    ensure_dir(PROJECTS_DIR).await;
    let now = now_ms();
    let proj = Project {
        id: gen_id(),
        name: req.name,
        description: req.description.unwrap_or_default(),
        instructions: req.instructions.unwrap_or_default(),
        enabled: req.enabled.unwrap_or(false),
        auto_inject: req.auto_inject.unwrap_or(false),
        created_at: now,
        updated_at: now,
    };
    match write_json_file(&project_path(&proj.id), &proj).await {
        Ok(()) => Json(ApiResponse::ok(proj)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_project(
    Path(id): Path<String>,
    Json(req): Json<ProjectReq>,
) -> Json<ApiResponse<Project>> {
    let path = project_path(&id);
    let mut proj: Project = match read_json_file(&path).await {
        Some(p) => p,
        None => return Json(ApiResponse::err("项目不存在")),
    };
    proj.name = req.name;
    proj.description = req.description.unwrap_or_default();
    proj.instructions = req.instructions.unwrap_or_default();
    if let Some(enabled) = req.enabled { proj.enabled = enabled; }
    if let Some(auto_inject) = req.auto_inject { proj.auto_inject = auto_inject; }
    proj.updated_at = now_ms();
    match write_json_file(&path, &proj).await {
        Ok(()) => Json(ApiResponse::ok(proj)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn delete_project(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    match fs::remove_file(&project_path(&id)).await {
        Ok(()) => Json(ApiResponse::ok("已删除".to_string())),
        Err(_) => Json(ApiResponse::err("删除失败")),
    }
}

// ==================== MCP 服务器配置 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub transport: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub auto_connect: bool,
    /// 工具权限控制：允许执行的工具列表，空表示全部允许
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// 工具缓存：上次发现的工具列表
    #[serde(default)]
    pub cached_tools: Vec<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct McpServerReq {
    pub name: String,
    pub transport: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
    pub enabled: Option<bool>,
    pub auto_connect: Option<bool>,
    pub allowed_tools: Option<Vec<String>>,
    pub cached_tools: Option<Vec<serde_json::Value>>,
}

fn mcp_path(id: &str) -> String {
    format!("{}/{}.json", MCP_DIR, id)
}

/// 获取所有已启用的MCP服务器（同步）
#[allow(dead_code)]
pub fn get_enabled_mcp_servers_sync() -> Vec<McpServer> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(MCP_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(data) = std::fs::read(&path) {
                    if let Ok(item) = serde_json::from_slice::<McpServer>(&data) {
                        if item.enabled {
                            items.push(item);
                        }
                    }
                }
            }
        }
    }
    items
}

pub async fn list_mcp_servers() -> Json<ApiResponse<Vec<McpServer>>> {
    let mut items: Vec<McpServer> = list_json_dir(MCP_DIR).await;
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Json(ApiResponse::ok(items))
}

pub async fn create_mcp_server(Json(req): Json<McpServerReq>) -> Json<ApiResponse<McpServer>> {
    ensure_dir(MCP_DIR).await;
    let server = McpServer {
        id: gen_id(),
        name: req.name,
        transport: req.transport.unwrap_or_else(|| "stdio".to_string()),
        command: req.command.unwrap_or_default(),
        args: req.args.unwrap_or_default(),
        env: req.env.unwrap_or_default(),
        url: req.url.unwrap_or_default(),
        enabled: req.enabled.unwrap_or(false),
        auto_connect: req.auto_connect.unwrap_or(false),
        allowed_tools: req.allowed_tools.unwrap_or_default(),
        cached_tools: req.cached_tools.unwrap_or_default(),
        created_at: now_ms(),
    };
    match write_json_file(&mcp_path(&server.id), &server).await {
        Ok(()) => Json(ApiResponse::ok(server)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_mcp_server(
    Path(id): Path<String>,
    Json(req): Json<McpServerReq>,
) -> Json<ApiResponse<McpServer>> {
    let path = mcp_path(&id);
    let mut server: McpServer = match read_json_file(&path).await {
        Some(s) => s,
        None => return Json(ApiResponse::err("MCP服务器不存在")),
    };
    server.name = req.name;
    if let Some(t) = req.transport { server.transport = t; }
    if let Some(c) = req.command { server.command = c; }
    if let Some(a) = req.args { server.args = a; }
    if let Some(e) = req.env { server.env = e; }
    if let Some(u) = req.url { server.url = u; }
    if let Some(enabled) = req.enabled { server.enabled = enabled; }
    if let Some(auto) = req.auto_connect { server.auto_connect = auto; }
    if let Some(at) = req.allowed_tools { server.allowed_tools = at; }
    if let Some(ct) = req.cached_tools { server.cached_tools = ct; }
    match write_json_file(&path, &server).await {
        Ok(()) => Json(ApiResponse::ok(server)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn delete_mcp_server(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    match fs::remove_file(&mcp_path(&id)).await {
        Ok(()) => Json(ApiResponse::ok("已删除".to_string())),
        Err(_) => Json(ApiResponse::err("删除失败")),
    }
}

// ==================== 截图+AI分析 ====================

#[derive(Debug, Deserialize)]
pub struct ScreenshotAnalyzeReq {
    pub prompt: Option<String>,
}

pub async fn screenshot_analyze(Json(req): Json<ScreenshotAnalyzeReq>) -> Json<ApiResponse<String>> {
    use crate::utils::adb;

    let providers = crate::api::ai::get_enabled_providers();
    let provider = match providers.first() {
        Some(p) => p.clone(),
        None => return Json(ApiResponse::err("没有可用的AI Provider")),
    };

    // 截图
    let img_base64 = match adb::adb_screencap_base64().await {
        Ok(b) => b,
        Err(e) => return Json(ApiResponse::err(&format!("截图失败: {}", e))),
    };

    // 调用支持图像的AI
    let prompt = req.prompt.unwrap_or_else(|| "请描述这个屏幕截图的内容，并给出操作建议。".to_string());
    match crate::api::ai::call_ai_image_analyze(&provider, &prompt, &img_base64).await {
        Ok(result) => Json(ApiResponse::ok(result)),
        Err(e) => Json(ApiResponse::err(&format!("AI分析失败: {}", e))),
    }
}

// ==================== 对话导出 ====================

#[derive(Debug, Deserialize)]
pub struct ExportReq {
    pub session_id: String,
    pub format: Option<String>,
}

pub async fn export_session(Json(req): Json<ExportReq>) -> Json<ApiResponse<String>> {
    let session: ChatSession = match read_json_file(&session_path(&req.session_id)).await {
        Some(s) => s,
        None => return Json(ApiResponse::err("对话不存在")),
    };

    let fmt = req.format.unwrap_or_else(|| "markdown".to_string());
    let content = match fmt.as_str() {
        "json" => serde_json::to_string_pretty(&session.messages).unwrap_or_default(),
        _ => {
            // Markdown 格式
            let mut md = format!("# {}\n\n", session.title);
            md.push_str(&format!("模型: {} | 时间: {}\n\n---\n\n", session.model, {
                let dt = chrono::DateTime::from_timestamp_millis(session.created_at);
                dt.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default()
            }));
            for msg in &session.messages {
                let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");
                let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                match role {
                    "user" => md.push_str(&format!("**You:** {}\n\n", content)),
                    "assistant" => md.push_str(&format!("**AI:** {}\n\n", content)),
                    "system" => md.push_str(&format!("> System: {}\n\n", content)),
                    "tool" => md.push_str(&format!("> Tool: {}\n\n", content)),
                    _ => {}
                }
            }
            md
        }
    };

    Json(ApiResponse::ok(content))
}

// ==================== Prompt 注入设置 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptSettings {
    #[serde(default = "default_true")]
    pub memory_enabled: bool,
    #[serde(default = "default_true")]
    pub system_prompt_enabled: bool,
    #[serde(default = "default_cadence")]
    pub preset_cadence: String,
    #[serde(default)]
    pub force_response_language: String,
    #[serde(default)]
    pub active_preset_id: String,
}

fn default_true() -> bool { true }
fn default_cadence() -> String { "default".to_string() }

impl Default for PromptSettings {
    fn default() -> Self {
        Self {
            memory_enabled: true,
            system_prompt_enabled: true,
            preset_cadence: "default".to_string(),
            force_response_language: String::new(),
            active_preset_id: String::new(),
        }
    }
}

const PROMPT_SETTINGS_FILE: &str = "/sdcard/TaskMod/prompt_settings.json";

pub fn get_prompt_settings_sync() -> PromptSettings {
    std::fs::read(PROMPT_SETTINGS_FILE)
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or_default()
}

pub async fn get_prompt_settings() -> Json<ApiResponse<PromptSettings>> {
    let settings: PromptSettings = read_json_file(PROMPT_SETTINGS_FILE).await.unwrap_or_default();
    Json(ApiResponse::ok(settings))
}

pub async fn update_prompt_settings(Json(req): Json<PromptSettings>) -> Json<ApiResponse<PromptSettings>> {
    match write_json_file(PROMPT_SETTINGS_FILE, &req).await {
        Ok(()) => Json(ApiResponse::ok(req)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

// ==================== 场景模板 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scenario {
    pub id: String,
    pub label: String,
    pub template: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub built_in: bool,
}

const SCENARIOS_FILE: &str = "/sdcard/TaskMod/scenarios.json";

fn default_scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: "summarize".into(), label: "总结".into(),
            template: "请用简洁的语言总结以下内容：\n\n{text}".into(),
            enabled: true, built_in: true,
        },
        Scenario {
            id: "explain".into(), label: "解释".into(),
            template: "请解释以下内容：\n\n{text}".into(),
            enabled: true, built_in: true,
        },
        Scenario {
            id: "translate_zh".into(), label: "翻译成中文".into(),
            template: "请将以下内容翻译成中文：\n\n{text}".into(),
            enabled: true, built_in: true,
        },
        Scenario {
            id: "translate_en".into(), label: "翻译成英文".into(),
            template: "Please translate the following into English:\n\n{text}".into(),
            enabled: true, built_in: true,
        },
        Scenario {
            id: "debug".into(), label: "调试分析".into(),
            template: "请分析以下错误/日志并给出修复建议：\n\n{text}".into(),
            enabled: true, built_in: true,
        },
    ]
}

fn load_scenarios_sync() -> Vec<Scenario> {
    std::fs::read(SCENARIOS_FILE)
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or_else(default_scenarios)
}

#[allow(dead_code)]
pub fn get_enabled_scenarios_sync() -> Vec<Scenario> {
    load_scenarios_sync().into_iter().filter(|s| s.enabled).collect()
}

pub async fn list_scenarios() -> Json<ApiResponse<Vec<Scenario>>> {
    let scenarios = load_scenarios_sync();
    Json(ApiResponse::ok(scenarios))
}

#[derive(Debug, Deserialize)]
pub struct ScenarioReq {
    pub label: String,
    pub template: String,
    pub enabled: Option<bool>,
}

pub async fn create_scenario(Json(req): Json<ScenarioReq>) -> Json<ApiResponse<Scenario>> {
    let mut scenarios = load_scenarios_sync();
    let scenario = Scenario {
        id: gen_id(),
        label: req.label,
        template: req.template,
        enabled: req.enabled.unwrap_or(true),
        built_in: false,
    };
    scenarios.push(scenario.clone());
    match write_json_file(SCENARIOS_FILE, &scenarios).await {
        Ok(()) => Json(ApiResponse::ok(scenario)),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn update_scenario(Path(id): Path<String>, Json(req): Json<ScenarioReq>) -> Json<ApiResponse<Scenario>> {
    let mut scenarios = load_scenarios_sync();
    if let Some(s) = scenarios.iter_mut().find(|s| s.id == id) {
        s.label = req.label;
        s.template = req.template;
        if let Some(enabled) = req.enabled { s.enabled = enabled; }
        let updated = s.clone();
        match write_json_file(SCENARIOS_FILE, &scenarios).await {
            Ok(()) => return Json(ApiResponse::ok(updated)),
            Err(e) => return Json(ApiResponse::err(&format!("保存失败: {}", e))),
        }
    }
    Json(ApiResponse::err("场景不存在"))
}

pub async fn delete_scenario(Path(id): Path<String>) -> Json<ApiResponse<String>> {
    let mut scenarios = load_scenarios_sync();
    let before = scenarios.len();
    scenarios.retain(|s| s.id != id || s.built_in);
    if scenarios.len() == before { return Json(ApiResponse::err("场景不存在或为内置场景")); }
    match write_json_file(SCENARIOS_FILE, &scenarios).await {
        Ok(()) => Json(ApiResponse::ok("已删除".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}

// ==================== 记忆智能选择（供AI对话注入） ====================

pub fn select_memories_for_prompt(user_message: &str, project_id: Option<&str>) -> Vec<Memory> {
    let settings = get_prompt_settings_sync();
    if !settings.memory_enabled { return Vec::new(); }

    let all = get_all_memories_sync();
    let msg_lower = user_message.to_lowercase();
    let msg_words: Vec<&str> = msg_lower.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() > 1)
        .collect();

    let mut scored: Vec<(f64, Memory)> = all.into_iter().filter_map(|m| {
        // 作用域过滤
        if m.scope == "project" {
            if let Some(pid) = project_id {
                if m.project_id != pid { return None; }
            } else {
                return None;
            }
        }

        let mut score = 0.0;

        // 标签匹配
        for tag in &m.tags {
            let tag_lower = tag.to_lowercase();
            if msg_words.iter().any(|w| tag_lower.contains(w)) { score += 20.0; }
        }

        // 名称匹配
        if !m.name.is_empty() {
            let name_lower = m.name.to_lowercase();
            if msg_words.iter().any(|w| name_lower.contains(w)) { score += 15.0; }
        }

        // 内容匹配
        let content_lower = m.content.to_lowercase();
        for w in &msg_words {
            if content_lower.contains(w) { score += 5.0; }
        }

        // 置顶加分
        if m.pinned { score += 30.0; }

        if score > 0.0 { Some((score, m)) } else { None }
    }).collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // 取前10条，限制token
    scored.into_iter().take(10).map(|(_, m)| m).collect()
}

pub fn build_memory_context(memories: &[Memory]) -> String {
    if memories.is_empty() { return String::new(); }
    let mut ctx = String::from("## 相关记忆\n\n");
    for m in memories {
        if !m.name.is_empty() {
            ctx.push_str(&format!("- **{}**: {}\n", m.name, m.content));
        } else {
            ctx.push_str(&format!("- {}\n", m.content));
        }
    }
    ctx
}

// ==================== 图片上传 ====================

#[derive(Debug, Deserialize)]
pub struct UploadImageReq {
    pub image_base64: String,
    pub mime_type: Option<String>,
}

pub async fn upload_image(Json(req): Json<UploadImageReq>) -> Json<serde_json::Value> {
    use crate::config::UPLOAD_DIR;
    let _ = fs::create_dir_all(UPLOAD_DIR).await;
    let id = gen_id();
    let ext = match req.mime_type.as_deref() {
        Some("image/png") => "png",
        Some("image/jpeg") | Some("image/jpg") => "jpg",
        Some("image/gif") => "gif",
        Some("image/webp") => "webp",
        _ => "png",
    };
    let path = format!("{}/{}.{}", UPLOAD_DIR, id, ext);
    match base64::engine::general_purpose::STANDARD.decode(&req.image_base64) {
        Ok(bytes) => {
            if fs::write(&path, &bytes).await.is_ok() {
                Json(json!({"ok": true, "id": id, "path": path, "url": format!("/api/ai/images/{}", id)}))
            } else {
                Json(json!({"ok": false, "error": "写入失败"}))
            }
        }
        Err(e) => Json(json!({"ok": false, "error": format!("base64解码失败: {}", e)})),
    }
}

pub async fn get_image(Path(id): Path<String>) -> Result<axum::response::Response, (axum::http::StatusCode, String)> {
    use crate::config::UPLOAD_DIR;
    for ext in &["png", "jpg", "gif", "webp"] {
        let path = format!("{}/{}.{}", UPLOAD_DIR, id, ext);
        if let Ok(bytes) = fs::read(&path).await {
            let mime = match *ext {
                "png" => "image/png",
                "jpg" => "image/jpeg",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            };
            return Ok(axum::response::Response::builder()
                .header("content-type", mime)
                .header("cache-control", "public, max-age=86400")
                .body(axum::body::boxed(axum::body::Full::from(bytes)))
                .unwrap());
        }
    }
    Err((axum::http::StatusCode::NOT_FOUND, "图片未找到".to_string()))
}

// ==================== Skill GitHub 导入 ====================

#[derive(Debug, Deserialize)]
pub struct SkillImportReq {
    pub github_url: String,
    pub name: Option<String>,
    pub category: Option<String>,
}

pub async fn import_skill_from_github(Json(req): Json<SkillImportReq>) -> Json<serde_json::Value> {
    // 将 GitHub URL 转换为 raw URL
    let raw_url = req.github_url
        .replace("github.com", "raw.githubusercontent.com")
        .replace("/blob/", "/")
        .replace("/tree/", "/");

    match reqwest::get(&raw_url).await {
        Ok(resp) => match resp.text().await {
            Ok(content) => {
                // 尝试解析为 JSON Skill 文件
                if let Ok(skill) = serde_json::from_str::<Skill>(&content) {
                    let id = skill.id.clone();
                    let path = skill_path(&id);
                    let _ = write_json_file(&path, &skill).await;
                    return Json(json!({"ok": true, "skill": skill}));
                }
                // 尝试解析为 JSON SkillReq 格式
                if let Ok(req_skill) = serde_json::from_str::<SkillReq>(&content) {
                    let id = gen_id();
                    let skill = Skill {
                        id: id.clone(),
                        name: req_skill.name,
                        description: req_skill.description.unwrap_or_default(),
                        prompt_template: req_skill.prompt_template.unwrap_or_default(),
                        variables: req_skill.variables.unwrap_or_default(),
                        enabled: req_skill.enabled.unwrap_or(true),
                        source: format!("github:{}", req.github_url),
                        category: req_skill.category.unwrap_or_default(),
                        created_at: now_ms(),
                    };
                    let path = skill_path(&id);
                    let _ = write_json_file(&path, &skill).await;
                    return Json(json!({"ok": true, "skill": skill}));
                }
                // 作为纯文本 prompt_template 处理
                let id = gen_id();
                let name = req.name.clone().unwrap_or_else(|| {
                    raw_url.split('/').last().unwrap_or("imported_skill").replace(".json", "").replace(".md", "")
                });
                let skill = Skill {
                    id: id.clone(),
                    name,
                    description: format!("从 GitHub 导入: {}", req.github_url),
                    prompt_template: content,
                    variables: vec![],
                    enabled: true,
                    source: format!("github:{}", req.github_url),
                    category: req.category.unwrap_or_default(),
                    created_at: now_ms(),
                };
                let path = skill_path(&id);
                let _ = write_json_file(&path, &skill).await;
                Json(json!({"ok": true, "skill": skill}))
            }
            Err(e) => Json(json!({"ok": false, "error": format!("读取内容失败: {}", e)})),
        },
        Err(e) => Json(json!({"ok": false, "error": format!("请求失败: {}", e)})),
    }
}

// ==================== MCP 工具历史 ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpToolCall {
    pub id: String,
    pub server_id: String,
    pub server_name: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<String>,
    pub success: bool,
    pub duration_ms: u64,
    pub called_at: i64,
}

fn mcp_history_path(id: &str) -> String {
    format!("{}/{}.json", MCP_HISTORY_DIR, id)
}

pub async fn list_mcp_history(Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let limit = params.get("limit").and_then(|v| v.parse::<usize>().ok()).unwrap_or(50);
    let server_id = params.get("server_id");

    if let Ok(mut entries) = fs::read_dir(MCP_HISTORY_DIR).await {
        let mut calls: Vec<McpToolCall> = vec![];
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(data) = fs::read_to_string(&path).await {
                    if let Ok(call) = serde_json::from_str::<McpToolCall>(&data) {
                        if server_id.map_or(true, |sid| &call.server_id == sid) {
                            calls.push(call);
                        }
                    }
                }
            }
        }
        calls.sort_by(|a, b| b.called_at.cmp(&a.called_at));
        calls.truncate(limit);
        Json(json!({"ok": true, "history": calls}))
    } else {
        Json(json!({"ok": true, "history": []}))
    }
}

pub async fn record_mcp_tool_call(Json(call): Json<McpToolCall>) -> Json<serde_json::Value> {
    let _ = fs::create_dir_all(MCP_HISTORY_DIR).await;
    let path = mcp_history_path(&call.id);
    let _ = write_json_file(&path, &call).await;
    Json(json!({"ok": true}))
}

pub async fn clear_mcp_history() -> Json<serde_json::Value> {
    let _ = fs::create_dir_all(MCP_HISTORY_DIR).await;
    if let Ok(mut entries) = fs::read_dir(MCP_HISTORY_DIR).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let _ = fs::remove_file(entry.path()).await;
        }
    }
    Json(json!({"ok": true}))
}

// ==================== 使用量统计 ====================

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UsageStats {
    pub total_calls: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_reasoning_tokens: u64,
    pub by_provider: HashMap<String, ProviderUsage>,
    pub by_date: HashMap<String, DailyUsage>,
    pub last_updated: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProviderUsage {
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DailyUsage {
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub struct RecordUsageReq {
    pub provider_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: Option<u64>,
}

pub async fn get_usage_stats() -> Json<serde_json::Value> {
    let stats: UsageStats = if let Ok(data) = fs::read_to_string(USAGE_STATS_FILE).await {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        UsageStats::default()
    };
    Json(json!({"ok": true, "stats": stats}))
}

pub async fn record_usage(Json(req): Json<RecordUsageReq>) -> Json<serde_json::Value> {
    let mut stats: UsageStats = if let Ok(data) = fs::read_to_string(USAGE_STATS_FILE).await {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        UsageStats::default()
    };

    let reasoning = req.reasoning_tokens.unwrap_or(0);
    stats.total_calls += 1;
    stats.total_input_tokens += req.input_tokens;
    stats.total_output_tokens += req.output_tokens;
    stats.total_reasoning_tokens += reasoning;
    stats.last_updated = ts_now();

    let provider = stats.by_provider.entry(req.provider_id).or_default();
    provider.calls += 1;
    provider.input_tokens += req.input_tokens;
    provider.output_tokens += req.output_tokens;
    provider.reasoning_tokens += reasoning;

    let today = chrono_date();
    let daily = stats.by_date.entry(today).or_default();
    daily.calls += 1;
    daily.input_tokens += req.input_tokens;
    daily.output_tokens += req.output_tokens;

    let _ = write_json_file(USAGE_STATS_FILE, &stats).await;
    Json(json!({"ok": true}))
}

fn chrono_date() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let days = secs / 86400;
    let y = 1970 + days / 365;
    let d = days % 365;
    let m = d / 30 + 1;
    let d = d % 30 + 1;
    format!("{:04}-{:02}-{:02}", y, m, d)
}

// ==================== 对话标题自动生成 ====================

pub fn generate_title_from_message(message: &str) -> String {
    let clean = message.trim();
    if clean.is_empty() { return "空对话".to_string(); }
    // 取前30个字符
    let title: String = clean.chars().take(30).collect();
    if clean.chars().count() > 30 { format!("{}...", title) } else { title }
}

// ==================== 统一初始化 ====================

pub async fn init_dirs() {
    for dir in &[SKILLS_DIR, MCP_DIR, MEMORY_DIR, PROJECTS_DIR, CHAT_HISTORY_DIR, SAVED_ITEMS_DIR, MCP_HISTORY_DIR, UPLOAD_DIR] {
        let _ = fs::create_dir_all(dir).await;
    }
    // 初始化预设文件
    if fs::metadata(PRESETS_FILE).await.is_err() {
        let _ = write_json_file(PRESETS_FILE, &Vec::<Preset>::new()).await;
    }
    // 初始化使用量统计
    if fs::metadata(USAGE_STATS_FILE).await.is_err() {
        let _ = write_json_file(USAGE_STATS_FILE, &UsageStats::default()).await;
    }
}
