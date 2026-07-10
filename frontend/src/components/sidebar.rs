use dioxus::prelude::*;
use eq_ui::prelude::*;

use crate::app::ActivePage;

#[derive(Props, PartialEq, Clone)]
pub struct SidebarProps {
    pub theme: Signal<String>,
    pub active_page: Signal<ActivePage>,
}

#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    let pages = vec![
        ActivePage::Dashboard,
        ActivePage::Chat,
        ActivePage::Mirror,
        ActivePage::Library,
        ActivePage::Tasks,
        ActivePage::Scripts,
        ActivePage::Files,
        ActivePage::Tts,
        ActivePage::Config,
        ActivePage::Logs,
    ];

    rsx! {
        nav {
            class: "flex items-end gap-1 min-h-[44px] px-2.5 overflow-x-auto border-b border-[var(--ds-border)] bg-[var(--ds-card)]",
            aria_label: "导航",

            // 页面导航标签
            for page in pages {
                {
                    let is_active = *props.active_page.read() == page;
                    let active_class = if is_active {
                        "text-[var(--ds-blue)]"
                    } else {
                        "text-[var(--ds-text-secondary)] hover:text-[var(--ds-text)] hover:bg-[color-mix(in_srgb,var(--ds-surface)_62%,transparent)]"
                    };
                    let indicator = if is_active {
                        rsx! {
                            span {
                                class: "absolute right-2.5 bottom-[-1px] left-2.5 h-[2px] bg-[var(--ds-blue)]",
                            }
                        }
                    } else {
                        rsx! {}
                    };

                    rsx! {
                        button {
                            class: "relative flex items-center justify-center gap-1.5 min-w-[52px] h-[44px] px-2 bg-transparent cursor-pointer text-xs font-semibold whitespace-nowrap transition-colors duration-150 {active_class}",
                            onclick: move |_| props.active_page.set(page),
                            // 图标
                            PageIcon { page: page },
                            // 标签
                            span { class: "hidden sm:inline", "{page.label()}" }
                            // 活跃指示器
                            {indicator}
                        }
                    }
                }
            }

            // 间隔
            div { class: "flex-1" }

            // 主题切换按钮
            button {
                class: "flex items-center justify-center h-[44px] px-2 bg-transparent cursor-pointer text-[var(--ds-text-secondary)] hover:text-[var(--ds-text)] transition-colors",
                title: "切换主题",
                onclick: move |_| {
                    let current = props.theme.read().clone();
                    let next = if current == "dark" { "light" } else { "dark" };
                    props.theme.set(next.to_string());
                },
                // 月亮/太阳图标
                if *props.theme.read() == "dark" {
                    svg {
                        class: "w-4 h-4",
                        fill: "none",
                        view_box: "0 0 24 24",
                        stroke: "currentColor",
                        stroke_width: "2",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            d: "M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z",
                        }
                    }
                } else {
                    svg {
                        class: "w-4 h-4",
                        fill: "none",
                        view_box: "0 0 24 24",
                        stroke: "currentColor",
                        stroke_width: "2",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            d: "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z",
                        }
                    }
                }
            }

            // GitHub 链接
            a {
                class: "flex items-center h-[44px] px-2 text-[var(--ds-text-secondary)] hover:text-[var(--ds-text)] transition-colors no-underline",
                href: "https://github.com/ojbkxc/TaskMod",
                target: "_blank",
                title: "GitHub",
                svg {
                    class: "w-4 h-4",
                    view_box: "0 0 16 16",
                    fill: "currentColor",
                    path {
                        d: "M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z",
                    }
                }
            }
        }
    }
}

/// 页面图标组件
#[component]
fn PageIcon(page: ActivePage) -> Element {
    match page {
        ActivePage::Dashboard => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" }
            }
        },
        ActivePage::Chat => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" }
            }
        },
        ActivePage::Mirror => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" }
            }
        },
        ActivePage::Library => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5s3.332.477 4.5 1.253v13C19.832 18.477 18.246 18 16.5 18s-3.332.477-4.5 1.253" }
            }
        },
        ActivePage::Tasks => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" }
            }
        },
        ActivePage::Scripts => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" }
            }
        },
        ActivePage::Files => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" }
            }
        },
        ActivePage::Tts => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.536 8.464a5 5 0 010 7.072M12 6l-4 4H4v4h4l4 4V6z" }
            }
        },
        ActivePage::Config => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" }
            }
        },
        ActivePage::Logs => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" }
            }
        },
    }
}
