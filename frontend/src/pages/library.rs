use dioxus::prelude::*;
use eq_ui::prelude::*;
use serde_json::json;
use chrono::prelude::*;
use crate::api::client::{
    list_memories, create_memory, update_memory, delete_memory,
    list_presets, save_preset, update_preset, delete_preset,
    list_skills, create_skill, update_skill, delete_skill,
    list_projects, create_project, update_project, delete_project,
    list_scenarios, Memory, Preset, Skill, Project, Scenario,
};

#[derive(Debug, Clone, PartialEq)]
enum TabType {
    Memory,
    Preset,
    Skill,
    Saved,
    Scenario,
    Project,
    MCP,
    Screenshot,
    PromptControl,
}

#[derive(Debug, Clone, PartialEq)]
struct LibraryState {
    active_tab: TabType,
    memories: Vec<Memory>,
    presets: Vec<Preset>,
    skills: Vec<Skill>,
    projects: Vec<Project>,
    scenarios: Vec<Scenario>,
    search_query: String,
    show_create_modal: bool,
    editing_item: Option<serde_json::Value>,
}

impl Default for LibraryState {
    fn default() -> Self {
        Self {
            active_tab: TabType::Memory,
            memories: Vec::new(),
            presets: Vec::new(),
            skills: Vec::new(),
            projects: Vec::new(),
            scenarios: Vec::new(),
            search_query: String::new(),
            show_create_modal: false,
            editing_item: None,
        }
    }
}

