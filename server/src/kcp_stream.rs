//! KCP 可靠 UDP 传输模块
//! 借鉴 RustDesk 的 KCP 低延迟传输方案，为局域网投屏提供比 WebSocket (TCP) 更低延迟的传输
//!
//! 性能优化:
//! - per-session 细粒度锁，避免一个慢客户端阻塞所有客户端
//! - FEC 缓冲区使用 Arc<Vec<u8>> 避免深拷贝
//! - update 间隔 20ms 平衡延迟与 CPU 占用

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
            Err(e) => Err(std::io::Error::other(e)),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

const KCP_PORT_OFFSET: u16 = 1;
const KCP_VIDEO_CONV: u32 = 0x544D;
const KCP_RECV_BUF_SIZE: usize = 65536;
const KCP_MTU: usize = 1400;
/// KCP 更新间隔（毫秒）- 20ms 平衡延迟与 CPU 占用
const KCP_UPDATE_INTERVAL_MS: u64 = 20;
const MAX_KCP_CLIENTS: usize = 10;
const FEC_GROUP_SIZE: usize = 5;

/// KCP 客户端会话（每个客户端独立锁，避免全局写锁阻塞）
struct KcpSession {
    kcp: kcp::Kcp<UdpOutput>,
    #[allow(dead_code)]
    addr: SocketAddr,
    last_active: std::time::Instant,
    /// FEC 组缓冲区：使用 Arc 避免深拷贝
    fec_buffer: Vec<Arc<Vec<u8>>>,
    fec_group_id: u32,
}

/// KCP 服务端
pub struct KcpServer {
    #[allow(dead_code)]
    socket: Arc<UdpSocket>,
    /// 外层 RwLock 只保护 HashMap 结构（增删客户端），内层 per-session Mutex 保护会话操作
    #[allow(dead_code)]
    sessions: Arc<RwLock<HashMap<SocketAddr, Arc<std::sync::Mutex<KcpSession>>>>>,
    video_rx: broadcast::Receiver<Vec<u8>>,
}

impl KcpServer {
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
        let sessions: Arc<RwLock<HashMap<SocketAddr, Arc<std::sync::Mutex<KcpSession>>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let server = KcpServer {
            socket: socket.clone(),
            sessions: sessions.clone(),
            video_rx,
        };

