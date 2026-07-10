use axum::{extract::{Multipart, Query}, response::IntoResponse, Json};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::data::response::ApiResponse;

#[derive(Deserialize)]
pub struct FilePath {
    path: String,
}

#[derive(Deserialize)]
pub struct FileWriteReq {
    path: String,
    content: String,
}

#[derive(Deserialize)]
pub struct FileRenameReq {
    from: String,
    to: String,
}

#[derive(Deserialize)]
pub struct FileOpReq {
    path: String,
}

#[derive(Deserialize)]
pub struct FileCopyReq {
    from: String,
    to: String,
}

#[derive(Deserialize)]
pub struct ChmodReq {
    path: String,
    mode: String, // 如 "755", "644", "rwxr-xr-x"
}

#[derive(Deserialize)]
pub struct ZipReq {
    source: String,      // 要压缩的文件或目录路径
    destination: String, // 输出 zip 文件路径
}

#[derive(Deserialize)]
pub struct UnzipReq {
    source: String,      // zip 文件路径
    destination: String, // 解压目标目录
}

#[derive(Serialize)]
pub struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: u64,
    permissions: String,
    extension: String,
}

/// 列出目录内容
pub async fn list_files(Query(q): Query<FilePath>) -> Json<ApiResponse<Vec<FileEntry>>> {
    let dir_path = normalize_path(&q.path);
    let dir = Path::new(&dir_path);

    if !dir.exists() {
        return Json(ApiResponse::err("目录不存在"));
    }
    if !dir.is_dir() {
        return Json(ApiResponse::err("路径不是目录"));
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => return Json(ApiResponse::err(&format!("读取失败: {}", e))),
    };

    let mut files: Vec<FileEntry> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let permissions = meta
            .as_ref()
            .map(|m| format!("{:?}", m.permissions().readonly()))
            .unwrap_or_default();
        let extension = if is_dir {
            String::new()
        } else {
            Path::new(&name)
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        let full_path = format!("{}/{}", dir_path.trim_end_matches('/'), name);
        files.push(FileEntry {
            name,
            path: full_path,
            is_dir,
            size,
            modified,
            permissions,
            extension,
        });
    }

    // 目录在前，文件在后，各自按名称排序
    files.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else if a.is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    Json(ApiResponse::ok(files))
}

/// 读取文件内容（文本文件，限制 2MB）
pub async fn read_file(Query(q): Query<FilePath>) -> Json<ApiResponse<String>> {
    let path = normalize_path(&q.path);
    let p = Path::new(&path);

    if !p.exists() {
        return Json(ApiResponse::err("文件不存在"));
    }
    if p.is_dir() {
        return Json(ApiResponse::err("路径是目录，不是文件"));
    }

    // 限制读取大小
    let meta = match fs::metadata(&p) {
        Ok(m) => m,
        Err(e) => return Json(ApiResponse::err(&format!("获取文件信息失败: {}", e))),
    };
    if meta.len() > 2 * 1024 * 1024 {
        return Json(ApiResponse::err("文件过大（>2MB），请使用其他工具查看"));
    }

    match fs::read_to_string(&p) {
        Ok(content) => Json(ApiResponse::ok(content)),
        Err(e) => Json(ApiResponse::err(&format!("读取失败: {}", e))),
    }
}

/// 写入文件内容
pub async fn write_file(Json(req): Json<FileWriteReq>) -> Json<ApiResponse<String>> {
    let path = normalize_path(&req.path);

    // 确保父目录存在
    if let Some(parent) = Path::new(&path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::write(&path, &req.content) {
        Ok(_) => {
            let _ = tokio::process::Command::new("/system/bin/chmod")
                .args(["644", &path])
                .status()
                .await;
            Json(ApiResponse::ok_msg("ok".to_string(), "文件已保存"))
        }
        Err(e) => Json(ApiResponse::err(&format!("写入失败: {}", e))),
    }
}

/// 创建目录
pub async fn create_dir(Json(req): Json<FileOpReq>) -> Json<ApiResponse<String>> {
    let path = normalize_path(&req.path);
    match fs::create_dir_all(&path) {
        Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "目录已创建")),
        Err(e) => Json(ApiResponse::err(&format!("创建失败: {}", e))),
    }
}

