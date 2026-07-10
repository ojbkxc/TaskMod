use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn FilesPage() -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            // 页面标题
            div { class: "flex items-start justify-between gap-3 p-4 pb-3 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "文件管理" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "浏览和管理设备文件" }
                }
            }

            // 文件管理器布局
            div { class: "flex flex-1 min-h-0 overflow-hidden",
                // 左侧目录树
                aside { class: "w-[200px] min-w-[160px] bg-[var(--ds-card)] border-r border-[var(--ds-border)] flex flex-col overflow-hidden shrink-0",
                    div { class: "flex items-center justify-between p-2.5 text-xs font-bold text-[var(--ds-text-secondary)] uppercase tracking-wider border-b border-[var(--ds-border)]",
                        span { "目录" }
                    }
                    div { class: "flex-1 overflow-y-auto p-1.5",
                        // 目录树节点
                        DirTreeItem { name: "/", icon: "📁" }
                    }
                }

                // 右侧主区域
                div { class: "flex-1 flex flex-col min-w-0 overflow-hidden",
                    // 工具栏
                    div { class: "flex items-center gap-1.5 p-2 bg-[var(--ds-card)] border-b border-[var(--ds-border)]",
                        EqButton {
                            variant: EqButtonVariant::Ghost,
                            size: EqButtonSize::Sm,
                            "上级目录"
                        }
                        EqButton {
                            variant: EqButtonVariant::Ghost,
                            size: EqButtonSize::Sm,
                            "刷新"
                        }
                        div { class: "w-px h-5 bg-[var(--ds-border)]" }
                        EqButton {
                            variant: EqButtonVariant::Ghost,
                            size: EqButtonSize::Sm,
                            "新建"
                        }
                        EqButton {
                            variant: EqButtonVariant::Ghost,
                            size: EqButtonSize::Sm,
                            "上传"
                        }
                        div { class: "w-px h-5 bg-[var(--ds-border)]" }
                        input {
                            class: "flex-1 max-w-[260px] h-8 px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            placeholder: "搜索文件名...",
                        }
                    }

                    // 面包屑
                    div { class: "flex items-center gap-1 p-2 bg-[var(--ds-surface)] border-b border-[var(--ds-border)] overflow-x-auto text-sm",
                        span { class: "text-[var(--ds-blue)] cursor-pointer px-1 py-0.5 rounded hover:bg-[var(--ds-blue-light)]",
                            "根目录"
                        }
                    }

                    // 文件列表区域
                    div { class: "flex-1 overflow-y-auto",
                        div { class: "flex flex-col items-center justify-center min-h-[280px] gap-2.5 p-10 text-center",
                            svg { class: "w-8 h-8 opacity-30", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" }
                            }
                            p { class: "text-sm text-[var(--ds-text-tertiary)]", "加载中..." }
                        }
                    }

                    // 状态栏
                    div { class: "flex items-center justify-between p-1.5 px-3 bg-[var(--ds-surface)] border-t border-[var(--ds-border)] text-[11px] text-[var(--ds-text-tertiary)]",
                        span { "就绪" }
                        span { "共 0 个项目" }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct DirTreeItemProps {
    name: &'static str,
    icon: &'static str,
}

#[component]
fn DirTreeItem(props: DirTreeItemProps) -> Element {
    rsx! {
        div { class: "flex items-center gap-1.5 px-3 py-1.5 text-sm cursor-pointer text-[var(--ds-text-secondary)] hover:bg-[var(--ds-surface-hover)] hover:text-[var(--ds-text)]",
            span { "{props.icon}" }
            span { "{props.name}" }
        }
    }
}
