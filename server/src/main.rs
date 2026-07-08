use axum::{
    extract::{Path as AxumPath, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Local};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use tower_http::cors::CorsLayer;

// === 配置 ===
const TASKMOD_DIR: &str = "/sdcard/TaskMod";
const SCHEDULE_FILE: &str = "/sdcard/TaskMod/schedule.conf";
const SCRIPTS_DIR: &str = "/sdcard/TaskMod/scripts";
const SCREENSHOTS_DIR: &str = "/sdcard/TaskMod/screenshots";
const EMAIL_CONF: &str = "/sdcard/TaskMod/email.conf";
const LOG_FILE: &str = "/data/adb/modules/TaskMod/TaskMod.log";
const WEB_PORT: u16 = 8080;

// === 数据结构 ===
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Task {
    id: usize,
    time: String,
    weeks: String,
    script: String,
    task_type: String,
    interval: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AddTaskRequest {
    time: String,
    weeks: Option<String>,
    script: String,
    task_type: String,
    interval: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct EmailConfig {
    smtp_server: String,
    smtp_port: u16,
    username: String,
    password: String,
    from: String,
    to: String,
    subject: String,
    body: String,
}

#[derive(Debug, Deserialize)]
struct TriggerRequest {
    script: String,
}

#[derive(Debug, Deserialize)]
struct ConfigUpdate {
    content: String,
}

#[derive(Debug, Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    fn err(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(msg.to_string()),
        }
    }
}

// === 工具函数 ===
fn ensure_dirs() {
    let _ = fs::create_dir_all(TASKMOD_DIR);
    let _ = fs::create_dir_all(SCRIPTS_DIR);
    let _ = fs::create_dir_all(SCREENSHOTS_DIR);
}

// 解析邮件配置文件
fn parse_email_conf() -> HashMap<String, String> {
    let mut config = HashMap::new();
    if let Ok(content) = fs::read_to_string(EMAIL_CONF) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                config.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }
    config
}

// 保存邮件配置
fn save_email_conf(config: &EmailConfig) -> Result<(), std::io::Error> {
    let content = format!(
        "# TaskMod 邮件配置\nsmtp_server={}\nsmtp_port={}\nusername={}\npassword={}\nfrom={}\nto={}",
        config.smtp_server, config.smtp_port, config.username, config.password, config.from, config.to
    );
    fs::write(EMAIL_CONF, content)
}

fn parse_tasks(content: &str) -> Vec<Task> {
    content
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                return None;
            }
            if parts[0] == "every" {
                if parts.len() >= 3 {
                    Some(Task {
                        id: idx + 1,
                        time: "every".to_string(),
                        weeks: "*".to_string(),
                        script: parts[2].to_string(),
                        task_type: "interval".to_string(),
                        interval: parts[1].parse().ok(),
                    })
                } else {
                    None
                }
            } else if parts.len() >= 3 {
                Some(Task {
                    id: idx + 1,
                    time: parts[0].to_string(),
                    weeks: parts[1].to_string(),
                    script: parts[2].to_string(),
                    task_type: "weekly".to_string(),
                    interval: None,
                })
            } else {
                Some(Task {
                    id: idx + 1,
                    time: parts[0].to_string(),
                    weeks: "1,2,3,4,5,6,7".to_string(),
                    script: parts[1].to_string(),
                    task_type: "daily".to_string(),
                    interval: None,
                })
            }
        })
        .collect()
}

