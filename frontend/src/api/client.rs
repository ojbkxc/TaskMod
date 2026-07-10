use serde::{Deserialize, Serialize};

/// API 基础 URL（相对路径，因为前端和后端在同一服务器）
const API_BASE: &str = "/api";

/// 通用 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

/// 系统状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemStatus {
    pub battery: Option<BatteryInfo>,
    pub cpu_usage: Option<f64>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
    pub uptime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatteryInfo {
    pub level: u32,
    pub status: String,
    pub temperature: f64,
}

/// 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub command: String,
    pub schedule: String,
    pub enabled: bool,
}

/// 脚本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub name: String,
    pub size: u64,
    pub modified: String,
}

/// AI 提供商
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiProvider {
    pub id: String,
    pub name: String,
    pub api_url: String,
    pub model: String,
}

/// 获取系统状态
pub async fn get_status() -> Result<SystemStatus, reqwest::Error> {
    let url = format!("{}/status", API_BASE);
    let resp: ApiResponse<SystemStatus> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 获取任务列表
pub async fn get_tasks() -> Result<Vec<Task>, reqwest::Error> {
    let url = format!("{}/tasks", API_BASE);
    let resp: ApiResponse<Vec<Task>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 获取脚本列表
pub async fn get_scripts() -> Result<Vec<Script>, reqwest::Error> {
    let url = format!("{}/scripts", API_BASE);
    let resp: ApiResponse<Vec<Script>> = reqwest::get(&url).await?.json().await?;
    Ok(resp.data.unwrap_or_default())
}

/// 获取日志
pub async fn get_logs() -> Result<String, reqwest::Error> {
    let url = format!("{}/logs", API_BASE);
    let text = reqwest::get(&url).await?.text().await?;
    Ok(text)
}
