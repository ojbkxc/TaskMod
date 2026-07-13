use dioxus::prelude::*;
use eq_ui::prelude::*;
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
struct Task {
    id: usize,
    time: String,
    weeks: String,
    script: String,
    task_type: String,
    interval: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
struct TasksState {
    tasks: Vec<Task>,
    scripts: Vec<String>,
    loading: bool,
    error: Option<String>,
    show_add: bool,
    show_edit: bool,
    editing_task: Option<Task>,
    form_time: String,
    form_weeks: String,
    form_script: String,
    form_task_type: String,
    form_interval: String,
}

impl Default for TasksState {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            scripts: Vec::new(),
            loading: false,
            error: None,
            show_add: false,
            show_edit: false,
            editing_task: None,
            form_time: String::new(),
            form_weeks: "1,2,3,4,5,6,7".to_string(),
            form_script: String::new(),
            form_task_type: "once".to_string(),
            form_interval: String::new(),
        }
    }
}

#[component]
pub fn TasksPage() -> Element {
    let state = use_signal(TasksState::default);

    let load_data = move || {
        let state = state.clone();
        async move {
            state.write().loading = true;
            state.write().error = None;
            
            let (tasks_res, scripts_res) = tokio::join!(
                crate::api::client::get_tasks(),
                crate::api::client::get_scripts()
            );
            
            match tasks_res {
                Ok(list) => state.write().tasks = list,
                Err(e) => state.write().error = Some(format!("加载任务失败: {}", e)),
            }
            
            match scripts_res {
                Ok(list) => state.write().scripts = list,
                Err(e) => {
                    if state.read().error.is_none() {
                        state.write().error = Some(format!("加载脚本失败: {}", e));
                    }
                }
            }
            
            state.write().loading = false;
        }
    };

    use_effect(move || {
        spawn(async move { load_data().await; });
    });

    let on_add = move |_| {
        let state = state.clone();
        async move {
            let s = state.read();
            let time = s.form_time.clone();
            let weeks = s.form_weeks.clone();
            let script = s.form_script.clone();
            let task_type = s.form_task_type.clone();
            let interval: Option<u32> = s.form_interval.parse().ok();
            
            if time.is_empty() || script.is_empty() {
                state.write().error = Some("时间和脚本不能为空".to_string());
                return;
            }
            
            match crate::api::client::add_task(&time, &weeks, &script, &task_type, interval).await {
                Ok(_) => {
                    state.write().show_add = false;
                    state.write().form_time = String::new();
                    state.write().form_script = String::new();
                    state.write().form_interval = String::new();
                    load_data().await;
                }
                Err(e) => state.write().error = Some(format!("添加失败: {}", e)),
            }
        }
    };

    let on_edit = move |task: Task| {
        state.write().editing_task = Some(task.clone());
        state.write().form_time = task.time.clone();
        state.write().form_weeks = task.weeks.clone();
        state.write().form_script = task.script.clone();
        state.write().form_task_type = task.task_type.clone();
        state.write().form_interval = task.interval.map(|i| i.to_string()).unwrap_or_default();
        state.write().show_edit = true;
    };

    let on_update = move |_| {
        let state = state.clone();
        async move {
            let s = state.read();
            let task = match s.editing_task.clone() {
                Some(t) => t,
                None => return,
            };
            let time = s.form_time.clone();
            let weeks = s.form_weeks.clone();
            let script = s.form_script.clone();
            let task_type = s.form_task_type.clone();
            let interval: Option<u32> = s.form_interval.parse().ok();
            
            if time.is_empty() || script.is_empty() {
                state.write().error = Some("时间和脚本不能为空".to_string());
                return;
            }
            
            let _ = crate::api::client::delete_task(task.id).await;
            match crate::api::client::add_task(&time, &weeks, &script, &task_type, interval).await {
                Ok(_) => {
                    state.write().show_edit = false;
                    state.write().editing_task = None;
                    state.write().form_time = String::new();
                    state.write().form_script = String::new();
                    state.write().form_interval = String::new();
                    load_data().await;
                }
                Err(e) => state.write().error = Some(format!("更新失败: {}", e)),
            }
        }
    };

    let on_delete = move |id: usize| {
        let state = state.clone();
        async move {
            match crate::api::client::delete_task(id).await {
                Ok(_) => load_data().await,
                Err(e) => state.write().error = Some(format!("删除失败: {}", e)),
            }
        }
    };

    let on_trigger = move |script: String| {
        let state = state.clone();
        async move {
            match crate::api::client::trigger_script(&script).await {
                Ok(msg) => {
                    state.write().error = Some(format!("{}", msg));
                    load_data().await;
                }
                Err(e) => state.write().error = Some(format!("执行失败: {}", e)),
            }
        }
    };

    let week_days = vec![
        (1, "周一"), (2, "周二"), (3, "周三"), (4, "周四"),
        (5, "周五"), (6, "周六"), (7, "周日"),
    ];

    let parse_weeks = |weeks: &str| -> Vec<i32> {
        weeks.split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect()
    };

    rsx! {
        div { class: "p-4 space-y-4",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "任务管理" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "管理定时任务与脚本执行" }
                }
                div { class: "flex gap-1.5",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        onclick: move |_| spawn(async move { load_data().await; }),
                        "刷新"
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        onclick: move |_| {
                            state.write().show_add = !state.read().show_add;
                            state.write().show_edit = false;
                        },
                        "添加任务"
                    }
                }
            }

            if let Some(err) = state.read().error.as_ref() {
                div { class: "p-2.5 rounded-md bg-[color-mix(in_srgb,var(--ds-error)_15%,transparent)] border border-[var(--ds-error)] text-[11px] text-[var(--ds-error)]",
                    "{err}"
                }
            }

            if state.read().show_add {
                div { class: "p-3 border border-[var(--ds-blue)] rounded-md space-y-3 bg-[color-mix(in_srgb,var(--ds-blue-light)_30%,transparent)]",
                    div { class: "flex items-center justify-between",
                        div { class: "text-xs font-bold text-[var(--ds-text)]", "新建任务" }
                        button {
                            class: "text-[10px] text-[var(--ds-text-tertiary)] hover:text-[var(--ds-text)]",
                            onclick: move |_| state.write().show_add = false,
                            "关闭"
                        }
                    }
                    div { class: "grid grid-cols-2 gap-2",
                        div {
                            label { class: "block text-[11px] font-bold mb-1", "执行时间" }
                            input {
                                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none focus:border-[var(--ds-blue)]",
                                r#type: "time",
                                value: "{state.read().form_time}",
                                oninput: move |e| state.write().form_time = e.value.clone(),
                            }
                        }
                        div {
                            label { class: "block text-[11px] font-bold mb-1", "任务类型" }
                            select {
                                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none",
                                value: "{state.read().form_task_type}",
                                onchange: move |e| state.write().form_task_type = e.value(),
                                option { value: "once", "定时执行" }
                                option { value: "interval", "间隔执行" }
                            }
                        }
                    }
                    if state.read().form_task_type == "interval" {
                        div {
                            label { class: "block text-[11px] font-bold mb-1", "间隔分钟数" }
                            input {
                                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none focus:border-[var(--ds-blue)]",
                                r#type: "number",
                                min: "1",
                                max: "120",
                                placeholder: "例如: 5",
                                value: "{state.read().form_interval}",
                                oninput: move |e| state.write().form_interval = e.value.clone(),
                            }
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-bold mb-1", "执行日期" }
                        div { class: "flex flex-wrap gap-1",
                            for (day_num, day_label) in week_days {
                                let selected_days = parse_weeks(&state.read().form_weeks);
                                let is_selected = selected_days.contains(&day_num);
                                button {
                                    class: "px-2 py-1 text-[10px] rounded-md transition-colors",
                                    class: if is_selected {
                                        "bg-[var(--ds-blue)] text-white"
                                    } else {
                                        "bg-[var(--ds-surface)] text-[var(--ds-text-secondary)] hover:bg-[var(--ds-border)]"
                                    },
                                    onclick: move |_| {
                                        let mut days = parse_weeks(&state.read().form_weeks);
                                        if is_selected {
                                            days.retain(|&d| d != day_num);
                                        } else {
                                            days.push(day_num);
                                            days.sort();
                                        }
                                        state.write().form_weeks = days.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",");
                                    },
                                    "{day_label}"
                                }
                            }
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-bold mb-1", "执行脚本" }
                        select {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none",
                            value: "{state.read().form_script}",
                            onchange: move |e| state.write().form_script = e.value(),
                            option { value: "", "请选择脚本..." }
                            for script in &state.read().scripts {
                                option { value: "{script}", "{script}" }
                            }
                        }
                    }
                    div { class: "flex gap-2",
                        EqButton {
                            variant: EqButtonVariant::Secondary,
                            onclick: move |_| state.write().show_add = false,
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

            if state.read().show_edit {
                div { class: "p-3 border border-[var(--ds-warning)] rounded-md space-y-3 bg-[color-mix(in_srgb,var(--ds-warning-light)_30%,transparent)]",
                    div { class: "flex items-center justify-between",
                        div { class: "text-xs font-bold text-[var(--ds-text)]", "编辑任务" }
                        button {
                            class: "text-[10px] text-[var(--ds-text-tertiary)] hover:text-[var(--ds-text)]",
                            onclick: move |_| {
                                state.write().show_edit = false;
                                state.write().editing_task = None;
                            },
                            "关闭"
                        }
                    }
                    div { class: "grid grid-cols-2 gap-2",
                        div {
                            label { class: "block text-[11px] font-bold mb-1", "执行时间" }
                            input {
                                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none focus:border-[var(--ds-blue)]",
                                r#type: "time",
                                value: "{state.read().form_time}",
                                oninput: move |e| state.write().form_time = e.value.clone(),
                            }
                        }
                        div {
                            label { class: "block text-[11px] font-bold mb-1", "任务类型" }
                            select {
                                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none",
                                value: "{state.read().form_task_type}",
                                onchange: move |e| state.write().form_task_type = e.value(),
                                option { value: "once", "定时执行" }
                                option { value: "interval", "间隔执行" }
                            }
                        }
                    }
                    if state.read().form_task_type == "interval" {
                        div {
                            label { class: "block text-[11px] font-bold mb-1", "间隔分钟数" }
                            input {
                                class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none focus:border-[var(--ds-blue)]",
                                r#type: "number",
                                min: "1",
                                max: "120",
                                value: "{state.read().form_interval}",
                                oninput: move |e| state.write().form_interval = e.value.clone(),
                            }
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-bold mb-1", "执行日期" }
                        div { class: "flex flex-wrap gap-1",
                            for (day_num, day_label) in week_days {
                                let selected_days = parse_weeks(&state.read().form_weeks);
                                let is_selected = selected_days.contains(&day_num);
                                button {
                                    class: "px-2 py-1 text-[10px] rounded-md transition-colors",
                                    class: if is_selected {
                                        "bg-[var(--ds-blue)] text-white"
                                    } else {
                                        "bg-[var(--ds-surface)] text-[var(--ds-text-secondary)] hover:bg-[var(--ds-border)]"
                                    },
                                    onclick: move |_| {
                                        let mut days = parse_weeks(&state.read().form_weeks);
                                        if is_selected {
                                            days.retain(|&d| d != day_num);
                                        } else {
                                            days.push(day_num);
                                            days.sort();
                                        }
                                        state.write().form_weeks = days.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",");
                                    },
                                    "{day_label}"
                                }
                            }
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-bold mb-1", "执行脚本" }
                        select {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs outline-none",
                            value: "{state.read().form_script}",
                            onchange: move |e| state.write().form_script = e.value(),
                            option { value: "", "请选择脚本..." }
                            for script in &state.read().scripts {
                                option { value: "{script}", "{script}" }
                            }
                        }
                    }
                    div { class: "flex gap-2",
                        EqButton {
                            variant: EqButtonVariant::Secondary,
                            onclick: move |_| {
                                state.write().show_edit = false;
                                state.write().editing_task = None;
                            },
                            "取消"
                        }
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            onclick: on_update,
                            "保存"
                        }
                    }
                }
            }

            if state.read().loading && state.read().tasks.is_empty() {
                div { class: "text-center py-8 text-xs text-[var(--ds-text-tertiary)]", "加载中..." }
            }

            if state.read().tasks.is_empty() && !state.read().loading {
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
                {state.read().tasks.iter().map(|t| {
                    let task = t.clone();
                    let task_id = t.id;
                    let is_interval = task.task_type == "interval";
                    let week_labels: Vec<String> = parse_weeks(&task.weeks)
                        .iter()
                        .filter_map(|&d| week_days.iter().find(|(n, _)| *n == d).map(|(_, l)| l.to_string()))
                        .collect();
                    rsx! {
                        EqCard { class: "p-3",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-3",
                                    div { 
                                        class: "w-8 h-8 flex items-center justify-center rounded-md text-xs font-bold",
                                        class: if is_interval { "bg-amber-50 text-amber-700" } else { "bg-[var(--ds-surface)] text-[var(--ds-text-secondary)]" },
                                        "{task.time}"
                                    }
                                    div {
                                        div { class: "text-sm font-medium text-[var(--ds-text)]", "{task.script}" }
                                        div { class: "text-[10px] text-[var(--ds-text-tertiary)] mt-0.5",
                                            if is_interval {
                                                "间隔执行 | 每 {task.interval.unwrap_or(0)} 分钟"
                                            } else {
                                                "定时执行 | {week_labels.join(\"、\")}"
                                            }
                                        }
                                    }
                                }
                                div { class: "flex items-center gap-1",
                                    EqButton {
                                        variant: EqButtonVariant::Ghost,
                                        size: EqButtonSize::Sm,
                                        onclick: move |_| on_trigger(task.script.clone()),
                                        "执行"
                                    }
                                    EqButton {
                                        variant: EqButtonVariant::Ghost,
                                        size: EqButtonSize::Sm,
                                        onclick: move |_| on_edit(task.clone()),
                                        "编辑"
                                    }
                                    EqButton {
                                        variant: EqButtonVariant::Ghost,
                                        size: EqButtonSize::Sm,
                                        onclick: move |_| on_delete(task_id),
                                        "删除"
                                    }
                                }
                            }
                        }
                    }
                })}
            }
        }
    }
}
