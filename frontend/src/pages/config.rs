use dioxus::prelude::*;
use eq_ui::prelude::*;
use serde::{Deserialize, Serialize};
use crate::api::client::{
    get_ai_providers, save_ai_provider, add_ai_provider, delete_ai_provider,
    get_email_config, save_email_config, get_mqtt_config, save_mqtt_config, execute_command,
    AiProvider, EmailConfig, MqttConfig,
};

#[component]
pub fn ConfigPage() -> Element {
    let mut providers = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let mut show_form = use_signal(|| false);
    let mut editing_id = use_signal(|| None::<String>);
    let mut form_name = use_signal(String::new);
    let mut form_base_url = use_signal(String::new);
    let mut form_api_key = use_signal(String::new);
    let mut form_model = use_signal(String::new);
    let mut form_enabled = use_signal(|| true);

    let mut test_status = use_signal(|| TestStatus::Idle);
    let mut refresh = use_signal(|| 0u32);

    let mut email_config = use_signal(|| EmailConfig::default());
    let mut mqtt_config = use_signal(|| MqttConfig::default());
    let mut cmd_output = use_signal(String::new);

    use_effect(move || {
        let _ = *refresh.read();
        spawn(async move {
            loading.set(true);
            error.set(None);
            match get_ai_providers().await {
                Ok(list) => providers.set(list),
                Err(e) => error.set(Some(format!("加载失败: {}", e))),
            }
            if let Ok(ec) = get_email_config().await {
                email_config.set(ec);
            }
            if let Ok(mc) = get_mqtt_config().await {
                mqtt_config.set(mc);
            }
            loading.set(false);
        });
    });

    let reset_form = move || {
        editing_id.set(None);
        form_name.set(String::new());
        form_base_url.set(String::new());
        form_api_key.set(String::new());
        form_model.set(String::new());
        form_enabled.set(true);
        test_status.set(TestStatus::Idle);
    };

    let on_add = move |_| {
        reset_form();
        show_form.set(true);
    };

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

    let on_cancel = move |_| {
        show_form.set(false);
        reset_form();
    };

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

        let provider = AiProvider {
            id: edit_id.clone().unwrap_or_default(),
            name,
            base_url,
            api_key,
            model,
            enabled,
        };

        spawn(async move {
            loading.set(true);
            let result = if edit_id.is_some() {
                save_ai_provider(&provider).await
            } else {
                add_ai_provider(&provider).await
            };

            match result {
                Ok(_) => {
                    show_form.set(false);
                    reset_form();
                    refresh.set(*refresh.read() + 1);
                }
                Err(e) => {
                    error.set(Some(format!("保存失败: {}", e)));
                }
            }
            loading.set(false);
        });
    };

    let on_test = move |_| {
        let base_url = form_base_url.read().clone();
        let api_key = form_api_key.read().clone();
        let model = form_model.read().clone();

        if base_url.is_empty() {
            error.set(Some("请先填写 Base URL".to_string()));
            return;
        }

        test_status.set(TestStatus::Testing);

        let provider = AiProvider {
            id: String::new(),
            name: "test".to_string(),
            base_url,
            api_key,
            model,
            enabled: true,
        };

        spawn(async move {
            match crate::api::client::test_ai_connection(&provider).await {
                Ok(latency) => {
                    test_status.set(TestStatus::Success(latency));
                }
                Err(e) => {
                    test_status.set(TestStatus::Failed(e));
                }
            }
        });
    };

    let on_test_provider = move |provider: AiProvider| {
        editing_id.set(Some(provider.id.clone()));
        form_name.set(provider.name.clone());
        form_base_url.set(provider.base_url.clone());
        form_api_key.set(provider.api_key.clone());
        form_model.set(provider.model.clone());
        form_enabled.set(provider.enabled);
        show_form.set(true);
        test_status.set(TestStatus::Testing);
        spawn(async move {
            match crate::api::client::test_ai_connection(&provider).await {
                Ok(latency) => {
                    test_status.set(TestStatus::Success(latency));
                }
                Err(e) => {
                    test_status.set(TestStatus::Failed(e));
                }
            }
        });
    };

    let on_delete = move |provider: AiProvider| {
        let id = provider.id.clone();
        spawn(async move {
            if let Ok(_) = delete_ai_provider(&id).await {
                refresh.set(*refresh.read() + 1);
                if editing_id.read().as_deref() == Some(&id) {
                    show_form.set(false);
                    reset_form();
                }
            }
        });
    };

    let on_save_email = move |_| {
        let config = email_config.read().clone();
        spawn(async move {
            match save_email_config(&config).await {
                Ok(_) => error.set(Some("邮件配置保存成功".to_string())),
                Err(e) => error.set(Some(format!("保存失败: {}", e))),
            }
        });
    };

    let on_save_mqtt = move |_| {
        let config = mqtt_config.read().clone();
        spawn(async move {
            match save_mqtt_config(&config).await {
                Ok(_) => error.set(Some("MQTT配置保存成功".to_string())),
                Err(e) => error.set(Some(format!("保存失败: {}", e))),
            }
        });
    };

    let on_execute_cmd = move |cmd: String| {
        spawn(async move {
            cmd_output.set("执行中...".to_string());
            match execute_command(&cmd).await {
                Ok(result) => cmd_output.set(result),
                Err(e) => cmd_output.set(format!("执行失败: {}", e)),
            }
        });
    };

    rsx! {
        div { class: "p-4 space-y-4",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "配置" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "系统与服务配置" }
                }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "mb-3 p-2.5 rounded-md bg-[color-mix(in_srgb,var(--ds-error)_15%,transparent)] border border-[var(--ds-error)] text-[11px] text-[var(--ds-error)]",
                    "{err}"
                }
            }

            div { class: "flex flex-col gap-4",
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

                    if *loading.read() && providers.read().is_empty() {
                        div { class: "text-center py-6 text-xs text-[var(--ds-text-tertiary)]",
                            "加载中..."
                        }
                    }

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

                    if providers.read().is_empty() && !*loading.read() && !*show_form.read() {
                        div { class: "text-center py-8 text-xs text-[var(--ds-text-tertiary)]",
                            "暂无 AI 通道，点击「添加通道」配置"
                        }
                    }

                    if *show_form.read() {
                        div { class: "mt-3 p-3 border border-[var(--ds-blue)] rounded-md space-y-3 bg-[color-mix(in_srgb,var(--ds-blue-light)_30%,transparent)]",
                            div { class: "text-xs font-bold text-[var(--ds-text)]",
                                if editing_id.read().is_some() { "编辑通道" } else { "新建通道" }
                            }

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
                                    onclick: on_test,
                                    "测试连接"
                                }
                            }

                            div { class: "text-[10px] text-[var(--ds-text-tertiary)]",
                                "提示：模型名称需手动输入，不同提供商的模型名不同。"
                            }
                        }
                    }
                }

                EqCard { class: "p-5",
                    h3 { class: "font-semibold mb-4 text-[var(--ds-text)]", "邮件配置 (SMTP)" }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "SMTP 服务器" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            value: "{email_config.read().smtp_server}",
                            oninput: move |e| email_config.write().smtp_server = e.value(),
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "端口" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            value: "{email_config.read().smtp_port}",
                            oninput: move |e| email_config.write().smtp_port = e.value(),
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "用户名" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            value: "{email_config.read().username}",
                            oninput: move |e| email_config.write().username = e.value(),
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "密码" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            r#type: "password",
                            value: "{email_config.read().password}",
                            oninput: move |e| email_config.write().password = e.value(),
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "收件人" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            value: "{email_config.read().to}",
                            oninput: move |e| email_config.write().to = e.value(),
                        }
                    }
                    div { class: "flex gap-2",
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            onclick: on_save_email,
                            "保存邮件配置"
                        }
                        EqButton {
                            variant: EqButtonVariant::Secondary,
                            onclick: move |_| {
                                let config = email_config.read().clone();
                                spawn(async move {
                                    match crate::api::client::send_test_email(&config).await {
                                        Ok(_) => error.set(Some("测试邮件发送成功".to_string())),
                                        Err(e) => error.set(Some(format!("发送失败: {}", e))),
                                    }
                                });
                            },
                            "发送测试邮件"
                        }
                    }
                }

                EqCard { class: "p-5",
                    h3 { class: "font-semibold mb-4 text-[var(--ds-text)]", "MQTT 配置" }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "Broker 地址" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            value: "{mqtt_config.read().broker}",
                            oninput: move |e| mqtt_config.write().broker = e.value(),
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "主题 (Topic)" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            value: "{mqtt_config.read().topic}",
                            oninput: move |e| mqtt_config.write().topic = e.value(),
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "客户端 ID" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            value: "{mqtt_config.read().client_id}",
                            oninput: move |e| mqtt_config.write().client_id = e.value(),
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "用户名 (可选)" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            value: "{mqtt_config.read().username}",
                            oninput: move |e| mqtt_config.write().username = e.value(),
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1", "密码 (可选)" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            r#type: "password",
                            value: "{mqtt_config.read().password}",
                            oninput: move |e| mqtt_config.write().password = e.value(),
                        }
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        onclick: on_save_mqtt,
                        "保存 MQTT 配置"
                    }
                }

                EqCard { class: "p-5",
                    h3 { class: "font-semibold mb-4 text-[var(--ds-text)]", "系统命令" }
                    let cmd_input = use_signal(String::new);
                    div { class: "mb-3",
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            placeholder: "输入要执行的 shell 命令...",
                            value: "{cmd_input}",
                            oninput: move |e| cmd_input.set(e.value()),
                            onkeydown: move |ev| {
                                if ev.key() == "Enter" {
                                    on_execute_cmd(cmd_input.read().clone());
                                }
                            },
                        }
                    }
                    div { class: "mb-3",
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                            "命令输出"
                        }
                        pre { class: "min-h-[60px] max-h-[200px] overflow-y-auto p-2.5 bg-[color-mix(in_srgb,var(--ds-blue-light)_58%,var(--ds-card))] border border-[var(--ds-selected-border)] rounded-md text-xs",
                            "{cmd_output}"
                        }
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        onclick: move |_| on_execute_cmd(cmd_input.read().clone()),
                        "执行"
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum TestStatus {
    Idle,
    Testing,
    Success(u64),
    Failed(String),
}
