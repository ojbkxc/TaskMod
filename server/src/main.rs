use axum::{routing::{delete, get, post}, Router};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use crate::config::WEB_PORT;
use crate::state::MirrorState;

mod api;
mod config;
mod data;
mod state;
mod tools;
mod utils;

fn ensure_dirs() {
    use crate::config::{TASKMOD_DIR, SCRIPTS_DIR, SCREENSHOTS_DIR, WORKFLOWS_DIR};
    let _ = std::fs::create_dir_all(TASKMOD_DIR);
    let _ = std::fs::create_dir_all(SCRIPTS_DIR);
    let _ = std::fs::create_dir_all(SCREENSHOTS_DIR);
    let _ = std::fs::create_dir_all(WORKFLOWS_DIR);
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    ensure_dirs();

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
        .route("/ws/mirror", get(api::mirror::mirror_ws))
        .route("/ws/audio", get(api::mirror::audio_ws))
        .with_state(mirror_state);

    let app = Router::new()
        .route("/", get(api::system::index))
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
        .route("/api/mqtt/config", get(api::system::get_mqtt_config).put(api::system::save_mqtt_config))
        .route("/api/workflows", get(api::system::list_workflows_api).post(api::system::save_workflow_api))
        .route("/api/workflows/:id", get(api::system::get_workflow).delete(api::system::delete_workflow_api))
        .route("/api/workflows/run", post(api::system::run_workflow))
        .route("/api/status", get(api::system::system_status))
        .route("/api/tts/engines", get(api::tts::get_tts_engines))
        .route("/api/tts/speak", post(api::tts::speak))
        .route("/api/ai/providers", get(api::ai::list_ai_providers).post(api::ai::add_ai_provider))
        .route("/api/ai/providers/:id", get(api::ai::get_ai_provider_api).put(api::ai::update_ai_provider).delete(api::ai::delete_ai_provider))
        .route("/ws/ai-chat", get(api::ai::ai_chat_ws))
        .merge(mirror_routes)
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], WEB_PORT));
    println!("TaskMod Web 管理服务已启动: http://0.0.0.0:{}", WEB_PORT);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}