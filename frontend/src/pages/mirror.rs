use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn MirrorPage() -> Element {
    rsx! {
        div { class: "p-4 space-y-3",
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

            div { class: "flex gap-4",
                div { class: "hidden md:flex flex-col gap-3 w-60 flex-shrink-0",
                    EqCard { class: "p-4",
                        div { class: "flex items-center gap-2 mb-3",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "ADB 命令" }
                        }
                        div { class: "flex gap-2",
                            input {
                                class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] font-mono outline-none focus:border-[var(--ds-blue)]",
                                placeholder: "输入命令...",
                            }
                            EqButton {
                                variant: EqButtonVariant::Primary,
                                "执行"
                            }
                        }
                        div { class: "grid grid-cols-2 gap-2 mt-3",
                            AdbCommandCard { label: "唤醒屏幕" }
                            AdbCommandCard { label: "息屏" }
                            AdbCommandCard { label: "上滑解锁" }
                            AdbCommandCard { label: "Home" }
                            AdbCommandCard { label: "返回" }
                        }
                    }

                    EqCard { class: "p-4",
                        div { class: "flex items-center gap-2 mb-3",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "命令输出" }
                        }
                        div { class: "min-h-[120px] max-h-[200px] overflow-y-auto p-3 bg-[var(--ds-bg)] border border-[var(--ds-border)] rounded-md text-xs font-mono text-[var(--ds-text-secondary)]",
                            "等待执行命令..."
                        }
                    }
                }

                div { class: "flex-1 flex items-center justify-center min-w-0",
                    div { class: "border border-[var(--ds-border)] rounded-md overflow-hidden bg-[#0a0a0f] flex items-center justify-center w-full max-w-[420px] aspect-[9/16] max-h-[80vh] shadow-sm",
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
                }

                div { class: "hidden md:flex flex-col gap-3 w-60 flex-shrink-0",
                    EqCard { class: "p-4",
                        div { class: "flex items-center gap-2 mb-3",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "设备工具" }
                        }
                        div { class: "grid grid-cols-2 gap-2",
                            DeviceToolCard { label: "截屏", icon: "screencap" }
                            DeviceToolCard { label: "电池信息", icon: "battery" }
                            DeviceToolCard { label: "设备型号", icon: "device" }
                            DeviceToolCard { label: "分辨率", icon: "resolution" }
                            DeviceToolCard { label: "WiFi信息", icon: "wifi" }
                            DeviceToolCard { label: "运行应用", icon: "apps" }
                        }
                    }

                    EqCard { class: "p-4",
                        div { class: "flex items-center gap-2 mb-3",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "应用管理" }
                        }
                        div { class: "grid grid-cols-2 gap-2",
                            DeviceToolCard { label: "启动应用", icon: "start" }
                            DeviceToolCard { label: "停止应用", icon: "stop" }
                        }
                    }

                    EqCard { class: "p-4",
                        div { class: "flex items-center gap-2 mb-3",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "系统操作" }
                        }
                        div { class: "grid grid-cols-2 gap-2",
                            DeviceToolCard { label: "重启设备", icon: "reboot" }
                            DeviceToolCard { label: "关闭设备", icon: "shutdown" }
                        }
                    }
                }
            }

            div { class: "md:hidden space-y-3",
                EqCard { class: "p-4",
                    div { class: "flex items-center gap-2 mb-3",
                        svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" }
                        }
                        span { class: "text-sm font-semibold text-[var(--ds-text)]", "ADB 命令" }
                    }
                    div { class: "flex gap-2",
                        input {
                            class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] font-mono outline-none focus:border-[var(--ds-blue)]",
                            placeholder: "输入命令...",
                        }
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            "执行"
                        }
                    }
                    div { class: "grid grid-cols-4 gap-2 mt-3",
                        AdbCommandCard { label: "唤醒屏幕" }
                        AdbCommandCard { label: "息屏" }
                        AdbCommandCard { label: "上滑解锁" }
                        AdbCommandCard { label: "Home" }
                        AdbCommandCard { label: "返回" }
                        DeviceToolCard { label: "截屏", icon: "screencap" }
                        DeviceToolCard { label: "电池信息", icon: "battery" }
                        DeviceToolCard { label: "设备型号", icon: "device" }
                    }
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

#[derive(Props, PartialEq, Clone)]
struct DeviceToolCardProps {
    label: &'static str,
    icon: &'static str,
}

#[component]
fn DeviceToolCard(props: DeviceToolCardProps) -> Element {
    rsx! {
        button {
            class: "flex flex-col items-center gap-1.5 p-3 bg-[var(--ds-surface)] border border-[var(--ds-border)] rounded-md cursor-pointer text-[11px] text-[var(--ds-text-secondary)] transition-all hover:bg-[var(--ds-blue-light)] hover:border-[var(--ds-blue)] hover:text-[var(--ds-blue)] active:scale-95",
            DeviceToolIcon { icon: props.icon },
            "{props.label}"
        }
    }
}

#[component]
fn DeviceToolIcon(icon: &'static str) -> Element {
    match icon {
        "screencap" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" }
            }
        },
        "battery" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" }
            }
        },
        "device" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z" }
            }
        },
        "resolution" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" }
            }
        },
        "wifi" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" }
            }
        },
        "apps" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" }
            }
        },
        "start" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" }
            }
        },
        "stop" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
            }
        },
        "reboot" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
            }
        },
        "shutdown" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
            }
        },
        _ => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" }
            }
        },
    }
}