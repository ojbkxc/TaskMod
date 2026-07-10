//! KCP 可靠 UDP 传输模块
//! 借鉴 RustDesk 的 KCP 低延迟传输方案，为局域网投屏提供比 WebSocket (TCP) 更低延迟的传输
//!
//! KCP 相比 TCP 的优势：
//! 1. 更小的延迟：无 TCP 的慢启动和拥塞控制
//! 2. 快速重传：丢包时快速重传，不等待超时
//! 3. 无队头阻塞：单个包丢失不会阻塞后续数据
//! 4. 可配置：可以调整 nodelay 参数实现极低延迟

use std::collections::HashMap;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, RwLock};

use crate::state::SharedMirrorState;

/// KCP 输出适配器：实现 `std::io::Write`，将数据通过 UDP socket 发送到指定地址
struct UdpOutput {
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
}

impl Write for UdpOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.socket.try_send_to(buf, self.addr) {
            Ok(n) => Ok(n),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// KCP 服务端口（HTTP 端口 + 1）
const KCP_PORT_OFFSET: u16 = 1;
/// KCP 会话 ID（用于标识视频流）
const KCP_VIDEO_CONV: u32 = 0x544D; // "TM" in ASCII
/// KCP 接收缓冲区大小
const KCP_RECV_BUF_SIZE: usize = 65536;
/// KCP MTU（局域网场景可以使用更大的 MTU）
const KCP_MTU: usize = 1400;
/// KCP 更新间隔（毫秒）- 越小延迟越低，但 CPU 占用越高
const KCP_UPDATE_INTERVAL_MS: u64 = 5;
/// 最大 KCP 客户端数
const MAX_KCP_CLIENTS: usize = 10;
/// FEC 前向纠错组大小（每N帧生成1个FEC包，20%冗余，借鉴 Sunshine 的 FEC 策略）
const FEC_GROUP_SIZE: usize = 5;

/// KCP 客户端会话
struct KcpSession {
    kcp: kcp::Kcp<UdpOutput>,
    addr: SocketAddr,
    last_active: std::time::Instant,
    /// FEC 组缓冲区：累积帧数据用于 XOR 纠错
    fec_buffer: Vec<Vec<u8>>,
    /// 当前 FEC 组 ID
    fec_group_id: u32,
}

/// KCP 服务端
pub struct KcpServer {
    socket: Arc<UdpSocket>,
    sessions: Arc<RwLock<HashMap<SocketAddr, KcpSession>>>,
    video_rx: broadcast::Receiver<Vec<u8>>,
}

impl KcpServer {
    /// 启动 KCP 服务端
    pub async fn start(
        listen_port: u16,
        state: SharedMirrorState,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let kcp_port = listen_port + KCP_PORT_OFFSET;
        let addr = SocketAddr::from(([0, 0, 0, 0], kcp_port));
        let socket = UdpSocket::bind(addr).await?;
        let socket = Arc::new(socket);

        tracing::info!("[KCP] 服务端启动在 UDP {}:{}", addr.ip(), kcp_port);

        let video_rx = state.get_video_rx().ok_or("投屏未启动")?;
        let sessions: Arc<RwLock<HashMap<SocketAddr, KcpSession>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let server = KcpServer {
            socket: socket.clone(),
            sessions: sessions.clone(),
            video_rx,
        };

        // 启动接收任务（处理客户端握手和数据）
        let recv_socket = socket.clone();
        let recv_sessions = sessions.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; KCP_RECV_BUF_SIZE];
            loop {
                match recv_socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        let data = &buf[..len];
                        let mut sessions = recv_sessions.write().await;

                        if let Some(session) = sessions.get_mut(&addr) {
                            // 更新现有会话
                            session.last_active = std::time::Instant::now();
                            if let Err(e) = session.kcp.input(data) {
                                tracing::warn!("[KCP] input error from {}: {}", addr, e);
                            }
                        } else if len >= 4 {
                            // 新客户端连接：检查握手包
                            // 握手协议: [4字节 conv (大端)] + [可选数据]
                            if data.len() >= 4 {
                                let conv = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                                if conv == KCP_VIDEO_CONV && sessions.len() < MAX_KCP_CLIENTS {
                                    tracing::info!("[KCP] 新客户端连接: {}", addr);
                                    let output = UdpOutput {
                                        socket: recv_socket.clone(),
                                        addr,
                                    };
                                    let mut kcp = kcp::Kcp::new(conv, output);
                                    // 设置极低延迟模式（借鉴 RustDesk 的 KCP 配置）
                                    // nodelay: 1 = 启用 nodelay
                                    // interval: 5ms 更新间隔
                                    // resend: 2 = 快速重传（2次ACK跨越就重传）
                                    // nc: 1 = 关闭流控
                                    let _ = kcp.set_nodelay(true, 5, 2, true);
                                    let _ = kcp.set_mtu(KCP_MTU);
                                    let _ = kcp.set_wndsize(256, 256); // 大窗口减少丢包

                                    if let Err(e) = kcp.input(data) {
                                        tracing::warn!("[KCP] 初始 input error: {}", e);
                                    }

                                    sessions.insert(addr, KcpSession {
                                        kcp,
                                        addr,
                                        last_active: std::time::Instant::now(),
                                        fec_buffer: Vec::with_capacity(FEC_GROUP_SIZE),
                                        fec_group_id: 0,
                                    });
                                } else if sessions.len() >= MAX_KCP_CLIENTS {
                                    tracing::warn!("[KCP] 客户端数已达上限，拒绝 {}", addr);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("[KCP] recv error: {}", e);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });

        // 启动发送任务（将视频帧发送给所有 KCP 客户端）
        let send_sessions = sessions.clone();
        tokio::spawn(async move {
            let mut video_rx = server.video_rx;
            loop {
                match video_rx.recv().await {
                    Ok(data) => {
                        let mut sessions = send_sessions.write().await;
                        let mut dead_addrs = Vec::new();

                        for (addr, session) in sessions.iter_mut() {
                            // 将数据写入 KCP 发送队列
                            match session.kcp.send(&data) {
                                Ok(_) => {
                                    // 立即刷新，确保最低延迟
                                    let _ = session.kcp.flush();
                                    // FEC 前向纠错：累积帧数据，每 FEC_GROUP_SIZE 帧生成 XOR FEC 包
                                    // 借鉴 Sunshine 的20% FEC 冗余策略，可恢复 UDP 传输中任意单帧丢失
                                    if data.len() <= 65000 {
                                        session.fec_buffer.push(data.clone());
                                        if session.fec_buffer.len() >= FEC_GROUP_SIZE {
                                            let fec_data = generate_kcp_fec_xor(&session.fec_buffer);
                                            let fec_msg = build_kcp_fec_message(session.fec_group_id, &fec_data);
                                            let _ = session.kcp.send(&fec_msg);
                                            let _ = session.kcp.flush();
                                            session.fec_buffer.clear();
                                            session.fec_group_id = session.fec_group_id.wrapping_add(1);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("[KCP] send error to {}: {}", addr, e);
                                    dead_addrs.push(*addr);
                                }
                            }
                        }

                        // 清理断开的客户端
                        for addr in dead_addrs {
                            tracing::info!("[KCP] 移除断开的客户端: {}", addr);
                            sessions.remove(&addr);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(_) => break,
                }
            }
        });

        // 启动 KCP 更新任务（定期调用 kcp.update() 驱动协议栈）
        let update_sessions = sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(KCP_UPDATE_INTERVAL_MS));
            let start = std::time::Instant::now();
            loop {
                interval.tick().await;
                let mut sessions = update_sessions.write().await;
                let now = std::time::Instant::now();
                let current_ms = now.duration_since(start).as_millis() as u32;
                let mut dead_addrs = Vec::new();

                for (addr, session) in sessions.iter_mut() {
                    // 超过 30 秒无活动的客户端断开
                    if now.duration_since(session.last_active).as_secs() > 30 {
                        dead_addrs.push(*addr);
                        continue;
                    }
                    if let Err(e) = session.kcp.update(current_ms) {
                        tracing::warn!("[KCP] update error for {}: {}", addr, e);
                    }
                }

                for addr in dead_addrs {
                    tracing::info!("[KCP] 超时移除客户端: {}", addr);
                    sessions.remove(&addr);
                }
            }
        });

        Ok(())
    }
}

/// 生成 XOR 前向纠错数据（所有帧异或，可恢复组内任意单帧丢失）
fn generate_kcp_fec_xor(frames: &[Vec<u8>]) -> Vec<u8> {
    if frames.is_empty() {
        return Vec::new();
    }
    let max_len = frames.iter().map(|f| f.len()).max().unwrap_or(0);
    let mut fec = vec![0u8; max_len];
    for frame in frames {
        for (i, &byte) in frame.iter().enumerate() {
            fec[i] ^= byte;
        }
    }
    fec
}

/// 构建 KCP FEC 消息
/// 格式: [tag="fec"(3B)][group_id(4B)][fec_data]
fn build_kcp_fec_message(group_id: u32, fec_data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(3 + 4 + fec_data.len());
    msg.extend_from_slice(b"fec");
    msg.extend_from_slice(&group_id.to_be_bytes());
    msg.extend_from_slice(fec_data);
    msg
}
