use axum::{routing::{delete, get, post, put}, Router};
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    ensure_dirs();

    let mirror_state = MirrorState::new_shared();

    tokio::spawn(async {
        utils::mqtt::start_mqtt().await;
    });

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
        .route("/api/scripts/:name", get(api::scripts::get_script).put(api::scripts::save_script))
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
        .route("/api/mirror/start", post(api::mirror::start_mirror).with_state(mirror_state.clone()))
        .route("/api/mirror/stop", post(api::mirror::stop_mirror).with_state(mirror_state.clone()))
        .route("/api/mirror/control", post(api::mirror::send_control).with_state(mirror_state.clone()))
        .route("/api/mirror/status", get(api::mirror::mirror_status).with_state(mirror_state.clone()))
        .route("/ws/mirror", get(api::mirror::mirror_ws).with_state(mirror_state.clone()))
        .route("/ws/ai-chat", get(api::ai::ai_chat_ws))
        .route("/ws/audio", get(api::mirror::audio_ws).with_state(mirror_state))
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], WEB_PORT));
    println!("TaskMod Web 管理服务已启动: http://0.0.0.0:{}", WEB_PORT);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}