use axum::{extract::Path as AxumPath, Json};
use chrono::{DateTime, Local};
use std::fs;
use std::path::Path;
use tokio::process::Command;

use crate::config::{SCHEDULE_FILE, SCRIPTS_DIR, EMAIL_CONF};
use crate::data::models::{AddTaskRequest, Task, TriggerRequest};
use crate::data::response::ApiResponse;
use crate::utils::email;

pub async fn list_tasks() -> Json<ApiResponse<Vec<Task>>> {
    let content = fs::read_to_string(SCHEDULE_FILE).unwrap_or_default();
    let tasks: Vec<Task> = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .enumerate()
        .map(|(idx, line)| {
            let parts: Vec<&str> = line.split('|').collect();
            Task {
                id: idx + 1,
                time: parts.get(0).map(|s| s.trim().to_string()).unwrap_or_default(),
                weeks: parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default(),
                script: parts.get(2).map(|s| s.trim().to_string()).unwrap_or_default(),
                task_type: parts.get(3).map(|s| s.trim().to_string()).unwrap_or_default(),
                interval: parts.get(4).and_then(|s| s.trim().parse().ok()),
            }
        })
        .collect();
    Json(ApiResponse::ok(tasks))
}

pub async fn add_task(Json(req): Json<AddTaskRequest>) -> Json<ApiResponse<String>> {
    let weeks = req.weeks.unwrap_or_else(|| "*".to_string());
    let new_task = format!(
        "{}|{}|{}|{}",
        req.time, weeks, req.script, req.task_type
    );

    let mut content = fs::read_to_string(SCHEDULE_FILE).unwrap_or_default();
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&new_task);
    content.push('\n');

    match fs::write(SCHEDULE_FILE, &content) {
        Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "任务已添加，30秒内自动生效")),
        Err(e) => Json(ApiResponse::err(&format!("添加失败: {}", e))),
    }
}

pub async fn delete_task(AxumPath(id): AxumPath<usize>) -> Json<ApiResponse<String>> {
    let content = fs::read_to_string(SCHEDULE_FILE).unwrap_or_default();
    let mut lines: Vec<&str> = content.lines().collect();
    
    if id > lines.len() {
        return Json(ApiResponse::err("任务不存在"));
    }
    
    lines.remove(id - 1);
    
    match fs::write(SCHEDULE_FILE, lines.join("\n")) {
        Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "任务已删除")),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}

pub async fn trigger_script(Json(req): Json<TriggerRequest>) -> Json<ApiResponse<String>> {
    let script_name = req.script.clone();
    let script_path = format!("{}/{}", SCRIPTS_DIR, script_name);
    
    if !Path::new(&script_path).exists() {
        return Json(ApiResponse::err(&format!("脚本不存在: {}", script_name)));
    }

    let script_name_clone = script_name.clone();
    tokio::spawn(async move {
        match Command::new("sh").arg(&script_path).output().await {
            Ok(output) => {
                let result = String::from_utf8_lossy(&output.stdout);
                let now: DateTime<Local> = Local::now();
                
                let email_conf = parse_email_conf();
                let enable_notify = email_conf.get("enable_notify")
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
                
                if enable_notify {
                    let config = email::EmailConfig {
                        enable_notify: true,
                        smtp_server: email_conf.get("smtp_server").unwrap_or(&String::new()).clone(),
                        smtp_port: email_conf.get("smtp_port")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(587),
                        username: email_conf.get("username").unwrap_or(&String::new()).clone(),
                        password: email_conf.get("password").unwrap_or(&String::new()).clone(),
                        from: email_conf.get("from").unwrap_or(&String::new()).clone(),
                        to: email_conf.get("to").unwrap_or(&String::new()).clone(),
                        subject: email_conf.get("subject")
                            .unwrap_or(&"TaskMod 通知".to_string())
                            .replace("{script}", &script_name_clone)
                            .replace("{time}", &now.format("%H:%M:%S").to_string())
                            .replace("{date}", &now.format("%Y-%m-%d").to_string()),
                        body: email_conf.get("body")
                            .unwrap_or(&"脚本已执行完成".to_string())
                            .replace("{script}", &script_name_clone)
                            .replace("{time}", &now.format("%H:%M:%S").to_string())
                            .replace("{date}", &now.format("%Y-%m-%d").to_string())
                            .replace("{result}", &result.to_string()),
                        timeout_secs: email_conf.get("timeout_secs").and_then(|s| s.parse().ok()).unwrap_or(30),
                        max_retries: email_conf.get("max_retries").and_then(|s| s.parse().ok()).unwrap_or(3),
                        retry_interval: email_conf.get("retry_interval").and_then(|s| s.parse().ok()).unwrap_or(1),
                    };
                    let _ = email::send_email(&config, None, None, None).await;
                }
            }
            Err(e) => {
                tracing::error!("脚本执行失败: {}", e);
            }
        }
    });

    Json(ApiResponse::ok(format!("脚本 {} 已触发", script_name)))
}

fn parse_email_conf() -> std::collections::HashMap<String, String> {
    let content = fs::read_to_string(EMAIL_CONF).unwrap_or_default();
    let mut conf = std::collections::HashMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            conf.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    conf
}