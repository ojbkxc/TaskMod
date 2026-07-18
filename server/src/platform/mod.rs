pub mod audio;
pub mod capture;
pub mod encoder;
pub mod info;
pub mod input;

/// 跨平台平台类型
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum Platform {
    Android,
    Windows,
    Linux,
    Unknown,
}

/// 检测当前运行平台
pub fn detect_platform() -> Platform {
    #[cfg(target_os = "android")]
    {
        Platform::Android
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(target_os = "linux")]
    {
        // 在 Linux 上检查是否是 Android（通过 screenrecord 是否存在判断）
        if std::path::Path::new("/system/bin/screenrecord").exists() {
            Platform::Android
        } else {
            Platform::Linux
        }
    }
    #[cfg(not(any(target_os = "android", target_os = "windows", target_os = "linux")))]
    {
        Platform::Unknown
    }
}
