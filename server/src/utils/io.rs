//! 通用 IO 工具函数
//!
//! 提供 JSON 文件读写、目录操作等常用功能，避免在多个模块中重复实现。

use serde::{de::DeserializeOwned, Serialize};
use tokio::fs;

/// 确保目录存在，若不存在则创建
pub async fn ensure_dir(path: &str) {
    let _ = fs::create_dir_all(path).await;
}

/// 从 JSON 文件读取数据，若文件不存在或解析失败则返回 None
pub async fn read_json_file<T: DeserializeOwned>(path: &str) -> Option<T> {
    let data = fs::read(path).await.ok()?;
    serde_json::from_slice(&data).ok()
}

/// 将数据写入 JSON 文件，使用 pretty 格式
pub async fn write_json_file<T: Serialize>(path: &str, data: &T) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?;
    fs::write(path, json).await.map_err(|e| e.to_string())
}

/// 列出目录下所有 JSON 文件，并反序列化为指定类型
pub async fn list_json_dir<T: DeserializeOwned>(dir: &str) -> Vec<T> {
    let mut paths = Vec::new();
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                paths.push(path);
            }
        }
    }
    let items: Vec<Option<T>> = futures::future::join_all(
        paths
            .iter()
            .map(|p| read_json_file::<T>(p.to_str().unwrap_or(""))),
    )
    .await;
    items.into_iter().flatten().collect()
}
