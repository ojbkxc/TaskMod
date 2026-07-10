//! IPC 通信模块
//!
//! 通过 Unix Domain Socket (datagram) 提供外部控制接口
//! 支持多隧道、多服务的增删改查和状态管理

use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};

use log::{error, info, warn};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TaskModError};

/// IPC Socket 路径
pub const SOCKET_PATH: &str = "/tmp/taskmod.sock";

/// PID 文件路径
pub const PID_FILE_PATH: &str = "/tmp/taskmod.pid";

/// IPC 命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // ========== 守护进程控制 ==========
    /// 查询守护进程整体状态
    Status,
    /// 停止守护进程
    Stop,
    /// 重启所有隧道
    RestartAll,

    // ========== 隧道管理 ==========
    /// 列出所有隧道
    ListTunnels,
    /// 获取隧道详情
    GetTunnel { name: String },
    /// 添加隧道
    AddTunnel { name: String, token: String, enabled: bool },
    /// 更新隧道
    UpdateTunnel { name: String, new_name: Option<String>, token: Option<String>, enabled: Option<bool> },
    /// 删除隧道
    DeleteTunnel { name: String },
    /// 启用隧道
    EnableTunnel { name: String },
    /// 禁用隧道
    DisableTunnel { name: String },
    /// 重启指定隧道
    RestartTunnel { name: String },

    // ========== 服务管理 ==========
    /// 列出隧道下的服务
    ListServices { tunnel_name: String },
    /// 获取服务详情
    GetService { tunnel_name: String, service_name: String },
    /// 添加服务
    AddService { tunnel_name: String, service_name: String, url: String, enabled: bool },
    /// 更新服务
    UpdateService { tunnel_name: String, service_name: String, new_name: Option<String>, url: Option<String>, enabled: Option<bool> },
    /// 删除服务
    DeleteService { tunnel_name: String, service_name: String },
    /// 启用服务
    EnableService { tunnel_name: String, service_name: String },
    /// 禁用服务
    DisableService { tunnel_name: String, service_name: String },

    // ========== 进程状态 ==========
    /// 列出所有运行中的进程
    ListProcesses,
    /// 获取指定隧道的进程状态
    GetProcessStatus { tunnel_name: String },
    /// 启动指定隧道
    StartTunnel { name: String },
    /// 停止指定隧道
    StopTunnel { name: String },
}

/// IPC 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// 成功
    Success(String),
    /// 错误
    Error(String),
    /// JSON 数据
    Json(serde_json::Value),
}

/// 创建 IPC socket
pub fn create_socket() -> Result<UnixDatagram> {
    let path = Path::new(SOCKET_PATH);

    if path.exists() {
        std::fs::remove_file(path).map_err(|e| TaskModError::SocketBind {
            path: path.to_path_buf(),
            source: e,
        })?;
    }

    let socket = UnixDatagram::bind(path).map_err(|e| TaskModError::SocketBind {
        path: path.to_path_buf(),
        source: e,
    })?;

    socket.set_nonblocking(true).map_err(TaskModError::Io)?;

    info!("IPC socket 已创建: {}", SOCKET_PATH);
    Ok(socket)
}

/// 从 socket 接收命令（非阻塞）
pub fn recv_command(socket: &UnixDatagram) -> Option<(Command, PathBuf)> {
    let mut buf = [0u8; 4096];

    match socket.recv_from(&mut buf) {
        Ok((size, sender_addr)) => {
            let msg = String::from_utf8_lossy(&buf[..size]);
            match serde_json::from_str::<Command>(&msg) {
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
pub fn send_response(socket: &UnixDatagram, target: &Path, response: &Response) {
    let json = serde_json::to_string(response).unwrap_or_else(|_| {
        r#"{"Error":"序列化失败"}"#.to_string()
    });

    if let Err(e) = socket.send_to(json.as_bytes(), target) {
        error!("IPC 响应发送失败: {}", e);
    }
}

/// 发送成功响应
pub fn send_success(socket: &UnixDatagram, target: &Path, msg: &str) {
    send_response(socket, target, &Response::Success(msg.to_string()));
}

/// 发送错误响应
pub fn send_error(socket: &UnixDatagram, target: &Path, msg: &str) {
    send_response(socket, target, &Response::Error(msg.to_string()));
}

/// 发送 JSON 响应
pub fn send_json(socket: &UnixDatagram, target: &Path, data: serde_json::Value) {
    send_response(socket, target, &Response::Json(data));
}

// ========== PID 文件管理 ==========

/// 写入 PID 文件
pub fn write_pid_file(pid: u32) -> Result<()> {
    std::fs::write(PID_FILE_PATH, pid.to_string())
        .map_err(|e| TaskModError::PidFile(format!("写入失败: {}", e)))?;
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
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}

/// 检查守护进程是否已在运行
pub fn check_existing_instance() -> Option<u32> {
    if let Some(pid) = read_pid_file() {
        if is_pid_alive(pid) {
            return Some(pid);
        }
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
pub fn client_send_command(cmd: &Command) -> Result<Response> {
    if check_existing_instance().is_none() {
        return Err(TaskModError::ProcessNotRunning);
    }

    let socket = UnixDatagram::bind("").map_err(TaskModError::SocketCreate)?;
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(TaskModError::Io)?;

    // 发送命令
    let json = serde_json::to_string(cmd)
        .map_err(|e| TaskModError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

    socket
        .send_to(json.as_bytes(), SOCKET_PATH)
        .map_err(TaskModError::SocketSend)?;

    // 接收响应
    let mut buf = [0u8; 4096];
    let size = socket.recv(&mut buf).map_err(TaskModError::SocketRecv)?;

    let response: Response = serde_json::from_slice(&buf[..size])
        .map_err(|e| TaskModError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

    Ok(response)
}