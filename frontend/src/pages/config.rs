use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn ConfigPage() -> Element {
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
