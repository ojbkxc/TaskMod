use axum::{routing::{delete, get, post, put}, Router, Json};
use serde_json;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::config::WEB_PORT;
use crate::state::MirrorState;

mod api;
mod config;
mod data;
mod state;
mod tools;
mod utils;

/// 看门狗：定期检测主循环是否卡死
/// 如果心跳超过 timeout_secs 没有更新，发送告警邮件
fn start_watchdog(heartbeat: Arc<AtomicBool>, timeout_secs: u64) {
    if timeout_secs == 0 {
        return;
    }
    std::thread::spawn(move || {
        let mut last_alive = true;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            if heartbeat.load(Ordering::Relaxed) {
                heartbeat.store(false, Ordering::Relaxed);
                last_alive = true;
            } else if last_alive {
                let email_conf = utils::email::get_email_config();
                if email_conf.enable_notify {
                    let now = chrono::Local::now();
                    let subject = format!("[WARNING] TaskMod 可能卡死 - {}", now.format("%Y-%m-%d %H:%M:%S"));
                    let body = format!(
                        "TaskMod 主循环已超过 {} 秒未响应心跳。\n\n可能原因：\n- 主线程阻塞\n- 资源耗尽（内存/CPU）\n- 死锁\n\n请检查设备状态。",
                        timeout_secs
                    );
                    std::thread::spawn(move || {
                        let rt = match tokio::runtime::Runtime::new() {
                            Ok(rt) => rt,
                            Err(_) => return,
                        };
                        let _ = rt.block_on(utils::email::send_email(
                            &email_conf,
                            Some(&subject),
                            Some(&body),
                            None,
                        ));
                    });
                }
                eprintln!("[看门狗] 警告: 主循环可能卡死，已超过 {} 秒未响应", timeout_secs);
                last_alive = false;
            }
        }
    });
}

async fn ensure_dirs() {
    use crate::config::{TASKMOD_DIR, SCRIPTS_DIR, SCREENSHOTS_DIR, WORKFLOWS_DIR};
    let _ = std::fs::create_dir_all(TASKMOD_DIR);
    let _ = std::fs::create_dir_all(SCRIPTS_DIR);
    let _ = std::fs::create_dir_all(SCREENSHOTS_DIR);
    let _ = std::fs::create_dir_all(WORKFLOWS_DIR);
    api::ai_hub::init_dirs().await;
}

fn handle_event(event: utils::event_monitor::SystemEvent) {
    use utils::event_monitor::SystemEvent;
    
    let workflows = api::system::list_workflows();
    
    let context = match &event {
        SystemEvent::WifiConnected { ssid, signal_level } => {
            serde_json::json!({
                "wifi_ssid": ssid,
                "wifi_signal": signal_level,
                "event_type": "wifi_connected"
            })
        }
        SystemEvent::WifiDisconnected => {
            serde_json::json!({
                "wifi_ssid": "",
                "event_type": "wifi_disconnected"
            })
        }
        SystemEvent::BatteryLow { capacity } => {
            serde_json::json!({
                "battery_capacity": capacity,
                "event_type": "battery_low"
            })
        }
        SystemEvent::BatteryCharging { capacity } => {
            serde_json::json!({
                "battery_capacity": capacity,
                "event_type": "battery_charging"
            })
        }
        SystemEvent::BatteryFull => {
            serde_json::json!({
                "battery_capacity": 100,
                "event_type": "battery_full"
            })
        }
        SystemEvent::ScreenOn => {
            serde_json::json!({
                "screen_on": true,
                "event_type": "screen_on"
            })
        }
        SystemEvent::ScreenOff => {
            serde_json::json!({
                "screen_on": false,
                "event_type": "screen_off"
            })
        }
    };

    for workflow in workflows {
        if !workflow.enabled {
            continue;
        }

        let trigger_matches = match (workflow.trigger_type.as_str(), &event) {
            ("wifi_connected", SystemEvent::WifiConnected { ssid, .. }) => {
                workflow.trigger_config.wifi_ssid.is_empty() || workflow.trigger_config.wifi_ssid == *ssid
            }
            ("wifi_disconnected", SystemEvent::WifiDisconnected) => true,
            ("battery_low", SystemEvent::BatteryLow { capacity }) => {
                *capacity <= workflow.trigger_config.battery_threshold
            }
            ("battery_charging", SystemEvent::BatteryCharging { .. }) => true,
            ("battery_full", SystemEvent::BatteryFull) => true,
            ("screen_on", SystemEvent::ScreenOn) => true,
            ("screen_off", SystemEvent::ScreenOff) => true,
            _ => false,
        };

        if trigger_matches {
            println!("[事件触发] 工作流: {} 被事件: {:?} 触发", workflow.name, event);
            let wf = workflow.clone();
            let ctx = context.clone();
            tokio::spawn(async move {
                api::system::execute_workflow(wf, Some(ctx)).await;
            });
        }
    }
}

