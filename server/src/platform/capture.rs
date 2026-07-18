use async_trait::async_trait;

/// 采集的帧数据
pub enum CapturedFrame {
    /// 已编码的 NALU 数据（Android screenrecord / 桌面 ffmpeg 编码后直接输出）
    EncodedNalu(Vec<u8>),
    /// 原始 BGRA 帧（未来 DXGI Desktop Duplication 零拷贝路径）
    #[allow(dead_code)]
    RawBgra {
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
        timestamp_us: u64,
    },
}

/// 跨平台屏幕采集 trait
#[async_trait]
pub trait ScreenCapture: Send {
    /// 启动采集，将帧通过 channel 发送
    async fn start(&mut self, tx: tokio::sync::mpsc::Sender<CapturedFrame>) -> Result<(), String>;
    /// 停止采集
    #[allow(dead_code)]
    async fn stop(&mut self) -> Result<(), String>;
    /// 请求关键帧
    async fn request_keyframe(&self) -> Result<(), String>;
    /// 更新编码参数（ABR 动态调整时调用）
    async fn update_params(&self, bitrate: u32, fps: u32, resolution_scale: u32) -> Result<(), String>;
}

// ==================== Android 实现 ====================

#[cfg(any(target_os = "android", target_os = "linux"))]
pub struct AndroidScreenCapture {
    codec: crate::platform::encoder::VideoCodec,
    bitrate: u32,
    fps: u32,
    running: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(any(target_os = "android", target_os = "linux"))]
impl AndroidScreenCapture {
    pub fn new(codec: crate::platform::encoder::VideoCodec, bitrate: u32, fps: u32) -> Self {
        Self {
            codec,
            bitrate,
            fps,
            running: None,
        }
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[async_trait]
impl ScreenCapture for AndroidScreenCapture {
    async fn start(&mut self, tx: tokio::sync::mpsc::Sender<CapturedFrame>) -> Result<(), String> {
        let codec_str = match self.codec {
            crate::platform::encoder::VideoCodec::H264 => "avc",
            crate::platform::encoder::VideoCodec::H265 => "hevc",
        };

        let bitrate_str = format!("{}", self.bitrate);
        let _fps_str = format!("{}", self.fps);

        let handle = tokio::spawn(async move {
            let mut child = match tokio::process::Command::new("screenrecord")
                .args([
                    "--output-format=h264",
                    &format!("--codec={}", codec_str),
                    &format!("--bit-rate={}", bitrate_str),
                    "--size=1280x720",
                    "--verbose",
                    "/dev/stdout",
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("[AndroidScreenCapture] screenrecord 启动失败: {}", e);
                    return;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    tracing::error!("[AndroidScreenCapture] 无法获取 stdout");
                    return;
                }
            };

            use tokio::io::AsyncReadExt;
            let mut reader = tokio::io::BufReader::with_capacity(524288, stdout);
            let mut buf = vec![0u8; 524288]; // 512KB buffer

            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx.send(CapturedFrame::EncodedNalu(buf[..n].to_vec())).await.is_err() {
                            break; // 接收端已关闭
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[AndroidScreenCapture] 读取错误: {}", e);
                        break;
                    }
                }
            }

            let _ = child.kill().await;
        });

        self.running = Some(handle);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        if let Some(handle) = self.running.take() {
            handle.abort();
        }
        Ok(())
    }

    async fn request_keyframe(&self) -> Result<(), String> {
        // Android screenrecord 不支持运行时请求关键帧
        // 通过重启采集器实现（由调用方处理）
        Ok(())
    }

    async fn update_params(&self, _bitrate: u32, _fps: u32, _resolution_scale: u32) -> Result<(), String> {
        // Android screenrecord 不支持运行时修改参数
        // 需要重启采集器（由 ABR 控制器触发 need_restart）
        Ok(())
    }
}

// ==================== 桌面实现（Windows/Linux）====================

#[cfg(not(target_os = "android"))]
pub struct DesktopScreenCapture {
    codec: crate::platform::encoder::VideoCodec,
    bitrate: u32,
    fps: u32,
    running: Option<tokio::task::JoinHandle<()>>,
    stop_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(not(target_os = "android"))]
