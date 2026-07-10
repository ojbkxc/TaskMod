use axum::{extract::Path as AxumPath, Json};
use std::path::Path;
use tokio::fs;

use crate::config::SCRIPTS_DIR;
use crate::data::models::ConfigUpdate;
use crate::data::response::ApiResponse;

pub async fn list_scripts() -> Json<ApiResponse<Vec<String>>> {
    let _ = fs::create_dir_all(SCRIPTS_DIR).await;
    let entries = match fs::read_dir(SCRIPTS_DIR).await {
        Ok(mut dir) => {
            let mut files: Vec<String> = Vec::new();
            while let Ok(Some(entry)) = dir.next_entry().await {
                if entry.path().extension().map(|ext| ext == "sh").unwrap_or(false) {
                    if let Ok(name) = entry.file_name().into_string() {
                        files.push(name);
                    }
                }
            }
            files.sort();
            files
        }
        Err(_) => Vec::new(),
    };
    Json(ApiResponse::ok(entries))
}

fn validate_script_name(name: &str) -> bool {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return false;
    }
    !name.is_empty() && name.ends_with(".sh")
}

pub async fn get_script(AxumPath(name): AxumPath<String>) -> Json<ApiResponse<String>> {
    if !validate_script_name(&name) {
        return Json(ApiResponse::err("无效的脚本名称"));
    }
    let script_path = format!("{}/{}", SCRIPTS_DIR, name);
    match fs::read_to_string(&script_path).await {
        Ok(content) => Json(ApiResponse::ok(content)),
        Err(e) => Json(ApiResponse::err(&format!("读取失败: {}", e))),
    }
}

pub async fn save_script(
    AxumPath(name): AxumPath<String>,
    Json(req): Json<ConfigUpdate>,
) -> Json<ApiResponse<String>> {
    if !validate_script_name(&name) {
        return Json(ApiResponse::err("无效的脚本名称"));
    }
    let _ = fs::create_dir_all(SCRIPTS_DIR).await;
    let script_path = format!("{}/{}", SCRIPTS_DIR, name);
    match fs::write(&script_path, &req.content).await {
        Ok(_) => {
            let _ = tokio::process::Command::new("/system/bin/chmod")
                .arg("+x")
                .arg(&script_path)
                .status()
                .await;
            Json(ApiResponse::ok_msg("ok".to_string(), "脚本已保存"))
        }
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}

pub async fn delete_script(AxumPath(name): AxumPath<String>) -> Json<ApiResponse<String>> {
    if !validate_script_name(&name) {
        return Json(ApiResponse::err("无效的脚本名称"));
    }
    let script_path = format!("{}/{}", SCRIPTS_DIR, name);
    if !Path::new(&script_path).exists() {
        return Json(ApiResponse::err("脚本不存在"));
    }
    match fs::remove_file(&script_path).await {
        Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "脚本已删除")),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}