/// 创建空文件
pub async fn create_file(Json(req): Json<FileOpReq>) -> Json<ApiResponse<String>> {
    let path = normalize_path(&req.path);
    if Path::new(&path).exists() {
        return Json(ApiResponse::err("文件已存在"));
    }
    if let Some(parent) = Path::new(&path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::write(&path, "") {
        Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "文件已创建")),
        Err(e) => Json(ApiResponse::err(&format!("创建失败: {}", e))),
    }
}

/// 删除文件或目录
pub async fn delete_file(Json(req): Json<FileOpReq>) -> Json<ApiResponse<String>> {
    let path = normalize_path(&req.path);
    let p = Path::new(&path);

    if !p.exists() {
        return Json(ApiResponse::err("路径不存在"));
    }

    // 安全检查：禁止删除关键目录
    let forbidden = ["/", "/system", "/data", "/vendor", "/proc", "/sys", "/dev"];
    if forbidden.contains(&path.as_str()) {
        return Json(ApiResponse::err("禁止删除系统关键目录"));
    }

    let result = if p.is_dir() {
        fs::remove_dir_all(&path)
    } else {
        fs::remove_file(&path)
    };

    match result {
        Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "已删除")),
        Err(e) => Json(ApiResponse::err(&format!("删除失败: {}", e))),
    }
}

/// 重命名/移动
pub async fn rename_file(Json(req): Json<FileRenameReq>) -> Json<ApiResponse<String>> {
    let from = normalize_path(&req.from);
    let to = normalize_path(&req.to);

    if !Path::new(&from).exists() {
        return Json(ApiResponse::err("源路径不存在"));
    }
    if Path::new(&to).exists() {
        return Json(ApiResponse::err("目标路径已存在"));
    }

    // 确保目标父目录存在
    if let Some(parent) = Path::new(&to).parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::rename(&from, &to) {
        Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "已重命名")),
        Err(e) => Json(ApiResponse::err(&format!("重命名失败: {}", e))),
    }
}

/// 复制文件（不支持目录递归复制，用系统命令）
pub async fn copy_file(Json(req): Json<FileCopyReq>) -> Json<ApiResponse<String>> {
    let from = normalize_path(&req.from);
    let to = normalize_path(&req.to);

    if !Path::new(&from).exists() {
        return Json(ApiResponse::err("源文件不存在"));
    }

    let p = Path::new(&from);
    if p.is_dir() {
        // 用 cp -r
        let result = tokio::process::Command::new("/system/bin/sh")
            .args(["-c", &format!("cp -r '{}' '{}'", from.replace('\'', "'\\''"), to.replace('\'', "'\\''"))])
            .status()
            .await;
        match result {
            Ok(s) if s.success() => Json(ApiResponse::ok_msg("ok".to_string(), "已复制")),
            Ok(s) => Json(ApiResponse::err(&format!("复制失败，退出码: {}", s))),
            Err(e) => Json(ApiResponse::err(&format!("复制失败: {}", e))),
        }
    } else {
        // 确保目标父目录存在
        if let Some(parent) = Path::new(&to).parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::copy(&from, &to) {
            Ok(_) => Json(ApiResponse::ok_msg("ok".to_string(), "已复制")),
            Err(e) => Json(ApiResponse::err(&format!("复制失败: {}", e))),
        }
    }
}

