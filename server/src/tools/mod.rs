use futures::future::BoxFuture;
use serde_json::{self, json, Value};
use std::collections::HashMap;

pub mod adb_tools;
pub mod script_tools;
pub mod task_tools;

pub trait AiTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    fn execute(&self, args: &str) -> BoxFuture<'_, String>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn AiTool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn AiTool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get_tools_json(&self) -> Value {
        let mut tools: Vec<Value> = Vec::new();
        for tool in self.tools.values() {
            tools.push(json!({
                "type": "function",
                "function": {
                    "name": tool.name(),
                    "description": tool.description(),
                    "parameters": tool.parameters(),
                }
            }));
        }
        Value::Array(tools)
    }

    pub async fn execute(&self, name: &str, args: &str) -> Option<String> {
        if let Some(tool) = self.tools.get(name) {
            Some(tool.execute(args).await)
        } else {
            None
        }
    }
}

pub fn parse_arg<T: serde::de::DeserializeOwned>(args: &str, name: &str) -> Result<T, String> {
    let args_json: Value = serde_json::from_str(args)
        .map_err(|_| "参数解析失败".to_string())?;
    
    let value = args_json.get(name)
        .ok_or_else(|| format!("缺少参数: {}", name))?
        .clone();
    
    serde_json::from_value(value)
        .map_err(|_| format!("参数类型错误: {}", name))
}

#[allow(dead_code)]
pub fn parse_args_str(args: &str) -> Result<String, String> {
    serde_json::from_str::<Value>(args)
        .map_err(|_| "参数解析失败".to_string())
        .map(|v| v.to_string())
}