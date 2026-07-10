//! 统一错误类型定义
//!
//! 使用 thiserror 派生宏，避免手动实现 Display/Error。
//! 所有模块的错误统一汇聚到 TaskModError，便于 main.rs 中统一处理。

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum TaskModError {
    // --- 配置相关 ---
    #[error("配置文件读取失败: {path}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("配置文件解析失败: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("配置字段缺失: {0}")]
    ConfigMissingField(String),

    // --- 进程相关 ---
    #[error("进程启动失败: {0}")]
    ProcessSpawn(#[source] std::io::Error),

    #[error("进程未运行")]
    ProcessNotRunning,

    #[error("进程关闭超时 (pid={pid})")]
    ProcessShutdownTimeout { pid: i32 },

    #[error("信号处理错误: {0}")]
    Signal(#[source] nix::Error),

    // --- IPC 相关 ---
    #[error("Unix Socket 创建失败: {0}")]
    SocketCreate(#[source] std::io::Error),

    #[error("Unix Socket 绑定失败: {path}")]
    SocketBind {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("IPC 消息发送失败: {0}")]
    SocketSend(#[source] std::io::Error),

    #[error("IPC 消息接收失败: {0}")]
    SocketRecv(#[source] std::io::Error),

    #[error("IPC 命令格式无效: {0}")]
    InvalidCommand(String),

    // --- 下载相关 ---
    #[error("HTTP 请求失败: {0}")]
    Http(#[from] reqwest::Error),

    #[error("文件校验失败: 期望 {expected}, 实际 {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("文件写入失败: {0}")]
    FileWrite(#[source] std::io::Error),

    // --- 通用 ---
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("守护进程已在运行 (pid={0})")]
    AlreadyRunning(u32),

    #[error("PID 文件操作失败: {0}")]
    PidFile(String),
}

/// 统一的 Result 类型别名
pub type Result<T> = std::result::Result<T, TaskModError>;