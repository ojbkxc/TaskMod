use crate::config::SCRIPTS_DIR;
use crate::config::LOG_FILE;
use crate::tools::{AiTool, parse_arg};
use serde_json::json;
use std::fs;
use std::path::Path;
use tokio::process::Command;

pub struct ListScriptsTool;
pub struct ReadScriptTool;
pub struct WriteScriptTool;
pub struct DeleteScriptTool;
pub struct RunScriptTool;
pub struct ViewLogsTool;

impl AiTool for ListScriptsTool {
    fn name(&self) -> &str { "list_scripts" }
    fn description(&self) -> &str { "列出scripts目录下所有脚本文件" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    async fn execute(&self, _args: &str) -> String {
        match fs::read_dir(SCRIPTS_DIR) {
            Ok(dir) => {
                let files: Vec<String> = dir
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                    .collect();
                if files.is_empty() {
                    "脚本目录为空".to_string()
                } else {
                    format!("可用脚本:\n{}", files.join("\n"))
                }
            }
            Err(e) => format!("读取脚本目录失败: {}", e),
        }
    }
}

impl AiTool for ReadScriptTool {
    fn name(&self) -> &str { "read_script" }
    fn description(&self) -> &str { "读取指定脚本文件的内容" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "filename": {"type": "string", "description": "脚本文件名，如 midea.sh"}
            },
            "required": ["filename"]
        })
    }
    async fn execute(&self, args: &str) -> String {
        match parse_arg::<String>(args, "filename") {
            Ok(filename) => {
                if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
                    "无效的脚本名称".to_string()
                } else {
                    let script_path = format!("{}/{}", SCRIPTS_DIR, filename);
                    match fs::read_to_string(&script_path) {
                        Ok(content) => format!("脚本内容:\n{}", content),
                        Err(e) => format!("读取脚本失败: {}", e),
                    }
                }
            }
            Err(e) => e,
        }
    }
}

impl AiTool for WriteScriptTool {
    fn name(&self) -> &str { "write_script" }
    fn description(&self) -> &str { "创建或覆盖脚本文件" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "filename": {"type": "string", "description": "脚本文件名，如 midea.sh"},
                "content": {"type": "string", "description": "脚本内容"}
            },
            "required": ["filename", "content"]
        })
    }
    async fn execute(&self, args: &str) -> String {
        match (parse_arg::<String>(args, "filename"), parse_arg::<String>(args, "content")) {
            (Ok(filename), Ok(content)) => {
                if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
                    "无效的脚本名称".to_string()
                } else {
                    let script_path = format!("{}/{}", SCRIPTS_DIR, filename);
                    match fs::write(&script_path, content) {
                        Ok(_) => {
                            let _ = std::process::Command::new("chmod")
                                .arg("+x")
                                .arg(&script_path)
                                .status();
                            format!("脚本保存成功: {}", filename)
                        }
                        Err(e) => format!("保存脚本失败: {}", e),
                    }
                }
            }
            (Err(e), _) | (_, Err(e)) => e,
        }
    }
}

impl AiTool for DeleteScriptTool {
    fn name(&self) -> &str { "delete_script" }
    fn description(&self) -> &str { "删除指定脚本文件" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "filename": {"type": "string", "description": "脚本文件名，如 midea.sh"}
            },
            "required": ["filename"]
        })
    }
    async fn execute(&self, args: &str) -> String {
        match parse_arg::<String>(args, "filename") {
            Ok(filename) => {
                if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
                    "无效的脚本名称".to_string()
                } else {
                    let script_path = format!("{}/{}", SCRIPTS_DIR, filename);
                    match fs::remove_file(&script_path) {
                        Ok(_) => format!("脚本删除成功: {}", filename),
                        Err(e) => format!("删除脚本失败: {}", e),
                    }
                }
            }
            Err(e) => e,
        }
    }
}

impl AiTool for RunScriptTool {
    fn name(&self) -> &str { "run_script" }
    fn description(&self) -> &str { "执行指定脚本文件" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "filename": {"type": "string", "description": "脚本文件名，如 midea.sh"}
            },
            "required": ["filename"]
        })
    }
    async fn execute(&self, args: &str) -> String {
        match parse_arg::<String>(args, "filename") {
            Ok(filename) => {
                if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
                    "无效的脚本名称".to_string()
                } else {
                    let script_path = format!("{}/{}", SCRIPTS_DIR, filename);
                    match Command::new("/system/bin/sh")
                        .arg(&script_path)
                        .output()
                        .await
                    {
                        Ok(o) => format!(
                            "脚本执行成功:\nstdout: {}\nstderr: {}",
                            String::from_utf8_lossy(&o.stdout),
                            String::from_utf8_lossy(&o.stderr)
                        ),
                        Err(e) => format!("脚本执行失败: {}", e),
                    }
                }
            }
            Err(e) => e,
        }
    }
}

impl AiTool for ViewLogsTool {
    fn name(&self) -> &str { "view_logs" }
    fn description(&self) -> &str { "查看TaskMod运行日志" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "lines": {"type": "integer", "description": "要显示的行数，默认100"}
            }
        })
    }
    async fn execute(&self, args: &str) -> String {
        let lines = match parse_arg::<usize>(args, "lines") {
            Ok(l) => l,
            Err(_) => 100,
        };
        match fs::read_to_string(LOG_FILE) {
            Ok(content) => {
                let log_lines: Vec<&str> = content.lines().collect();
                let start = log_lines.len().saturating_sub(lines);
                format!("最近{}行日志:\n{}", lines, log_lines[start..].join("\n"))
            }
            Err(e) => format!("读取日志失败: {}", e),
        }
    }
}

pub fn register_script_tools(registry: &mut crate::tools::ToolRegistry) {
    registry.register(Box::new(ListScriptsTool));
    registry.register(Box::new(ReadScriptTool));
    registry.register(Box::new(WriteScriptTool));
    registry.register(Box::new(DeleteScriptTool));
    registry.register(Box::new(RunScriptTool));
    registry.register(Box::new(ViewLogsTool));
}