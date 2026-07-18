use std::sync::RwLock;

pub const TASKMOD_DIR: &str = "/sdcard/TaskMod";
pub const SCHEDULE_FILE: &str = "/sdcard/TaskMod/schedule.conf";
pub const SCRIPTS_DIR: &str = "/sdcard/TaskMod/scripts";
pub const SCREENSHOTS_DIR: &str = "/sdcard/TaskMod/screenshots";
pub const EMAIL_CONF: &str = "/sdcard/TaskMod/email.conf";
pub const WORKFLOWS_DIR: &str = "/sdcard/TaskMod/workflows";
pub const AI_CONF: &str = "/sdcard/TaskMod/ai.conf";
pub const MQTT_CONF: &str = "/sdcard/TaskMod/mqtt.conf";
pub const LOG_FILE: &str = "/data/adb/modules/TaskMod/TaskMod.log";
pub const WEB_PORT: u16 = 9527;

/// 获取实际监听端口，优先使用环境变量 TASKMOD_PORT
pub fn get_listen_port() -> u16 {
    std::env::var("TASKMOD_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(WEB_PORT)
}
#[allow(dead_code)]
pub const MOD_DIR: &str = "/data/adb/modules/TaskMod";

// AI 功能模块目录
pub const SKILLS_DIR: &str = "/sdcard/TaskMod/skills";
pub const MCP_DIR: &str = "/sdcard/TaskMod/mcp";
pub const MEMORY_DIR: &str = "/sdcard/TaskMod/memory";
pub const PROJECTS_DIR: &str = "/sdcard/TaskMod/projects";
pub const CHAT_HISTORY_DIR: &str = "/sdcard/TaskMod/chat_history";
pub const SAVED_ITEMS_DIR: &str = "/sdcard/TaskMod/saved_items";
pub const PRESETS_FILE: &str = "/sdcard/TaskMod/presets.json";
pub const MCP_HISTORY_DIR: &str = "/sdcard/TaskMod/mcp_history";
pub const USAGE_STATS_FILE: &str = "/sdcard/TaskMod/usage_stats.json";
pub const UPLOAD_DIR: &str = "/sdcard/TaskMod/uploads";

// ==================== 可配置结构体 ====================

/// 运行时配置，支持环境变量覆盖
#[derive(Debug, Clone)]
pub struct Config {
    pub skills_dir: String,
    pub mcp_dir: String,
    pub memory_dir: String,
    pub projects_dir: String,
    pub chat_history_dir: String,
    pub saved_items_dir: String,
    pub presets_file: String,
    pub mcp_history_dir: String,
    pub usage_stats_file: String,
    pub upload_dir: String,
    pub prompt_settings_file: String,
    pub scenarios_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            skills_dir: SKILLS_DIR.to_string(),
            mcp_dir: MCP_DIR.to_string(),
            memory_dir: MEMORY_DIR.to_string(),
            projects_dir: PROJECTS_DIR.to_string(),
            chat_history_dir: CHAT_HISTORY_DIR.to_string(),
            saved_items_dir: SAVED_ITEMS_DIR.to_string(),
            presets_file: PRESETS_FILE.to_string(),
            mcp_history_dir: MCP_HISTORY_DIR.to_string(),
            usage_stats_file: USAGE_STATS_FILE.to_string(),
            upload_dir: UPLOAD_DIR.to_string(),
            prompt_settings_file: "/sdcard/TaskMod/prompt_settings.json".to_string(),
            scenarios_file: "/sdcard/TaskMod/scenarios.json".to_string(),
        }
    }
}

impl Config {
    /// 从环境变量加载配置，未设置时使用默认值
    pub fn load() -> Self {
        let default = Self::default();
        Self {
            skills_dir: env_or("TASKMOD_SKILLS_DIR", &default.skills_dir),
            mcp_dir: env_or("TASKMOD_MCP_DIR", &default.mcp_dir),
            memory_dir: env_or("TASKMOD_MEMORY_DIR", &default.memory_dir),
            projects_dir: env_or("TASKMOD_PROJECTS_DIR", &default.projects_dir),
            chat_history_dir: env_or("TASKMOD_CHAT_HISTORY_DIR", &default.chat_history_dir),
            saved_items_dir: env_or("TASKMOD_SAVED_ITEMS_DIR", &default.saved_items_dir),
            presets_file: env_or("TASKMOD_PRESETS_FILE", &default.presets_file),
            mcp_history_dir: env_or("TASKMOD_MCP_HISTORY_DIR", &default.mcp_history_dir),
            usage_stats_file: env_or("TASKMOD_USAGE_STATS_FILE", &default.usage_stats_file),
            upload_dir: env_or("TASKMOD_UPLOAD_DIR", &default.upload_dir),
            prompt_settings_file: env_or(
                "TASKMOD_PROMPT_SETTINGS_FILE",
                &default.prompt_settings_file,
            ),
            scenarios_file: env_or("TASKMOD_SCENARIOS_FILE", &default.scenarios_file),
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// 全局配置缓存（首次访问时加载，后续复用）
static GLOBAL_CONFIG: RwLock<Option<Config>> = RwLock::new(None);

/// 获取全局配置（首次调用时加载，后续直接返回缓存）
pub fn get_config() -> Config {
    // 先尝试读锁
    if let Ok(guard) = GLOBAL_CONFIG.read() {
        if let Some(ref config) = *guard {
            return config.clone();
        }
    }
    // 未初始化，获取写锁并加载
    if let Ok(mut guard) = GLOBAL_CONFIG.write() {
        if guard.is_none() {
            *guard = Some(Config::load());
        }
        return guard.as_ref().unwrap().clone();
    }
    // 极端情况：读写锁都失败，直接返回默认配置
    Config::default()
}
