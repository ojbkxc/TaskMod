use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: usize,
    pub time: String,
    pub weeks: String,
    pub script: String,
    pub task_type: String,
    pub interval: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AddTaskRequest {
    pub time: String,
    pub weeks: Option<String>,
    pub script: String,
    pub task_type: String,
    pub interval: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct EmailConfig {
    #[serde(default)]
    pub enable_notify: String,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
    #[serde(default)]
    pub timeout_secs: u64,
    #[serde(default)]
    pub max_retries: u32,
    #[serde(default)]
    pub retry_interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct TriggerRequest {
    pub script: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigUpdate {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct CommandRequest {
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct AiProviderRequest {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct AiChatRequest {
    pub provider_id: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub x: f64,
    pub y: f64,
    pub label: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct WorkflowTriggerConfig {
    #[serde(default)]
    pub wifi_ssid: String,
    #[serde(default)]
    pub battery_threshold: i32,
    #[serde(default)]
    pub screen_state: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub trigger_type: String,
    #[serde(default)]
    pub trigger_config: WorkflowTriggerConfig,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowSaveRequest {
    pub workflow: Workflow,
}

#[derive(Debug, Deserialize)]
pub struct WorkflowRunRequest {
    pub workflow_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MqttConfigRequest {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_broker")]
    pub broker: String,
    #[serde(default = "default_topic_prefix")]
    pub topic_prefix: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_client_id")]
    pub client_id: String,
}

fn default_broker() -> String { "tcp://localhost:1883".to_string() }
fn default_topic_prefix() -> String { "taskmod".to_string() }
fn default_client_id() -> String { "taskmod-device".to_string() }

