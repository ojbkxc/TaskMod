use dioxus::prelude::*;
use eq_ui::prelude::*;
use serde::{Deserialize, Serialize};

/// AI 供应商
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub enabled: bool,
}

#[component]
pub fn ConfigPage() -> Element {
    // AI 供应商列表
    let mut providers = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // 表单状态
    let mut show_form = use_signal(|| false);
    let mut editing_id = use_signal(|| None::<String>);
    let mut form_name = use_signal(String::new);
    let mut form_base_url = use_signal(String::new);
    let mut form_api_key = use_signal(String::new);
    let mut form_model = use_signal(String::new);
    let mut form_enabled = use_signal(|| true);

    // 测试状态: None | 测试中 | 成功(latency) | 失败(msg)
    let mut test_status = use_signal(|| TestStatus::Idle);
    let mut refresh = use_signal(|| 0u32);

    // 加载 AI 供应商列表
    use_effect(move || {
        let _ = *refresh.read();
        spawn(async move {
            loading.set(true);
            error.set(None);
            match reqwest::get("/api/ai/providers").await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = data["data"].as_array() {
                            let list: Vec<AiProvider> = arr
                                .iter()
                                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                                .collect();
                            providers.set(list);
                        }
                    }
                }
                Err(e) => {
                    error.set(Some(format!("加载失败: {}", e)));
                }
            }
            loading.set(false);
        });
    });

    // 重置表单
    let reset_form = move || {
        editing_id.set(None);
        form_name.set(String::new());
        form_base_url.set(String::new());
        form_api_key.set(String::new());
        form_model.set(String::new());
        form_enabled.set(true);
        test_status.set(TestStatus::Idle);
    };

    // 点击"添加通道"
    let on_add = move |_| {
        reset_form();
        show_form.set(true);
    };

    // 点击"编辑"
    let on_edit = move |provider: AiProvider| {
        editing_id.set(Some(provider.id.clone()));
        form_name.set(provider.name.clone());
        form_base_url.set(provider.base_url.clone());
        form_api_key.set(provider.api_key.clone());
        form_model.set(provider.model.clone());
        form_enabled.set(provider.enabled);
        test_status.set(TestStatus::Idle);
        show_form.set(true);
    };

    // 点击"取消"
    let on_cancel = move |_| {
        show_form.set(false);
        reset_form();
    };

    // 保存
    let on_save = move |_| {
        let name = form_name.read().clone();
        let base_url = form_base_url.read().clone();
        let api_key = form_api_key.read().clone();
        let model = form_model.read().clone();
        let enabled = *form_enabled.read();
        let edit_id = editing_id.read().clone();

        if name.is_empty() || base_url.is_empty() || model.is_empty() {
            error.set(Some("名称、Base URL、模型名称不能为空".to_string()));
            return;
        }

        let body = serde_json::json!({
            "name": name,
            "base_url": base_url,
            "api_key": api_key,
            "model": model,
            "enabled": enabled,
        });

        spawn(async move {
            loading.set(true);
            let result = if let Some(id) = &edit_id {
                let url = format!("/api/ai/providers/{}", id);
                reqwest::Client::new().put(&url).json(&body).send().await
            } else {
                reqwest::Client::new()
                    .post("/api/ai/providers")
                    .json(&body)
                    .send()
                    .await
            };

            match result {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if data["ok"].as_bool() == Some(true) {
                            show_form.set(false);
                            reset_form();
                            refresh += 1;
                        } else {
                            let msg = data["message"]
                                .as_str()
                                .unwrap_or("保存失败")
                                .to_string();
                            error.set(Some(msg));
                        }
                    }
                }
                Err(e) => {
                    error.set(Some(format!("请求失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    // 测试连接（使用表单数据）
    let on_test = move |_| {
        let base_url = form_base_url.read().clone();
        let api_key = form_api_key.read().clone();
        let model = form_model.read().clone();

        if base_url.is_empty() {
            error.set(Some("请先填写 Base URL".to_string()));
            return;
        }

        test_status.set(TestStatus::Testing);

        let body = serde_json::json!({
            "base_url": base_url,
            "api_key": api_key,
            "model": if model.is_empty() { None } else { Some(model) },
        });

        spawn(async move {
            match reqwest::Client::new()
                .post("/api/ai/test-connection")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if data["ok"].as_bool() == Some(true) {
                            let latency = data["data"]["latency"]
                                .as_u64()
                                .unwrap_or(0);
                            test_status.set(TestStatus::Success(latency));
                        } else {
                            let msg = data["message"]
                                .as_str()
                                .unwrap_or("连接失败")
                                .to_string();
                            test_status.set(TestStatus::Failed(msg));
                        }
                    } else {
                        test_status.set(TestStatus::Failed("解析响应失败".to_string()));
                    }
                }
                Err(e) => {
                    test_status.set(TestStatus::Failed(format!("请求失败: {}", e)));
                }
            }
        });
    };

    // 列表中点击"测试"：加载到表单并自动测试
    let on_test_provider = move |provider: AiProvider| {
        editing_id.set(Some(provider.id.clone()));
        form_name.set(provider.name.clone());
        form_base_url.set(provider.base_url.clone());
        form_api_key.set(provider.api_key.clone());
        form_model.set(provider.model.clone());
        form_enabled.set(provider.enabled);
        show_form.set(true);
        // 自动触发测试
        let base_url = provider.base_url.clone();
        let api_key = provider.api_key.clone();
        let model = provider.model.clone();
        test_status.set(TestStatus::Testing);
        spawn(async move {
            let body = serde_json::json!({
                "base_url": base_url,
                "api_key": api_key,
                "model": if model.is_empty() { None } else { Some(model) },
            });
            match reqwest::Client::new()
                .post("/api/ai/test-connection")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if data["ok"].as_bool() == Some(true) {
                            let latency = data["data"]["latency"].as_u64().unwrap_or(0);
                            test_status.set(TestStatus::Success(latency));
                        } else {
                            let msg = data["message"].as_str().unwrap_or("连接失败").to_string();
                            test_status.set(TestStatus::Failed(msg));
                        }
                    }
                }
                Err(e) => {
                    test_status.set(TestStatus::Failed(format!("请求失败: {}", e)));
                }
            }
        });
    };

    // 删除
    let on_delete = move |provider: AiProvider| {
        let id = provider.id.clone();
        spawn(async move {
            let url = format!("/api/ai/providers/{}", id);
            match reqwest::Client::new().delete(&url).send().await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if data["ok"].as_bool() == Some(true) {
                            refresh += 1;
                            if editing_id.read().as_deref() == Some(&id) {
                                show_form.set(false);
                                reset_form();
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        });
    };

    rsx! {
        div { class: "p-4 space-y-4",
            // 页面标题
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "配置" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "系统与服务配置" }
                }
            }

            div { class: "flex flex-col gap-4",
                // AI 通道配置
                EqCard { class: "p-5",
                    div { class: "flex items-center justify-between mb-4",
                        h3 { class: "font-semibold text-[var(--ds-text)]", "AI 通道" }
                        EqButton {
                            variant: EqButtonVariant::Ghost,
                            onclick: on_add,
                            disabled: *show_form.read(),
                            "添加通道"
                        }
                    }
                    div { class: "text-[11px] text-[var(--ds-text-tertiary)] mb-4",
                        "兼容 OpenAI API 格式，支持 DeepSeek、通义千问、Moonshot 等。URL 无需包含 /v1 后缀（自动处理）。"
                    }

                    // 错误提示
                    if let Some(err) = error.read().as_ref() {
                        div { class: "mb-3 p-2.5 rounded-md bg-[color-mix(in_srgb,var(--ds-error)_15%,transparent)] border border-[var(--ds-error)] text-[11px] text-[var(--ds-error)]",
                            "{err}"
                        }
                    }

                    // 加载中
                    if *loading.read() && providers.read().is_empty() {
                        div { class: "text-center py-6 text-xs text-[var(--ds-text-tertiary)]",
                            "加载中..."
                        }
                    }

                    // 已配置的通道列表
                    div { class: "space-y-2",
                        {providers.read().iter().map(|p| {
                            let provider = p.clone();
                            rsx! {
                                div { class: "p-3 border border-[var(--ds-border)] rounded-md",
                                    div { class: "flex items-center justify-between",
                                        div { class: "flex items-center gap-2",
                                            span {
                                                class: "w-2 h-2 rounded-full",
                                                style: if p.enabled {
                                                    "background: var(--ds-success)"
                                                } else {
                                                    "background: var(--ds-text-tertiary)"
                                                },
                                            }
                                            span { class: "text-sm font-medium text-[var(--ds-text)]", "{p.name}" }
                                            span { class: "px-1.5 py-0.5 rounded-full bg-[var(--ds-surface)] text-[10px] text-[var(--ds-text-tertiary)]",
                                                "{p.model}"
                                            }
                                        }
                                        div { class: "flex items-center gap-1",
                                            EqButton {
                                                variant: EqButtonVariant::Ghost,
                                                onclick: move |_| on_test_provider(provider.clone()),
                                                "测试"
                                            }
                                            EqButton {
                                                variant: EqButtonVariant::Ghost,
                                                onclick: move |_| on_edit(provider.clone()),
                                                "编辑"
                                            }
                                            EqButton {
                                                variant: EqButtonVariant::Ghost,
                                                onclick: move |_| on_delete(provider.clone()),
                                                "删除"
                                            }
                                        }
                                    }
                                    div { class: "mt-1.5 text-[10px] text-[var(--ds-text-tertiary)]",
                                        "URL: {p.base_url}"
                                    }
                                }
                            }
                        })}
                    }

                    // 空状态
                    if providers.read().is_empty() && !*loading.read() && !*show_form.read() {
                        div { class: "text-center py-8 text-xs text-[var(--ds-text-tertiary)]",
                            "暂无 AI 通道，点击「添加通道」配置"
                        }
                    }

                    // 新建/编辑表单
                    if *show_form.read() {
                        div { class: "mt-3 p-3 border border-[var(--ds-blue)] rounded-md space-y-3 bg-[color-mix(in_srgb,var(--ds-blue-light)_30%,transparent)]",
                            div { class: "text-xs font-bold text-[var(--ds-text)]",
                                if editing_id.read().is_some() { "编辑通道" } else { "新建通道" }
                            }

                            // 名称
                            div { class: "mb-3",
                                label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                                    "名称"
                                }
                                input {
                                    class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                    r#type: "text",
                                    placeholder: "DeepSeek",
                                    value: "{form_name}",
                                    oninput: move |e| form_name.set(e.value.clone()),
                                }
                            }

                            // Base URL
                            div { class: "mb-3",
                                label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                                    "Base URL"
                                }
                                input {
                                    class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                    r#type: "text",
                                    placeholder: "https://api.deepseek.com 或 https://api.deepseek.com/v1",
                                    value: "{form_base_url}",
                                    oninput: move |e| form_base_url.set(e.value.clone()),
                                }
                            }

                            // API Key
                            div { class: "mb-3",
                                label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                                    "API Key"
                                }
                                input {
                                    class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                    r#type: "password",
                                    placeholder: "sk-...",
                                    value: "{form_api_key}",
                                    oninput: move |e| form_api_key.set(e.value.clone()),
                                }
                            }

                            // 模型名称
                            div { class: "mb-3",
                                label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                                    "模型名称"
                                }
                                input {
                                    class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                    r#type: "text",
                                    placeholder: "deepseek-chat, gpt-4o, qwen-plus 等（手动输入）",
                                    value: "{form_model}",
                                    oninput: move |e| form_model.set(e.value.clone()),
                                }
                            }

                            // 启用开关
                            div { class: "flex items-center justify-between mb-3",
                                label { class: "flex items-center gap-2 cursor-pointer",
                                    input {
                                        r#type: "checkbox",
                                        class: "cursor-pointer",
                                        checked: *form_enabled.read(),
                                        onchange: move |e| form_enabled.set(e.checked()),
                                    }
                                    span { class: "text-xs text-[var(--ds-text-secondary)]", "启用此通道" }
                                }
                            }

                            // 测试结果
                            {match test_status.read().as_ref() {
                                TestStatus::Testing => rsx! {
                                    div { class: "text-[11px] text-[var(--ds-text-secondary)]",
                                        "测试中..."
                                    }
                                },
                                TestStatus::Success(latency) => rsx! {
                                    div { class: "text-[11px] text-[var(--ds-success)]",
                                        "连接成功 (延迟: {latency}ms)"
                                    }
                                },
                                TestStatus::Failed(msg) => rsx! {
                                    div { class: "text-[11px] text-[var(--ds-error)] break-all",
                                        "连接失败: {msg}"
                                    }
                                },
                                TestStatus::Idle => rsx! { div {} },
                            }}

                            // 按钮组
                            div { class: "flex items-center justify-between gap-2",
                                div { class: "flex gap-2",
                                    EqButton {
                                        variant: EqButtonVariant::Secondary,
                                        onclick: on_cancel,
                                        "取消"
                                    }
                                    EqButton {
                                        variant: EqButtonVariant::Primary,
                                        onclick: on_save,
                                        disabled: *loading.read(),
                                        if editing_id.read().is_some() { "更新" } else { "保存" }
                                    }
                                }
                                EqButton {
                                    variant: EqButtonVariant::Ghost,
                                    onclick: move |_| on_test(()),
                                    "测试连接"
                                }
                            }

                            div { class: "text-[10px] text-[var(--ds-text-tertiary)]",
                                "提示：模型名称需手动输入，不同提供商的模型名不同。"
                            }
                        }
                    }
                }

                // 语音配置
                EqCard { class: "p-5",
                    h3 { class: "font-semibold mb-4 text-[var(--ds-text)]", "语音" }
                    ToggleRow {
                        title: "语音输入",
                        desc: "在对话中使用浏览器语音识别输入文本",
                    }
                    ToggleRow {
                        title: "朗读回复",
                        desc: "AI回复后自动使用TTS朗读",
                    }
                }

                // 邮件配置
                EqCard { class: "p-5",
                    h3 { class: "font-semibold mb-4 text-[var(--ds-text)]", "邮件配置 (SMTP)" }
                    FormField { label: "SMTP 服务器", placeholder: "smtp.example.com" }
                    FormField { label: "端口", placeholder: "587" }
                    FormField { label: "用户名", placeholder: "user@example.com" }
                    FormField { label: "密码", placeholder: "••••••••", input_type: "password" }
                    FormField { label: "收件人", placeholder: "recipient@example.com" }
                    div { class: "flex gap-2 mt-2",
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            "保存邮件配置"
                        }
                        EqButton {
                            variant: EqButtonVariant::Secondary,
                            "发送测试邮件"
                        }
                    }
                }

                // MQTT 配置
                EqCard { class: "p-5",
                    h3 { class: "font-semibold mb-4 text-[var(--ds-text)]", "MQTT 配置" }
                    FormField { label: "Broker 地址", placeholder: "mqtt://broker.example.com:1883" }
                    FormField { label: "主题 (Topic)", placeholder: "taskmod/commands" }
                    FormField { label: "客户端 ID", placeholder: "taskmod-client" }
                    FormField { label: "用户名 (可选)", placeholder: "" }
                    FormField { label: "密码 (可选)", placeholder: "••••••••", input_type: "password" }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        "保存 MQTT 配置"
                    }
                }

                // 系统命令
                EqCard { class: "p-5",
                    h3 { class: "font-semibold mb-4 text-[var(--ds-text)]", "系统命令" }
                    FormField { label: "执行命令", placeholder: "输入要执行的 shell 命令..." }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                            "命令输出"
                        }
                        pre { class: "min-h-[60px] max-h-[200px] overflow-y-auto p-2.5 bg-[color-mix(in_srgb,var(--ds-blue-light)_58%,var(--ds-card))] border border-[var(--ds-selected-border)] rounded-md text-xs",
                            "等待执行..."
                        }
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        "执行"
                    }
                }
            }
        }
    }
}

/// 测试状态
#[derive(Clone, PartialEq)]
enum TestStatus {
    Idle,
    Testing,
    Success(u64),
    Failed(String),
}

#[derive(Props, PartialEq, Clone)]
struct ToggleRowProps {
    title: &'static str,
    desc: &'static str,
}

#[component]
fn ToggleRow(props: ToggleRowProps) -> Element {
    rsx! {
        div { class: "flex items-center justify-between py-2 border-b border-[var(--ds-border)] last:border-b-0",
            div {
                div { class: "text-sm font-medium text-[var(--ds-text)]", "{props.title}" }
                div { class: "text-[11px] text-[var(--ds-text-tertiary)] mt-0.5", "{props.desc}" }
            }
            EqSwitch { }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct FormFieldProps {
    label: &'static str,
    placeholder: &'static str,
    #[props(default = "text")]
    input_type: &'static str,
}

#[component]
fn FormField(props: FormFieldProps) -> Element {
    rsx! {
        div { class: "mb-3",
            label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                "{props.label}"
            }
            input {
                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                r#type: "{props.input_type}",
                placeholder: "{props.placeholder}",
            }
        }
    }
}
