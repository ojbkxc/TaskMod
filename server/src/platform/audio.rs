use async_trait::async_trait;

/// 音频编码格式
pub enum AudioCodec {
    Pcm,
    Opus,
}

/// 采集的音频帧
pub enum AudioFrame {
    Pcm(Vec<u8>),
    Opus(Vec<u8>),
}

/// 跨平台音频采集 trait
#[async_trait]
pub trait AudioCapture: Send {
    /// 启动音频采集
    async fn start(&mut self, tx: tokio::sync::mpsc::Sender<AudioFrame>) -> Result<(), String>;
    /// 停止音频采集
    async fn stop(&mut self) -> Result<(), String>;
}

// ==================== Android 实现 ====================

#[cfg(any(target_os = "android", target_os = "linux"))]
pub struct AndroidAudioCapture {
    codec: AudioCodec,
    running: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(any(target_os = "android", target_os = "linux"))]
impl AndroidAudioCapture {
    pub fn new(codec: AudioCodec) -> Self {
        Self { codec, running: None }
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[async_trait]
impl AudioCapture for AndroidAudioCapture {
    async fn start(&mut self, tx: tokio::sync::mpsc::Sender<AudioFrame>) -> Result<(), String> {
        let handle = tokio::spawn(async move {
            // 使用 tinycap 采集音频（Android 内置工具）
            let mut child = match tokio::process::Command::new("tinycap")
                .args(["/dev/stdout", "-d", "0", "-c", "1", "-b", "16", "-r", "48000"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("[AndroidAudioCapture] tinycap 启动失败: {}", e);
                    return;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => return,
            };

            use tokio::io::AsyncReadExt;
            let mut reader = tokio::io::BufReader::with_capacity(65536, stdout);
            let mut buf = vec![0u8; 65536];
            let mut wav_header_skipped = false;
            let mut header_buf = Vec::new();

            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if !wav_header_skipped {
                            header_buf.extend_from_slice(&buf[..n]);
                            // 查找 WAV 数据块标记 "data"
                            if let Some(pos) = find_wav_data_offset(&header_buf) {
                                wav_header_skipped = true;
                                let pcm_start = pos + 8; // 跳过 "data" + 4字节长度
                                if pcm_start < header_buf.len() {
                                    let pcm_data = header_buf[pcm_start..].to_vec();
                                    if tx.send(AudioFrame::Pcm(pcm_data)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        } else {
                            if tx.send(AudioFrame::Pcm(buf[..n].to_vec())).await.is_err() {
                                break;
                            }
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
        if let Some(handle) = self.running.take() {
            handle.abort();
        }
        Ok(())
    }
}

// ==================== 桌面实现 ====================

#[cfg(not(target_os = "android"))]
pub struct DesktopAudioCapture {
    codec: AudioCodec,
    running: Option<tokio::task::JoinHandle<()>>,
}

#[cfg(not(target_os = "android"))]
impl DesktopAudioCapture {
    pub fn new(codec: AudioCodec) -> Self {
        Self { codec, running: None }
    }
}

#[cfg(not(target_os = "android"))]
#[async_trait]
impl AudioCapture for DesktopAudioCapture {
    async fn start(&mut self, tx: tokio::sync::mpsc::Sender<AudioFrame>) -> Result<(), String> {
        let platform = crate::platform::detect_platform();

        let handle = tokio::spawn(async move {
            // 桌面平台使用 ffmpeg 采集系统音频
            let (input_fmt, input_dev) = match platform {
                crate::platform::Platform::Windows => ("dshow", "audio=virtual-audio-capturer"),
                _ => ("pulse", "default"),
            };

            let mut child = match tokio::process::Command::new("ffmpeg")
                .args([
                    "-f", input_fmt,
                    "-i", input_dev,
                    "-ar", "48000",
                    "-ac", "1",
                    "-f", "s16le",
                    "pipe:1",
                ])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("[DesktopAudioCapture] ffmpeg 音频采集启动失败: {}", e);
                    return;
                }
            };

            let stdout = match child.stdout.take() {
                Some(s) => s,
                None => return,
            };

            use tokio::io::AsyncReadExt;
            let mut reader = tokio::io::BufReader::with_capacity(65536, stdout);
            let mut buf = vec![0u8; 65536];

            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(AudioFrame::Pcm(buf[..n].to_vec())).await.is_err() {
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
        if let Some(handle) = self.running.take() {
            handle.abort();
        }
        Ok(())
    }
}

// ==================== 工厂函数 ====================

/// 创建跨平台音频采集器
pub async fn create_capture(codec: AudioCodec) -> Result<Box<dyn AudioCapture>, String> {
    let platform = crate::platform::detect_platform();

    match platform {
        crate::platform::Platform::Android => {
            #[cfg(any(target_os = "android", target_os = "linux"))]
            {
                Ok(Box::new(AndroidAudioCapture::new(codec)))
            }
            #[cfg(not(any(target_os = "android", target_os = "linux")))]
            {
                Err("Android 音频采集不可用".to_string())
            }
        }
        _ => {
            #[cfg(not(target_os = "android"))]
            {
                Ok(Box::new(DesktopAudioCapture::new(codec)))
            }
            #[cfg(target_os = "android")]
            {
                Err("桌面音频采集在 Android 上不可用".to_string())
            }
        }
    }
}

// ==================== 工具函数 ====================

/// 查找 WAV 文件中 "data" 块的偏移位置
fn find_wav_data_offset(data: &[u8]) -> Option<usize> {
    for i in 0..data.len().saturating_sub(4) {
        if &data[i..i + 4] == b"data" {
            return Some(i);
        }
    }
    None
}
