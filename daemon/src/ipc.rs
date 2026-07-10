//! IPC 通信模块
//!
//! 通过 Unix Domain Socket (datagram) 提供外部控制接口。
//! 守护进程监听 /tmp/taskmod.sock，客户端通过同名 socket 发送命令。
//!
//! 支持的命令：
//! - STATUS: 返回进程状态 JSON
//! - STOP: 优雅关闭守护进程
//! - RESTART: 触发热重载（先启动新进程，再关闭旧进程）
//!
//! 使用 datagram 而非 stream，因为：
//! 1. 消息边界清晰，无需处理粘包
//! 2. 无需维护连接状态
//! 3. 实现更简单

use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};

use log::{error, info, warn};
use serde::Serialize;

use crate::error::{Result, TaskModError};

/// IPC Socket 路径
pub const SOCKET_PATH: &str = "/tmp/taskmod.sock";

/// PID 文件路径
pub const PID_FILE_PATH: &str = "/tmp/taskmod.pid";

/// 守护进程支持的 IPC 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Status,
    Stop,
    Restart,
}

/// 进程状态响应（序列化为 JSON 返回给客户端）
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub pid: u32,
    pub uptime_secs: u64,
}

impl Command {
    /// 从字符串解析命令
    pub fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_uppercase().as_str() {
            "STATUS" => Ok(Command::Status),
            "STOP" => Ok(Command::Stop),
            "RESTART" => Ok(Command::Restart),
            _ => Err(TaskModError::InvalidCommand(s.to_string())),
        }
    }
}

/// 创建 IPC socket 并绑定
///
/// 如果 socket 文件已存在（上次未清理），先删除再创建
pub fn create_socket() -> Result<UnixDatagram> {
    let path = Path::new(SOCKET_PATH);

    // 清理可能残留的旧 socket 文件
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| TaskModError::SocketBind {
            path: path.to_path_buf(),
            source: e,
        })?;
    }

    let socket =
        UnixDatagram::bind(path).map_err(|e| TaskModError::SocketBind {
            path: path.to_path_buf(),
            source: e,
        })?;

    // 设置为非阻塞模式，避免 epoll_wait 被 socket 阻塞
    socket
        .set_nonblocking(true)
        .map_err(TaskModError::Io)?;

    info!("IPC socket 已创建: {}", SOCKET_PATH);
    Ok(socket)
}

/// 从 socket 接收命令（非阻塞）
///
/// 返回 (命令, 客户端地址) 或 None（无数据时）
pub fn recv_command(socket: &UnixDatagram) -> Option<(Command, PathBuf)> {
    let mut buf = [0u8; 1024];

    match socket.recv_from(&mut buf) {
        Ok((size, sender_addr)) => {
            let msg = String::from_utf8_lossy(&buf[..size]);
            match Command::from_str(&msg) {
                Ok(cmd) => {
                    info!("收到 IPC 命令: {:?}", cmd);
                    let path = match sender_addr.as_pathname() {
                        Some(p) => p.to_path_buf(),
                        None => PathBuf::from(SOCKET_PATH),
                    };
                    Some((cmd, path))
                }
                Err(e) => {
                    warn!("无效的 IPC 命令: {} - {}", msg, e);
                    None
                }
            }
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
        Err(e) => {
            error!("IPC 接收错误: {}", e);
            None
        }
    }
}

/// 向客户端发送响应
pub fn send_response(socket: &UnixDatagram, target: &Path, response: &str) {
    if let Err(e) = socket.send_to(response.as_bytes(), target) {
        error!("IPC 响应发送失败: {}", e);
    }
}

// --- PID 文件管理 ---

/// 写入 PID 文件
pub fn write_pid_file(pid: u32) -> Result<()> {
    std::fs::write(PID_FILE_PATH, pid.to_string()).map_err(|e| {
        TaskModError::PidFile(format!("写入失败: {}", e))
    })?;
    info!("PID 文件已写入: {} (pid={})", PID_FILE_PATH, pid);
    Ok(())
}

/// 读取 PID 文件
pub fn read_pid_file() -> Option<u32> {
    std::fs::read_to_string(PID_FILE_PATH)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
}

/// 检查 PID 对应的进程是否存活
fn is_pid_alive(pid: u32) -> bool {
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        None,
    )
    .is_ok()
}

/// 检查守护进程是否已在运行
///
/// 1. 读取 PID 文件
/// 2. 检查对应进程是否存活
/// 3. 若存活则返回 PID，否则清理残留文件
pub fn check_existing_instance() -> Option<u32> {
    if let Some(pid) = read_pid_file() {
        if is_pid_alive(pid) {
            return Some(pid);
        }
        // 进程已死，清理残留
        let _ = std::fs::remove_file(PID_FILE_PATH);
    }
    None
}

/// 清理 PID 文件和 socket 文件
pub fn cleanup() {
    let _ = std::fs::remove_file(PID_FILE_PATH);
    let _ = std::fs::remove_file(SOCKET_PATH);
}

/// 客户端：发送命令到守护进程
///
/// 用于 taskmod stop/status/restart 子命令
pub fn client_send_command(cmd: &str) -> Result<String> {
    // 检查守护进程是否在运行
    if check_existing_instance().is_none() {
        return Err(TaskModError::ProcessNotRunning);
    }

    let socket = UnixDatagram::bind("").map_err(|e| TaskModError::SocketCreate(e))?;
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(TaskModError::Io)?;

    // 发送命令
    socket
        .send_to(cmd.as_bytes(), SOCKET_PATH)
        .map_err(TaskModError::SocketSend)?;

    // 接收响应
    let mut buf = [0u8; 4096];
    let size = socket.recv(&mut buf).map_err(TaskModError::SocketRecv)?;

    String::from_utf8_lossy(&buf[..size])
        .into_owned()
        .pipe(|s| Ok(s))
}

/// 辅助 trait，用于链式调用
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}