#[component]
pub fn LibraryPage() -> Element {
    let state = use_signal(LibraryState::default);

    use_effect(move || {
        let state = state.clone();
        async move {
            load_data(state).await;
        }
    });

    let tabs = vec![
        ("记忆", TabType::Memory),
        ("预设", TabType::Preset),
        ("技能", TabType::Skill),
        ("保存项", TabType::Saved),
        ("场景", TabType::Scenario),
        ("项目", TabType::Project),
        ("MCP", TabType::MCP),
        ("截图", TabType::Screenshot),
        ("Prompt控制", TabType::PromptControl),
    ];

    let handle_tab_change = move |tab: TabType| {
        state.write().active_tab = tab;
    };

    let handle_search = move |query: String| {
        state.write().search_query = query;
    };

    let open_create_modal = move || {
        state.write().show_create_modal = true;
        state.write().editing_item = None;
    };

    let close_modal = move || {
        state.write().show_create_modal = false;
        state.write().editing_item = None;
    };

    let handle_create = move |data: serde_json::Value| {
        let state = state.clone();
        async move {
            match state.read().active_tab {
                TabType::Memory => {
                    let content = data.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    let name = data.get("name").and_then(|n| n.as_str());
                    let category = data.get("category").and_then(|c| c.as_str());
                    let _ = create_memory(content, name, category, None).await;
                }
                TabType::Preset => {
                    let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let description = data.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let system_prompt = data.get("system_prompt").and_then(|s| s.as_str()).unwrap_or("");
                    let enabled = data.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                    let _ = save_preset(name, description, system_prompt, enabled).await;
                }
                TabType::Skill => {
                    let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let description = data.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let prompt_template = data.get("prompt_template").and_then(|p| p.as_str()).unwrap_or("");
                    let enabled = data.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                    let _ = create_skill(name, description, prompt_template, enabled).await;
                }
                TabType::Project => {
                    let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let description = data.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let instructions = data.get("instructions").and_then(|i| i.as_str()).unwrap_or("");
                    let enabled = data.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                    let auto_inject = data.get("auto_inject").and_then(|a| a.as_bool()).unwrap_or(false);
                    let _ = create_project(name, description, instructions, enabled, auto_inject).await;
                }
                _ => {}
            }
            close_modal();
            load_data(state).await;
        }
    };

    let handle_edit = move |item: serde_json::Value| {
        state.write().editing_item = Some(item);
        state.write().show_create_modal = true;
    };

    let handle_update = move |data: serde_json::Value| {
        let state = state.clone();
        async move {
            let id = data.get("id").and_then(|i| i.as_str()).unwrap_or("");
            match state.read().active_tab {
                TabType::Memory => {
                    let content = data.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    let name = data.get("name").and_then(|n| n.as_str());
                    let category = data.get("category").and_then(|c| c.as_str());
                    let _ = update_memory(id, content, name, category, None).await;
                }
                TabType::Preset => {
                    let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let description = data.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let system_prompt = data.get("system_prompt").and_then(|s| s.as_str()).unwrap_or("");
                    let enabled = data.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                    let _ = update_preset(id, name, description, system_prompt, enabled).await;
                }
                TabType::Skill => {
                    let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let description = data.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let prompt_template = data.get("prompt_template").and_then(|p| p.as_str()).unwrap_or("");
                    let enabled = data.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
                    let _ = update_skill(id, name, description, prompt_template, enabled).await;
                }
                TabType::Project => {
                    let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let description = data.get("description").and_then(|d| d.as_str()).unwrap_or("");
                    let instructions = data.get("instructions").and_then(|i| i.as_str()).unwrap_or("");
                    let enabled = data.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                    let auto_inject = data.get("auto_inject").and_then(|a| a.as_bool()).unwrap_or(false);
                    let _ = update_project(id, name, description, instructions, enabled, auto_inject).await;
                }
                _ => {}
            }
            close_modal();
            load_data(state).await;
        }
    };

    let handle_delete = move |id: String| {
        let state = state.clone();
        async move {
            match state.read().active_tab {
                TabType::Memory => { let _ = delete_memory(&id).await; }
                TabType::Preset => { let _ = delete_preset(&id).await; }
                TabType::Skill => { let _ = delete_skill(&id).await; }
                TabType::Project => { let _ = delete_project(&id).await; }
                _ => {}
            }
            load_data(state).await;
        }
    };

    let filtered_memories = state.read().memories.iter()
        .filter(|m| {
            let q = &state.read().search_query.to_lowercase();
            m.name.to_lowercase().contains(q) ||
            m.content.to_lowercase().contains(q) ||
            m.tags.iter().any(|t| t.to_lowercase().contains(q))
        })
        .cloned()
        .collect::<Vec<_>>();

    let filtered_presets = state.read().presets.iter()
        .filter(|p| {
            let q = &state.read().search_query.to_lowercase();
            p.name.to_lowercase().contains(q) ||
            p.description.to_lowercase().contains(q) ||
            p.system_prompt.to_lowercase().contains(q)
        })
        .cloned()
        .collect::<Vec<_>>();

    let filtered_skills = state.read().skills.iter()
        .filter(|s| {
            let q = &state.read().search_query.to_lowercase();
            s.name.to_lowercase().contains(q) ||
            s.description.to_lowercase().contains(q)
        })
        .cloned()
        .collect::<Vec<_>>();

    let filtered_projects = state.read().projects.iter()
        .filter(|p| {
            let q = &state.read().search_query.to_lowercase();
            p.name.to_lowercase().contains(q) ||
            p.description.to_lowercase().contains(q)
        })
        .cloned()
        .collect::<Vec<_>>();

    let filtered_scenarios = state.read().scenarios.iter()
        .filter(|s| {
            let q = &state.read().search_query.to_lowercase();
            s.label.to_lowercase().contains(q)
        })
        .cloned()
        .collect::<Vec<_>>();

    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "知识库" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "管理记忆、预设和技能" }
                }
            }

            div { class: "flex gap-1 overflow-x-auto pb-3 border-b border-[var(--ds-border)]",
                for (label, tab) in tabs {
                    EqTab {
                        active: state.read().active_tab == tab,
                        onclick: move |_| handle_tab_change(tab.clone()),
                        "{label}"
                    }
                }
            }

            div { class: "flex-1 overflow-hidden",
                match state.read().active_tab {
                    TabType::Memory => {
                        rsx! {
                            MemoryList {
                                memories: filtered_memories,
                                on_edit: handle_edit,
                                on_delete: handle_delete,
                                on_create: open_create_modal,
                                search_query: state.read().search_query.clone(),
                                on_search: handle_search,
                            }
                        }
                    }
                    TabType::Preset => {
                        rsx! {
                            PresetList {
                                presets: filtered_presets,
                                on_edit: handle_edit,
                                on_delete: handle_delete,
                                on_create: open_create_modal,
                                search_query: state.read().search_query.clone(),
                                on_search: handle_search,
                            }
                        }
                    }
                    TabType::Skill => {
                        rsx! {
                            SkillList {
                                skills: filtered_skills,
                                on_edit: handle_edit,
                                on_delete: handle_delete,
                                on_create: open_create_modal,
                                search_query: state.read().search_query.clone(),
                                on_search: handle_search,
                            }
                        }
                    }
                    TabType::Project => {
                        rsx! {
                            ProjectList {
                                projects: filtered_projects,
                                on_edit: handle_edit,
                                on_delete: handle_delete,
                                on_create: open_create_modal,
                                search_query: state.read().search_query.clone(),
                                on_search: handle_search,
                            }
                        }
                    }
                    TabType::Scenario => {
                        rsx! {
                            ScenarioList {
                                scenarios: filtered_scenarios,
                                search_query: state.read().search_query.clone(),
                                on_search: handle_search,
                            }
                        }
                    }
                    _ => {
                        rsx! {
                            div { class: "flex-1 flex items-center justify-center text-[var(--ds-text-tertiary)]",
                                "该功能开发中..."
                            }
                        }
                    }
                }
            }

            if state.read().show_create_modal {
                CreateModal {
                    tab: state.read().active_tab.clone(),
                    editing_item: state.read().editing_item.clone(),
                    on_close: close_modal,
                    on_create: handle_create,
                    on_update: handle_update,
                }
            }
        }
    }
}

