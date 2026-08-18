use axum::{extract::Path as AxumPath, Json};
use chrono::{DateTime, Local};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

use crate::config::{SCHEDULE_FILE, SCRIPTS_DIR};
use crate::data::models::{AddTaskRequest, Task, TriggerRequest};
use crate::data::response::ApiResponse;
use crate::utils::email;

pub async fn list_tasks() -> Json<ApiResponse<Vec<Task>>> {
    let content = fs::read_to_string(SCHEDULE_FILE).await.unwrap_or_default();
    let tasks: Vec<Task> = content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .enumerate()
        .map(|(idx, line)| {
            let parts: Vec<&str> = line.split('|').collect();
            Task {
                id: idx + 1,
                time: parts
                    .first()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                weeks: parts
                    .get(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                script: parts
                    .get(2)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                task_type: parts
                    .get(3)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                interval: parts.get(4).and_then(|s| s.trim().parse().ok()),
            }
        })
        .collect();
    Json(ApiResponse::ok(tasks))
}

pub async fn add_task(Json(req): Json<AddTaskRequest>) -> Json<ApiResponse<String>> {
    let weeks = req.weeks.unwrap_or_else(|| "*".to_string());
    let new_task = if let Some(interval) = req.interval {
        format!(
            "{}|{}|{}|{}|{}",
            req.time, weeks, req.script, req.task_type, interval
        )
    } else {
        format!("{}|{}|{}|{}", req.time, weeks, req.script, req.task_type)
    };

    let mut content = fs::read_to_string(SCHEDULE_FILE).await.unwrap_or_default();
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&new_task);
    content.push('\n');

    match fs::write(SCHEDULE_FILE, &content).await {
        Ok(_) => Json(ApiResponse::ok_msg(
            "ok".to_string(),
            "任务已添加，30秒内自动生效",
        )),
        Err(_) => {
            if let Some(parent) = Path::new(SCHEDULE_FILE).parent() {
                let _ = fs::create_dir_all(parent).await;
            }
            match fs::write(SCHEDULE_FILE, &content).await {
                Ok(_) => Json(ApiResponse::ok_msg(
                    "ok".to_string(),
                    "任务已添加，30秒内自动生效",
                )),
                Err(e) => Json(ApiResponse::err(&format!("添加失败: {}", e))),
            }
        }
    }
}

pub async fn delete_task(AxumPath(id): AxumPath<usize>) -> Json<ApiResponse<String>> {
    let content = fs::read_to_string(SCHEDULE_FILE).await.unwrap_or_default();
    let lines: Vec<&str> = content.lines().collect();

    // Build index mapping: filtered_index -> raw_line_index
    let mut filtered_indices: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !line.trim().is_empty() && !line.starts_with('#') {
            filtered_indices.push(i);
        }
    }

    if id == 0 || id > filtered_indices.len() {
        return Json(ApiResponse::err("任务不存在"));
    }

    let raw_index = filtered_indices[id - 1];
    let mut new_lines: Vec<&str> = lines.to_vec();
    new_lines.remove(raw_index);

    match fs::write(SCHEDULE_FILE, new_lines.join("\n")).await {
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
        match Command::new("/system/bin/sh")
            .arg(&script_path)
            .output()
            .await
        {
            Ok(output) => {
                let result = String::from_utf8_lossy(&output.stdout);
                let now: DateTime<Local> = Local::now();

                let email_conf = email::get_email_config();

                if email_conf.enable_notify {
                    let mut config = email_conf;
                    config.subject = config
                        .subject
                        .replace("{script}", &script_name_clone)
                        .replace("{time}", &now.format("%H:%M:%S").to_string())
                        .replace("{date}", &now.format("%Y-%m-%d").to_string());
                    config.body = config
                        .body
                        .replace("{script}", &script_name_clone)
                        .replace("{time}", &now.format("%H:%M:%S").to_string())
                        .replace("{date}", &now.format("%Y-%m-%d").to_string())
                        .replace("{result}", result.as_ref());
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
