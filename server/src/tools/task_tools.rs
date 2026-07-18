use crate::tools::{parse_arg, AiTool};
use futures::future::BoxFuture;
use serde_json::json;
use std::fs;
use std::path::Path;

use crate::config::{SCHEDULE_FILE, SCRIPTS_DIR};

pub struct ListTasksTool;
pub struct AddTaskTool;
pub struct DeleteTaskTool;
pub struct ModifyTaskTool;
pub struct ListScriptsForTaskTool;

/// 读取所有任务
fn read_tasks() -> Vec<(String, String, String, String, Option<u32>)> {
    let content = fs::read_to_string(SCHEDULE_FILE).unwrap_or_default();
    content
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            (
                parts
                    .first()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                parts
                    .get(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "*".to_string()),
                parts
                    .get(2)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default(),
                parts
                    .get(3)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "daily".to_string()),
                parts.get(4).and_then(|s| s.trim().parse().ok()),
            )
        })
        .collect()
}

/// 写入所有任务
fn write_tasks(tasks: &[(String, String, String, String, Option<u32>)]) -> Result<(), String> {
    let content: String = tasks
        .iter()
        .map(|(time, weeks, script, task_type, interval)| {
            if let Some(iv) = interval {
                format!("{}|{}|{}|{}|{}", time, weeks, script, task_type, iv)
            } else {
                format!("{}|{}|{}|{}", time, weeks, script, task_type)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut content = content;
    content.push('\n');
    fs::write(SCHEDULE_FILE, content).map_err(|e| format!("写入失败: {}", e))
}

/// 列出所有可用脚本
fn list_available_scripts() -> Vec<String> {
    if let Ok(entries) = fs::read_dir(SCRIPTS_DIR) {
        entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
            .collect()
    } else {
        Vec::new()
    }
}

impl AiTool for ListTasksTool {
    fn name(&self) -> &str {
        "list_tasks"
    }
    fn description(&self) -> &str {
        "查看所有定时任务列表，返回每个任务的ID、时间、星期、脚本名、类型"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({"type": "object", "properties": {}})
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async {
            let tasks = read_tasks();
            if tasks.is_empty() {
                return "当前没有定时任务。".to_string();
            }
            let mut result = format!("共 {} 个定时任务：\n", tasks.len());
            for (i, (time, weeks, script, task_type, interval)) in tasks.iter().enumerate() {
                result.push_str(&format!(
                    "ID:{} | 时间:{} | 星期:{} | 脚本:{} | 类型:{}",
                    i + 1,
                    time,
                    weeks,
                    script,
                    task_type
                ));
                if let Some(iv) = interval {
                    result.push_str(&format!(" | 间隔:{}秒", iv));
                }
                result.push('\n');
            }
            result
        })
    }
}

impl AiTool for AddTaskTool {
    fn name(&self) -> &str {
        "add_task"
    }
    fn description(&self) -> &str {
        "添加新的定时任务。time格式如 08:30，weeks用逗号分隔如 1,2,3,4,5（1=周一）或 * 表示每天，task_type可选 daily/interval/weekly"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "time": {"type": "string", "description": "执行时间，格式 HH:MM，如 08:30"},
                "weeks": {"type": "string", "description": "星期，逗号分隔如 1,2,3,4,5 或 * 表示每天（默认*）"},
                "script": {"type": "string", "description": "脚本文件名"},
                "task_type": {"type": "string", "description": "任务类型：daily/interval/weekly（默认daily）"},
                "interval": {"type": "integer", "description": "间隔秒数（仅interval类型需要）"}
            },
            "required": ["time", "script"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            let time = match parse_arg::<String>(&args, "time") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let script = match parse_arg::<String>(&args, "script") {
                Ok(v) => v,
                Err(e) => return e,
            };
            let weeks = parse_arg::<String>(&args, "weeks").unwrap_or_else(|_| "*".to_string());
            let task_type =
                parse_arg::<String>(&args, "task_type").unwrap_or_else(|_| "daily".to_string());
            let interval = parse_arg::<i64>(&args, "interval")
                .ok()
                .and_then(|v| u32::try_from(v).ok());

            // 检查脚本是否存在
            let script_path = format!("{}/{}", SCRIPTS_DIR, script);
            if !Path::new(&script_path).exists() {
                return format!(
                    "脚本不存在: {}，可用脚本: {}",
                    script,
                    list_available_scripts().join(", ")
                );
            }

            let mut tasks = read_tasks();
            tasks.push((
                time.clone(),
                weeks.clone(),
                script.clone(),
                task_type.clone(),
                interval,
            ));
            match write_tasks(&tasks) {
                Ok(()) => format!(
                    "任务已添加：{} {} {} [{}] 30秒内自动生效",
                    time, weeks, script, task_type
                ),
                Err(e) => e,
            }
        })
    }
}

