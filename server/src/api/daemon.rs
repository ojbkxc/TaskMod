//! 守护进程 API 模块
//!
//! 提供完整的隧道和服务管理 API
//! 通过 Unix Socket/TCP Socket IPC 与 taskmod-daemon 通信

use crate::data::response::ApiResponse;
use axum::{
    extract::Path,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::UnixDatagram;
#[cfg(windows)]
use tokio::net::TcpStream;

const SOCKET_PATH: &str = "/tmp/taskmod.sock";
#[allow(dead_code)]
const TCP_IPC_ADDR: &str = "127.0.0.1:8765";

#[derive(Debug, Serialize, Deserialize)]
enum IpcCommand {
    Status,
    Stop,
    RestartAll,
    GetCloudflaredStatus,
    DownloadCloudflared {
        version: String,
    },
    ListCloudflaredVersions,
    ListTunnels,
    GetTunnel {
        name: String,
    },
    AddTunnel {
        name: String,
        token: String,
        enabled: bool,
    },
    UpdateTunnel {
        name: String,
        new_name: Option<String>,
        token: Option<String>,
        enabled: Option<bool>,
    },
    DeleteTunnel {
        name: String,
    },
    EnableTunnel {
        name: String,
    },
    DisableTunnel {
        name: String,
    },
    RestartTunnel {
        name: String,
    },
    ListServices {
        tunnel_name: String,
    },
    GetService {
        tunnel_name: String,
        service_name: String,
    },
    AddService {
        tunnel_name: String,
        service_name: String,
        url: String,
        enabled: bool,
    },
    UpdateService {
        tunnel_name: String,
        service_name: String,
        new_name: Option<String>,
        url: Option<String>,
        enabled: Option<bool>,
    },
    DeleteService {
        tunnel_name: String,
        service_name: String,
    },
    EnableService {
        tunnel_name: String,
        service_name: String,
    },
    DisableService {
        tunnel_name: String,
        service_name: String,
    },
    ListProcesses,
    GetProcessStatus {
        tunnel_name: String,
    },
    StartTunnel {
        name: String,
    },
    StopTunnel {
        name: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
enum IpcResponse {
    Success(String),
    Error(String),
    Json(serde_json::Value),
}

pub fn routes() -> Router<()> {
    Router::new()
        .route("/api/daemon/status", get(get_status))
        .route("/api/daemon/stop", post(stop_daemon))
        .route("/api/daemon/restart", post(restart_all))
        .route(
            "/api/daemon/cloudflared/status",
            get(get_cloudflared_status),
        )
        .route(
            "/api/daemon/cloudflared/download",
            post(download_cloudflared),
        )
        .route(
            "/api/daemon/cloudflared/versions",
            get(list_cloudflared_versions),
        )
        .route("/api/tunnels", get(list_tunnels).post(add_tunnel))
        .route(
            "/api/tunnels/:name",
            get(get_tunnel).put(update_tunnel).delete(delete_tunnel),
        )
        .route("/api/tunnels/:name/enable", post(enable_tunnel))
        .route("/api/tunnels/:name/disable", post(disable_tunnel))
        .route("/api/tunnels/:name/restart", post(restart_tunnel))
        .route("/api/tunnels/:name/start", post(start_tunnel))
        .route("/api/tunnels/:name/stop", post(stop_tunnel))
        .route(
            "/api/tunnels/:tunnel_name/services",
            get(list_services).post(add_service),
        )
        .route(
            "/api/tunnels/:tunnel_name/services/:service_name",
            get(get_service).put(update_service).delete(delete_service),
        )
        .route(
            "/api/tunnels/:tunnel_name/services/:service_name/enable",
            post(enable_service),
        )
        .route(
            "/api/tunnels/:tunnel_name/services/:service_name/disable",
            post(disable_service),
        )
        .route("/api/processes", get(list_processes))
        .route("/api/processes/:tunnel_name", get(get_process_status))
}

fn default_true() -> bool {
    true
}

async fn get_status() -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::Status).await {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn stop_daemon() -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::Stop).await {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn restart_all() -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::RestartAll).await {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn get_cloudflared_status() -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::GetCloudflaredStatus).await {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

#[derive(Deserialize)]
struct DownloadCloudflaredRequest {
    version: String,
}

async fn download_cloudflared(
    Json(req): Json<DownloadCloudflaredRequest>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DownloadCloudflared {
        version: req.version,
    })
    .await
    {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn list_cloudflared_versions() -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::ListCloudflaredVersions).await {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn list_tunnels() -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::ListTunnels).await {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn get_tunnel(Path(name): Path<String>) -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::GetTunnel { name }).await {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

#[derive(Deserialize)]
struct AddTunnelRequest {
    name: String,
    token: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

async fn add_tunnel(Json(req): Json<AddTunnelRequest>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::AddTunnel {
        name: req.name,
        token: req.token,
        enabled: req.enabled,
    })
    .await
    {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

#[derive(Deserialize)]
struct UpdateTunnelRequest {
    new_name: Option<String>,
    token: Option<String>,
    enabled: Option<bool>,
}

async fn update_tunnel(
    Path(name): Path<String>,
    Json(req): Json<UpdateTunnelRequest>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::UpdateTunnel {
        name,
        new_name: req.new_name,
        token: req.token,
        enabled: req.enabled,
    })
    .await
    {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn delete_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DeleteTunnel { name }).await {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn enable_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::EnableTunnel { name }).await {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn disable_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DisableTunnel { name }).await {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn restart_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::RestartTunnel { name }).await {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn start_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::StartTunnel { name }).await {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn stop_tunnel(Path(name): Path<String>) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::StopTunnel { name }).await {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn list_services(Path(tunnel_name): Path<String>) -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::ListServices { tunnel_name }).await {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn get_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
) -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::GetService {
        tunnel_name,
        service_name,
    })
    .await
    {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

#[derive(Deserialize)]
struct AddServiceRequest {
    name: String,
    url: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

async fn add_service(
    Path(tunnel_name): Path<String>,
    Json(req): Json<AddServiceRequest>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::AddService {
        tunnel_name,
        service_name: req.name,
        url: req.url,
        enabled: req.enabled,
    })
    .await
    {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

#[derive(Deserialize)]
struct UpdateServiceRequest {
    new_name: Option<String>,
    url: Option<String>,
    enabled: Option<bool>,
}

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
    })
    .await
    {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn delete_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DeleteService {
        tunnel_name,
        service_name,
    })
    .await
    {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn enable_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::EnableService {
        tunnel_name,
        service_name,
    })
    .await
    {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn disable_service(
    Path((tunnel_name, service_name)): Path<(String, String)>,
) -> Json<ApiResponse<String>> {
    match send_ipc(&IpcCommand::DisableService {
        tunnel_name,
        service_name,
    })
    .await
    {
        Ok(IpcResponse::Success(msg)) => Json(ApiResponse {
            success: true,
            data: Some(msg),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn list_processes() -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::ListProcesses).await {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn get_process_status(
    Path(tunnel_name): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match send_ipc(&IpcCommand::GetProcessStatus { tunnel_name }).await {
        Ok(IpcResponse::Json(data)) => Json(ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }),
        Ok(IpcResponse::Error(e)) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            data: None,
            message: Some(e),
        }),
        _ => Json(ApiResponse {
            success: false,
            data: None,
            message: Some("未知响应".to_string()),
        }),
    }
}

async fn send_ipc(cmd: &IpcCommand) -> Result<IpcResponse, String> {
    let json = serde_json::to_string(cmd).map_err(|e| format!("序列化命令失败: {}", e))?;

    #[cfg(unix)]
    {
        let socket = UnixDatagram::bind("").map_err(|e| format!("创建 socket 失败: {}", e))?;

        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| format!("设置超时失败: {}", e))?;

        socket
            .send_to(json.as_bytes(), SOCKET_PATH)
            .map_err(|e| format!("发送命令失败: {}", e))?;

        let mut buf = [0u8; 65536];
        let size = socket
            .recv(&mut buf)
            .map_err(|e| format!("接收响应失败: {}", e))?;

        serde_json::from_slice(&buf[..size]).map_err(|e| format!("解析响应失败: {}", e))
    }

    #[cfg(windows)]
    {
        let mut stream = TcpStream::connect(TCP_IPC_ADDR)
            .await
            .map_err(|e| format!("连接 IPC 服务失败: {}", e))?;

        stream
            .set_nodelay(true)
            .map_err(|e| format!("设置 TCP_NODELAY 失败: {}", e))?;

        let json_bytes = json.as_bytes();
        let len = json_bytes.len() as u32;
        let len_bytes = len.to_be_bytes();

        tokio::io::write_all(&mut stream, &len_bytes)
            .await
            .map_err(|e| format!("发送长度失败: {}", e))?;
        tokio::io::write_all(&mut stream, json_bytes)
            .await
            .map_err(|e| format!("发送命令失败: {}", e))?;

        let mut len_buf = [0u8; 4];
        tokio::io::read_exact(&mut stream, &mut len_buf)
            .await
            .map_err(|e| format!("读取响应长度失败: {}", e))?;
        let resp_len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; resp_len];
        tokio::io::read_exact(&mut stream, &mut buf)
            .await
            .map_err(|e| format!("读取响应失败: {}", e))?;

        serde_json::from_slice(&buf).map_err(|e| format!("解析响应失败: {}", e))
    }
}