/// 获取文件/目录属性
pub async fn file_info(Query(q): Query<FilePath>) -> Json<ApiResponse<serde_json::Value>> {
    let path = normalize_path(&q.path);
    let p = Path::new(&path);

    if !p.exists() {
        return Json(ApiResponse::err("路径不存在"));
    }

    let meta = match fs::metadata(&p) {
        Ok(m) => m,
        Err(e) => return Json(ApiResponse::err(&format!("获取信息失败: {}", e))),
    };

    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let is_dir = meta.is_dir();
    let size = meta.len();
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let readonly = meta.permissions().readonly();
    let extension = if is_dir {
        String::new()
    } else {
        p.extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    // 如果是目录，统计子项数量
    let children_count = if is_dir {
        fs::read_dir(&p).map(|d| d.count()).unwrap_or(0)
    } else {
        0
    };

    Json(ApiResponse::ok(serde_json::json!({
        "name": name,
        "path": path,
        "is_dir": is_dir,
        "size": size,
        "modified": modified,
        "readonly": readonly,
        "extension": extension,
        "children_count": children_count
    })))
}

/// 文件上传（multipart）
pub async fn upload_file(mut multipart: Multipart) -> Json<ApiResponse<String>> {
    let mut dest_dir = String::new();
    let mut file_name = String::new();
    let mut file_data: Option<Bytes> = None;

    // 先收集所有字段
    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();

        if name == "dir" {
            if let Ok(val) = field.text().await {
                dest_dir = normalize_path(&val);
            }
        } else if name == "file" {
            file_name = field
                .file_name()
                .unwrap_or("upload.bin")
                .to_string();
            match field.bytes().await {
                Ok(data) => file_data = Some(data),
                Err(e) => return Json(ApiResponse::err(&format!("读取上传数据失败: {}", e))),
            }
        }
    }

    if dest_dir.is_empty() {
        dest_dir = "/sdcard/TaskMod".to_string();
    }

    let data = match file_data {
        Some(d) => d,
        None => return Json(ApiResponse::err("未收到文件")),
    };

    let safe_name = sanitize_filename(&file_name);
    if safe_name.is_empty() {
        return Json(ApiResponse::err("无效的文件名"));
    }

    let _ = fs::create_dir_all(&dest_dir);
    let dest_path = format!("{}/{}", dest_dir.trim_end_matches('/'), safe_name);

    if let Err(e) = fs::write(&dest_path, &data) {
        return Json(ApiResponse::err(&format!("写入失败: {}", e)));
    }

    let _ = tokio::process::Command::new("/system/bin/chmod")
        .args(["644", &dest_path])
        .status()
        .await;

    Json(ApiResponse::ok_msg(
        dest_path.clone(),
        &format!("已上传: {} ({} bytes)", safe_name, data.len()),
    ))
}

/// 文件下载（返回二进制）
pub async fn download_file(Query(q): Query<FilePath>) -> impl axum::response::IntoResponse {
    let path = normalize_path(&q.path);
    let p = Path::new(&path);

    if !p.exists() {
        return (axum::http::StatusCode::NOT_FOUND, "文件不存在").into_response();
    }
    if p.is_dir() {
        return (axum::http::StatusCode::BAD_REQUEST, "无法下载目录").into_response();
    }

    let data = match fs::read(&p) {
        Ok(d) => d,
        Err(e) => return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("读取失败: {}", e)).into_response(),
    };

    let file_name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".to_string());

    let content_type = mime_guess(&file_name);

    (
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, content_type),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", file_name),
            ),
            (
                axum::http::header::CONTENT_LENGTH,
                data.len().to_string(),
            ),
        ],
        data,
    )
        .into_response()
}

/// 修改文件权限
pub async fn chmod_file(Json(req): Json<ChmodReq>) -> Json<ApiResponse<String>> {
    let path = normalize_path(&req.path);
    if !Path::new(&path).exists() {
        return Json(ApiResponse::err("路径不存在"));
    }

    // 支持数字模式 (755) 或符号模式 (rwxr-xr-x)
    let result = tokio::process::Command::new("/system/bin/chmod")
        .args([&req.mode, &path])
        .status()
        .await;

    match result {
        Ok(s) if s.success() => Json(ApiResponse::ok_msg("ok".to_string(), &format!("权限已修改为 {}", req.mode))),
        Ok(s) => Json(ApiResponse::err(&format!("chmod 失败，退出码: {}", s))),
        Err(e) => Json(ApiResponse::err(&format!("chmod 失败: {}", e))),
    }
}