impl AiTool for DeleteTaskTool {
    fn name(&self) -> &str {
        "delete_task"
    }
    fn description(&self) -> &str {
        "根据ID删除定时任务，ID从1开始"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer", "description": "任务ID（从1开始）"}
            },
            "required": ["id"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            let id = match parse_arg::<i64>(&args, "id") {
                Ok(v) => v as usize,
                Err(e) => return e,
            };
            let mut tasks = read_tasks();
            if id == 0 || id > tasks.len() {
                return format!("任务ID不存在，当前共{}个任务", tasks.len());
            }
            let removed = tasks.remove(id - 1);
            match write_tasks(&tasks) {
                Ok(()) => format!(
                    "已删除任务ID{}: {} {} {}",
                    id, removed.0, removed.1, removed.2
                ),
                Err(e) => e,
            }
        })
    }
}

impl AiTool for ModifyTaskTool {
    fn name(&self) -> &str {
        "modify_task"
    }
    fn description(&self) -> &str {
        "修改指定ID的定时任务，只需传入要修改的字段"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "id": {"type": "integer", "description": "任务ID（从1开始）"},
                "time": {"type": "string", "description": "新的执行时间 HH:MM（可选）"},
                "weeks": {"type": "string", "description": "新的星期（可选）"},
                "script": {"type": "string", "description": "新的脚本名（可选）"},
                "task_type": {"type": "string", "description": "新的任务类型（可选）"},
                "interval": {"type": "integer", "description": "新的间隔秒数（可选）"}
            },
            "required": ["id"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            let id = match parse_arg::<i64>(&args, "id") {
                Ok(v) => v as usize,
                Err(e) => return e,
            };
            let mut tasks = read_tasks();
            if id == 0 || id > tasks.len() {
                return format!("任务ID不存在，当前共{}个任务", tasks.len());
            }

            let task = &mut tasks[id - 1];

            if let Ok(time) = parse_arg::<String>(&args, "time") {
                task.0 = time;
            }
            if let Ok(weeks) = parse_arg::<String>(&args, "weeks") {
                task.1 = weeks;
            }
            if let Ok(script) = parse_arg::<String>(&args, "script") {
                let script_path = format!("{}/{}", SCRIPTS_DIR, script);
                if !Path::new(&script_path).exists() {
                    return format!(
                        "脚本不存在: {}，可用脚本: {}",
                        script,
                        list_available_scripts().join(", ")
                    );
                }
                task.2 = script;
            }
            if let Ok(task_type) = parse_arg::<String>(&args, "task_type") {
                task.3 = task_type;
            }
            if let Ok(interval) = parse_arg::<i64>(&args, "interval") {
                task.4 = u32::try_from(interval).ok();
            }

            let desc = format!("{} {} {} [{}]", task.0, task.1, task.2, task.3);
            match write_tasks(&tasks) {
                Ok(()) => format!("任务ID{}已修改为: {} 30秒内自动生效", id, desc),
                Err(e) => e,
            }
        })
    }
}

impl AiTool for ListScriptsForTaskTool {
    fn name(&self) -> &str {
        "list_available_scripts"
    }
    fn description(&self) -> &str {
        "列出所有可用于定时任务的脚本文件"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({"type": "object", "properties": {}})
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async {
            let scripts = list_available_scripts();
            if scripts.is_empty() {
                "脚本目录为空，请先创建脚本。".to_string()
            } else {
                format!("可用脚本（{}个）：\n{}", scripts.len(), scripts.join("\n"))
            }
        })
    }
}
