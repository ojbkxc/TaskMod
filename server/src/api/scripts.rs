use axum::{extract::Path as AxumPath, Json};
use std::fs;
use std::path::Path;

use crate::config::SCRIPTS_DIR;
use crate::data::models::ConfigUpdate;
use crate::data::response::ApiResponse;

pub async fn list_scripts() -> Json<ApiResponse<Vec<String>>> {
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

pub async fn get_script(AxumPath(name): AxumPath<String>) -> Json<ApiResponse<String>> {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Json(ApiResponse::err("无效的脚本名称"));
    }
    let script_path = format!("{}/{}", SCRIPTS_DIR, name);
    match fs::read_to_string(&script_path) {
        Ok(content) => Json(ApiResponse::ok(content)),
        Err(e) => Json(ApiResponse::err(&format!("读取失败: {}", e))),
    }
}

pub async fn save_script(
    AxumPath(name): AxumPath<String>,
    Json(req): Json<ConfigUpdate>,
) -> Json<ApiResponse<String>> {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Json(ApiResponse::err("无效的脚本名称"));
    }
    let script_path = format!("{}/{}", SCRIPTS_DIR, name);
    match fs::write(&script_path, &req.content) {
        Ok(_) => {
            let _ = std::process::Command::new("chmod").arg("+x").arg(&script_path).status();
            Json(ApiResponse::ok_msg("ok".to_string(), "脚本已保存"))
        }
        Err(e) => Json(ApiResponse::err(&format!("保存失败: {}", e))),
    }
}