async fn save_mqtt_config_handler(Json(req): Json<crate::data::models::MqttConfigRequest>) -> Json<crate::data::response::ApiResponse<String>> {
    let mqtt_config = utils::mqtt::MqttConfig { 
        enabled: req.enabled, 
        broker: req.broker, 
        topic_prefix: req.topic_prefix, 
        username: req.username, 
        password: req.password, 
        client_id: req.client_id 
    };
    
    if let Err(e) = utils::mqtt::save_mqtt_config(&mqtt_config) {
        return Json(crate::data::response::ApiResponse::err(&format!("保存失败: {}", e)));
    }
    
    utils::mqtt::stop_mqtt().await;
    tokio::spawn(async { utils::mqtt::start_mqtt().await; });
    
    Json(crate::data::response::ApiResponse::ok_msg("ok".to_string(), "MQTT配置已保存，服务已重启"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    ensure_dirs().await;

    // 设置 panic hook，在进程崩溃时发送告警邮件
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        let location = info.location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "Unknown location".to_string());
        let now = chrono::Local::now();
        eprintln!("[CRITICAL] TaskMod 崩溃: {} at {}", msg, location);
        let email_conf = utils::email::get_email_config();
        if email_conf.enable_notify {
            let subject = format!("[CRITICAL] TaskMod 崩溃 - {}", now.format("%Y-%m-%d %H:%M:%S"));
            let body = format!("位置: {}\n错误: {}\n\n请检查设备状态并重启服务。", location, msg);
            // 在独立线程中发送邮件，避免嵌套运行时 panic
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(_) => return,
                };
                let _ = rt.block_on(
                    utils::email::send_email(&email_conf, Some(&subject), Some(&body), None)
                );
            });
        }
        default_hook(info);
    }));

    // 启动看门狗（60秒超时）
    let heartbeat = Arc::new(AtomicBool::new(true));
    start_watchdog(heartbeat.clone(), 60);

    // 先启动心跳更新，再执行可能耗时的初始化
    let heartbeat_clone = heartbeat.clone();
    tokio::spawn(async move {
        loop {
            heartbeat_clone.store(true, Ordering::Relaxed);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    // 启动记忆清理定时任务（每天清理一次，归档30天未更新的记忆）
    tokio::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(86400)).await;
            api::ai_hub::archive_stale_memories(30).await;
        }
    });

    let mirror_state = MirrorState::new_shared();

    utils::event_monitor::register_event_handler(handle_event);
    utils::event_monitor::start_monitor(10000);

    tokio::spawn(async {
        utils::mqtt::start_mqtt().await;
    });

    let mirror_routes = Router::new()
        .route("/api/mirror/start", post(api::mirror::start_mirror))
        .route("/api/mirror/stop", post(api::mirror::stop_mirror))
        .route("/api/mirror/control", post(api::mirror::send_control))
        .route("/api/mirror/status", get(api::mirror::mirror_status))
        .route("/api/mirror/screencap", get(api::mirror::screencap_jpeg))
        .route("/ws/mirror", get(api::mirror::mirror_ws))
        .route("/ws/audio", get(api::mirror::audio_ws))
        .with_state(mirror_state);

    let app = Router::new()
        .route("/", get(api::system::index))
        .route("/static/style.css", get(api::system::static_css))
        .route("/static/app.js", get(api::system::static_js))
        .route("/api/tasks", get(api::tasks::list_tasks).post(api::tasks::add_task))
        .route("/api/tasks/:id", delete(api::tasks::delete_task))
        .route("/api/logs", get(api::system::get_logs))
        .route("/api/logs/clear", post(api::system::clear_logs))
        .route("/api/screenshots", get(api::system::list_screenshots))
        .route("/api/screenshots/take", post(api::system::take_screenshot))
        .route("/api/screenshots/:filename", get(api::system::get_screenshot).delete(api::system::delete_screenshot))
        .route("/api/scripts", get(api::scripts::list_scripts))
        .route("/api/scripts/:name", get(api::scripts::get_script).put(api::scripts::save_script).delete(api::scripts::delete_script))
        .route("/api/trigger", post(api::tasks::trigger_script))
        .route("/api/command", post(api::system::exec_command))
        .route("/api/config", get(api::system::get_config).put(api::system::update_config))
        .route("/api/email/config", get(api::system::get_email_config).put(api::system::save_email_config))
        .route("/api/send-email", post(api::system::send_email))
        .route("/api/mqtt/config", get(api::system::get_mqtt_config))
        .route("/api/mqtt/config", put(save_mqtt_config_handler))
        .route("/api/workflows", get(api::system::list_workflows_api).post(api::system::save_workflow_api))
        .route("/api/workflows/:id", get(api::system::get_workflow).delete(api::system::delete_workflow_api))
        .route("/api/workflows/run", post(api::system::run_workflow))
        .route("/api/status", get(api::system::system_status))
        .route("/api/tts/engines", get(api::tts::get_tts_engines))
        .route("/api/tts/speak", post(api::tts::speak))
        .route("/api/tts/stop", post(api::tts::stop_tts))
        .route("/api/ai/providers", get(api::ai::list_ai_providers).post(api::ai::add_ai_provider))
        .route("/api/ai/providers/:id", get(api::ai::get_ai_provider_api).put(api::ai::update_ai_provider).delete(api::ai::delete_ai_provider))
        .route("/ws/ai-chat", get(api::ai::ai_chat_ws))
        // AI Hub: 对话历史
        .route("/api/ai/sessions", get(api::ai_hub::list_sessions).post(api::ai_hub::create_session))
        .route("/api/ai/sessions/:id", get(api::ai_hub::get_session).put(api::ai_hub::update_session).delete(api::ai_hub::delete_session))
        // AI Hub: Prompt预设
        .route("/api/ai/presets", get(api::ai_hub::list_presets).post(api::ai_hub::save_preset))
        .route("/api/ai/presets/:id", put(api::ai_hub::update_preset).delete(api::ai_hub::delete_preset))
        // AI Hub: 记忆系统
        .route("/api/ai/memories", get(api::ai_hub::list_memories).post(api::ai_hub::create_memory))
        .route("/api/ai/memories/:id", put(api::ai_hub::update_memory).delete(api::ai_hub::delete_memory))
        // AI Hub: Skill系统
        .route("/api/ai/skills", get(api::ai_hub::list_skills).post(api::ai_hub::create_skill))
        .route("/api/ai/skills/:id", put(api::ai_hub::update_skill).delete(api::ai_hub::delete_skill))
        // AI Hub: 保存项
        .route("/api/ai/saved", get(api::ai_hub::list_saved_items).post(api::ai_hub::create_saved_item))
        .route("/api/ai/saved/:id", put(api::ai_hub::update_saved_item).delete(api::ai_hub::delete_saved_item))
        // AI Hub: 项目上下文
        .route("/api/ai/projects", get(api::ai_hub::list_projects).post(api::ai_hub::create_project))
        .route("/api/ai/projects/:id", put(api::ai_hub::update_project).delete(api::ai_hub::delete_project))
        // AI Hub: MCP服务器
        .route("/api/ai/mcp", get(api::ai_hub::list_mcp_servers).post(api::ai_hub::create_mcp_server))
        .route("/api/ai/mcp/:id", put(api::ai_hub::update_mcp_server).delete(api::ai_hub::delete_mcp_server))
        // AI Hub: 截图分析 & 对话导出
        .route("/api/ai/screenshot", post(api::ai_hub::screenshot_analyze))
        .route("/api/ai/export", post(api::ai_hub::export_session))
        // AI Hub: Prompt设置 & 场景模板
        .route("/api/ai/prompt-settings", get(api::ai_hub::get_prompt_settings).put(api::ai_hub::update_prompt_settings))
        .route("/api/ai/scenarios", get(api::ai_hub::list_scenarios).post(api::ai_hub::create_scenario))
        .route("/api/ai/scenarios/:id", put(api::ai_hub::update_scenario).delete(api::ai_hub::delete_scenario))
        // AI Hub: 图片上传 & Skill GitHub导入
        .route("/api/ai/upload-image", post(api::ai_hub::upload_image))
        .route("/api/ai/images/:id", get(api::ai_hub::get_image))
        .route("/api/ai/skills/import", post(api::ai_hub::import_skill_from_github))
        // AI Hub: MCP工具历史 & 使用量统计
        .route("/api/ai/mcp-history", get(api::ai_hub::list_mcp_history).post(api::ai_hub::record_mcp_tool_call).delete(api::ai_hub::clear_mcp_history))
        .route("/api/ai/usage", get(api::ai_hub::get_usage_stats).post(api::ai_hub::record_usage))
        // 设备文件上传 & 剪贴板同步 (借鉴QtScrcpy)
        .route("/api/device/upload-file", post(api::mirror::upload_file_to_device))
        .route("/api/device/clipboard", get(api::mirror::get_device_clipboard).put(api::mirror::set_device_clipboard))
        .route("/api/device/info", get(api::mirror::get_device_info))
        .merge(mirror_routes)
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], WEB_PORT));
    println!("TaskMod Web 管理服务已启动: http://0.0.0.0:{}", WEB_PORT);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}