impl DesktopScreenCapture {
    pub fn new(codec: crate::platform::encoder::VideoCodec, bitrate: u32, fps: u32) -> Self {
        Self {
            codec,
            bitrate,
            fps,
            running: None,
            stop_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    async fn detect_best_encoder(&self) -> Vec<String> {
        let is_h265 = self.codec == crate::platform::encoder::VideoCodec::H265;

        if let Ok(output) = tokio::process::Command::new("ffmpeg")
            .args(["-encoders"])
            .output()
            .await
        {
            let encoders = String::from_utf8_lossy(&output.stdout);
            let mut candidates = Vec::new();

            if is_h265 {
                for enc in ["hevc_nvenc", "hevc_amf", "hevc_qsv", "libx265"] {
                    if encoders.contains(enc) {
                        candidates.push(enc.to_string());
                    }
                }
            } else {
                for enc in ["h264_nvenc", "h264_amf", "h264_qsv", "libx264"] {
                    if encoders.contains(enc) {
                        candidates.push(enc.to_string());
                    }
                }
            }

            if !candidates.is_empty() {
                return candidates;
            }
        }

        // Fallback: 软件编码
        if is_h265 {
            vec!["libx265".to_string()]
        } else {
            vec!["libx264".to_string()]
        }
    }
}

#[cfg(not(target_os = "android"))]
#[async_trait]
impl ScreenCapture for DesktopScreenCapture {
    async fn start(&mut self, tx: tokio::sync::mpsc::Sender<CapturedFrame>) -> Result<(), String> {
        let encoders = self.detect_best_encoder().await;
        let encoder = encoders.first().ok_or("无可用编码器")?.clone();

        let platform = crate::platform::detect_platform();
        let input_source = match platform {
            crate::platform::Platform::Windows => "gdigrab",
            _ => "x11grab",
        };
        let input_device = match platform {
            crate::platform::Platform::Windows => "desktop",
            _ => ":0.0",
        };

        let bitrate_str = format!("{}k", self.bitrate / 1000);
        let fps_str = format!("{}", self.fps);
        let stop = self.stop_flag.clone();

        tracing::info!("[DesktopScreenCapture] 使用编码器: {}，输入: {}，码率: {}，帧率: {}", encoder, input_source, bitrate_str, fps_str);

        let handle = tokio::spawn(async move {
            let mut child = match tokio::process::Command::new("ffmpeg")
                .args([
                    "-f", input_source,
                    "-i", input_device,
                    "-r", &fps_str,
                    "-c:v", &encoder,
                    "-b:v", &bitrate_str,
                    "-preset", "ultrafast",
                    "-tune", "zerolatency",
                    "-g", "30",  // GOP size = 30 帧（每30帧一个关键帧）
                    "-bf", "0",  // 无 B 帧（降低延迟）
                    "-f", if encoder.starts_with("hevc") || encoder == "libx265" { "hevc" } else { "h264" },
                    "pipe:1",
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("[DesktopScreenCapture] ffmpeg 启动失败: {}", e);
                    return;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => return,
            };

            use tokio::io::AsyncReadExt;
            let mut reader = tokio::io::BufReader::with_capacity(524288, stdout);
            let mut buf = vec![0u8; 524288];

            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(CapturedFrame::EncodedNalu(buf[..n].to_vec())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            let _ = child.kill().await;
        });

        self.running = Some(handle);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.running.take() {
            handle.abort();
        }
        Ok(())
    }

    async fn request_keyframe(&self) -> Result<(), String> {
        // ffmpeg 不支持运行时插入关键帧
        // 通过重启编码器实现（由 ABR 控制器触发）
        Ok(())
    }

    async fn update_params(&self, _bitrate: u32, _fps: u32, _resolution_scale: u32) -> Result<(), String> {
        // 需要重启编码器才能修改参数
        Ok(())
    }
}

// ==================== 工厂函数 ====================

/// 创建跨平台屏幕采集器
pub async fn create_capture(
    codec: crate::platform::encoder::VideoCodec,
    bitrate: u32,
    fps: u32,
) -> Result<Box<dyn ScreenCapture>, String> {
    let platform = crate::platform::detect_platform();

    match platform {
        crate::platform::Platform::Android => {
            #[cfg(any(target_os = "android", target_os = "linux"))]
            {
                Ok(Box::new(AndroidScreenCapture::new(codec, bitrate, fps)))
            }
            #[cfg(not(any(target_os = "android", target_os = "linux")))]
            {
                Err("Android 平台不可用".to_string())
            }
        }
        _ => {
            #[cfg(not(target_os = "android"))]
            {
                Ok(Box::new(DesktopScreenCapture::new(codec, bitrate, fps)))
            }
            #[cfg(target_os = "android")]
            {
                Err("桌面平台在 Android 上不可用".to_string())
            }
        }
    }
}