async fn load_data(state: Signal<LibraryState>) {
    let (memories_res, presets_res, skills_res, projects_res, scenarios_res) = tokio::join!(
        list_memories(None, None),
        list_presets(),
        list_skills(),
        list_projects(),
        list_scenarios()
    );

    let mut s = state.write();
    if let Ok(m) = memories_res {
        s.memories = m;
    } else if let Err(e) = memories_res {
        eprintln!("加载记忆失败: {}", e);
    }
    if let Ok(p) = presets_res {
        s.presets = p;
    } else if let Err(e) = presets_res {
        eprintln!("加载预设失败: {}", e);
    }
    if let Ok(sl) = skills_res {
        s.skills = sl;
    } else if let Err(e) = skills_res {
        eprintln!("加载技能失败: {}", e);
    }
    if let Ok(p) = projects_res {
        s.projects = p;
    } else if let Err(e) = projects_res {
        eprintln!("加载项目失败: {}", e);
    }
    if let Ok(sc) = scenarios_res {
        s.scenarios = sc;
    } else if let Err(e) = scenarios_res {
        eprintln!("加载场景失败: {}", e);
    }
}

#[derive(Props, PartialEq, Clone)]
struct MemoryListProps {
    memories: Vec<Memory>,
    on_edit: EventHandler<serde_json::Value>,
    on_delete: EventHandler<String>,
    on_create: EventHandler<()>,
    search_query: String,
    on_search: EventHandler<String>,
}

