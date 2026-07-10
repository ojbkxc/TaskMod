use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn TtsPage() -> Element {
    rsx! {
        div { class: "p-4 space-y-4",
            // 页面标题
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "TTS 语音合成" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "文本转语音控制面板 · 配置持久化到设备" }
                }
                div { class: "flex gap-1.5",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        "加载设置"
                    }
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        "刷新引擎"
                    }
                }
            }

            // 语音引擎卡片
            EqCard { class: "p-4",
                div { class: "flex items-center justify-between mb-3",
                    span { class: "text-sm font-semibold text-[var(--ds-text)] flex items-center gap-1.5",
                        svg { class: "w-3 h-3 opacity-50", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2z" }
                        }
                        "语音引擎"
                    }
                    span { class: "inline-flex items-center gap-1 text-[11px] text-[var(--ds-success)] font-medium",
                        span { class: "w-1.5 h-1.5 rounded-full bg-[var(--ds-success)]" }
                        "就绪"
                    }
                }
                div { class: "flex gap-2 items-center",
                    select { class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)]",
                        option { "加载中..." }
                    }
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        "试听"
                    }
                }
            }

            // 声音参数卡片
            EqCard { class: "p-4",
                div { class: "mb-3",
                    span { class: "text-sm font-semibold text-[var(--ds-text)]", "声音参数" }
                }
                SliderRow { label: "语速", value: "1.00" }
                SliderRow { label: "音调", value: "1.00" }
                SliderRow { label: "音量", value: "1.00" }
            }

            // 文本输入区
            EqCard { class: "p-4",
                div { class: "flex items-center justify-between mb-3",
                    span { class: "text-sm font-semibold text-[var(--ds-text)]", "文本输入" }
                }
                textarea {
                    class: "w-full min-h-[100px] px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] resize-y outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "输入要朗读的文本...",
                }
                div { class: "flex items-center justify-between mt-2",
                    span { class: "text-[11px] text-[var(--ds-text-tertiary)]", "0 字" }
                    div { class: "flex gap-2",
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            "朗读"
                        }
                        EqButton {
                            variant: EqButtonVariant::Destructive,
                            "停止"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct SliderRowProps {
    label: &'static str,
    value: &'static str,
}

#[component]
fn SliderRow(props: SliderRowProps) -> Element {
    rsx! {
        div { class: "flex items-center gap-2.5 mb-2.5 last:mb-0",
            label { class: "w-9 text-[11px] font-semibold text-[var(--ds-text-secondary)] uppercase tracking-wider",
                "{props.label}"
            }
            input {
                class: "flex-1 h-1.5 rounded bg-[var(--ds-border)] cursor-pointer accent-[var(--ds-blue)]",
                type: "range",
                min: "0.5",
                max: "3.0",
                step: "0.05",
                value: "1.0",
            }
            span { class: "w-[34px] text-xs font-semibold text-[var(--ds-blue)] text-right",
                "{props.value}"
            }
        }
    }
}
