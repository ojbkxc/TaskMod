use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn ChatPage() -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            // 聊天头部
            div { class: "flex items-center justify-between p-3 border-b border-[var(--ds-border)]",
                div { class: "flex items-center gap-2",
                    span { class: "text-sm font-semibold text-[var(--ds-text)]", "AI 助手" }
                    span { class: "px-1.5 py-0.5 rounded-full bg-[var(--ds-surface)] text-[10px] text-[var(--ds-text-tertiary)]",
                        "未选择"
                    }
                }
                div { class: "flex items-center gap-2",
                    EqButton {
                        variant: EqButtonVariant::Ghost,
                        size: EqButtonSize::Sm,
                        "截图分析"
                    }
                    EqButton {
                        variant: EqButtonVariant::Ghost,
                        size: EqButtonSize::Sm,
                        "新对话"
                    }
                    EqButton {
                        variant: EqButtonVariant::Ghost,
                        size: EqButtonSize::Sm,
                        "管理"
                    }
                }
            }

            // 消息区域
            div { class: "flex-1 overflow-y-auto p-4",
                // 空状态
                div { class: "flex flex-col items-center justify-center min-h-full gap-2 text-center",
                    div { class: "flex items-center justify-center w-7 h-7 border border-[var(--ds-border)] rounded-md text-[var(--ds-text-tertiary)]",
                        svg { class: "w-6 h-6", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.8",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" }
                        }
                    }
                    p { class: "text-sm font-semibold text-[var(--ds-text)]", "欢迎使用 AI 助手" }
                    p { class: "text-xs text-[var(--ds-text-tertiary)] max-w-[290px]",
                        "选择一个AI提供商，然后输入消息控制设备。"
                    }
                    // 快捷提示卡片
                    div { class: "grid grid-cols-2 gap-2 mt-4 max-w-[320px]",
                        QuickPromptCard { label: "查看设备状态" }
                        QuickPromptCard { label: "截图分析" }
                        QuickPromptCard { label: "打开设置" }
                        QuickPromptCard { label: "列出应用" }
                    }
                }
            }

            // 输入区域
            div { class: "border-t border-[var(--ds-border)] p-3 bg-[color-mix(in_srgb,var(--ds-bg)_94%,var(--ds-surface))]",
                div { class: "border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] shadow-sm p-2",
                    textarea {
                        class: "w-full min-h-[42px] max-h-[150px] resize-none bg-transparent text-sm text-[var(--ds-text)] outline-none",
                        placeholder: "输入消息...",
                    }
                    div { class: "flex items-center justify-between pt-1",
                        span { class: "text-[10px] text-[var(--ds-text-tertiary)] px-2 py-0.5 rounded-full bg-[var(--ds-surface)]",
                            "--"
                        }
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            size: EqButtonSize::Sm,
                            "发送"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct QuickPromptCardProps {
    label: &'static str,
}

#[component]
fn QuickPromptCard(props: QuickPromptCardProps) -> Element {
    rsx! {
        button {
            class: "flex items-center gap-2 p-2.5 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-[var(--ds-text-secondary)] cursor-pointer text-xs font-medium transition-all hover:border-[var(--ds-blue)] hover:bg-[var(--ds-blue-light)] hover:text-[var(--ds-blue)]",
            "{props.label}"
        }
    }
}
