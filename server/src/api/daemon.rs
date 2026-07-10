//! 守护进程 API 模块
//!
//! 提供完整的隧道和服务管理 API
//! 通过 Unix Socket IPC 与 taskmod-daemon 通信

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::os::unix::net::UnixDatagram;
use std::time::Duration;

use crate::state::AppState;

/// IPC Socket 路径
const SOCKET_PATH: &str = "/tmp/taskmod.sock";

/// IPC 命令（与 daemon 同步）
#[derive(Debug, Serialize, Deserialize)]
enum IpcCommand {
    Status,
    Stop,
    RestartAll,
    ListTunnels,
    GetTunnel { name: String },
    AddTunnel { name: String, token: String, enabled: bool },
    UpdateTunnel { name: String, new_name: Option<String>, token: Option<String>, enabled: Option<bool> },
    DeleteTunnel { name: String },
    EnableTunnel { name: String },
    DisableTunnel { name: String },
    RestartTunnel { name: String },
    ListServices { tunnel_name: String },
    GetService { tunnel_name: String, service_name: String },
    AddService { tunnel_name: String, service_name: String, url: String, enabled: bool },
    UpdateService { tunnel_name: String, service_name: String, new_name: Option<String>, url: Option<String>, enabled: Option<bool> },
    DeleteService { tunnel_name: String, service_name: String },
    EnableService { tunnel_name: String, service_name: String },
    DisableService { tunnel_name: String, service_name: String },
    ListProcesses,
    GetProcessStatus { tunnel_name: String },
    StartTunnel { name: String },
    StopTunnel { name: String },
}

/// IPC 响应
#[derive(Debug, Serialize, Deserialize)]
enum IpcResponse {
    Success(String),
    Error(String),
    Json(serde_json::Value),
}

/// API 响应包装
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// 创建隧道管理 API 路由
pub fn routes() -> Router<AppState> {
    Router::new()
        // 守护进程控制
        .route("/api/daemon/status", get(get_status))
        .route("/api/daemon/stop", post(stop_daemon))
        .route("/api/daemon/restart", post(restart_all))
        // 隧道管理
        .route("/api/tunnels", get(list_tunnels).post(add_tunnel))
        .route("/api/tunnels/:name", get(get_tunnel).put(update_tunnel).delete(delete_tunnel))
        .route("/api/tunnels/:name/enable", post(enable_tunnel))
        .route("/api/tunnels/:name/disable", post(disable_tunnel))
        .route("/api/tunnels/:name/restart", post(restart_tunnel))
        .route("/api/tunnels/:name/start", post(start_tunnel))
        .route("/api/tunnels/:name/stop", post(stop_tunnel))
        // 服务管理
        .route("/api/tunnels/:tunnel_name/services", get(list_services).post(add_service))
        .route("/api/tunnels/:tunnel_name/services/:service_name", get(get_service).put(update_service).delete(delete_service))
        .route("/api/tunnels/:tunnel_name/services/:service_name/enable", post(enable_service))
        .route("/api/tunnels/:tunnel_name/services/:service_name/disable", post(disable_service))
        // 进程状态
        .route("/api/processes", get(list_processes))
        .route("/api/processes/:tunnel_name", get(get_process_status))
}

/// 查询守护进程状态
async fn get_status() -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::Status) {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse { success: true, data: Some(data), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 停止守护进程
async fn stop_daemon() -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::Stop) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 重启所有隧道
async fn restart_all() -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::RestartAll) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 列出所有隧道
async fn list_tunnels() -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::ListTunnels) {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse { success: true, data: Some(data), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 获取隧道详情
async fn get_tunnel(Path(name): Path<String>) -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::GetTunnel { name }) {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse { success: true, data: Some(data), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 添加隧道请求
#[derive(Deserialize)]
struct AddTunnelRequest {
    name: String,
    token: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_true() -> bool { true }

/// 添加隧道
async fn add_tunnel(Json(req): Json<AddTunnelRequest>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::AddTunnel { name: req.name, token: req.token, enabled: req.enabled }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 更新隧道请求
#[derive(Deserialize)]
struct UpdateTunnelRequest {
    new_name: Option<String>,
    token: Option<String>,
    enabled: Option<bool>,
}

/// 更新隧道
async fn update_tunnel(
    Path(name): Path<String>,
    Json(req): Json<UpdateTunnelRequest>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::UpdateTunnel {
        name,
        new_name: req.new_name,
        token: req.token,
        enabled: req.enabled,
    }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 删除隧道
async fn delete_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DeleteTunnel { name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 启用隧道
async fn enable_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::EnableTunnel { name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 禁用隧道
async fn disable_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DisableTunnel { name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 重启隧道
async fn restart_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::RestartTunnel { name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 启动隧道进程
async fn start_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::StartTunnel { name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 停止隧道进程
async fn stop_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::StopTunnel { name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 列出隧道下的服务
async fn list_services(Path(tunnel_name): Path<String>) -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::ListServices { tunnel_name }) {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse { success: true, data: Some(data), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 获取服务详情
async fn get_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
) -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::GetService { tunnel_name, service_name }) {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse { success: true, data: Some(data), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 添加服务请求
#[derive(Deserialize)]
struct AddServiceRequest {
    name: String,
    url: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

/// 添加服务
async fn add_service(
    Path(tunnel_name): Path<String>,
    Json(req): Json<AddServiceRequest>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::AddService {
        tunnel_name,
        service_name: req.name,
        url: req.url,
        enabled: req.enabled,
    }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 更新服务请求
#[derive(Deserialize)]
struct UpdateServiceRequest {
    new_name: Option<String>,
    url: Option<String>,
    enabled: Option<bool>,
}

/// 更新服务
async fn update_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
    Json(req): Json<UpdateServiceRequest>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::UpdateService {
        tunnel_name,
        service_name,
        new_name: req.new_name,
        url: req.url,
        enabled: req.enabled,
    }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 删除服务
async fn delete_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DeleteService { tunnel_name, service_name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 启用服务
async fn enable_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::EnableService { tunnel_name, service_name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 禁用服务
async fn disable_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DisableService { tunnel_name, service_name }) {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse { success: true, data: Some(msg), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 列出所有进程
async fn list_processes() -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::ListProcesses) {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse { success: true, data: Some(data), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 获取指定隧道进程状态
async fn get_process_status(Path(tunnel_name): Path<String>) -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::GetProcessStatus { tunnel_name }) {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse { success: true, data: Some(data), error: None }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        Err(e) => Json(ApiResponse { success: false, data: None, error: Some(e) }),
        _ => Json(ApiResponse { success: false, data: None, error: Some("未知响应".to_string()) }),
    }
}

/// 通过 Unix Socket 发送 IPC 命令
fn send_ipc(cmd: &IpcCommand) -> Result<IpcResponse, String> {
    let socket = UnixDatagram::bind("")
        .map_err(|e| format!("创建 socket 失败: {}", e))?;

    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("设置超时失败: {}", e))?;

    let json = serde_json::to_string(cmd)
        .map_err(|e| format!("序列化命令失败: {}", e))?;

    socket
        .send_to(json.as_bytes(), SOCKET_PATH)
        .map_err(|e| format!("发送命令失败: {}", e))?;

    let mut buf = [0u8; 65536];
    let size = socket
        .recv(&mut buf)
        .map_err(|e| format!("接收响应失败: {}", e))?;

    serde_json::from_slice(&buf[..size])
        .map_err(|e| format!("解析响应失败: {}", e))
}