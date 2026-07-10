//! 配置文件解析模块
//!
//! 支持多隧道、多服务的配置结构
//! 支持动态增删改查操作

use std::env;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TaskModError};

/// 顶层配置结构体
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// 全局设置
    pub global: GlobalConfig,

    /// 隧道列表
    #[serde(default)]
    pub tunnels: Vec<TunnelConfig>,
}

/// 全局设置
#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalConfig {
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

/// 隧道配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TunnelConfig {
    /// 隧道名称（唯一标识）
    pub name: String,

    /// Cloudflare Tunnel Token
    pub token: String,

    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// 该隧道绑定的服务列表
    #[serde(default)]
    pub services: Vec<ServiceConfig>,
}

/// 服务配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServiceConfig {
    /// 服务名称（唯一标识）
    pub name: String,

    /// 本地服务 URL
    pub url: String,

    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// 默认值函数
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

fn default_true() -> bool {
    true
}

/// 获取配置文件路径
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

    if !path.exists() {
        // 如果配置文件不存在，返回默认配置
        return Ok(Config {
            global: GlobalConfig {
                version: default_version(),
                retry_interval_secs: default_retry_interval(),
                max_retries: default_max_retries(),
                shutdown_timeout_secs: default_shutdown_timeout(),
            },
            tunnels: Vec::new(),
        });
    }

    let content = fs::read_to_string(&path).map_err(|e| TaskModError::ConfigRead {
        path: path.clone(),
        source: e,
    })?;

    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

/// 保存配置到文件
pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;

    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(TaskModError::Io)?;
    }

    let content = toml::to_string_pretty(config)
        .map_err(|e| TaskModError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

    fs::write(&path, content).map_err(TaskModError::Io)?;

    Ok(())
}

/// 获取 taskmod 数据目录 (~/.taskmod)
pub fn data_dir() -> Result<PathBuf> {
    let home = dirs_next::home_dir().ok_or_else(|| {
        TaskModError::ConfigMissingField("无法确定用户主目录".to_string())
    })?;

    let dir = home.join(".taskmod");
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(TaskModError::Io)?;
    }

    Ok(dir)
}

/// 获取 cloudflared 二进制路径
pub fn cloudflared_bin_path(version: &str) -> Result<PathBuf> {
    let dir = data_dir()?;
    Ok(dir.join("bin").join(format!("cloudflared-{}", version)))
}

/// 配置管理器，提供增删改查操作
pub struct ConfigManager {
    config: Config,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new() -> Result<Self> {
        let config = load_config()?;
        Ok(ConfigManager { config })
    }

    /// 获取配置引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取可变配置引用
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 保存配置
    pub fn save(&self) -> Result<()> {
        save_config(&self.config)
    }

    // ========== 隧道管理 ==========

    /// 获取所有隧道
    pub fn list_tunnels(&self) -> &[TunnelConfig] {
        &self.config.tunnels
    }

    /// 根据名称获取隧道
    pub fn get_tunnel(&self, name: &str) -> Option<&TunnelConfig> {
        self.config.tunnels.iter().find(|t| t.name == name)
    }