#[component]
fn MemoryList(props: MemoryListProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex gap-2 p-3 border-b border-[var(--ds-border)]",
                input {
                    class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "搜索记忆...",
                    value: props.search_query.clone(),
                    oninput: move |e| props.on_search.call(e.value()),
                }
                EqButton {
                    variant: EqButtonVariant::Primary,
                    size: EqButtonSize::Sm,
                    onclick: move |_| props.on_create.call(()),
                    "新建记忆"
                }
            }
            div { class: "flex-1 overflow-y-auto p-3 space-y-3",
                if props.memories.is_empty() {
                    div { class: "text-center text-[var(--ds-text-tertiary)] py-8", "暂无记忆" }
                } else {
                    for mem in &props.memories {
                        div {
                            class: "p-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] hover:border-[var(--ds-blue)] transition-colors",
                            div { class: "flex items-start justify-between gap-2",
                                div { class: "flex-1",
                                    div { class: "flex items-center gap-2 mb-1",
                                        span { class: "text-sm font-medium text-[var(--ds-text)]", "{mem.name}" }
                                        if !mem.category.is_empty() {
                                            span { class: "text-[10px] px-1.5 py-0.5 rounded bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)]", "{mem.category}" }
                                        }
                                        if mem.pinned {
                                            svg { class: "w-3 h-3 text-amber-500", fill: "currentColor", view_box: "0 0 24 24",
                                                path { d: "M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" }
                                            }
                                        }
                                    }
                                    p { class: "text-xs text-[var(--ds-text-secondary)] line-clamp-2", "{mem.content}" }
                                    if !mem.tags.is_empty() {
                                        div { class: "flex gap-1 mt-2",
                                            for tag in &mem.tags {
                                                span { class: "text-[10px] px-1.5 py-0.5 rounded-full bg-[var(--ds-blue-light)] text-[var(--ds-blue)]", "{tag}" }
                                            }
                                        }
                                    }
                                    div { class: "flex items-center gap-3 mt-2 text-[10px] text-[var(--ds-text-tertiary)]",
                                        span { "访问: {mem.access_count}" }
                                        span { "创建: {format_time(mem.created_at)}" }
                                    }
                                }
                                div { class: "flex items-center gap-1",
                                    button {
                                        class: "p-1.5 hover:bg-[var(--ds-surface)] rounded transition-colors",
                                        onclick: move |_| props.on_edit.call(json!({
                                            "id": mem.id,
                                            "name": mem.name,
                                            "content": mem.content,
                                            "category": mem.category,
                                            "tags": mem.tags,
                                        })),
                                        svg { class: "w-4 h-4 text-[var(--ds-text-secondary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" }
                                        }
                                    }
                                    button {
                                        class: "p-1.5 hover:bg-red-50 rounded transition-colors",
                                        onclick: move |_| props.on_delete.call(mem.id.clone()),
                                        svg { class: "w-4 h-4 text-red-500", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct PresetListProps {
    presets: Vec<Preset>,
    on_edit: EventHandler<serde_json::Value>,
    on_delete: EventHandler<String>,
    on_create: EventHandler<()>,
    search_query: String,
    on_search: EventHandler<String>,
}

#[component]
fn PresetList(props: PresetListProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex gap-2 p-3 border-b border-[var(--ds-border)]",
                input {
                    class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "搜索预设...",
                    value: props.search_query.clone(),
                    oninput: move |e| props.on_search.call(e.value()),
                }
                EqButton {
                    variant: EqButtonVariant::Primary,
                    size: EqButtonSize::Sm,
                    onclick: move |_| props.on_create.call(()),
                    "新建预设"
                }
            }
            div { class: "flex-1 overflow-y-auto p-3 space-y-3",
                if props.presets.is_empty() {
                    div { class: "text-center text-[var(--ds-text-tertiary)] py-8", "暂无预设" }
                } else {
                    for preset in &props.presets {
                        div {
                            class: "p-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] hover:border-[var(--ds-blue)] transition-colors",
                            div { class: "flex items-start justify-between gap-2",
                                div { class: "flex-1",
                                    div { class: "flex items-center gap-2 mb-1",
                                        span { class: "text-sm font-medium text-[var(--ds-text)]", "{preset.name}" }
                                        if preset.enabled {
                                            div { class: "w-2 h-2 rounded-full bg-green-500" }
                                        }
                                    }
                                    p { class: "text-xs text-[var(--ds-text-secondary)] mb-2", "{preset.description}" }
                                    div { class: "p-2 bg-[var(--ds-surface)] rounded text-xs text-[var(--ds-text-tertiary)] font-mono line-clamp-3", "{preset.system_prompt}" }
                                }
                                div { class: "flex items-center gap-1",
                                    button {
                                        class: "p-1.5 hover:bg-[var(--ds-surface)] rounded transition-colors",
                                        onclick: move |_| props.on_edit.call(json!({
                                            "id": preset.id,
                                            "name": preset.name,
                                            "description": preset.description,
                                            "system_prompt": preset.system_prompt,
                                            "enabled": preset.enabled,
                                        })),
                                        svg { class: "w-4 h-4 text-[var(--ds-text-secondary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" }
                                        }
                                    }
                                    button {
                                        class: "p-1.5 hover:bg-red-50 rounded transition-colors",
                                        onclick: move |_| props.on_delete.call(preset.id.clone()),
                                        svg { class: "w-4 h-4 text-red-500", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct SkillListProps {
    skills: Vec<Skill>,
    on_edit: EventHandler<serde_json::Value>,
    on_delete: EventHandler<String>,
    on_create: EventHandler<()>,
    search_query: String,
    on_search: EventHandler<String>,
}

#[component]
fn SkillList(props: SkillListProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex gap-2 p-3 border-b border-[var(--ds-border)]",
                input {
                    class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "搜索技能...",
                    value: props.search_query.clone(),
                    oninput: move |e| props.on_search.call(e.value()),
                }
                EqButton {
                    variant: EqButtonVariant::Primary,
                    size: EqButtonSize::Sm,
                    onclick: move |_| props.on_create.call(()),
                    "新建技能"
                }
            }
            div { class: "flex-1 overflow-y-auto p-3 space-y-3",
                if props.skills.is_empty() {
                    div { class: "text-center text-[var(--ds-text-tertiary)] py-8", "暂无技能" }
                } else {
                    for skill in &props.skills {
                        div {
                            class: "p-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] hover:border-[var(--ds-blue)] transition-colors",
                            div { class: "flex items-start justify-between gap-2",
                                div { class: "flex-1",
                                    div { class: "flex items-center gap-2 mb-1",
                                        span { class: "text-sm font-medium text-[var(--ds-text)]", "{skill.name}" }
                                        if !skill.category.is_empty() {
                                            span { class: "text-[10px] px-1.5 py-0.5 rounded bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)]", "{skill.category}" }
                                        }
                                        if skill.enabled {
                                            div { class: "w-2 h-2 rounded-full bg-green-500" }
                                        }
                                    }
                                    p { class: "text-xs text-[var(--ds-text-secondary)] mb-2", "{skill.description}" }
                                    if !skill.variables.is_empty() {
                                        div { class: "flex flex-wrap gap-1 mb-2",
                                            for var in &skill.variables {
                                                span { class: "text-[10px] px-1.5 py-0.5 rounded-full bg-amber-50 text-amber-700", "{var.name}" }
                                            }
                                        }
                                    }
                                    div { class: "text-[10px] text-[var(--ds-text-tertiary)]", "来源: {skill.source}" }
                                }
                                div { class: "flex items-center gap-1",
                                    button {
                                        class: "p-1.5 hover:bg-[var(--ds-surface)] rounded transition-colors",
                                        onclick: move |_| props.on_edit.call(json!({
                                            "id": skill.id,
                                            "name": skill.name,
                                            "description": skill.description,
                                            "prompt_template": skill.prompt_template,
                                            "enabled": skill.enabled,
                                        })),
                                        svg { class: "w-4 h-4 text-[var(--ds-text-secondary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" }
                                        }
                                    }
                                    button {
                                        class: "p-1.5 hover:bg-red-50 rounded transition-colors",
                                        onclick: move |_| props.on_delete.call(skill.id.clone()),
                                        svg { class: "w-4 h-4 text-red-500", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct ProjectListProps {
    projects: Vec<Project>,
    on_edit: EventHandler<serde_json::Value>,
    on_delete: EventHandler<String>,
    on_create: EventHandler<()>,
    search_query: String,
    on_search: EventHandler<String>,
}

#[component]
fn ProjectList(props: ProjectListProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex gap-2 p-3 border-b border-[var(--ds-border)]",
                input {
                    class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "搜索项目...",
                    value: props.search_query.clone(),
                    oninput: move |e| props.on_search.call(e.value()),
                }
                EqButton {
                    variant: EqButtonVariant::Primary,
                    size: EqButtonSize::Sm,
                    onclick: move |_| props.on_create.call(()),
                    "新建项目"
                }
            }
            div { class: "flex-1 overflow-y-auto p-3 space-y-3",
                if props.projects.is_empty() {
                    div { class: "text-center text-[var(--ds-text-tertiary)] py-8", "暂无项目" }
                } else {
                    for project in &props.projects {
                        div {
                            class: "p-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] hover:border-[var(--ds-blue)] transition-colors",
                            div { class: "flex items-start justify-between gap-2",
                                div { class: "flex-1",
                                    div { class: "flex items-center gap-2 mb-1",
                                        span { class: "text-sm font-medium text-[var(--ds-text)]", "{project.name}" }
                                        if project.enabled {
                                            div { class: "w-2 h-2 rounded-full bg-green-500" }
                                        }
                                        if project.auto_inject {
                                            span { class: "text-[10px] px-1.5 py-0.5 rounded-full bg-blue-50 text-blue-700", "自动注入" }
                                        }
                                    }
                                    p { class: "text-xs text-[var(--ds-text-secondary)] mb-2", "{project.description}" }
                                    div { class: "p-2 bg-[var(--ds-surface)] rounded text-xs text-[var(--ds-text-tertiary)] line-clamp-2", "{project.instructions}" }
                                }
                                div { class: "flex items-center gap-1",
                                    button {
                                        class: "p-1.5 hover:bg-[var(--ds-surface)] rounded transition-colors",
                                        onclick: move |_| props.on_edit.call(json!({
                                            "id": project.id,
                                            "name": project.name,
                                            "description": project.description,
                                            "instructions": project.instructions,
                                            "enabled": project.enabled,
                                            "auto_inject": project.auto_inject,
                                        })),
                                        svg { class: "w-4 h-4 text-[var(--ds-text-secondary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" }
                                        }
                                    }
                                    button {
                                        class: "p-1.5 hover:bg-red-50 rounded transition-colors",
                                        onclick: move |_| props.on_delete.call(project.id.clone()),
                                        svg { class: "w-4 h-4 text-red-500", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct ScenarioListProps {
    scenarios: Vec<Scenario>,
    search_query: String,
    on_search: EventHandler<String>,
}

#[component]
fn ScenarioList(props: ScenarioListProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex gap-2 p-3 border-b border-[var(--ds-border)]",
                input {
                    class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "搜索场景...",
                    value: props.search_query.clone(),
                    oninput: move |e| props.on_search.call(e.value()),
                }
            }
            div { class: "flex-1 overflow-y-auto p-3 space-y-2",
                if props.scenarios.is_empty() {
                    div { class: "text-center text-[var(--ds-text-tertiary)] py-8", "暂无场景" }
                } else {
                    for scenario in &props.scenarios {
                        div {
                            class: "flex items-center justify-between p-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)]",
                            div { class: "flex items-center gap-2",
                                span { class: "text-sm text-[var(--ds-text)]", "{scenario.label}" }
                                if scenario.built_in {
                                    span { class: "text-[10px] px-1.5 py-0.5 rounded bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)]", "内置" }
                                }
                                if scenario.enabled {
                                    div { class: "w-2 h-2 rounded-full bg-green-500" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct CreateModalProps {
    tab: TabType,
    editing_item: Option<serde_json::Value>,
    on_close: EventHandler<()>,
    on_create: EventHandler<serde_json::Value>,
    on_update: EventHandler<serde_json::Value>,
}

#[component]
fn CreateModal(props: CreateModalProps) -> Element {
    let is_editing = props.editing_item.is_some();
    let editing = props.editing_item.clone().unwrap_or_default();

    let name = use_signal(|| editing.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string());
    let description = use_signal(|| editing.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string());
    let content = use_signal(|| editing.get("content").and_then(|c| c.as_str()).unwrap_or("").to_string());
    let system_prompt = use_signal(|| editing.get("system_prompt").and_then(|s| s.as_str()).unwrap_or("").to_string());
    let prompt_template = use_signal(|| editing.get("prompt_template").and_then(|p| p.as_str()).unwrap_or("").to_string());
    let instructions = use_signal(|| editing.get("instructions").and_then(|i| i.as_str()).unwrap_or("").to_string());
    let category = use_signal(|| editing.get("category").and_then(|c| c.as_str()).unwrap_or("").to_string());
    let enabled = use_signal(|| editing.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true));
    let auto_inject = use_signal(|| editing.get("auto_inject").and_then(|a| a.as_bool()).unwrap_or(false));
    let first_input_ref = use_signal(|| None::<ElementRef>);

    let handle_submit = move |_| {
        let data = match props.tab {
            TabType::Memory => json!({
                "id": editing.get("id").and_then(|i| i.as_str()).unwrap_or(""),
                "name": name.read(),
                "content": content.read(),
                "category": category.read(),
            }),
            TabType::Preset => json!({
                "id": editing.get("id").and_then(|i| i.as_str()).unwrap_or(""),
                "name": name.read(),
                "description": description.read(),
                "system_prompt": system_prompt.read(),
                "enabled": enabled.read(),
            }),
            TabType::Skill => json!({
                "id": editing.get("id").and_then(|i| i.as_str()).unwrap_or(""),
                "name": name.read(),
                "description": description.read(),
                "prompt_template": prompt_template.read(),
                "enabled": enabled.read(),
            }),
            TabType::Project => json!({
                "id": editing.get("id").and_then(|i| i.as_str()).unwrap_or(""),
                "name": name.read(),
                "description": description.read(),
                "instructions": instructions.read(),
                "enabled": enabled.read(),
                "auto_inject": auto_inject.read(),
            }),
            _ => json!({}),
        };
        if is_editing {
            spawn(async move { props.on_update.call(data); });
        } else {
            spawn(async move { props.on_create.call(data); });
        }
    };

    let handle_keydown = move |ev: Event<KeyboardEvent>| {
        if ev.key() == "Escape" {
            props.on_close.call(());
        }
    };

    let title = match (props.tab, is_editing) {
        (TabType::Memory, true) => "编辑记忆",
        (TabType::Memory, false) => "新建记忆",
        (TabType::Preset, true) => "编辑预设",
        (TabType::Preset, false) => "新建预设",
        (TabType::Skill, true) => "编辑技能",
        (TabType::Skill, false) => "新建技能",
        (TabType::Project, true) => "编辑项目",
        (TabType::Project, false) => "新建项目",
        _ => "新建",
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
            onclick: move |_| props.on_close.call(()),
            onkeydown: handle_keydown,
            tabindex: "0",
            onmounted: move |_| {
                if let Some(el) = first_input_ref.read().as_ref() {
                    if let Some(input) = el.get_element() {
                        input.focus();
                    }
                }
            },
            div {
                class: "bg-[var(--ds-bg)] border border-[var(--ds-border)] rounded-lg shadow-xl w-full max-w-lg p-4",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex items-center justify-between mb-4",
                    h3 { class: "text-base font-semibold text-[var(--ds-text)]", "{title}" }
                    button {
                        class: "p-1 hover:bg-[var(--ds-surface)] rounded",
                        onclick: move |_| props.on_close.call(()),
                        svg { class: "w-5 h-5 text-[var(--ds-text-secondary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                        }
                    }
                }
                div { class: "space-y-3",
                    match props.tab {
                        TabType::Memory => {
                            rsx! {
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "名称" }
                                    input {
                                        ref: first_input_ref,
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "分类" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: category.read().clone(),
                                        oninput: move |e| category.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "内容" }
                                    textarea {
                                        class: "w-full min-h-[100px] px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)] resize-none",
                                        value: content.read().clone(),
                                        oninput: move |e| content.write(e.value()),
                                    }
                                }
                            }
                        }
                        TabType::Preset => {
                            rsx! {
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "名称" }
                                    input {
                                        ref: first_input_ref,
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "描述" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: description.read().clone(),
                                        oninput: move |e| description.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "系统提示词" }
                                    textarea {
                                        class: "w-full min-h-[100px] px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)] resize-none font-mono",
                                        value: system_prompt.read().clone(),
                                        oninput: move |e| system_prompt.write(e.value()),
                                    }
                                }
                                div { class: "flex items-center gap-2",
                                    input {
                                        r#type: "checkbox",
                                        checked: enabled.read(),
                                        onchange: move |e| enabled.write(e.checked()),
                                    }
                                    label { class: "text-xs text-[var(--ds-text-secondary)]", "启用" }
                                }
                            }
                        }
                        TabType::Skill => {
                            rsx! {
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "名称" }
                                    input {
                                        ref: first_input_ref,
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "描述" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: description.read().clone(),
                                        oninput: move |e| description.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "提示词模板" }
                                    textarea {
                                        class: "w-full min-h-[100px] px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)] resize-none font-mono",
                                        value: prompt_template.read().clone(),
                                        oninput: move |e| prompt_template.write(e.value()),
                                    }
                                }
                                div { class: "flex items-center gap-2",
                                    input {
                                        r#type: "checkbox",
                                        checked: enabled.read(),
                                        onchange: move |e| enabled.write(e.checked()),
                                    }
                                    label { class: "text-xs text-[var(--ds-text-secondary)]", "启用" }
                                }
                            }
                        }
                        TabType::Project => {
                            rsx! {
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "名称" }
                                    input {
                                        ref: first_input_ref,
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "描述" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: description.read().clone(),
                                        oninput: move |e| description.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "指令" }
                                    textarea {
                                        class: "w-full min-h-[100px] px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)] resize-none",
                                        value: instructions.read().clone(),
                                        oninput: move |e| instructions.write(e.value()),
                                    }
                                }
                                div { class: "flex items-center gap-2",
                                    input {
                                        r#type: "checkbox",
                                        checked: enabled.read(),
                                        onchange: move |e| enabled.write(e.checked()),
                                    }
                                    label { class: "text-xs text-[var(--ds-text-secondary)]", "启用" }
                                }
                                div { class: "flex items-center gap-2",
                                    input {
                                        r#type: "checkbox",
                                        checked: auto_inject.read(),
                                        onchange: move |e| auto_inject.write(e.checked()),
                                    }
                                    label { class: "text-xs text-[var(--ds-text-secondary)]", "自动注入上下文" }
                                }
                            }
                        }
                        _ => rsx! { div { "不支持的类型" } }
                    }
                }
                div { class: "flex justify-end gap-2 mt-4",
                    EqButton {
                        variant: EqButtonVariant::Ghost,
                        onclick: move |_| props.on_close.call(()),
                        "取消"
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        onclick: handle_submit,
                        if is_editing { "更新" } else { "创建" }
                    }
                }
            }
        }
    }
}

fn format_time(ts: i64) -> String {
    let secs = (ts / 1000) as i64;
    let dt = chrono::DateTime::<chrono::Utc>::from_utc(
        chrono::NaiveDateTime::from_timestamp_opt(secs, 0).unwrap_or_default(),
        chrono::Utc,
    );
    dt.format("%Y-%m-%d").to_string()
}