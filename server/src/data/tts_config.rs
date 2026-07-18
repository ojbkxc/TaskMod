use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

/// TTS 配置文件路径（Magisk 模块持久化目录）
const CONFIG_PATH: &str = "/data/adb/TaskMod/tts_config.json";

/// 文本替换规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceRule {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub replacement: String,
    #[serde(default)]
    pub is_regex: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub order: i32,
}

fn default_true() -> bool {
    true
}

/// 单个引擎的独立参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineParams {
    pub engine: String,
    #[serde(default = "default_f32_1")]
    pub rate: f32,
    #[serde(default = "default_f32_1")]
    pub pitch: f32,
    #[serde(default = "default_f32_1")]
    pub volume: f32,
}

fn default_f32_1() -> f32 {
    1.0
}

/// 完整的 TTS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    #[serde(default)]
    pub default_engine: String,
    #[serde(default = "default_f32_1")]
    pub global_rate: f32,
    #[serde(default = "default_f32_1")]
    pub global_pitch: f32,
    #[serde(default = "default_f32_1")]
    pub global_volume: f32,
    #[serde(default)]
    pub engine_params: Vec<EngineParams>,
    #[serde(default)]
    pub replace_rules: Vec<ReplaceRule>,
    /// 是否启用文本替换
    #[serde(default)]
    pub replace_enabled: bool,
    /// 是否启用分句
    #[serde(default)]
    pub split_enabled: bool,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            default_engine: String::new(),
            global_rate: 1.0,
            global_pitch: 1.0,
            global_volume: 1.0,
            engine_params: Vec::new(),
            replace_rules: Vec::new(),
            replace_enabled: false,
            split_enabled: true,
        }
    }
}

impl TtsConfig {
    /// 从文件加载配置，文件不存在则返回默认配置
    pub async fn load() -> Self {
        if !Path::new(CONFIG_PATH).exists() {
            return Self::default();
        }
        match fs::read_to_string(CONFIG_PATH).await {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                eprintln!("[TTS Config] 解析配置文件失败: {}, 使用默认配置", e);
                Self::default()
            }),
            Err(e) => {
                eprintln!("[TTS Config] 读取配置文件失败: {}, 使用默认配置", e);
                Self::default()
            }
        }
    }

    /// 保存配置到文件
    pub async fn save(&self) -> Result<(), String> {
        // 确保目录存在
        if let Some(parent) = Path::new(CONFIG_PATH).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            }
        }
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("序列化配置失败: {}", e))?;
        fs::write(CONFIG_PATH, json)
            .await
            .map_err(|e| format!("写入配置文件失败: {}", e))?;
        Ok(())
    }

    /// 获取指定引擎的参数（未配置则返回全局参数）
    pub fn get_engine_params(&self, engine: &str) -> (f32, f32, f32) {
        if let Some(params) = self.engine_params.iter().find(|p| p.engine == engine) {
            (params.rate, params.pitch, params.volume)
        } else {
            (self.global_rate, self.global_pitch, self.global_volume)
        }
    }

    /// 对文本执行替换规则
    pub fn apply_replace_rules(&self, text: &str) -> String {
        if !self.replace_enabled || self.replace_rules.is_empty() {
            return text.to_string();
        }
        let mut result = text.to_string();
        let mut rules: Vec<&ReplaceRule> =
            self.replace_rules.iter().filter(|r| r.enabled).collect();
        rules.sort_by_key(|r| r.order);
        for rule in rules {
            if rule.is_regex {
                match regex::Regex::new(&rule.pattern) {
                    Ok(re) => {
                        result = re
                            .replace_all(&result, rule.replacement.as_str())
                            .to_string();
                    }
                    Err(e) => {
                        eprintln!("[TTS Replace] 正则表达式错误 '{}': {}", rule.pattern, e);
                    }
                }
            } else {
                result = result.replace(&rule.pattern, &rule.replacement);
            }
        }
        result
    }

    /// 智能分句：按标点符号切分长文本
    pub fn split_sentences(&self, text: &str) -> Vec<String> {
        if !self.split_enabled || text.chars().count() <= 200 {
            return vec![text.to_string()];
        }
        split_text(text)
    }
}

/// 按中英文标点符号和换行符分句
fn split_text(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    // 分句标点：。！？；\n
    let delimiters: &[char] = &['。', '！', '？', '；', '\n', '.', '!', '?', ';'];

    for ch in text.chars() {
        current.push(ch);
        if delimiters.contains(&ch) {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
        }
    }
    // 处理末尾没有标点的部分
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }

    // 如果分句结果只有一段（没有标点），按逗号再分
    if sentences.len() <= 1 {
        let original = sentences.first().cloned().unwrap_or_default();
        if original.chars().count() > 200 {
            let mut sub_sentences = Vec::new();
            let mut sub_current = String::new();
            let sub_delimiters: &[char] = &['，', ',', '、', ' '];
            for ch in original.chars() {
                sub_current.push(ch);
                if sub_delimiters.contains(&ch) && sub_current.chars().count() >= 50 {
                    let trimmed = sub_current.trim().to_string();
                    if !trimmed.is_empty() {
                        sub_sentences.push(trimmed);
                    }
                    sub_current.clear();
                }
            }
            let trimmed = sub_current.trim().to_string();
            if !trimmed.is_empty() {
                sub_sentences.push(trimmed);
            }
            if sub_sentences.len() > 1 {
                return sub_sentences;
            }
        }
    }

    if sentences.is_empty() {
        vec![text.to_string()]
    } else {
        sentences
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sentences() {
        let config = TtsConfig::default();
        let text = "你好世界。这是第二句！这是第三句？";
        let result = config.split_sentences(text);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "你好世界。");
        assert_eq!(result[1], "这是第二句！");
        assert_eq!(result[2], "这是第三句？");
    }

    #[test]
    fn test_replace_rules() {
        let mut config = TtsConfig::default();
        config.replace_enabled = true;
        config.replace_rules.push(ReplaceRule {
            id: "1".to_string(),
            name: "test".to_string(),
            pattern: "&".to_string(),
            replacement: "和".to_string(),
            is_regex: false,
            enabled: true,
            order: 0,
        });
        assert_eq!(config.apply_replace_rules("A&B"), "A和B");
    }
}
