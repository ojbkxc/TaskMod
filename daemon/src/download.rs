//! cloudflared 二进制下载模块
//!
//! 从 GitHub Releases 下载指定版本的 cloudflared，并校验 SHA256。
//! 支持自动检测系统架构（Linux amd64/arm64/arm）。
//!
//! 下载流程：
//! 1. 检查本地是否已有目标版本
//! 2. 下载 checksums.txt 获取期望的 SHA256
//! 3. 下载二进制文件到 ~/.taskmod/bin/
//! 4. 校验 SHA256
//! 5. 设置可执行权限

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use log::{info, warn};
use sha2::{Digest, Sha256};

use crate::config::{cloudflared_bin_path, data_dir};
use crate::error::{Result, TaskModError};

/// GitHub Releases 基础 URL
const GITHUB_BASE_URL: &str =
    "https://github.com/cloudflare/cloudflared/releases/download";

/// 获取当前系统架构对应的 cloudflared 文件名后缀
/// 
/// GitHub Releases 上的文件名格式: cloudflared-linux-amd64, cloudflared-linux-arm64, cloudflared-linux-arm
pub fn get_target_arch() -> String {
    #[cfg(target_arch = "aarch64")]
    return "linux-arm64".to_string();
    
    #[cfg(target_arch = "arm")]
    return "linux-arm".to_string();
    
    #[cfg(target_arch = "x86_64")]
    return "linux-amd64".to_string();
    
    #[cfg(target_arch = "i686")]
    return "linux-386".to_string();
    
    #[cfg(not(any(target_arch = "aarch64", target_arch = "arm", target_arch = "x86_64", target_arch = "i686")))]
    compile_error!("不支持的目标架构");
}

/// 检查 cloudflared 二进制是否已存在
pub fn is_binary_available(version: &str) -> Result<bool> {
    let path = cloudflared_bin_path(version)?;
    Ok(path.exists())
}

/// 确保 cloudflared 二进制可用
///
/// 如果本地不存在，自动下载
pub fn ensure_binary(version: &str) -> Result<()> {
    if is_binary_available(version)? {
        info!("cloudflared {} 已存在，跳过下载", version);
        return Ok(());
    }

    info!("开始下载 cloudflared {}", version);
    download_binary(version)?;
    info!("cloudflared {} 下载完成", version);

    Ok(())
}

/// 下载 cloudflared 二进制
/// 
/// GitHub Releases 上的文件名格式: cloudflared-linux-amd64 (不带版本号)
pub fn download_binary(version: &str) -> Result<()> {
    let arch = get_target_arch();
    let bin_name = format!("cloudflared-{}", arch);
    let download_url =
        format!("{}/{}/{}", GITHUB_BASE_URL, version, bin_name);
    let checksum_url = format!(
        "{}/{}/checksums.txt",
        GITHUB_BASE_URL, version
    );

    info!("下载 cloudflared {} ({})", version, arch);
    info!("下载校验文件: {}", checksum_url);
    let checksums = download_text(&checksum_url)?;
    let expected_hash = parse_checksum(&checksums, &bin_name)?;

    // 2. 下载二进制文件
    info!("下载二进制: {}", download_url);
    let binary_data = download_bytes(&download_url)?;

    // 3. 校验 SHA256
    let actual_hash = sha256_hex(&binary_data);
    if actual_hash != expected_hash {
        return Err(TaskModError::ChecksumMismatch {
            expected: expected_hash,
            actual: actual_hash,
        });
    }
    info!("SHA256 校验通过");

    // 4. 写入文件
    let bin_path = cloudflared_bin_path(version)?;
    let bin_dir = bin_path.parent().ok_or_else(|| {
        TaskModError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "无效的二进制路径",
        ))
    })?;

    fs::create_dir_all(bin_dir).map_err(TaskModError::Io)?;

    let mut file = File::create(&bin_path).map_err(TaskModError::FileWrite)?;
    file.write_all(&binary_data)
        .map_err(TaskModError::FileWrite)?;

    // 5. 设置可执行权限 (rwxr-xr-x)
    fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755))
        .map_err(TaskModError::FileWrite)?;

    info!("二进制已保存: {}", bin_path.display());

    Ok(())
}

/// 下载文本内容
fn download_text(url: &str) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(TaskModError::Http)?;

    let response = client.get(url).send().map_err(TaskModError::Http)?;

    if !response.status().is_success() {
        return Err(TaskModError::Http(
            response.error_for_status().unwrap_err(),
        ));
    }

    response.text().map_err(TaskModError::Http)
}

/// 下载二进制数据
fn download_bytes(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 大文件下载，超时放宽
        .build()
        .map_err(TaskModError::Http)?;

    let response = client.get(url).send().map_err(TaskModError::Http)?;

    if !response.status().is_success() {
        return Err(TaskModError::Http(
            response.error_for_status().unwrap_err(),
        ));
    }

    let mut bytes = Vec::new();
    response
        .copy_to(&mut bytes)
        .map_err(TaskModError::Http)?;

    Ok(bytes)
}

/// 从 checksums.txt 中解析指定文件的 SHA256
///
/// checksums.txt 格式：
/// <hash>  <filename>
fn parse_checksum(checksums: &str, filename: &str) -> Result<String> {
    for line in checksums.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[1] == filename {
            return Ok(parts[0].to_lowercase());
        }
    }

    Err(TaskModError::ChecksumMismatch {
        expected: format!("在 checksums.txt 中未找到 {}", filename),
        actual: "未找到".to_string(),
    })
}

/// 计算 SHA256 并返回十六进制字符串
fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex_encode(&result)
}

/// 手动实现 hex 编码（避免引入额外依赖）
fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}