    /// 添加隧道
    pub fn add_tunnel(&mut self, tunnel: TunnelConfig) -> Result<()> {
        // 检查名称是否重复
        if self.config.tunnels.iter().any(|t| t.name == tunnel.name) {
            return Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 已存在", tunnel.name),
            ));
        }
        self.config.tunnels.push(tunnel);
        self.save()
    }

    /// 更新隧道
    pub fn update_tunnel(&mut self, name: &str, tunnel: TunnelConfig) -> Result<()> {
        let index = self.config.tunnels.iter().position(|t| t.name == name);
        match index {
            Some(i) => {
                // 如果名称改变，检查新名称是否重复
                if tunnel.name != name
                    && self.config.tunnels.iter().any(|t| t.name == tunnel.name)
                {
                    return Err(TaskModError::ConfigMissingField(
                        format!("隧道 '{}' 已存在", tunnel.name),
                    ));
                }
                self.config.tunnels[i] = tunnel;
                self.save()
            }
            None => Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 不存在", name),
            )),
        }
    }

    /// 删除隧道
    pub fn delete_tunnel(&mut self, name: &str) -> Result<()> {
        let index = self.config.tunnels.iter().position(|t| t.name == name);
        match index {
            Some(i) => {
                self.config.tunnels.remove(i);
                self.save()
            }
            None => Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 不存在", name),
            )),
        }
    }

    /// 启用/禁用隧道
    pub fn set_tunnel_enabled(&mut self, name: &str, enabled: bool) -> Result<()> {
        let tunnel = self.config.tunnels.iter_mut().find(|t| t.name == name);
        match tunnel {
            Some(t) => {
                t.enabled = enabled;
                self.save()
            }
            None => Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 不存在", name),
            )),
        }
    }

    /// 更新隧道 Token
    pub fn update_tunnel_token(&mut self, name: &str, token: String) -> Result<()> {
        let tunnel = self.config.tunnels.iter_mut().find(|t| t.name == name);
        match tunnel {
            Some(t) => {
                t.token = token;
                self.save()
            }
            None => Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 不存在", name),
            )),
        }
    }

    // ========== 服务管理 ==========

    /// 获取隧道下的所有服务
    pub fn list_services(&self, tunnel_name: &str) -> Result<&[ServiceConfig]> {
        let tunnel = self.get_tunnel(tunnel_name).ok_or_else(|| {
            TaskModError::ConfigMissingField(format!("隧道 '{}' 不存在", tunnel_name))
        })?;
        Ok(&tunnel.services)
    }

    /// 获取指定服务
    pub fn get_service(
        &self,
        tunnel_name: &str,
        service_name: &str,
    ) -> Option<&ServiceConfig> {
        self.get_tunnel(tunnel_name).and_then(|t| {
            t.services.iter().find(|s| s.name == service_name)
        })
    }

    /// 添加服务到隧道
    pub fn add_service(
        &mut self,
        tunnel_name: &str,
        service: ServiceConfig,
    ) -> Result<()> {
        let tunnel = self.config.tunnels.iter_mut().find(|t| t.name == tunnel_name);
        match tunnel {
            Some(t) => {
                // 检查服务名称是否重复
                if t.services.iter().any(|s| s.name == service.name) {
                    return Err(TaskModError::ConfigMissingField(
                        format!("服务 '{}' 已存在于隧道 '{}'", service.name, tunnel_name),
                    ));
                }
                t.services.push(service);
                self.save()
            }
            None => Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 不存在", tunnel_name),
            )),
        }
    }

    /// 更新服务
    pub fn update_service(
        &mut self,
        tunnel_name: &str,
        service_name: &str,
        service: ServiceConfig,
    ) -> Result<()> {
        let tunnel = self.config.tunnels.iter_mut().find(|t| t.name == tunnel_name);
        match tunnel {
            Some(t) => {
                let index = t.services.iter().position(|s| s.name == service_name);
                match index {
                    Some(i) => {
                        // 如果名称改变，检查新名称是否重复
                        if service.name != service_name
                            && t.services.iter().any(|s| s.name == service.name)
                        {
                            return Err(TaskModError::ConfigMissingField(
                                format!("服务 '{}' 已存在于隧道 '{}'", service.name, tunnel_name),
                            ));
                        }
                        t.services[i] = service;
                        self.save()
                    }
                    None => Err(TaskModError::ConfigMissingField(
                        format!("服务 '{}' 不存在于隧道 '{}'", service_name, tunnel_name),
                    )),
                }
            }
            None => Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 不存在", tunnel_name),
            )),
        }
    }

    /// 删除服务
    pub fn delete_service(
        &mut self,
        tunnel_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let tunnel = self.config.tunnels.iter_mut().find(|t| t.name == tunnel_name);
        match tunnel {
            Some(t) => {
                let index = t.services.iter().position(|s| s.name == service_name);
                match index {
                    Some(i) => {
                        t.services.remove(i);
                        self.save()
                    }
                    None => Err(TaskModError::ConfigMissingField(
                        format!("服务 '{}' 不存在于隧道 '{}'", service_name, tunnel_name),
                    )),
                }
            }
            None => Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 不存在", tunnel_name),
            )),
        }
    }

    /// 启用/禁用服务
    pub fn set_service_enabled(
        &mut self,
        tunnel_name: &str,
        service_name: &str,
        enabled: bool,
    ) -> Result<()> {
        let tunnel = self.config.tunnels.iter_mut().find(|t| t.name == tunnel_name);
        match tunnel {
            Some(t) => {
                let service = t.services.iter_mut().find(|s| s.name == service_name);
                match service {
                    Some(s) => {
                        s.enabled = enabled;
                        self.save()
                    }
                    None => Err(TaskModError::ConfigMissingField(
                        format!("服务 '{}' 不存在于隧道 '{}'", service_name, tunnel_name),
                    )),
                }
            }
            None => Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 不存在", tunnel_name),
            )),
        }
    }
}