fn tasks_to_config(tasks: &[Task]) -> String {
    tasks
        .iter()
        .map(|t| {
            if t.task_type == "interval" {
                format!("every {} {}", t.interval.unwrap_or(5), t.script)
            } else if t.weeks == "1,2,3,4,5,6,7" || t.weeks == "*" {
                format!("{} {}", t.time, t.script)
            } else {
                format!("{} {} {}", t.time, t.weeks, t.script)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// === 路由处理 ===

// 首页
async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

// 任务列表
async fn list_tasks() -> Json<ApiResponse<Vec<Task>>> {
    let content = fs::read_to_string(SCHEDULE_FILE).unwrap_or_default();
    let tasks = parse_tasks(&content);
    Json(ApiResponse::ok(tasks))
}

// 添加任务
async fn add_task(Json(req): Json<AddTaskRequest>) -> Json<ApiResponse<String>> {
    let content = fs::read_to_string(SCHEDULE_FILE).unwrap_or_default();
    let mut tasks = parse_tasks(&content);

    let new_task = Task {
        id: tasks.len() + 1,
        time: req.time.clone(),
        weeks: req.weeks.unwrap_or_else(|| "1,2,3,4,5,6,7".to_string()),
        script: req.script,
        task_type: req.task_type,
        interval: req.interval,
    };
    tasks.push(new_task);

    let config = tasks_to_config(&tasks);
    match fs::write(SCHEDULE_FILE, config) {
        Ok(_) => Json(ApiResponse::ok("任务添加成功".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("写入失败: {}", e))),
    }
}

// 删除任务
async fn delete_task(AxumPath(id): AxumPath<usize>) -> Json<ApiResponse<String>> {
    let content = fs::read_to_string(SCHEDULE_FILE).unwrap_or_default();
    let mut tasks = parse_tasks(&content);

    let original_len = tasks.len();
    tasks.retain(|t| t.id != id);

    if tasks.len() == original_len {
        return Json(ApiResponse::err("任务不存在"));
    }

    let config = tasks_to_config(&tasks);
    match fs::write(SCHEDULE_FILE, config) {
        Ok(_) => Json(ApiResponse::ok("任务删除成功".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("写入失败: {}", e))),
    }
}

// 获取日志
async fn get_logs(Query(params): Query<HashMap<String, String>>) -> Json<ApiResponse<Vec<String>>> {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(200);

    let content = fs::read_to_string(LOG_FILE).unwrap_or_else(|_| "暂无日志".to_string());
    let lines: Vec<String> = content.lines().rev().take(limit).map(String::from).collect();
    Json(ApiResponse::ok(lines))
}

// 清空日志
async fn clear_logs() -> Json<ApiResponse<String>> {
    match fs::write(LOG_FILE, "") {
        Ok(_) => Json(ApiResponse::ok("日志已清空".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("清空失败: {}", e))),
    }
}

// 截图列表
async fn list_screenshots() -> Json<ApiResponse<Vec<String>>> {
    let entries = fs::read_dir(SCREENSHOTS_DIR)
        .map(|dir| {
            let mut files: Vec<String> = dir
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "png")
                        .unwrap_or(false)
                })
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            files.sort();
            files.reverse();
            files
        })
        .unwrap_or_default();
    Json(ApiResponse::ok(entries))
}

// 执行截图
async fn take_screenshot() -> Json<ApiResponse<String>> {
    let timestamp: DateTime<Local> = Local::now();
    let filename = format!("{}.png", timestamp.format("%Y%m%d_%H%M%S"));
    let filepath = format!("{}/{}", SCREENSHOTS_DIR, filename);

    match Command::new("screencap").arg("-p").arg(&filepath).status() {
        Ok(status) => {
            if status.success() {
                Json(ApiResponse::ok(filename))
            } else {
                Json(ApiResponse::err("截图命令执行失败"))
            }
        }
        Err(e) => Json(ApiResponse::err(&format!("截图失败: {}", e))),
    }
}

// 获取截图文件
async fn get_screenshot(AxumPath(filename): AxumPath<String>) -> impl IntoResponse {
    // 安全检查：防止目录遍历
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let filepath = format!("{}/{}", SCREENSHOTS_DIR, filename);
    match fs::read(&filepath) {
        Ok(data) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "image/png")],
            data,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

// 删除截图
async fn delete_screenshot(AxumPath(filename): AxumPath<String>) -> Json<ApiResponse<String>> {
    // 安全检查：防止目录遍历
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Json(ApiResponse::err("无效的文件名"));
    }
    let filepath = format!("{}/{}", SCREENSHOTS_DIR, filename);
    match fs::remove_file(&filepath) {
        Ok(_) => Json(ApiResponse::ok("截图已删除".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}

// 手动触发脚本
async fn trigger_script(Json(req): Json<TriggerRequest>) -> Json<ApiResponse<String>> {
    let script_path = format!("{}/{}", SCRIPTS_DIR, req.script);
    if !Path::new(&script_path).exists() {
        return Json(ApiResponse::err(&format!("脚本不存在: {}", req.script)));
    }

    match Command::new("sh").arg(&script_path).spawn() {
        Ok(_) => Json(ApiResponse::ok(format!("脚本 {} 已触发", req.script))),
        Err(e) => Json(ApiResponse::err(&format!("触发失败: {}", e))),
    }
}

// 获取脚本列表
async fn list_scripts() -> Json<ApiResponse<Vec<String>>> {
    let entries = fs::read_dir(SCRIPTS_DIR)
        .map(|dir| {
            let mut files: Vec<String> = dir
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "sh")
                        .unwrap_or(false)
                })
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            files.sort();
            files
        })
        .unwrap_or_default();
    Json(ApiResponse::ok(entries))
}

// 获取配置
async fn get_config() -> Json<ApiResponse<String>> {
    let content = fs::read_to_string(SCHEDULE_FILE).unwrap_or_default();
    Json(ApiResponse::ok(content))
}

