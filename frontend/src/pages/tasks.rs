use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn TasksPage() -> Element {
    let mut tasks = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut refresh = use_signal(|| 0u32);
    let mut show_add = use_signal(|| false);
    let mut form_time = use_signal(String::new);
    let mut form_weeks = use_signal(|| "1,2,3,4,5,6,7".to_string());
    let mut form_script = use_signal(String::new);
    let mut form_task_type = use_signal(|| "once".to_string());

    use_effect(move || {
        let _ = *refresh.read();
        spawn(async move {
            loading.set(true);
            error.set(None);
            match crate::api::client::get_tasks().await {
                Ok(list) => tasks.set(list),
                Err(e) => error.set(Some(format!("加载失败: {}", e))),
            }
            loading.set(false);
        });
    });

    let on_add = move |_| {
        let time = form_time.read().clone();
        let weeks = form_weeks.read().clone();
        let script = form_script.read().clone();
        let task_type = form_task_type.read().clone();
        if time.is_empty() || script.is_empty() {
            error.set(Some("时间和脚本不能为空".to_string()));
            return;
        }
        spawn(async move {
            match crate::api::client::add_task(&time, &weeks, &script, &task_type, None).await {
                Ok(_) => {
                    show_add.set(false);
                    form_time.set(String::new());
                    form_script.set(String::new());
                    refresh += 1;
                }
                Err(e) => error.set(Some(format!("添加失败: {}", e))),
            }
        });
    };

    rsx! {
        div { class: "p-4 space-y-4",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "任务管理" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "管理定时任务与命令" }
                }
                div { class: "flex gap-1.5",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        onclick: move |_| refresh += 1,
                        "刷新"
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        onclick: move |_| show_add.set(!*show_add.read()),
                        "添加任务"
                    }
                }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "p-2.5 rounded-md bg-[color-mix(in_srgb,var(--ds-error)_15%,transparent)] border border-[var(--ds-error)] text-[11px] text-[var(--ds-error)]",
                    "{err}"
                }
            }

            if *show_add.read() {
                div { class: "p-3 border border-[var(--ds-blue)] rounded-md space-y-3 bg-[color-mix(in_srgb,var(--ds-blue-light)_30%,transparent)]",
                    div { class: "text-xs font-bold text-[var(--ds-text)]", "新建任务" }
                    div { class: "grid grid-cols-2 gap-2",
                        div {
                            label { class: "block text-[11px] font-bold mb-1", "时间" }
                            input {
                                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none focus:border-[var(--ds-blue)]",
                                r#type: "text",
                                placeholder: "08:00",
                                value: "{form_time}",
                                oninput: move |e| form_time.set(e.value.clone()),
                            }
                        }
                        div {
                            label { class: "block text-[11px] font-bold mb-1", "类型" }
                            select {
                                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none",
                                value: "{form_task_type}",
                                onchange: move |e| form_task_type.set(e.value()),
                                option { value: "once", "单次" }
                                option { value: "loop", "循环" }
                            }
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-bold mb-1", "执行日期 (1=周一,7=周日)" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none focus:border-[var(--ds-blue)]",
                            r#type: "text",
                            placeholder: "1,2,3,4,5,6,7",
                            value: "form_weeks}",
                            oninput: move |e| form_weeks.set(e.value.clone()),
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-bold mb-1", "脚本名称" }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none focus:border-[var(--ds-blue)]",
                            r#type: "text",
                            placeholder: "script.sh",
                            value: "{form_script}",
                            oninput: move |e| form_script.set(e.value.clone()),
                        }
                    }
                    div { class: "flex gap-2",
                        EqButton {
                            variant: EqButtonVariant::Secondary,
                            onclick: move |_| show_add.set(false),
                            "取消"
                        }
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            onclick: on_add,
                            "保存"
                        }
                    }
                }
            }

            if *loading.read() && tasks.read().is_empty() {
                div { class: "text-center py-8 text-xs text-[var(--ds-text-tertiary)]", "加载中..." }
            }

            if tasks.read().is_empty() && !*loading.read() {
                div { class: "flex flex-col items-center justify-center min-h-[280px] gap-2.5 p-6 text-center",
                    div { class: "flex items-center justify-center w-7 h-7 border border-[var(--ds-border)] rounded-md text-[var(--ds-text-tertiary)]",
                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" }
                        }
                    }
                    p { class: "text-sm font-semibold text-[var(--ds-text)]", "暂无任务" }
                    p { class: "text-xs text-[var(--ds-text-tertiary)]", "点击\"添加任务\"创建定时任务" }
                }
            }

            div { class: "space-y-2",
                {tasks.read().iter().map(|t| {
                    let task = t.clone();
                    let task_id = t.id;
                    rsx! {
                        EqCard { class: "p-3",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-3",
                                    div { class: "w-8 h-8 flex items-center justify-center rounded-md bg-[var(--ds-surface)] text-xs font-bold text-[var(--ds-text-secondary)]",
                                        "{task.time}"
                                    }
                                    div {
                                        div { class: "text-sm font-medium text-[var(--ds-text)]", "{task.script}" }
                                        div { class: "text-[10px] text-[var(--ds-text-tertiary)] mt-0.5",
                                            "类型: {task.task_type} | 星期: {task.weeks}"
                                            if let Some(iv) = task.interval {
                                                " | 间隔: {iv}s"
                                            }
                                        }
                                    }
                                }
                                EqButton {
                                    variant: EqButtonVariant::Ghost,
                                    onclick: move |_| {
                                        let id = task_id;
                                        spawn(async move {
                                            let _ = crate::api::client::delete_task(id).await;
                                            refresh += 1;
                                        });
                                    },
                                    "删除"
                                }
                            }
                        }
                    }
                })}
            }
        }
    }
}
