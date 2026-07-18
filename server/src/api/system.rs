use axum::{
    extract::{Path as AxumPath, Query},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Local};
use serde_json::json;
use std::collections::HashMap;
use tokio::fs;
use tokio::process::Command;

use crate::config::{LOG_FILE, SCHEDULE_FILE, SCREENSHOTS_DIR, SCRIPTS_DIR, WORKFLOWS_DIR};
use crate::data::models::{
    CommandRequest, ConfigUpdate, EmailConfig, Workflow, WorkflowRunRequest, WorkflowSaveRequest,
};
use crate::data::response::ApiResponse;
use crate::utils::adb;
use crate::utils::email;
use crate::utils::mqtt;

pub async fn index() -> axum::response::Response {
    use axum::http::header;
    match tokio::fs::read_to_string("static/index.html").await {
        Ok(content) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            content,
        )
            .into_response(),
        Err(_) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            include_str!("../../static/index.html").to_string(),
        )
            .into_response(),
    }
}

pub async fn static_css() -> axum::response::Response {
    use axum::http::header;
    match tokio::fs::read_to_string("static/style.css").await {
        Ok(content) => (
            [
                (header::CONTENT_TYPE, "text/css; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            content,
        )
            .into_response(),
        Err(_) => (
            [
                (header::CONTENT_TYPE, "text/css; charset=utf-8"),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            include_str!("../../static/style.css").to_string(),
        )
            .into_response(),
    }
}

pub async fn static_js() -> axum::response::Response {
    use axum::http::header;
    match tokio::fs::read_to_string("static/app.js").await {
        Ok(content) => (
            [
                (
                    header::CONTENT_TYPE,
                    "application/javascript; charset=utf-8",
                ),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            content,
        )
            .into_response(),
        Err(_) => (
            [
                (
                    header::CONTENT_TYPE,
                    "application/javascript; charset=utf-8",
                ),
                (header::CACHE_CONTROL, "public, max-age=3600"),
            ],
            include_str!("../../static/app.js").to_string(),
        )
            .into_response(),
    }
}

pub async fn api_docs() -> axum::response::Response {
    use axum::http::header;
    match tokio::fs::read_to_string("static/api-docs.html").await {
        Ok(content) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            content,
        )
            .into_response(),
        Err(_) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            include_str!("../../static/api-docs.html").to_string(),
        )
            .into_response(),
    }
}

pub async fn get_logs(
    Query(params): Query<HashMap<String, String>>,
) -> Json<ApiResponse<Vec<String>>> {
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(200);

    let content = fs::read_to_string(LOG_FILE)
        .await
        .unwrap_or_else(|_| "暂无日志".to_string());
    let lines: Vec<String> = content
        .lines()
        .rev()
        .take(limit)
        .map(String::from)
        .collect();
    Json(ApiResponse::ok(lines))
}

pub async fn clear_logs() -> Json<ApiResponse<String>> {
    match fs::write(LOG_FILE, "").await {
        Ok(_) => Json(ApiResponse::ok("日志已清空".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("清空失败: {}", e))),
    }
}

pub async fn list_screenshots() -> Json<ApiResponse<Vec<String>>> {
    let entries = match fs::read_dir(SCREENSHOTS_DIR).await {
        Ok(mut dir) => {
            let mut files: Vec<String> = Vec::new();
            while let Ok(Some(entry)) = dir.next_entry().await {
                if entry
                    .path()
                    .extension()
                    .map(|ext| ext == "png")
                    .unwrap_or(false)
                {
                    if let Ok(name) = entry.file_name().into_string() {
                        files.push(name);
                    }
                }
            }
            files.sort();
            files.reverse();
            files
        }
        Err(_) => Vec::new(),
    };
    Json(ApiResponse::ok(entries))
}

pub async fn take_screenshot() -> Json<ApiResponse<String>> {
    let timestamp: DateTime<Local> = Local::now();
    let filename = format!("{}.png", timestamp.format("%Y%m%d_%H%M%S"));
    let filepath = format!("{}/{}", SCREENSHOTS_DIR, filename);

    match Command::new("screencap")
        .arg("-p")
        .arg(&filepath)
        .status()
        .await
    {
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

pub async fn get_screenshot(AxumPath(filename): AxumPath<String>) -> impl IntoResponse {
    use axum::http::StatusCode;

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let filepath = format!("{}/{}", SCREENSHOTS_DIR, filename);
    match fs::read(&filepath).await {
        Ok(data) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "image/png")],
            data,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn delete_screenshot(AxumPath(filename): AxumPath<String>) -> Json<ApiResponse<String>> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Json(ApiResponse::err("无效的文件名"));
    }
    let filepath = format!("{}/{}", SCREENSHOTS_DIR, filename);
    match fs::remove_file(&filepath).await {
        Ok(_) => Json(ApiResponse::ok("截图已删除".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}

pub async fn exec_command(Json(req): Json<CommandRequest>) -> Json<ApiResponse<String>> {
    let cmd = req.command.trim();
    if cmd.is_empty() {
        return Json(ApiResponse::err("命令不能为空"));
    }

    match Command::new("/system/bin/sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let result = if !stderr.is_empty() {
                format!("{}\n[stderr] {}", stdout, stderr)
            } else {
                stdout.to_string()
            };
            Json(ApiResponse::ok_msg(result, "命令执行完成"))
        }
        Err(e) => Json(ApiResponse::err(&format!("执行失败: {}", e))),
    }
}

pub async fn get_config() -> Json<ApiResponse<String>> {
    let content = fs::read_to_string(SCHEDULE_FILE).await.unwrap_or_default();
    Json(ApiResponse::ok(content))
}

pub async fn update_config(Json(req): Json<ConfigUpdate>) -> Json<ApiResponse<String>> {
    match fs::write(SCHEDULE_FILE, &req.content).await {
        Ok(_) => Json(ApiResponse::ok_msg(
            "ok".to_string(),
            "配置已保存，30秒内自动生效",
        )),
        Err(e) => Json(ApiResponse::err(&format!("更新失败: {}", e))),
    }
}

pub async fn send_email(Json(config): Json<EmailConfig>) -> Json<ApiResponse<String>> {
    let email_config = email::EmailConfig {
        enable_notify: true,
        smtp_server: config.smtp_server.clone(),
        smtp_port: config.smtp_port,
        username: config.username.clone(),
        password: config.password.clone(),
        from: config.from.clone(),
        to: config.to.clone(),
        subject: config.subject.clone(),
        body: config.body.clone(),
        timeout_secs: config.timeout_secs,
        max_retries: config.max_retries,
        retry_interval: config.retry_interval,
    };

    match email::send_email(&email_config, None, None, None).await {
        Ok(_) => Json(ApiResponse::ok("邮件发送成功".to_string())),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

pub async fn get_email_config() -> Json<serde_json::Value> {
    let config = email::get_email_config();
    Json(json!({
        "success": true,
        "data": {
            "enable_notify": config.enable_notify,
            "smtp_server": config.smtp_server,
            "smtp_port": config.smtp_port,
            "username": config.username,
            "password": config.password,
            "from": config.from,
            "to": config.to,
            "subject": config.subject,
            "body": config.body,
            "timeout_secs": config.timeout_secs,
            "max_retries": config.max_retries,
            "retry_interval": config.retry_interval,
        }
    }))
}

pub async fn save_email_config(Json(config): Json<EmailConfig>) -> Json<ApiResponse<String>> {
    let email_config = email::EmailConfig {
        enable_notify: config.enable_notify == "true" || config.enable_notify == "1",
        smtp_server: config.smtp_server,
        smtp_port: config.smtp_port,
        username: config.username,
        password: config.password,
        from: config.from,
        to: config.to,
        subject: config.subject,
        body: config.body,
        timeout_secs: config.timeout_secs,
        max_retries: config.max_retries,
        retry_interval: config.retry_interval,
    };

    match email::save_email_conf(&email_config) {
        Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "邮件配置已保存")),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn get_mqtt_config() -> Json<serde_json::Value> {
    let config = mqtt::get_mqtt_config();
    if let Some(c) = config {
        Json(json!({
            "success": true,
            "data": {
                "enabled": c.enabled,
                "broker": c.broker,
                "topic_prefix": c.topic_prefix,
                "username": c.username,
                "password": c.password,
                "client_id": c.client_id,
            }
        }))
    } else {
        Json(json!({
            "success": true,
            "data": {
                "enabled": false,
                "broker": "tcp://localhost:1883",
                "topic_prefix": "taskmod",
                "username": "",
                "password": "",
                "client_id": "taskmod-device",
            }
        }))
    }
}

pub async fn system_status() -> Json<serde_json::Value> {
    let uptime = Command::new("/system/bin/uptime")
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "N/A".to_string());

    let disk = Command::new("/system/bin/df")
        .args(["-h", "/sdcard"])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "N/A".to_string());

    let tasks_count = fs::read_to_string(SCHEDULE_FILE)
        .await
        .map(|c| {
            c.lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .count()
        })
        .unwrap_or(0);

    let screenshots_count = match fs::read_dir(SCREENSHOTS_DIR).await {
        Ok(mut d) => {
            let mut count = 0usize;
            while let Ok(Some(_)) = d.next_entry().await {
                count += 1;
            }
            count
        }
        Err(_) => 0,
    };

    let battery_capacity =
        match fs::read_to_string("/sys/class/power_supply/battery/capacity").await {
            Ok(s) => s.trim().to_string(),
            Err(_) => fs::read_to_string("/sys/class/power_supply/battery0/capacity")
                .await
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "N/A".to_string()),
        };

    let battery_temp = match fs::read_to_string("/sys/class/power_supply/battery/temp").await {
        Ok(s) => match s.trim().parse::<i32>() {
            Ok(t) => format!("{:.1}", t as f64 / 10.0),
            Err(_) => s.trim().to_string(),
        },
        Err(_) => match fs::read_to_string("/sys/class/power_supply/battery0/temp").await {
            Ok(s) => match s.trim().parse::<i32>() {
                Ok(t) => format!("{:.1}", t as f64 / 10.0),
                Err(_) => s.trim().to_string(),
            },
            Err(_) => "N/A".to_string(),
        },
    };

    let battery_status = match fs::read_to_string("/sys/class/power_supply/battery/status").await {
        Ok(s) => s.trim().to_string(),
        Err(_) => fs::read_to_string("/sys/class/power_supply/battery0/status")
            .await
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "N/A".to_string()),
    };

    Json(json!({
        "success": true,
        "data": {
            "uptime": uptime.trim(),
            "disk": disk.trim(),
            "tasks_count": tasks_count,
            "screenshots_count": screenshots_count,
            "battery": {
                "capacity": battery_capacity,
                "temperature": battery_temp,
                "status": battery_status,
            }
        }
    }))
}

pub async fn app_status() -> Json<serde_json::Value> {
    let tasks_count = fs::read_to_string(SCHEDULE_FILE)
        .await
        .map(|c| {
            c.lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .count()
        })
        .unwrap_or(0);

    let screenshots_count = match fs::read_dir(SCREENSHOTS_DIR).await {
        Ok(mut d) => {
            let mut count = 0usize;
            while let Ok(Some(_)) = d.next_entry().await {
                count += 1;
            }
            count
        }
        Err(_) => 0,
    };

    Json(json!({
        "success": true,
        "data": {
            "tasks_count": tasks_count,
            "screenshots_count": screenshots_count,
        }
    }))
}

pub async fn list_workflows_api() -> Json<ApiResponse<Vec<Workflow>>> {
    let workflows = list_workflows();
    Json(ApiResponse::ok(workflows))
}

pub async fn get_workflow(AxumPath(id): AxumPath<String>) -> Json<ApiResponse<Workflow>> {
    match load_workflow(&id).await {
        Some(workflow) => Json(ApiResponse::ok(workflow)),
        None => Json(ApiResponse::err("工作流不存在")),
    }
}

pub async fn save_workflow_api(Json(req): Json<WorkflowSaveRequest>) -> Json<ApiResponse<String>> {
    match save_workflow(&req.workflow).await {
        Ok(_) => Json(ApiResponse::ok_msg(req.workflow.id.clone(), "工作流已保存")),
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn delete_workflow_api(AxumPath(id): AxumPath<String>) -> Json<ApiResponse<String>> {
    match delete_workflow(&id).await {
        Ok(_) => Json(ApiResponse::ok("已删除".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}

pub async fn run_workflow(Json(req): Json<WorkflowRunRequest>) -> Json<ApiResponse<String>> {
    let workflow = match load_workflow(&req.workflow_id).await {
        Some(w) => w,
        None => return Json(ApiResponse::err("工作流不存在")),
    };

    let start_node = workflow.nodes.iter().find(|n| n.node_type == "start");
    if start_node.is_none() {
        return Json(ApiResponse::err("工作流缺少开始节点"));
    }

    let wf = workflow.clone();
    tokio::spawn(async move {
        execute_workflow(wf, None).await;
    });

    Json(ApiResponse::ok(format!(
        "工作流 {} 已开始执行",
        workflow.name
    )))
}

async fn save_workflow(workflow: &Workflow) -> Result<(), std::io::Error> {
    let _ = fs::create_dir_all(WORKFLOWS_DIR).await;
    let path = format!("{}/{}.json", WORKFLOWS_DIR, workflow.id);
    let content = serde_json::to_string_pretty(workflow).unwrap_or_default();
    fs::write(path, content).await
}

async fn load_workflow(id: &str) -> Option<Workflow> {
    let path = format!("{}/{}.json", WORKFLOWS_DIR, id);
    fs::read_to_string(&path)
        .await
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
}

pub fn list_workflows() -> Vec<Workflow> {
    let _ = std::fs::create_dir_all(WORKFLOWS_DIR);
    std::fs::read_dir(WORKFLOWS_DIR)
        .map(|dir| {
            dir.filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .filter_map(|e| std::fs::read_to_string(e.path()).ok())
                .filter_map(|content| serde_json::from_str(&content).ok())
                .collect()
        })
        .unwrap_or_default()
}

async fn delete_workflow(id: &str) -> Result<(), std::io::Error> {
    let path = format!("{}/{}.json", WORKFLOWS_DIR, id);
    fs::remove_file(path).await
}

pub async fn execute_workflow(workflow: Workflow, context: Option<serde_json::Value>) {
    let log = |msg: &str| {
        let now: DateTime<Local> = Local::now();
        let log_msg = format!(
            "[{}] [工作流: {}] {}",
            now.format("%Y-%m-%d %H:%M:%S"),
            workflow.name,
            msg
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(LOG_FILE)
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{}", log_msg)
            });
    };

    log("开始执行");

    let mut adj: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for edge in &workflow.edges {
        adj.entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
    }

    let start = match workflow.nodes.iter().find(|n| n.node_type == "start") {
        Some(n) => n.clone(),
        None => {
            log("错误: 缺少开始节点");
            return;
        }
    };

    let mut queue = vec![start.id.clone()];
    let mut visited = std::collections::HashSet::new();
    let mut context_vars = context
        .unwrap_or(serde_json::json!({}))
        .as_object()
        .unwrap_or(&serde_json::Map::new())
        .clone();

    let mut skip_nodes = std::collections::HashSet::new();

    while let Some(node_id) = queue.pop() {
        if visited.contains(&node_id) || skip_nodes.contains(&node_id) {
            continue;
        }
        visited.insert(node_id.clone());

        let node = match workflow.nodes.iter().find(|n| n.id == node_id) {
            Some(n) => n.clone(),
            None => continue,
        };

        log(&format!("执行节点: {}", node.label));

        match node.node_type.as_str() {
            "start" => {}
            "script" => {
                let script_name = node
                    .config
                    .get("script")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !script_name.is_empty() {
                    let script_path = format!("{}/{}", SCRIPTS_DIR, script_name);
                    match Command::new("sh").arg(&script_path).output().await {
                        Ok(output) => {
                            let result = String::from_utf8_lossy(&output.stdout);
                            log(&format!("脚本 {} 执行完成: {}", script_name, result));
                        }
                        Err(e) => {
                            log(&format!("脚本 {} 执行失败: {}", script_name, e));
                        }
                    }
                }
            }
            "command" => {
                let mut cmd = node
                    .config
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                for (key, value) in &context_vars {
                    let placeholder = format!("{{{}}}", key);
                    cmd = cmd.replace(&placeholder, &value.to_string());
                }

                if !cmd.is_empty() {
                    match Command::new("sh").arg("-c").arg(cmd).output().await {
                        Ok(output) => {
                            let result = String::from_utf8_lossy(&output.stdout);
                            log(&format!("命令执行完成: {}", result));
                        }
                        Err(e) => {
                            log(&format!("命令执行失败: {}", e));
                        }
                    }
                }
            }
            "delay" => {
                let seconds = node
                    .config
                    .get("seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1);
                log(&format!("延时 {} 秒", seconds));
                tokio::time::sleep(tokio::time::Duration::from_secs(seconds)).await;
            }
            "email" => {
                let email_conf = email::get_email_config();
                let mut to = node
                    .config
                    .get("to")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&email_conf.to)
                    .to_string();
                let mut subject = node
                    .config
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .unwrap_or("工作流通知")
                    .to_string();
                let mut body = node
                    .config
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("工作流节点执行完成")
                    .to_string();

                for (key, value) in &context_vars {
                    let placeholder = format!("{{{}}}", key);
                    to = to.replace(&placeholder, &value.to_string());
                    subject = subject.replace(&placeholder, &value.to_string());
                    body = body.replace(&placeholder, &value.to_string());
                }

                let config = email::EmailConfig {
                    enable_notify: true,
                    smtp_server: email_conf.smtp_server,
                    smtp_port: email_conf.smtp_port,
                    username: email_conf.username,
                    password: email_conf.password,
                    from: email_conf.from,
                    to,
                    subject,
                    body,
                    timeout_secs: email_conf.timeout_secs,
                    max_retries: email_conf.max_retries,
                    retry_interval: email_conf.retry_interval,
                };

                let _ = email::send_email(&config, None, None, None).await;
                log("邮件已发送");
            }
            "email_attachment" => {
                let email_conf = email::get_email_config();
                let mut to = node
                    .config
                    .get("to")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&email_conf.to)
                    .to_string();
                let mut subject = node
                    .config
                    .get("subject")
                    .and_then(|v| v.as_str())
                    .unwrap_or("工作流通知")
                    .to_string();
                let mut body = node
                    .config
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("工作流节点执行完成")
                    .to_string();

                for (key, value) in &context_vars {
                    let placeholder = format!("{{{}}}", key);
                    to = to.replace(&placeholder, &value.to_string());
                    subject = subject.replace(&placeholder, &value.to_string());
                    body = body.replace(&placeholder, &value.to_string());
                }

                let mut attachments = Vec::new();
                if let Some(attachments_json) = node.config.get("attachments") {
                    if let Some(attachment_list) = attachments_json.as_array() {
                        for attachment in attachment_list {
                            if let Some(filename) = attachment.as_str() {
                                let filepath = format!("{}/{}", SCREENSHOTS_DIR, filename);
                                if let Ok(content) = fs::read(&filepath).await {
                                    attachments.push((filename.to_string(), content));
                                    log(&format!("添加附件: {}", filename));
                                } else {
                                    log(&format!("附件不存在: {}", filepath));
                                }
                            }
                        }
                    }
                }

                let config = email::EmailConfig {
                    enable_notify: true,
                    smtp_server: email_conf.smtp_server,
                    smtp_port: email_conf.smtp_port,
                    username: email_conf.username,
                    password: email_conf.password,
                    from: email_conf.from,
                    to,
                    subject,
                    body,
                    timeout_secs: email_conf.timeout_secs,
                    max_retries: email_conf.max_retries,
                    retry_interval: email_conf.retry_interval,
                };

                let attachment_count = attachments.len();
                let _ = email::send_email(&config, None, None, Some(attachments)).await;
                log(&format!("邮件已发送，附件数: {}", attachment_count));
            }
            "tts" => {
                let mut text = node
                    .config
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                for (key, value) in &context_vars {
                    let placeholder = format!("{{{}}}", key);
                    text = text.replace(&placeholder, &value.to_string());
                }

                if !text.is_empty() {
                    let engine = node
                        .config
                        .get("engine")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let _ = adb::tts_speak(&text, engine).await;
                    log(&format!("TTS语音播放: {}", text));
                }
            }
            "ai_generate" => {
                let provider_id = node
                    .config
                    .get("provider_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let mut prompt = node
                    .config
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                for (key, value) in &context_vars {
                    let placeholder = format!("{{{}}}", key);
                    prompt = prompt.replace(&placeholder, &value.to_string());
                }

                if prompt.is_empty() {
                    log("AI生成失败: 提示词为空");
                    continue;
                }

                // 支持回退：如果指定了provider_id就用单个，否则尝试所有已启用的
                let result = if provider_id == "default" || provider_id.is_empty() {
                    crate::api::ai::call_ai_with_fallback(&prompt, None).await
                } else {
                    crate::api::ai::call_ai_with_fallback(&prompt, Some(&[provider_id.to_string()]))
                        .await
                };

                match result {
                    Ok(response) => {
                        log("AI生成成功");
                        let output_var = node
                            .config
                            .get("output_var")
                            .and_then(|v| v.as_str())
                            .unwrap_or("ai_result");
                        context_vars
                            .insert(output_var.to_string(), serde_json::Value::String(response));
                    }
                    Err(e) => {
                        log(&format!("AI生成失败: {}", e));
                    }
                }
            }
            "condition" => {
                let expression = node
                    .config
                    .get("expression")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let true_next = node
                    .config
                    .get("true_next")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let false_next = node
                    .config
                    .get("false_next")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut expr = expression.to_string();
                for (key, value) in &context_vars {
                    let placeholder = format!("{{{}}}", key);
                    expr = expr.replace(&placeholder, &value.to_string());
                }

                let result = evaluate_condition(&expr);

                if let Some(next_ids) = adj.get(&node_id) {
                    for next_id in next_ids {
                        let target_node = workflow.nodes.iter().find(|n| n.id == *next_id);
                        if let Some(target) = target_node {
                            let should_execute = if result && !true_next.is_empty() {
                                target.label == true_next || target.id == true_next
                            } else if !result && !false_next.is_empty() {
                                target.label == false_next || target.id == false_next
                            } else {
                                result
                            };

                            if !should_execute {
                                skip_nodes.insert(next_id.clone());
                            }
                        }
                    }
                }
            }
            "mqtt_publish" => {
                let topic = node
                    .config
                    .get("topic")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let mut payload = node
                    .config
                    .get("payload")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                for (key, value) in &context_vars {
                    let placeholder = format!("{{{}}}", key);
                    payload = payload.replace(&placeholder, &value.to_string());
                }

                if !topic.is_empty() {
                    if let Err(e) = mqtt::publish(topic, payload).await {
                        log(&format!("MQTT发布失败: {}", e));
                    } else {
                        log(&format!("MQTT发布成功: {}", topic));
                    }
                }
            }
            "end" => {
                log("工作流执行完成");
                break;
            }
            _ => {
                log(&format!("未知节点类型: {}", node.node_type));
            }
        }

        if let Some(next_ids) = adj.get(&node_id) {
            for next_id in next_ids {
                if !skip_nodes.contains(next_id) {
                    queue.push(next_id.clone());
                }
            }
        }
    }
}

fn evaluate_condition(expr: &str) -> bool {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() < 3 {
        return false;
    }

    let left = parts[0];
    let op = parts[1];
    let right = parts[2];

    let left_val = if let Ok(n) = left.parse::<i32>() {
        n
    } else if let Ok(n) = left.parse::<f64>() {
        n as i32
    } else if left == "true" {
        return op == "==" && right == "true" || op == "!=" && right != "true";
    } else if left == "false" {
        return op == "==" && right == "false" || op == "!=" && right != "false";
    } else {
        return match op {
            "==" => left == right,
            "!=" => left != right,
            _ => false,
        };
    };

    let right_val = if let Ok(n) = right.parse::<i32>() {
        n
    } else if let Ok(n) = right.parse::<f64>() {
        n as i32
    } else if right == "true" {
        1
    } else if right == "false" {
        0
    } else {
        return match op {
            "==" => left == right,
            "!=" => left != right,
            _ => false,
        };
    };

    match op {
        "==" => left_val == right_val,
        "!=" => left_val != right_val,
        ">" => left_val > right_val,
        "<" => left_val < right_val,
        ">=" => left_val >= right_val,
        "<=" => left_val <= right_val,
        _ => false,
    }
}