// 更新配置
async fn update_config(Json(req): Json<ConfigUpdate>) -> Json<ApiResponse<String>> {
    match fs::write(SCHEDULE_FILE, &req.content) {
        Ok(_) => Json(ApiResponse::ok("配置已更新".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("更新失败: {}", e))),
    }
}

// 获取脚本内容
async fn get_script(AxumPath(name): AxumPath<String>) -> Json<ApiResponse<String>> {
    // 安全检查：防止目录遍历
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Json(ApiResponse::err("无效的脚本名称"));
    }
    let script_path = format!("{}/{}", SCRIPTS_DIR, name);
    match fs::read_to_string(&script_path) {
        Ok(content) => Json(ApiResponse::ok(content)),
        Err(e) => Json(ApiResponse::err(&format!("读取失败: {}", e))),
    }
}

// 保存脚本
async fn save_script(
    AxumPath(name): AxumPath<String>,
    Json(req): Json<ConfigUpdate>,
) -> Json<ApiResponse<String>> {
    // 安全检查：防止目录遍历
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Json(ApiResponse::err("无效的脚本名称"));
    }
    let script_path = format!("{}/{}", SCRIPTS_DIR, name);
    match fs::write(&script_path, &req.content) {
        Ok(_) => {
            let _ = Command::new("chmod").arg("755").arg(&script_path).status();
            Json(ApiResponse::ok("脚本已保存".to_string()))
        }
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

// 发送邮件
async fn send_email(Json(config): Json<EmailConfig>) -> Json<ApiResponse<String>> {
    // 保存配置
    let _ = save_email_conf(&config);

    let from_addr = match config.from.parse() {
        Ok(addr) => addr,
        Err(e) => return Json(ApiResponse::err(&format!("发件人地址无效: {}", e))),
    };
    let to_addr = match config.to.parse() {
        Ok(addr) => addr,
        Err(e) => return Json(ApiResponse::err(&format!("收件人地址无效: {}", e))),
    };

    let email = match Message::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(config.subject)
        .header(ContentType::TEXT_PLAIN)
        .body(config.body)
    {
        Ok(e) => e,
        Err(e) => return Json(ApiResponse::err(&format!("邮件构建失败: {}", e))),
    };

    let creds = Credentials::new(config.username.clone(), config.password.clone());

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_server)
        .port(config.smtp_port)
        .credentials(creds)
        .build();

    match mailer.send(email).await {
        Ok(_) => Json(ApiResponse::ok("邮件发送成功".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("发送失败: {}", e))),
    }
}

// 获取邮件配置
async fn get_email_config() -> Json<serde_json::Value> {
    let config = parse_email_conf();
    Json(serde_json::json!({
        "success": true,
        "data": {
            "smtp_server": config.get("smtp_server").unwrap_or(&String::new()),
            "smtp_port": config.get("smtp_port").unwrap_or(&"587".to_string()),
            "username": config.get("username").unwrap_or(&String::new()),
            "from": config.get("from").unwrap_or(&String::new()),
            "to": config.get("to").unwrap_or(&String::new()),
        }
    }))
}

// 系统状态
async fn system_status() -> Json<serde_json::Value> {
    let uptime = Command::new("uptime")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "N/A".to_string());

    let disk = Command::new("df")
        .args(["-h", "/sdcard"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "N/A".to_string());

    let tasks_count = fs::read_to_string(SCHEDULE_FILE)
        .map(|c| parse_tasks(&c).len())
        .unwrap_or(0);

    let screenshots_count = fs::read_dir(SCREENSHOTS_DIR)
        .map(|d| d.filter_map(|e| e.ok()).count())
        .unwrap_or(0);

    Json(serde_json::json!({
        "success": true,
        "data": {
            "uptime": uptime.trim(),
            "disk": disk.trim(),
            "tasks_count": tasks_count,
            "screenshots_count": screenshots_count,
        }
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    ensure_dirs();

    let app = Router::new()
        .route("/", get(index))
        .route("/api/tasks", get(list_tasks).post(add_task))
        .route("/api/tasks/:id", delete(delete_task))
        .route("/api/logs", get(get_logs))
        .route("/api/logs/clear", post(clear_logs))
        .route("/api/screenshots", get(list_screenshots))
        .route("/api/screenshots/take", post(take_screenshot))
        .route("/api/screenshots/:filename", get(get_screenshot).delete(delete_screenshot))
        .route("/api/scripts", get(list_scripts))
        .route("/api/scripts/:name", get(get_script).put(save_script))
        .route("/api/trigger", post(trigger_script))
        .route("/api/config", get(get_config).put(update_config))
        .route("/api/email", get(get_email_config).post(send_email))
        .route("/api/status", get(system_status))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], WEB_PORT));
    println!("TaskMod Web 管理服务已启动: http://0.0.0.0:{}", WEB_PORT);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
