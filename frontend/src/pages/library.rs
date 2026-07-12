use dioxus::prelude::*;
use eq_ui::prelude::*;
use serde_json::json;
use chrono::prelude::*;
use crate::api::client::{
    list_memories, create_memory, update_memory, delete_memory,
    list_presets, save_preset, update_preset, delete_preset,
    list_skills, create_skill, update_skill, delete_skill,
    list_projects, create_project, update_project, delete_project,
    list_scenarios, list_saved_items, create_saved_item, update_saved_item, delete_saved_item,
    list_mcp_servers, create_mcp_server, update_mcp_server, delete_mcp_server,
    list_screenshots, get_prompt_settings, update_prompt_settings, PromptSettings,
    Memory, Preset, Skill, Project, Scenario, SavedItem, McpServer,
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
    saved_items: Vec<SavedItem>,
    mcp_servers: Vec<McpServer>,
    screenshots: Vec<String>,
    prompt_settings: PromptSettings,
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
            saved_items: Vec::new(),
            mcp_servers: Vec::new(),
            screenshots: Vec::new(),
            prompt_settings: PromptSettings {
                memory_enabled: true,
                system_prompt_enabled: true,
                preset_cadence: "every".to_string(),
                force_response_language: "".to_string(),
                active_preset_id: "".to_string(),
            },
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
                TabType::Saved => {
                    let title = data.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let content = data.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    let kind = data.get("kind").and_then(|k| k.as_str());
                    let tags = data.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<_>>());
                    let source_url = data.get("source_url").and_then(|s| s.as_str());
                    let _ = create_saved_item(title, content, kind, tags.as_deref(), source_url).await;
                }
                TabType::MCP => {
                    let name = data.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let transport = data.get("transport").and_then(|t| t.as_str());
                    let command = data.get("command").and_then(|c| c.as_str());
                    let url = data.get("url").and_then(|u| u.as_str());
                    let enabled = data.get("enabled").and_then(|e| e.as_bool());
                    let auto_connect = data.get("auto_connect").and_then(|ac| ac.as_bool());
                    let _ = create_mcp_server(name, transport, command, None, url, enabled, auto_connect).await;
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
                TabType::Saved => {
                    let title = data.get("title").and_then(|t| t.as_str()).unwrap_or("");
                    let content = data.get("content").and_then(|c| c.as_str()).unwrap_or("");
                    let kind = data.get("kind").and_then(|k| k.as_str());
                    let tags = data.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect::<Vec<_>>());
                    let source_url = data.get("source_url").and_then(|s| s.as_str());
                    let _ = update_saved_item(id, title, content, kind, tags.as_deref(), source_url).await;
                }
                TabType::MCP => {
                    let name = data.get("name").and_then(|n| n.as_str());
                    let transport = data.get("transport").and_then(|t| t.as_str());
                    let command = data.get("command").and_then(|c| c.as_str());
                    let url = data.get("url").and_then(|u| u.as_str());
                    let enabled = data.get("enabled").and_then(|e| e.as_bool());
                    let auto_connect = data.get("auto_connect").and_then(|ac| ac.as_bool());
                    let _ = update_mcp_server(id, name, transport, command, None, url, enabled, auto_connect).await;
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
                TabType::Saved => { let _ = delete_saved_item(&id).await; }
                TabType::MCP => { let _ = delete_mcp_server(&id).await; }
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

    let filtered_saved_items = state.read().saved_items.iter()
        .filter(|si| {
            let q = &state.read().search_query.to_lowercase();
            si.title.to_lowercase().contains(q) ||
            si.content.to_lowercase().contains(q) ||
            si.kind.to_lowercase().contains(q) ||
            si.tags.iter().any(|t| t.to_lowercase().contains(q))
        })
        .cloned()
        .collect::<Vec<_>>();

    let filtered_mcp_servers = state.read().mcp_servers.iter()
        .filter(|m| {
            let q = &state.read().search_query.to_lowercase();
            m.name.to_lowercase().contains(q) ||
            m.command.to_lowercase().contains(q) ||
            m.url.to_lowercase().contains(q)
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
                    TabType::Saved => {
                        rsx! {
                            SavedList {
                                items: filtered_saved_items,
                                on_edit: handle_edit,
                                on_delete: handle_delete,
                                on_create: open_create_modal,
                                search_query: state.read().search_query.clone(),
                                on_search: handle_search,
                            }
                        }
                    }
                    TabType::MCP => {
                        rsx! {
                            McpList {
                                servers: filtered_mcp_servers,
                                on_edit: handle_edit,
                                on_delete: handle_delete,
                                on_create: open_create_modal,
                                search_query: state.read().search_query.clone(),
                                on_search: handle_search,
                            }
                        }
                    }
                    TabType::Screenshot => {
                        rsx! {
                            ScreenshotList {
                                screenshots: state.read().screenshots.clone(),
                                search_query: state.read().search_query.clone(),
                                on_search: handle_search,
                                on_refresh: move || {
                                    let state = state.clone();
                                    spawn(async move { load_data(state).await; });
                                },
                            }
                        }
                    }
                    TabType::PromptControl => {
                        rsx! {
                            PromptControlPanel {
                                settings: state.read().prompt_settings.clone(),
                                on_update: move |new_settings| {
                                    let state = state.clone();
                                    async move {
                                        let _ = update_prompt_settings(&new_settings).await;
                                        state.write().prompt_settings = new_settings;
                                    }
                                },
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
    let memories_res = list_memories(None, None).await;
    let presets_res = list_presets().await;
    let skills_res = list_skills().await;
    let projects_res = list_projects().await;
    let scenarios_res = list_scenarios().await;
    let saved_items_res = list_saved_items().await;
    let mcp_servers_res = list_mcp_servers().await;
    let screenshots_res = list_screenshots().await;
    let prompt_settings_res = get_prompt_settings().await;

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
    if let Ok(si) = saved_items_res {
        s.saved_items = si;
    } else if let Err(e) = saved_items_res {
        eprintln!("加载保存项失败: {}", e);
    }
    if let Ok(mcp) = mcp_servers_res {
        s.mcp_servers = mcp;
    } else if let Err(e) = mcp_servers_res {
        eprintln!("加载MCP服务器失败: {}", e);
    }
    if let Ok(ss) = screenshots_res {
        s.screenshots = ss;
    } else if let Err(e) = screenshots_res {
        eprintln!("加载截图失败: {}", e);
    }
    if let Ok(ps) = prompt_settings_res {
        s.prompt_settings = ps;
    } else if let Err(e) = prompt_settings_res {
        eprintln!("加载Prompt设置失败: {}", e);
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
struct SavedListProps {
    items: Vec<SavedItem>,
    on_edit: EventHandler<serde_json::Value>,
    on_delete: EventHandler<String>,
    on_create: EventHandler<()>,
    search_query: String,
    on_search: EventHandler<String>,
}

#[component]
fn SavedList(props: SavedListProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex gap-2 p-3 border-b border-[var(--ds-border)]",
                input {
                    class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "搜索保存项...",
                    value: props.search_query.clone(),
                    oninput: move |e| props.on_search.call(e.value()),
                }
                EqButton {
                    variant: EqButtonVariant::Primary,
                    size: EqButtonSize::Sm,
                    onclick: move |_| props.on_create.call(()),
                    "+ 添加"
                }
            }
            div { class: "flex-1 overflow-y-auto p-3 space-y-3",
                if props.items.is_empty() {
                    div { class: "text-center text-[var(--ds-text-tertiary)] py-8", "暂无保存项" }
                } else {
                    for item in &props.items {
                        div {
                            class: "p-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] hover:border-[var(--ds-blue)] transition-colors",
                            div { class: "flex items-start justify-between gap-2",
                                div { class: "flex-1",
                                    div { class: "flex items-center gap-2 mb-1",
                                        span { class: "text-sm font-medium text-[var(--ds-text)]", "{item.title}" }
                                        span { class: "text-[10px] px-1.5 py-0.5 rounded-full bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)]", "{item.kind}" }
                                    }
                                    p { class: "text-xs text-[var(--ds-text-secondary)] line-clamp-2", "{item.content}" }
                                    if !item.tags.is_empty() {
                                        div { class: "flex gap-1 mt-2 flex-wrap",
                                            for tag in &item.tags {
                                                span { class: "text-[10px] px-1.5 py-0.5 rounded bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)]", "{tag}" }
                                            }
                                        }
                                    }
                                }
                                div { class: "flex items-center gap-1",
                                    button {
                                        class: "p-1.5 hover:bg-[var(--ds-surface)] rounded transition-colors",
                                        onclick: move |_| props.on_edit.call(json!({
                                            "id": item.id,
                                            "title": item.title,
                                            "content": item.content,
                                            "kind": item.kind,
                                            "tags": item.tags,
                                            "source_url": item.source_url,
                                        })),
                                        svg { class: "w-4 h-4 text-[var(--ds-text-secondary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" }
                                        }
                                    }
                                    button {
                                        class: "p-1.5 hover:bg-red-50 rounded transition-colors",
                                        onclick: move |_| props.on_delete.call(item.id.clone()),
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
struct McpListProps {
    servers: Vec<McpServer>,
    on_edit: EventHandler<serde_json::Value>,
    on_delete: EventHandler<String>,
    on_create: EventHandler<()>,
    search_query: String,
    on_search: EventHandler<String>,
}

#[component]
fn McpList(props: McpListProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex gap-2 p-3 border-b border-[var(--ds-border)]",
                input {
                    class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "搜索MCP服务器...",
                    value: props.search_query.clone(),
                    oninput: move |e| props.on_search.call(e.value()),
                }
                EqButton {
                    variant: EqButtonVariant::Primary,
                    size: EqButtonSize::Sm,
                    onclick: move |_| props.on_create.call(()),
                    "+ 添加"
                }
            }
            div { class: "flex-1 overflow-y-auto p-3 space-y-3",
                if props.servers.is_empty() {
                    div { class: "text-center text-[var(--ds-text-tertiary)] py-8", "暂无MCP服务器" }
                } else {
                    for server in &props.servers {
                        div {
                            class: "p-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] hover:border-[var(--ds-blue)] transition-colors",
                            div { class: "flex items-start justify-between gap-2",
                                div { class: "flex-1",
                                    div { class: "flex items-center gap-2 mb-1",
                                        span { class: "text-sm font-medium text-[var(--ds-text)]", "{server.name}" }
                                        if server.enabled {
                                            div { class: "w-2 h-2 rounded-full bg-green-500" }
                                        }
                                        if server.auto_connect {
                                            span { class: "text-[10px] px-1.5 py-0.5 rounded-full bg-blue-50 text-blue-700", "自动连接" }
                                        }
                                    }
                                    p { class: "text-xs text-[var(--ds-text-secondary)] mb-2", "传输: {server.transport} | 命令: {server.command}" }
                                    if !server.url.is_empty() {
                                        p { class: "text-xs text-[var(--ds-text-tertiary)]", "URL: {server.url}" }
                                    }
                                }
                                div { class: "flex items-center gap-1",
                                    button {
                                        class: "p-1.5 hover:bg-[var(--ds-surface)] rounded transition-colors",
                                        onclick: move |_| props.on_edit.call(json!({
                                            "id": server.id,
                                            "name": server.name,
                                            "transport": server.transport,
                                            "command": server.command,
                                            "url": server.url,
                                            "enabled": server.enabled,
                                            "auto_connect": server.auto_connect,
                                        })),
                                        svg { class: "w-4 h-4 text-[var(--ds-text-secondary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" }
                                        }
                                    }
                                    button {
                                        class: "p-1.5 hover:bg-red-50 rounded transition-colors",
                                        onclick: move |_| props.on_delete.call(server.id.clone()),
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
struct PromptControlPanelProps {
    settings: PromptSettings,
    on_update: EventHandler<PromptSettings>,
}

#[component]
fn PromptControlPanel(props: PromptControlPanelProps) -> Element {
    let memory_enabled = use_signal(|| props.settings.memory_enabled);
    let system_prompt_enabled = use_signal(|| props.settings.system_prompt_enabled);
    let preset_cadence = use_signal(|| props.settings.preset_cadence.clone());
    let force_response_language = use_signal(|| props.settings.force_response_language.clone());
    let active_preset_id = use_signal(|| props.settings.active_preset_id.clone());

    let handle_save = move |_| {
        let new_settings = PromptSettings {
            memory_enabled: memory_enabled.read(),
            system_prompt_enabled: system_prompt_enabled.read(),
            preset_cadence: preset_cadence.read().clone(),
            force_response_language: force_response_language.read().clone(),
            active_preset_id: active_preset_id.read().clone(),
        };
        props.on_update.call(new_settings);
    };

    rsx! {
        div { class: "flex flex-col h-full p-4",
            div { class: "space-y-4",
                div {
                    label { class: "flex items-center gap-2 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: memory_enabled.read(),
                            onchange: move |e| memory_enabled.write(e.checked()),
                            class: "w-4 h-4 rounded border-[var(--ds-border)] text-[var(--ds-blue)] focus:ring-[var(--ds-blue)]",
                        }
                        span { class: "text-sm text-[var(--ds-text)]", "启用记忆注入" }
                    }
                }
                div {
                    label { class: "flex items-center gap-2 cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: system_prompt_enabled.read(),
                            onchange: move |e| system_prompt_enabled.write(e.checked()),
                            class: "w-4 h-4 rounded border-[var(--ds-border)] text-[var(--ds-blue)] focus:ring-[var(--ds-blue)]",
                        }
                        span { class: "text-sm text-[var(--ds-text)]", "启用系统提示" }
                    }
                }
                div {
                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "预设注入频率" }
                    select {
                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                        value: preset_cadence.read().clone(),
                        onchange: move |e| preset_cadence.write(e.value()),
                        option { value: "every", "每次对话" }
                        option { value: "first", "仅首次" }
                        option { value: "none", "不注入" }
                    }
                }
                div {
                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "强制响应语言" }
                    input {
                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                        placeholder: "如: zh-CN, en-US",
                        value: force_response_language.read().clone(),
                        oninput: move |e| force_response_language.write(e.value()),
                    }
                }
                div {
                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "活动预设ID" }
                    input {
                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                        placeholder: "留空使用默认",
                        value: active_preset_id.read().clone(),
                        oninput: move |e| active_preset_id.write(e.value()),
                    }
                }
            }
            div { class: "mt-auto pt-4 border-t border-[var(--ds-border)]",
                EqButton {
                    variant: EqButtonVariant::Primary,
                    size: EqButtonSize::Md,
                    onclick: handle_save,
                    "保存设置"
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct ScreenshotListProps {
    screenshots: Vec<String>,
    search_query: String,
    on_search: EventHandler<String>,
    on_refresh: EventHandler<()>,
}

#[component]
fn ScreenshotList(props: ScreenshotListProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex gap-2 p-3 border-b border-[var(--ds-border)]",
                input {
                    class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "搜索截图...",
                    value: props.search_query.clone(),
                    oninput: move |e| props.on_search.call(e.value()),
                }
                EqButton {
                    variant: EqButtonVariant::Ghost,
                    size: EqButtonSize::Sm,
                    onclick: move |_| props.on_refresh.call(()),
                    "刷新"
                }
            }
            div { class: "flex-1 overflow-y-auto p-4",
                if props.screenshots.is_empty() {
                    div { class: "flex flex-col items-center justify-center py-12 text-[var(--ds-text-tertiary)]",
                        div { class: "w-16 h-16 rounded-full bg-[var(--ds-surface)] flex items-center justify-center mb-4",
                            svg { class: "w-8 h-8", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" }
                            }
                        }
                        p { class: "text-sm", "暂无截图" }
                    }
                } else {
                    div { class: "grid grid-cols-3 gap-3",
                        for (idx, screenshot) in props.screenshots.iter().enumerate() {
                            let image_url = if screenshot.starts_with("http") {
                                screenshot.clone()
                            } else {
                                format!("/screenshots/{}", screenshot)
                            };
                            div {
                                class: "relative group rounded-lg overflow-hidden border border-[var(--ds-border)] bg-[var(--ds-card)] cursor-pointer hover:border-[var(--ds-blue)] transition-colors",
                                img {
                                    src: "{image_url}",
                                    class: "w-full h-32 object-cover",
                                    alt: "截图",
                                }
                                div { class: "absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors flex items-center justify-center gap-2 opacity-0 group-hover:opacity-100",
                                    button {
                                        class: "p-2 bg-white/90 rounded-full hover:bg-white transition-colors",
                                        onclick: move |_| {
                                            let window = web_sys::window().unwrap();
                                            let _ = window.open_with_url_and_target(&image_url, "_blank");
                                        },
                                        svg { class: "w-4 h-4 text-gray-700", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" }
                                        }
                                    }
                                }
                                div { class: "absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/60 to-transparent p-2",
                                    span { class: "text-[10px] text-white truncate block", "{screenshot}" }
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
    let title = use_signal(|| editing.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string());
    let kind = use_signal(|| editing.get("kind").and_then(|k| k.as_str()).unwrap_or("snippet").to_string());
    let transport = use_signal(|| editing.get("transport").and_then(|t| t.as_str()).unwrap_or("stdio").to_string());
    let command = use_signal(|| editing.get("command").and_then(|c| c.as_str()).unwrap_or("").to_string());
    let url = use_signal(|| editing.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string());
    let auto_connect = use_signal(|| editing.get("auto_connect").and_then(|ac| ac.as_bool()).unwrap_or(false));
    

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
            TabType::Saved => json!({
                "id": editing.get("id").and_then(|i| i.as_str()).unwrap_or(""),
                "title": title.read(),
                "content": content.read(),
                "kind": kind.read(),
                "tags": editing.get("tags").unwrap_or(&json!([])),
                "source_url": url.read(),
            }),
            TabType::MCP => json!({
                "id": editing.get("id").and_then(|i| i.as_str()).unwrap_or(""),
                "name": name.read(),
                "transport": transport.read(),
                "command": command.read(),
                "url": url.read(),
                "enabled": enabled.read(),
                "auto_connect": auto_connect.read(),
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

    let modal_title = match (props.tab, is_editing) {
        (TabType::Memory, true) => "编辑记忆",
        (TabType::Memory, false) => "新建记忆",
        (TabType::Preset, true) => "编辑预设",
        (TabType::Preset, false) => "新建预设",
        (TabType::Skill, true) => "编辑技能",
        (TabType::Skill, false) => "新建技能",
        (TabType::Project, true) => "编辑项目",
        (TabType::Project, false) => "新建项目",
        (TabType::Saved, true) => "编辑保存项",
        (TabType::Saved, false) => "新建保存项",
        (TabType::MCP, true) => "编辑MCP服务器",
        (TabType::MCP, false) => "新建MCP服务器",
        _ => "新建",
    };

    rsx! {
        div {
            class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
            onclick: move |_| props.on_close.call(()),
            onkeydown: handle_keydown,
            tabindex: "0",
            div {
                class: "bg-[var(--ds-bg)] border border-[var(--ds-border)] rounded-lg shadow-xl w-full max-w-lg p-4",
                onclick: move |e| e.stop_propagation(),
                div { class: "flex items-center justify-between mb-4",
                    h3 { class: "text-base font-semibold text-[var(--ds-text)]", "{modal_title}" }
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
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                        onmounted: move |md| {
                                            md.get().ok().map(|el| el.focus());
                                        },
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
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                        onmounted: move |md| {
                                            md.get().ok().map(|el| el.focus());
                                        },
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
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                        onmounted: move |md| {
                                            md.get().ok().map(|el| el.focus());
                                        },
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
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                        onmounted: move |md| {
                                            md.get().ok().map(|el| el.focus());
                                        },
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
                        TabType::Saved => {
                            rsx! {
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "标题" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: title.read().clone(),
                                        oninput: move |e| title.write(e.value()),
                                        onmounted: move |md| {
                                            md.get().ok().map(|el| el.focus());
                                        },
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "类型" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: kind.read().clone(),
                                        oninput: move |e| kind.write(e.value()),
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
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "来源URL" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: url.read().clone(),
                                        oninput: move |e| url.write(e.value()),
                                    }
                                }
                            }
                        }
                        TabType::MCP => {
                            rsx! {
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "名称" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: name.read().clone(),
                                        oninput: move |e| name.write(e.value()),
                                        onmounted: move |md| {
                                            md.get().ok().map(|el| el.focus());
                                        },
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "传输方式" }
                                    select {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: transport.read().clone(),
                                        onchange: move |e| transport.write(e.value()),
                                        option { value: "stdio", "标准输入输出" }
                                        option { value: "tcp", "TCP连接" }
                                        option { value: "ws", "WebSocket" }
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "命令" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: command.read().clone(),
                                        oninput: move |e| command.write(e.value()),
                                    }
                                }
                                div {
                                    label { class: "block text-xs font-medium text-[var(--ds-text-secondary)] mb-1", "URL" }
                                    input {
                                        class: "w-full min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-card)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                                        value: url.read().clone(),
                                        oninput: move |e| url.write(e.value()),
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
                                        checked: auto_connect.read(),
                                        onchange: move |e| auto_connect.write(e.checked()),
                                    }
                                    label { class: "text-xs text-[var(--ds-text-secondary)]", "自动连接" }
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