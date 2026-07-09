use axum::{Json, extract::Path, extract::Query};
use serde::{Deserialize, Serialize};
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
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionReq {
    pub title: Option<String>,
    pub provider_id: String,
    pub provider_name: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionReq {
    pub title: Option<String>,
    pub messages: Option<Vec<serde_json::Value>>,
    pub pinned: Option<bool>,
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
    pub content: String,
    #[serde(default)]
    pub category: String,
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
    pub category: Option<String>,
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
        content: req.content,
        category: req.category.unwrap_or_default(),
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
    if let Some(cat) = req.category { mem.category = cat; }
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
    pub prompt_template: String,
    pub variables: Option<Vec<SkillVariable>>,
    pub enabled: Option<bool>,
    pub category: Option<String>,
}

fn skill_path(id: &str) -> String {
    format!("{}/{}.json", SKILLS_DIR, id)
}

/// 获取所有已启用的Skill（同步，供AI模块调用）
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
        prompt_template: req.prompt_template,
        variables: req.variables.unwrap_or_default(),
        enabled: req.enabled.unwrap_or(true),
        category: req.category.unwrap_or_default(),
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
    skill.prompt_template = req.prompt_template;
    if let Some(vars) = req.variables { skill.variables = vars; }
    if let Some(enabled) = req.enabled { skill.enabled = enabled; }
    if let Some(cat) = req.category { skill.category = cat; }
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
}

fn mcp_path(id: &str) -> String {
    format!("{}/{}.json", MCP_DIR, id)
}

/// 获取所有已启用的MCP服务器（同步）
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

// ==================== 统一初始化 ====================

pub async fn init_dirs() {
    for dir in &[SKILLS_DIR, MCP_DIR, MEMORY_DIR, PROJECTS_DIR, CHAT_HISTORY_DIR, SAVED_ITEMS_DIR] {
        let _ = fs::create_dir_all(dir).await;
    }
    // 初始化预设文件
    if fs::metadata(PRESETS_FILE).await.is_err() {
        let _ = write_json_file(PRESETS_FILE, &Vec::<Preset>::new()).await;
    }
}
