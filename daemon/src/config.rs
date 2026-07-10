//! 配置文件解析模块
//!
//! 支持从 TOML 文件加载配置，路径优先级：
//! 1. 环境变量 TASKMOD_CONFIG
//! 2. 默认路径 ~/.taskmod/config.toml
//!
//! 解析失败直接报错退出，不做容错。

use std::env;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{Result, TaskModError};

/// 顶层配置结构体
#[derive(Debug, Deserialize)]
pub struct Config {
    pub tunnel: TunnelConfig,

    /// cloudflared 二进制版本号
    #[serde(default = "default_version")]
    pub version: String,

    /// 子进程崩溃后重试间隔（秒）
    #[serde(default = "default_retry_interval")]
    pub retry_interval_secs: u64,

    /// 最大重试次数，0 表示无限重试
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// graceful shutdown 超时（秒）
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
}

/// cloudflared 隧道配置
#[derive(Debug, Deserialize)]
pub struct TunnelConfig {
    /// Cloudflare Tunnel Token
    pub token: String,

    /// 隧道指向的本地服务 URL
    #[serde(default = "default_url")]
    pub url: String,
}

fn default_version() -> String {
    "2024.10.1".to_string()
}

fn default_retry_interval() -> u64 {
    5
}

fn default_max_retries() -> u32 {
    10
}

fn default_shutdown_timeout() -> u64 {
    10
}

fn default_url() -> String {
    "http://localhost:8080".to_string()
}

/// 获取配置文件路径
///
/// 优先使用 TASKMOD_CONFIG 环境变量，否则使用 ~/.taskmod/config.toml
pub fn config_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("TASKMOD_CONFIG") {
        return Ok(PathBuf::from(path));
    }

    let home = dirs_next::home_dir().ok_or_else(|| {
        TaskModError::ConfigMissingField("无法确定用户主目录".to_string())
    })?;

    Ok(home.join(".taskmod").join("config.toml"))
}

/// 从文件加载配置
pub fn load_config() -> Result<Config> {
    let path = config_path()?;

    let content = fs::read_to_string(&path).map_err(|e| TaskModError::ConfigRead {
        path: path.clone(),
        source: e,
    })?;

    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

/// 获取 taskmod 数据目录 (~/.taskmod)
pub fn data_dir() -> Result<PathBuf> {
    let home = dirs_next::home_dir().ok_or_else(|| {
        TaskModError::ConfigMissingField("无法确定用户主目录".to_string())
    })?;

    let dir = home.join(".taskmod");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| TaskModError::Io(e))?;
    }

    Ok(dir)
}

/// 获取 cloudflared 二进制路径
pub fn cloudflared_bin_path(version: &str) -> Result<PathBuf> {
    let dir = data_dir()?;
    Ok(dir.join("bin").join(format!("cloudflared-{}", version)))
}