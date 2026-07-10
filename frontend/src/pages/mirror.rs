use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn MirrorPage() -> Element {
    rsx! {
        div { class: "p-4 space-y-3",
            // 顶部操作栏
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-2",
                    span { class: "w-2.5 h-2.5 rounded-full bg-[var(--ds-text-tertiary)]" }
                    span { class: "text-sm text-[var(--ds-text-secondary)]", "未连接" }
                }
                div { class: "flex items-center gap-2",
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        "开始投屏"
                    }
                }
            }

            // 投屏区域
            div { class: "border border-[var(--ds-border)] rounded-md overflow-hidden bg-[#0a0a0f] flex items-center justify-center min-h-[420px] aspect-[9/16] max-h-[80vh] shadow-sm",
                div { class: "flex flex-col items-center gap-3 p-10 text-center",
                    div { class: "w-20 h-20 flex items-center justify-center bg-[var(--ds-surface)] border border-[var(--ds-border)] rounded-2xl text-3xl text-[var(--ds-text-tertiary)] opacity-60",
                        svg { class: "w-8 h-8", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" }
                        }
                    }
                    p { class: "text-base font-semibold text-[var(--ds-text-secondary)]", "设备未连接" }
                    p { class: "text-sm text-[var(--ds-text-tertiary)]", "点击上方\"开始投屏\"连接设备屏幕" }
                }
            }

            // ADB 命令折叠面板
            EqCard { class: "p-4",
                div { class: "flex items-center gap-2 mb-3",
                    svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" }
                    }
                    span { class: "text-sm font-semibold text-[var(--ds-text)]", "ADB 命令" }
                    span { class: "text-[11px] text-[var(--ds-text-tertiary)] ml-auto", "发送 shell 命令到设备" }
                }
                div { class: "flex gap-2",
                    input {
                        class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] font-mono outline-none focus:border-[var(--ds-blue)]",
                        placeholder: "输入命令，如: input tap 500 500",
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        "执行"
                    }
                }
                // 常用命令卡片
                div { class: "grid grid-cols-[repeat(auto-fill,minmax(100px,1fr))] gap-2 mt-3",
                    AdbCommandCard { label: "唤醒屏幕" }
                    AdbCommandCard { label: "息屏" }
                    AdbCommandCard { label: "上滑解锁" }
                    AdbCommandCard { label: "Home" }
                    AdbCommandCard { label: "返回" }
                    AdbCommandCard { label: "电池信息" }
                    AdbCommandCard { label: "设备型号" }
                    AdbCommandCard { label: "分辨率" }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct AdbCommandCardProps {
    label: &'static str,
}

#[component]
fn AdbCommandCard(props: AdbCommandCardProps) -> Element {
    rsx! {
        button {
            class: "flex flex-col items-center gap-1.5 p-3 bg-[var(--ds-surface)] border border-[var(--ds-border)] rounded-md cursor-pointer text-[11px] text-[var(--ds-text-secondary)] transition-all hover:bg-[var(--ds-blue-light)] hover:border-[var(--ds-blue)] hover:text-[var(--ds-blue)] active:scale-95",
            "{props.label}"
        }
    }
}