/// 压缩 zip
pub async fn zip_file(Json(req): Json<ZipReq>) -> Json<ApiResponse<String>> {
    let source = normalize_path(&req.source);
    let dest = normalize_path(&req.destination);

    if !Path::new(&source).exists() {
        return Json(ApiResponse::err("源路径不存在"));
    }

    // 确保目标父目录存在
    if let Some(parent) = Path::new(&dest).parent() {
        let _ = fs::create_dir_all(parent);
    }

    // 使用 zip 命令压缩
    let cmd = if Path::new(&source).is_dir() {
        format!(
            "cd '{}' && zip -r '{}' .",
            source.replace('\'', "'\\''"),
            dest.replace('\'', "'\\''")
        )
    } else {
        let name = Path::new(&source)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let parent = Path::new(&source)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        format!(
            "cd '{}' && zip '{}' '{}'",
            parent.replace('\'', "'\\''"),
            dest.replace('\'', "'\\''"),
            name.replace('\'', "'\\''")
        )
    };

    let result = tokio::process::Command::new("/system/bin/sh")
        .args(["-c", &cmd])
        .output()
        .await;

    match result {
        Ok(output) => {
            if output.status.success() {
                let size = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
                Json(ApiResponse::ok_msg(
                    dest.clone(),
                    &format!("已压缩: {} ({} bytes)", dest, size),
                ))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Json(ApiResponse::err(&format!("压缩失败: {}", stderr.trim())))
            }
        }
        Err(e) => Json(ApiResponse::err(&format!("执行失败: {}", e))),
    }
}

/// 解压 zip
pub async fn unzip_file(Json(req): Json<UnzipReq>) -> Json<ApiResponse<String>> {
    let source = normalize_path(&req.source);
    let dest = normalize_path(&req.destination);

    if !Path::new(&source).exists() {
        return Json(ApiResponse::err("zip 文件不存在"));
    }

    let _ = fs::create_dir_all(&dest);

    let result = tokio::process::Command::new("/system/bin/sh")
        .args([
            "-c",
            &format!(
                "unzip -o '{}' -d '{}'",
                source.replace('\'', "'\\''"),
                dest.replace('\'', "'\\''")
            ),
        ])
        .output()
        .await;

    match result {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                let count = lines.iter().filter(|l| !l.trim().is_empty()).count();
                Json(ApiResponse::ok_msg(
                    dest.clone(),
                    &format!("已解压 {} 个文件到 {}", count, dest),
                ))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Json(ApiResponse::err(&format!("解压失败: {}", stderr.trim())))
            }
        }
        Err(e) => Json(ApiResponse::err(&format!("执行失败: {}", e))),
    }
}

/// 获取目录大小（递归）
pub async fn dir_size(Query(q): Query<FilePath>) -> Json<ApiResponse<serde_json::Value>> {
    let path = normalize_path(&q.path);
    if !Path::new(&path).exists() {
        return Json(ApiResponse::err("路径不存在"));
    }

    // 优先使用 Rust 递归计算（Android du 输出格式不一致）
    let size = calc_dir_size_rust(&path);
    Json(ApiResponse::ok(serde_json::json!({
        "path": path,
        "size": size
    })))
}

fn calc_dir_size_rust(path: &str) -> u64 {
    use std::collections::VecDeque;
    let mut total: u64 = 0;
    let mut dirs: VecDeque<String> = VecDeque::new();
    dirs.push_back(path.to_string());

    while let Some(dir) = dirs.pop_front() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        dirs.push_back(entry.path().to_string_lossy().to_string());
                    } else {
                        total += meta.len();
                    }
                }
            }
        }
    }
    total
}

/// 清理文件名中的危险字符
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control() && *c != '/' && *c != '\\' && *c != ':' && *c != '*' && *c != '?' && *c != '"' && *c != '<' && *c != '>' && *c != '|')
        .collect::<String>()
        .trim()
        .to_string()
}

/// 简单 MIME 类型猜测
fn mime_guess(name: &str) -> String {
    let ext = Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "txt" | "log" | "conf" | "cfg" | "ini" => "text/plain",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "ts" => "application/javascript",
        "sh" | "bash" => "application/x-sh",
        "py" => "text/x-python",
        "rs" => "text/x-rust",
        "java" | "kt" => "text/x-java",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        "7z" => "application/x-7z-compressed",
        "rar" => "application/vnd.rar",
        "pdf" => "application/pdf",
        "apk" => "application/vnd.android.package-archive",
        "bin" | "dat" => "application/octet-stream",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// 规范化路径，防止路径穿越
fn normalize_path(path: &str) -> String {
    let p = path.trim();
    if p.is_empty() {
        return "/".to_string();
    }
    // 去掉多余的斜杠
    let mut result = p.replace('\\', "/");
    while result.contains("//") {
        result = result.replace("//", "/");
    }
    // 去掉末尾斜杠（根目录除外）
    if result.len() > 1 && result.ends_with('/') {
        result.pop();
    }
    result
}