        // 接收任务：只需读锁查找 session，per-session 锁执行 input
        let recv_socket = socket.clone();
        let recv_sessions = sessions.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; KCP_RECV_BUF_SIZE];
            loop {
                match recv_socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        let data = &buf[..len];

                        // 读锁查找 session（短暂持有）
                        let session_arc = {
                            let sessions = recv_sessions.read().await;
                            sessions.get(&addr).cloned()
                        };

                        if let Some(session_arc) = session_arc {
                            // per-session 锁，不阻塞其他客户端
                            if let Ok(mut session) = session_arc.lock() {
                                session.last_active = std::time::Instant::now();
                                if let Err(e) = session.kcp.input(data) {
                                    tracing::warn!("[KCP] input error from {}: {}", addr, e);
                                }
                            }
                        } else if len >= 4 {
                            // 新客户端握手
                            let conv = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                            if conv == KCP_VIDEO_CONV {
                                let can_accept = {
                                    let sessions = recv_sessions.read().await;
                                    sessions.len() < MAX_KCP_CLIENTS
                                };
                                if can_accept {
                                    tracing::info!("[KCP] 新客户端连接: {}", addr);
                                    let output = UdpOutput {
                                        socket: recv_socket.clone(),
                                        addr,
                                    };
                                    let mut kcp = kcp::Kcp::new(conv, output);
                                    kcp.set_nodelay(true, 5, 2, true);
                                    kcp.set_mtu(KCP_MTU);
                                    kcp.set_wndsize(256, 256);

                                    if let Err(e) = kcp.input(data) {
                                        tracing::warn!("[KCP] 初始 input error: {}", e);
                                    }

                                    let session = Arc::new(std::sync::Mutex::new(KcpSession {
                                        kcp,
                                        addr,
                                        last_active: std::time::Instant::now(),
                                        fec_buffer: Vec::with_capacity(FEC_GROUP_SIZE),
                                        fec_group_id: 0,
                                    }));

                                    let mut sessions = recv_sessions.write().await;
                                    if sessions.len() < MAX_KCP_CLIENTS {
                                        sessions.insert(addr, session);
                                    } else {
                                        tracing::warn!("[KCP] 客户端数已达上限，拒绝 {}", addr);
                                    }
                                } else {
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

        // 发送任务：读锁获取 session 列表，per-session 锁发送（不阻塞其他客户端）
        let send_sessions = sessions.clone();
        tokio::spawn(async move {
            let mut video_rx = server.video_rx;
            loop {
                match video_rx.recv().await {
                    Ok(data) => {
                        // 将 data 包装为 Arc 避免多客户端 clone
                        let data_arc = Arc::new(data);

                        // 读锁获取所有 session（短暂持有，不阻塞 recv/update）
                        let session_list: Vec<(SocketAddr, Arc<std::sync::Mutex<KcpSession>>)> = {
                            let sessions = send_sessions.read().await;
                            sessions.iter().map(|(a, s)| (*a, s.clone())).collect()
                        };

                        let mut dead_addrs = Vec::new();

                        for (addr, session_arc) in session_list {
                            if let Ok(mut session) = session_arc.lock() {
                                match session.kcp.send(&data_arc[..]) {
                                    Ok(_) => {
                                        let _ = session.kcp.flush();

                                        // FEC: 使用 Arc clone（仅引用计数，零拷贝）
                                        if data_arc.len() <= 65000 {
                                            session.fec_buffer.push(data_arc.clone());
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
                                        dead_addrs.push(addr);
                                    }
                                }
                            }
                        }

                        // 写锁清理断开的客户端
                        if !dead_addrs.is_empty() {
                            let mut sessions = send_sessions.write().await;
                            for addr in &dead_addrs {
                                tracing::info!("[KCP] 移除断开的客户端: {}", addr);
                                sessions.remove(addr);
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        continue;
                    }
                    Err(_) => break,
                }
            }
        });

        // KCP 更新任务：同样使用读锁 + per-session 锁
        let update_sessions = sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(KCP_UPDATE_INTERVAL_MS));
            let start = std::time::Instant::now();
            loop {
                interval.tick().await;
                let now = std::time::Instant::now();
                let current_ms = now.duration_since(start).as_millis() as u32;

                let session_list: Vec<(SocketAddr, Arc<std::sync::Mutex<KcpSession>>)> = {
                    let sessions = update_sessions.read().await;
                    sessions.iter().map(|(a, s)| (*a, s.clone())).collect()
                };

                let mut dead_addrs = Vec::new();

                for (addr, session_arc) in session_list {
                    if let Ok(mut session) = session_arc.lock() {
                        if now.duration_since(session.last_active).as_secs() > 30 {
                            dead_addrs.push(addr);
                            continue;
                        }
                        if let Err(e) = session.kcp.update(current_ms) {
                            tracing::warn!("[KCP] update error for {}: {}", addr, e);
                        }
                    }
                }

                if !dead_addrs.is_empty() {
                    let mut sessions = update_sessions.write().await;
                    for addr in &dead_addrs {
                        tracing::info!("[KCP] 超时移除客户端: {}", addr);
                        sessions.remove(addr);
                    }
                }
            }
        });

        Ok(())
    }
}

/// 生成 XOR 前向纠错数据
fn generate_kcp_fec_xor(frames: &[Arc<Vec<u8>>]) -> Vec<u8> {
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

fn build_kcp_fec_message(group_id: u32, fec_data: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(3 + 4 + fec_data.len());
    msg.extend_from_slice(b"fec");
    msg.extend_from_slice(&group_id.to_be_bytes());
    msg.extend_from_slice(fec_data);
    msg
}
