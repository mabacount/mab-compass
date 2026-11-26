pub mod db;
pub mod ecosystem_core;
pub mod file_extractor;
pub mod llm;
pub mod mab_focus_sync;
pub mod mcp_tools;
pub mod method_engine;
pub mod models;
pub mod projector;
pub mod vault_manager;

fn merge_written_artifacts(
    mcp_arts: &[models::WrittenArtifactRef],
    env_arts: &[models::WrittenArtifactRef],
    extra: Vec<models::WrittenArtifactRef>,
) -> Vec<models::WrittenArtifactRef> {
    let mut out: Vec<models::WrittenArtifactRef> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for art in mcp_arts.iter().chain(env_arts.iter()).chain(extra.iter()) {
        if art.title.trim().is_empty() {
            continue;
        }
        let key = format!("{}::{}", art.artifact_type, art.entity_id);
        if seen.insert(key) {
            out.push(art.clone());
        }
    }
    out
}

fn resolve_chat_session_project_id(
    conn: &Connection,
    session_id: Option<&str>,
    fallback_pid: &str,
) -> String {
    if let Some(sid) = session_id.map(str::trim).filter(|s| !s.is_empty()) {
        if let Ok(pid) = conn.query_row(
            "SELECT project_id FROM chat_sessions WHERE id = ?1",
            [sid],
            |r| r.get::<_, String>(0),
        ) {
            return pid;
        }
    }
    if get_active_chat_scope(conn) == "GLOBAL" {
        return GLOBAL_CHAT_PROJECT_ID.to_string();
    }
    fallback_pid.to_string()
}

fn resolve_chat_archive_workspace(workspace_path: &str, chat_pid: &str) -> String {
    if chat_pid == GLOBAL_CHAT_PROJECT_ID {
        let p = std::path::Path::new(workspace_path);
        if let Some(parent) = p.parent() {
            return parent.join("_global_compass_chats").to_string_lossy().to_string();
        }
        return format!("{}_global", workspace_path);
    }
    workspace_path.to_string()
}

fn resolve_project_id_by_ref(projects: &[Project], raw_ref: Option<&str>) -> Option<(String, String)> {
    let raw = raw_ref.map(str::trim).filter(|s| {
        !s.is_empty() && !s.eq_ignore_ascii_case(GLOBAL_CHAT_PROJECT_ID)
    })?;
    if let Some(p) = projects
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(raw) || p.name.trim().eq_ignore_ascii_case(raw))
    {
        return Some((p.id.clone(), p.name.clone()));
    }
    let raw_lower = raw.to_lowercase();
    let partial: Vec<&Project> = projects
        .iter()
        .filter(|p| {
            p.id.to_lowercase().contains(&raw_lower) || p.name.to_lowercase().contains(&raw_lower)
        })
        .collect();
    if partial.len() == 1 {
        return Some((partial[0].id.clone(), partial[0].name.clone()));
    }
    None
}

fn sync_touched_project_projections(
    conn: &Connection,
    primary_dash: &DashboardState,
    artifacts: &[WrittenArtifactRef],
    meeting_title: Option<&str>,
    meeting_summary: Option<&str>,
) -> Result<(), String> {
    let mut synced: std::collections::HashSet<String> = std::collections::HashSet::new();
    sync_all_project_projections(primary_dash, meeting_title, meeting_summary)?;
    synced.insert(primary_dash.active_project.id.clone());

    for art in artifacts {
        if let Some(ref pid) = art.project_id {
            if !pid.is_empty()
                && pid != GLOBAL_CHAT_PROJECT_ID
                && synced.insert(pid.clone())
            {
                if let Ok(other_dash) = get_dashboard_state_for_project(conn, Some(pid)) {
                    let _ = sync_all_project_projections(&other_dash, None, None);
                }
            }
        }
    }
    Ok(())
}

fn apply_envelope_cross_project(
    conn: &mut Connection,
    projects: &[Project],
    fallback_pid: &str,
    fallback_pname: &str,
    raw_text: &str,
    archived_path: Option<&str>,
    source_origin: &str,
    envelope: &CopilotEnvelope,
    raw_event_id: Option<i64>,
) -> Result<(String, Vec<WrittenArtifactRef>), String> {
    let mut grouped: std::collections::BTreeMap<String, (String, Vec<AtomicMutation>)> =
        std::collections::BTreeMap::new();

    for m in &envelope.mutations {
        let entity_owner = if m.project_id.is_none() {
            m.target_id.as_deref().and_then(|tid| {
                conn.query_row(
                    "SELECT project_id FROM work_entities WHERE id = ?1 LIMIT 1",
                    [tid.trim()],
                    |r| r.get::<_, String>(0),
                )
                .ok()
                .and_then(|pid| {
                    projects
                        .iter()
                        .find(|p| p.id == pid)
                        .map(|p| (p.id.clone(), p.name.clone()))
                })
            })
        } else {
            None
        };
        let (target_pid, target_pname) = resolve_project_id_by_ref(projects, m.project_id.as_deref())
            .or(entity_owner)
            .unwrap_or_else(|| (fallback_pid.to_string(), fallback_pname.to_string()));
        grouped
            .entry(target_pid)
            .or_insert_with(|| (target_pname, Vec::new()))
            .1
            .push(m.clone());
    }

    if grouped.is_empty() {
        let (badge, mut arts) = apply_copilot_envelope_with_artifacts(
            conn,
            fallback_pid,
            raw_text,
            archived_path,
            source_origin,
            envelope,
            raw_event_id,
        )
        .map_err(|e| e.to_string())?;
        for a in &mut arts {
            if a.project_id.is_none() {
                a.project_id = Some(fallback_pid.to_string());
                a.project_name = Some(fallback_pname.to_string());
            }
        }
        return Ok((badge, arts));
    }

    let mut badges: Vec<String> = Vec::new();
    let mut all_arts: Vec<WrittenArtifactRef> = Vec::new();
    for (idx, (target_pid, (target_pname, muts))) in grouped.into_iter().enumerate() {
        let sub_env = CopilotEnvelope {
            conversational_reply: envelope.conversational_reply.clone(),
            mutations: muts,
            method_playbooks: if idx == 0 {
                envelope.method_playbooks.clone()
            } else {
                Vec::new()
            },
            ..Default::default()
        };
        let (badge, mut arts) = apply_copilot_envelope_with_artifacts(
            conn,
            &target_pid,
            raw_text,
            archived_path,
            source_origin,
            &sub_env,
            if idx == 0 { raw_event_id } else { None },
        )
        .map_err(|e| e.to_string())?;
        if !badge.is_empty() && badge != "بدون تغییر گراف" {
            badges.push(badge);
        }
        for a in &mut arts {
            if a.project_id.is_none() {
                a.project_id = Some(target_pid.clone());
                a.project_name = Some(target_pname.clone());
            }
        }
        all_arts.extend(arts);
    }

    let final_badge = if badges.is_empty() {
        "بدون تغییر گراف".to_string()
    } else {
        badges.join(" • ")
    };
    Ok((final_badge, all_arts))
}

fn format_turn_badge(mcp_ops: &[String], envelope_badge: &str, transport_label: &str) -> String {
    if !mcp_ops.is_empty() {
        let mcp_part = format!("🛠 MCP: {}", mcp_ops.join(" • "));
        if envelope_badge.trim().is_empty() || envelope_badge == "بدون تغییر گراف" {
            mcp_part
        } else {
            format!("{} • {}", mcp_part, envelope_badge)
        }
    } else if transport_label == "PROSE_FALLBACK" && envelope_badge == "بدون تغییر گراف" {
        "💬 پاسخ مستقیم مدل".to_string()
    } else {
        envelope_badge.to_string()
    }
}

use db::{queries::*, schema::init_database};
use file_extractor::extract_text_from_base64;
use llm::{prompts::*, router::*};
use mab_focus_sync::synchronize_with_mab_focus;
use models::*;
use projector::{excel_writer::export_client_xlsx, markdown_writer::sync_all_project_projections};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use vault_manager::{archive_onboarding_document, archive_raw_user_input};

pub struct AppState {
    pub db: Mutex<Connection>,
    pub app_data_dir: PathBuf,
    pub last_core_revision: Mutex<i64>,
}

fn derive_title_from_content(raw_title: &str, content: &str, fallback: &str) -> String {
    let t = raw_title.trim();
    if !t.is_empty() {
        return t.to_string();
    }
    let first_line = content
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if first_line.is_empty() {
        return fallback.to_string();
    }
    let clipped: String = first_line.chars().take(48).collect();
    if first_line.chars().count() > 48 {
        format!("{}…", clipped.trim())
    } else {
        clipped
    }
}

fn format_vault_user_summary(
    effective_title: &str,
    original_filename: Option<&str>,
    detected_format: Option<&str>,
    extracted_text: &str,
) -> String {
    let char_count = extracted_text.chars().count();
    if let (Some(fname), Some(fmt)) = (original_filename, detected_format) {
        format!(
            "📄 فایل «{}» ارسال و استخراج شد ({} • {} کاراکتر).",
            fname, fmt, char_count
        )
    } else if char_count <= 500 {
        format!("📝 متن ارسالی «{}»:\n{}", effective_title, extracted_text.trim())
    } else {
        let preview: String = extracted_text.trim().chars().take(420).collect();
        format!(
            "📝 متن ارسالی «{}» ({} کاراکتر):\n{}…",
            effective_title, char_count, preview
        )
    }
}

#[tauri::command]
fn cmd_get_db_data_version(state: State<'_, AppState>) -> Result<i64, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut core_rev: i64 = 0;
    if let Some(core_path) = ecosystem_core::resolve_default_core_db_path() {
        let rev = ecosystem_core::get_core_revision_at_path(&core_path);
        if rev > 0 {
            let mut last = state
                .last_core_revision
                .lock()
                .map_err(|e| e.to_string())?;
            if rev != *last {
                if let Ok(settings) = get_settings(&conn) {
                    if settings.auto_sync_mab_focus {
                        let _ = synchronize_with_mab_focus(&mut conn);
                    } else {
                        let _ = ecosystem_core::reconcile_compass_with_core_at_path(
                            &mut conn,
                            &core_path,
                            &settings.vault_root_path,
                        );
                    }
                }
                let updated_rev = ecosystem_core::get_core_revision_at_path(&core_path);
                *last = updated_rev;
                core_rev = updated_rev;
            } else {
                core_rev = rev;
            }
        }
    }
    let local_ver: i64 = conn
        .query_row("PRAGMA data_version", [], |r| r.get::<_, i64>(0))
        .map_err(|e| e.to_string())?;
    Ok(local_ver + core_rev)
}

#[tauri::command]
fn cmd_get_dashboard(state: State<'_, AppState>) -> Result<DashboardState, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    if let Ok(s) = get_settings(&conn) {
        if s.auto_sync_mab_focus {
            let _ = synchronize_with_mab_focus(&mut conn);
        }
    }
    if let Some(core_path) = ecosystem_core::resolve_default_core_db_path() {
        if let Ok(mut last) = state.last_core_revision.lock() {
            *last = ecosystem_core::get_core_revision_at_path(&core_path);
        }
    }
    let decoupled_done: i64 = conn
        .query_row(
            "SELECT COALESCE(chat_decoupled_v1, 0) FROM app_settings WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if decoupled_done == 0 {
        set_active_chat_scope(&conn, "GLOBAL");
        let _ = conn.execute("UPDATE app_settings SET chat_decoupled_v1 = 1 WHERE id = 1", []);
    }
    if get_active_chat_scope(&conn) == "GLOBAL" {
        let _ = ensure_active_chat_session(&conn, GLOBAL_CHAT_PROJECT_ID, "COPILOT");
    }
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    Ok(dash)
}

#[tauri::command]
fn cmd_switch_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE projects SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END",
        [&project_id],
    )
    .map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    Ok(dash)
}

#[tauri::command]
fn cmd_create_project(
    state: State<'_, AppState>,
    id_code: String,
    name: String,
    parent_id: Option<String>,
    realm: Option<String>,
    color: Option<String>,
    lifecycle_status: Option<String>,
) -> Result<DashboardState, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let created = create_new_project_with_options(
        &conn,
        &id_code,
        &name,
        parent_id.as_deref(),
        realm.as_deref(),
        color.as_deref(),
        lifecycle_status.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    if let Some(core_path) = ecosystem_core::resolve_default_core_db_path() {
        let _ = ecosystem_core::push_compass_project_upsert(&conn, &core_path, &created);
        if let Ok(mut last) = state.last_core_revision.lock() {
            *last = ecosystem_core::get_core_revision_at_path(&core_path);
        }
    }
    let _ = synchronize_with_mab_focus(&mut conn);
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    Ok(dash)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn cmd_update_project(
    state: State<'_, AppState>,
    project_id: String,
    name: Option<String>,
    parent_id: Option<String>,
    clear_parent: Option<bool>,
    realm: Option<String>,
    lifecycle_status: Option<String>,
    color: Option<String>,
    daily_quota_minutes: Option<i32>,
    weekly_quota_minutes: Option<i32>,
    monthly_quota_minutes: Option<i32>,
    hourly_rate: Option<f64>,
) -> Result<DashboardState, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let updated = update_project_details(
        &conn,
        &project_id,
        name.as_deref(),
        parent_id.as_deref(),
        clear_parent.unwrap_or(false),
        realm.as_deref(),
        lifecycle_status.as_deref(),
        color.as_deref(),
        daily_quota_minutes,
        weekly_quota_minutes,
        monthly_quota_minutes,
        hourly_rate,
    )
    .map_err(|e| e.to_string())?;

    if let Some(core_path) = ecosystem_core::resolve_default_core_db_path() {
        let _ = ecosystem_core::push_compass_project_upsert(&conn, &core_path, &updated);
        if let Ok(mut last) = state.last_core_revision.lock() {
            *last = ecosystem_core::get_core_revision_at_path(&core_path);
        }
    }
    let _ = synchronize_with_mab_focus(&mut conn);
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    Ok(dash)
}

#[tauri::command]
fn cmd_merge_projects(
    state: State<'_, AppState>,
    source_id: String,
    target_id: String,
) -> Result<DashboardState, String> {
    if source_id.trim() == target_id.trim() {
        return Err("پروژه مبدا و مقصد نمی‌توانند یکسان باشند.".into());
    }
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let projects = ecosystem_core::load_compass_projects(&conn)?;
    let src = projects
        .iter()
        .find(|p| p.id == source_id)
        .cloned()
        .ok_or_else(|| "پروژه مبدا یافت نشد.".to_string())?;
    let tgt = projects
        .iter()
        .find(|p| p.id == target_id)
        .cloned()
        .ok_or_else(|| "پروژه مقصد یافت نشد.".to_string())?;

    let src_core_id = src
        .external_focus_id
        .clone()
        .unwrap_or_else(|| src.id.clone());
    let tgt_core_id = tgt
        .external_focus_id
        .clone()
        .unwrap_or_else(|| tgt.id.clone());

    ecosystem_core::merge_compass_projects_atomic(&mut conn, &src.id, &tgt.id)?;

    if let Some(core_path) = ecosystem_core::resolve_default_core_db_path() {
        let _ = ecosystem_core::push_compass_project_merge(
            &conn,
            &core_path,
            &src_core_id,
            &tgt_core_id,
            &src.id,
            &src.name,
        );
        if let Ok(mut last) = state.last_core_revision.lock() {
            *last = ecosystem_core::get_core_revision_at_path(&core_path);
        }
    }
    let _ = synchronize_with_mab_focus(&mut conn);
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    Ok(dash)
}

#[tauri::command]
fn cmd_delete_project(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<DashboardState, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let deleted_core_ids = delete_project_local(&mut conn, &project_id).map_err(|e| e.to_string())?;
    if deleted_core_ids.is_empty() {
        return Err("حداقل یک پروژه باید در اکوسیستم باقی بماند.".into());
    }
    if let Some(core_path) = ecosystem_core::resolve_default_core_db_path() {
        let _ = ecosystem_core::push_compass_project_delete(&core_path, &deleted_core_ids);
        if let Ok(mut last) = state.last_core_revision.lock() {
            *last = ecosystem_core::get_core_revision_at_path(&core_path);
        }
    }
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    Ok(dash)
}

#[tauri::command]
fn cmd_create_chat_session(
    state: State<'_, AppState>,
    project_id: Option<String>,
    channel: String,
    title: Option<String>,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    create_chat_session(&conn, project_id.as_deref(), &channel, title.as_deref())
        .map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_switch_chat_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    switch_chat_session(&conn, &session_id).map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    Ok(dash)
}

#[tauri::command]
fn cmd_delete_chat_session(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    delete_chat_session(&conn, &session_id).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_commit_user_chat_message(
    state: State<'_, AppState>,
    channel: String,
    session_id: Option<String>,
    message_text: String,
) -> Result<CommittedChatTurn, String> {
    if message_text.trim().is_empty() {
        return Err("پیام خالی است.".into());
    }
    let norm_ch = normalize_chat_channel(&channel);
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let d = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let pid = resolve_chat_session_project_id(&conn, session_id.as_deref(), &d.active_project.id);
    let workspace_path = resolve_chat_archive_workspace(&d.active_project.workspace_path, &pid);

    let (arch_tag, source_type) = if norm_ch == "VAULT" {
        ("vault_dialogue", "VAULT_CHAT")
    } else {
        ("user_chat", "USER_CHAT")
    };
    let archived_path = archive_raw_user_input(&workspace_path, arch_tag, &message_text)?;
    let (resolved_sid, raw_event_id, user_message_id) = commit_user_chat_turn(
        &conn,
        &pid,
        session_id.as_deref(),
        norm_ch,
        &message_text,
        Some(&archived_path),
        source_type,
    )
    .map_err(|e| e.to_string())?;

    let preview: String = message_text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or(&message_text)
        .chars()
        .take(72)
        .collect();
    let _ = insert_pending_queue_item(
        &conn,
        PendingQueueInput {
            project_id: &pid,
            session_id: &resolved_sid,
            channel: norm_ch,
            user_message_id: Some(user_message_id),
            system_message_id: None,
            knowledge_doc_id: None,
            raw_event_id: Some(raw_event_id),
            title_preview: &preview,
            source_type,
            archived_path: Some(&archived_path),
            last_error: None,
        },
    );

    let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    Ok(CommittedChatTurn {
        dashboard: updated,
        session_id: resolved_sid,
        raw_event_id,
        user_message_id,
        archived_path,
    })
}

#[tauri::command]
async fn cmd_run_assistant_turn(
    state: State<'_, AppState>,
    channel: String,
    session_id: String,
    message_text: String,
    archived_path: String,
    raw_event_id: Option<i64>,
    user_message_id: i64,
) -> Result<AssistantTurnOutcome, String> {
    let norm_ch = normalize_chat_channel(&channel).to_string();
    let (
        chat_pid,
        active_pid,
        active_pname,
        all_projects,
        settings,
        open_tasks,
        active_decisions,
        active_sops,
        knowledge_docs,
        methods_playbooks,
        copilot_history,
        vault_history,
        fts_hits,
        portfolio_snapshot,
    ) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let d = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
        let chat_pid = resolve_chat_session_project_id(&conn, Some(&session_id), &d.active_project.id);
        let fts_pid = if chat_pid == GLOBAL_CHAT_PROJECT_ID {
            d.active_project.id.as_str()
        } else {
            chat_pid.as_str()
        };
        let hits = search_raw_events_fts(&conn, fts_pid, &message_text)
            .unwrap_or_default();
        let portfolio_snap = build_cross_project_portfolio_snapshot(&conn);
        let filtered_copilot: Vec<ChatMessage> = d
            .chat_history
            .iter()
            .filter(|m| m.id != user_message_id)
            .cloned()
            .collect();
        let filtered_vault: Vec<ChatMessage> = d
            .vault_chat_history
            .iter()
            .filter(|m| m.id != user_message_id)
            .cloned()
            .collect();
        (
            chat_pid,
            d.active_project.id.clone(),
            d.active_project.name.clone(),
            d.projects.clone(),
            d.settings.clone(),
            d.open_tasks.clone(),
            d.active_decisions.clone(),
            d.active_sops.clone(),
            d.knowledge_docs.clone(),
            d.methods_playbooks.clone(),
            filtered_copilot,
            filtered_vault,
            hits,
            portfolio_snap,
        )
    };

    let base_prompt = if norm_ch == "VAULT" {
        build_vault_chat_prompt(
            &chat_pid,
            &message_text,
            &vault_history,
            &open_tasks,
            &active_decisions,
            &active_sops,
            &knowledge_docs,
            &fts_hits,
        )
    } else {
        build_copilot_chat_prompt(
            &chat_pid,
            &message_text,
            &copilot_history,
            &vault_history,
            &open_tasks,
            &active_decisions,
            &active_sops,
            &knowledge_docs,
            &fts_hits,
        )
    };
    let enriched_base = if portfolio_snapshot.trim().is_empty() {
        base_prompt
    } else {
        format!("{base_prompt}\n\n{portfolio_snapshot}")
    };
    let prompt = append_cross_project_routing_instructions(enriched_base, &chat_pid, &all_projects);

    let (mcp_sys_prompt, mcp_user_prompt) = build_mcp_tool_turn_prompts_with_portfolio(
        &chat_pid,
        &norm_ch,
        &message_text,
        if norm_ch == "VAULT" { &vault_history } else { &copilot_history },
        &open_tasks,
        &active_decisions,
        &active_sops,
        &knowledge_docs,
        &methods_playbooks,
        &fts_hits,
        None,
        false,
        &all_projects,
        &portfolio_snapshot,
    );

    let llm_cfg = LlmConfig::from(&settings);
    let source_origin = if norm_ch == "VAULT" {
        "VAULT_CHAT"
    } else {
        "USER_CHAT"
    };
    match run_copilot_turn(
        &state.db,
        state.app_data_dir.clone(),
        &llm_cfg,
        &chat_pid,
        raw_event_id,
        source_origin,
        &mcp_sys_prompt,
        &mcp_user_prompt,
        &prompt,
    )
    .await
    {
        Ok(turn) => {
            let envelope = turn.envelope;
            let mut conn = state.db.lock().map_err(|e| e.to_string())?;
            let (env_badge, env_artifacts) =
                if !envelope.mutations.is_empty()
                    || !envelope.method_playbooks.is_empty()
                    || turn.mcp_ops_executed.is_empty()
                {
                    apply_envelope_cross_project(
                        &mut conn,
                        &all_projects,
                        &active_pid,
                        &active_pname,
                        &message_text,
                        Some(&archived_path),
                        source_origin,
                        &envelope,
                        raw_event_id,
                    )?
                } else {
                    ("بدون تغییر گراف".to_string(), Vec::new())
                };
            let badge = format_turn_badge(&turn.mcp_ops_executed, &env_badge, &turn.transport_label);

            let mut extra_artifacts: Vec<WrittenArtifactRef> = Vec::new();
            if let (Some(mtitle), Some(mbody)) = (
                envelope.meeting_title.as_deref().map(str::trim).filter(|s| !s.is_empty()),
                envelope.meeting_summary_markdown.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            ) {
                extra_artifacts.push(WrittenArtifactRef {
                    artifact_type: "MEETING_MINUTES".to_string(),
                    entity_id: format!("MTG-{}", mtitle.chars().take(24).collect::<String>()),
                    title: mtitle.to_string(),
                    summary: mbody.chars().take(160).collect(),
                    file_path: None,
                    action: "CREATED".to_string(),
                    version: None,
                    project_id: Some(active_pid.clone()),
                    project_name: Some(active_pname.clone()),
                });
            }
            let written_artifacts =
                merge_written_artifacts(&turn.mcp_written_artifacts, &env_artifacts, extra_artifacts);

            append_assistant_chat_message_with_artifacts(
                &conn,
                &chat_pid,
                &session_id,
                &norm_ch,
                "ASSISTANT",
                &envelope.conversational_reply,
                Some(&archived_path),
                &badge,
                &envelope.clarifying_questions,
                &written_artifacts,
            )
            .map_err(|e| e.to_string())?;

            let _ = complete_pending_items_for_user_msg(&conn, user_message_id, None);

            let updated_dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            sync_touched_project_projections(
                &conn,
                &updated_dash,
                &written_artifacts,
                envelope.meeting_title.as_deref(),
                envelope.meeting_summary_markdown.as_deref(),
            )?;

            Ok(AssistantTurnOutcome {
                dashboard: updated_dash,
                assistant_error: None,
            })
        }
        Err(err) => {
            let notice = format!(
                "⚠️ پیام شما با موفقیت در حافظه و آرشیو پروژه ذخیره شد، اما پاسخ هوش مصنوعی دریافت نشد:\n{}",
                err
            );
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let sys_id = append_assistant_chat_message(
                &conn,
                &chat_pid,
                &session_id,
                &norm_ch,
                "SYSTEM",
                &notice,
                Some(&archived_path),
                "ثبت در حافظه • خطای موتور هوش مصنوعی",
                &[],
            )
            .ok();
            let preview: String = message_text
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or(&message_text)
                .chars()
                .take(72)
                .collect();
            let _ = enqueue_pending_ai_item(
                &conn,
                &chat_pid,
                &session_id,
                &norm_ch,
                Some(user_message_id),
                sys_id,
                None,
                raw_event_id,
                &preview,
                source_origin,
                Some(&archived_path),
                Some(&err),
            );
            let updated_dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            Ok(AssistantTurnOutcome {
                dashboard: updated_dash,
                assistant_error: Some(err),
            })
        }
    }
}

fn strip_raw_input_frontmatter(raw: &str) -> String {
    let normalized = raw.replace("\r\n", "\n");
    if let Some(after_start) = normalized.strip_prefix("---\n") {
        if let Some(end_idx) = after_start.find("\n---\n") {
            return after_start[end_idx + "\n---\n".len()..].trim().to_string();
        }
    }
    normalized.trim().to_string()
}

fn strip_knowledge_archive_header(raw: &str) -> String {
    let normalized = raw.replace("\r\n", "\n");
    let marker = "## متن کامل سند اصلی\n";
    if let Some(idx) = normalized.find(marker) {
        return normalized[idx + marker.len()..].trim().to_string();
    }
    normalized.trim().to_string()
}

#[tauri::command]
async fn cmd_retry_system_message(
    state: State<'_, AppState>,
    system_message_id: i64,
) -> Result<AssistantTurnOutcome, String> {
    let (
        ctx,
        active_pid,
        active_pname,
        all_projects,
        portfolio_snapshot,
        settings,
        open_tasks,
        active_decisions,
        active_sops,
        knowledge_docs,
        methods_playbooks,
        copilot_history,
        vault_history,
        fts_hits,
        recovered_text,
    ) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let ctx = get_retry_context_for_system_message(&conn, system_message_id)
            .map_err(|e| format!("پیام کاربر متناظر برای ارسال مجدد یافت نشد: {}", e))?;

        if !ctx.session_id.is_empty() {
            let _ = switch_chat_session(&conn, &ctx.session_id);
        }
        let d = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
        let portfolio_snap = build_cross_project_portfolio_snapshot(&conn);

        let recovered_text = ctx
            .user_archived_path
            .as_deref()
            .and_then(|p| fs::read_to_string(p).ok())
            .map(|content| strip_raw_input_frontmatter(&content))
            .filter(|s| !s.is_empty())
            .or_else(|| ctx.raw_content.clone().filter(|s| !s.trim().is_empty()))
            .unwrap_or_else(|| ctx.user_message_text.clone());

        let fts_pid = if ctx.project_id == GLOBAL_CHAT_PROJECT_ID {
            d.active_project.id.as_str()
        } else {
            ctx.project_id.as_str()
        };
        let hits = search_raw_events_fts(&conn, fts_pid, &recovered_text)
            .unwrap_or_default();

        let filtered_copilot: Vec<ChatMessage> = d
            .chat_history
            .iter()
            .filter(|m| m.id != ctx.user_message_id && m.id != system_message_id)
            .cloned()
            .collect();
        let filtered_vault: Vec<ChatMessage> = d
            .vault_chat_history
            .iter()
            .filter(|m| m.id != ctx.user_message_id && m.id != system_message_id)
            .cloned()
            .collect();

        (
            ctx,
            d.active_project.id.clone(),
            d.active_project.name.clone(),
            d.projects.clone(),
            portfolio_snap,
            d.settings.clone(),
            d.open_tasks.clone(),
            d.active_decisions.clone(),
            d.active_sops.clone(),
            d.knowledge_docs.clone(),
            d.methods_playbooks.clone(),
            filtered_copilot,
            filtered_vault,
            hits,
            recovered_text,
        )
    };

    let is_vault_doc = ctx.channel == "VAULT"
        && ctx.raw_source_type.as_deref() == Some("DOC_ONBOARDING");

    let base_prompt = if is_vault_doc {
        let derived_title = derive_title_from_content("", &recovered_text, "سند بازیابی‌شده");
        build_document_onboarding_prompt(
            &ctx.project_id,
            &derived_title,
            "ONBOARDING_FILE",
            "recovered_doc.md",
            &recovered_text,
            &vault_history,
            &open_tasks,
            &active_decisions,
        )
    } else if ctx.channel == "VAULT" {
        build_vault_chat_prompt(
            &ctx.project_id,
            &recovered_text,
            &vault_history,
            &open_tasks,
            &active_decisions,
            &active_sops,
            &knowledge_docs,
            &fts_hits,
        )
    } else {
        build_copilot_chat_prompt(
            &ctx.project_id,
            &recovered_text,
            &copilot_history,
            &vault_history,
            &open_tasks,
            &active_decisions,
            &active_sops,
            &knowledge_docs,
            &fts_hits,
        )
    };
    let enriched_base = if portfolio_snapshot.trim().is_empty() {
        base_prompt
    } else {
        format!("{base_prompt}\n\n{portfolio_snapshot}")
    };
    let prompt = append_cross_project_routing_instructions(enriched_base, &ctx.project_id, &all_projects);

    let doc_meta_owned = if is_vault_doc {
        Some((
            derive_title_from_content("", &recovered_text, "سند بازیابی‌شده"),
            "ONBOARDING_FILE".to_string(),
            "recovered_doc.md".to_string(),
        ))
    } else {
        None
    };
    let doc_meta_ref = doc_meta_owned
        .as_ref()
        .map(|(t, dt, f)| (t.as_str(), dt.as_str(), f.as_str()));

    let (mcp_sys_prompt, mcp_user_prompt) = build_mcp_tool_turn_prompts_with_portfolio(
        &ctx.project_id,
        &ctx.channel,
        &recovered_text,
        if ctx.channel == "VAULT" { &vault_history } else { &copilot_history },
        &open_tasks,
        &active_decisions,
        &active_sops,
        &knowledge_docs,
        &methods_playbooks,
        &fts_hits,
        doc_meta_ref,
        false,
        &all_projects,
        &portfolio_snapshot,
    );

    let llm_cfg = LlmConfig::from(&settings);
    let source_origin = ctx.raw_source_type.as_deref().unwrap_or(
        if ctx.channel == "VAULT" {
            "VAULT_CHAT"
        } else {
            "USER_CHAT"
        },
    );
    match run_copilot_turn(
        &state.db,
        state.app_data_dir.clone(),
        &llm_cfg,
        &ctx.project_id,
        ctx.raw_event_id,
        source_origin,
        &mcp_sys_prompt,
        &mcp_user_prompt,
        &prompt,
    )
    .await
    {
        Ok(turn) => {
            let envelope = turn.envelope;
            let mut conn = state.db.lock().map_err(|e| e.to_string())?;
            let (env_badge, env_artifacts) =
                if !envelope.mutations.is_empty()
                    || !envelope.method_playbooks.is_empty()
                    || turn.mcp_ops_executed.is_empty()
                {
                    apply_envelope_cross_project(
                        &mut conn,
                        &all_projects,
                        &active_pid,
                        &active_pname,
                        &recovered_text,
                        ctx.user_archived_path.as_deref(),
                        source_origin,
                        &envelope,
                        ctx.raw_event_id,
                    )?
                } else {
                    ("بدون تغییر گراف".to_string(), Vec::new())
                };
            let badge = format_turn_badge(&turn.mcp_ops_executed, &env_badge, &turn.transport_label);

            let items_count = (envelope.mutations.len() + turn.mcp_ops_executed.len()) as i32;
            let matched_doc_id = find_unanalyzed_knowledge_doc_id(
                &conn,
                &active_pid,
                ctx.system_archived_path.as_deref(),
                &ctx.user_message_text,
            );
            let mut extra_artifacts: Vec<WrittenArtifactRef> = Vec::new();
            if let Some(doc_id) = matched_doc_id {
                let new_summary = envelope
                    .executive_doc_summary
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| envelope.conversational_reply.clone());
                let doc_title_opt = get_knowledge_document_by_id(&conn, doc_id).map(|d| d.title);
                let new_kp = get_knowledge_document_by_id(&conn, doc_id).and_then(|doc| {
                    let dash_proj = get_dashboard_state_for_project(&conn, Some(&active_pid)).ok()?;
                    archive_onboarding_document(
                        &dash_proj.active_project.workspace_path,
                        &doc.title,
                        &recovered_text,
                        &new_summary,
                    )
                    .ok()
                });
                let _ = update_knowledge_doc_analysis(
                    &conn,
                    doc_id,
                    &new_summary,
                    items_count,
                    new_kp.as_deref(),
                );
                extra_artifacts.push(WrittenArtifactRef {
                    artifact_type: "KNOWLEDGE_DOC".to_string(),
                    entity_id: doc_id.to_string(),
                    title: doc_title_opt.unwrap_or_else(|| "سند آرشیو دانش".to_string()),
                    summary: new_summary.chars().take(180).collect(),
                    file_path: new_kp,
                    action: "UPDATED".to_string(),
                    version: None,
                    project_id: Some(active_pid.clone()),
                    project_name: Some(active_pname.clone()),
                });
            }

            let _ = delete_system_chat_message(&conn, system_message_id);
            let written_artifacts =
                merge_written_artifacts(&turn.mcp_written_artifacts, &env_artifacts, extra_artifacts);

            append_assistant_chat_message_with_artifacts(
                &conn,
                &ctx.project_id,
                &ctx.session_id,
                &ctx.channel,
                "ASSISTANT",
                &envelope.conversational_reply,
                ctx.user_archived_path.as_deref(),
                &badge,
                &envelope.clarifying_questions,
                &written_artifacts,
            )
            .map_err(|e| e.to_string())?;

            let _ = complete_pending_items_for_user_msg(&conn, ctx.user_message_id, matched_doc_id);

            let updated_dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            sync_touched_project_projections(
                &conn,
                &updated_dash,
                &written_artifacts,
                envelope.meeting_title.as_deref(),
                envelope.meeting_summary_markdown.as_deref(),
            )?;

            Ok(AssistantTurnOutcome {
                dashboard: updated_dash,
                assistant_error: None,
            })
        }
        Err(err) => {
            let notice = format!(
                "⚠️ پیام شما در حافظه و آرشیو پروژه ذخیره است، اما در ارسال مجدد به هوش مصنوعی خطا رخ داد:\n{}",
                err
            );
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let _ = update_system_chat_message(&conn, system_message_id, &notice);
            let matched_doc_id = find_unanalyzed_knowledge_doc_id(
                &conn,
                &ctx.project_id,
                ctx.system_archived_path.as_deref(),
                &ctx.user_message_text,
            );
            let preview: String = ctx
                .user_message_text
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty())
                .unwrap_or(&ctx.user_message_text)
                .chars()
                .take(72)
                .collect();
            let _ = enqueue_pending_ai_item(
                &conn,
                &ctx.project_id,
                &ctx.session_id,
                &ctx.channel,
                Some(ctx.user_message_id),
                Some(system_message_id),
                matched_doc_id,
                ctx.raw_event_id,
                &preview,
                source_origin,
                ctx.user_archived_path.as_deref(),
                Some(&err),
            );
            let updated_dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            Ok(AssistantTurnOutcome {
                dashboard: updated_dash,
                assistant_error: Some(err),
            })
        }
    }
}

#[tauri::command]
fn cmd_commit_user_file_message(
    state: State<'_, AppState>,
    channel: String,
    session_id: Option<String>,
    original_filename: String,
    content_base64: String,
    user_note: String,
    title: Option<String>,
    doc_type: Option<String>,
) -> Result<CommittedFileTurn, String> {
    let extracted = extract_text_from_base64(&original_filename, &content_base64)?;
    let norm_ch = normalize_chat_channel(&channel).to_string();
    let effective_title = derive_title_from_content(
        title.as_deref().unwrap_or(""),
        &original_filename,
        &original_filename,
    );
    let effective_doc_type = match doc_type.as_deref().map(str::trim) {
        Some(dt) if !dt.is_empty() && dt != "ONBOARDING_FILE" => dt.to_string(),
        _ => extracted.detected_format.clone(),
    };

    let display_user_msg = if norm_ch == "VAULT" {
        format_vault_user_summary(
            &effective_title,
            Some(&original_filename),
            Some(&extracted.detected_format),
            &extracted.text,
        )
    } else if user_note.trim().is_empty() {
        format!(
            "📎 فایل ضمیمه: «{}» ({} • {} کاراکتر)",
            original_filename,
            extracted.detected_format,
            extracted.text.chars().count()
        )
    } else {
        format!(
            "{}\n📎 ضمیمه: «{}» ({} • {} کاراکتر)",
            user_note.trim(),
            original_filename,
            extracted.detected_format,
            extracted.text.chars().count()
        )
    };

    let combined_prompt_text = if user_note.trim().is_empty() {
        format!(
            "فایل ضمیمه‌شده: {} (فرمت: {})\n\n{}",
            original_filename, extracted.detected_format, extracted.text
        )
    } else {
        format!(
            "یادداشت کاربر: {}\n\nفایل ضمیمه‌شده: {} (فرمت: {})\n\n{}",
            user_note.trim(),
            original_filename,
            extracted.detected_format,
            extracted.text
        )
    };

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let d = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let chat_pid = resolve_chat_session_project_id(&conn, session_id.as_deref(), &d.active_project.id);
    let doc_pid = d.active_project.id.clone();
    let workspace_path = resolve_chat_archive_workspace(&d.active_project.workspace_path, &chat_pid);

    let archived_path = archive_raw_user_input(
        &workspace_path,
        if norm_ch == "VAULT" { "vault_file" } else { "copilot_file" },
        &combined_prompt_text,
    )?;

    let file_source_type = if norm_ch == "VAULT" { "DOC_ONBOARDING" } else { "COPILOT_FILE" };
    let (resolved_sid, raw_event_id, user_message_id) = commit_user_chat_turn(
        &conn,
        &chat_pid,
        session_id.as_deref(),
        &norm_ch,
        &display_user_msg,
        Some(&archived_path),
        file_source_type,
    )
    .map_err(|e| e.to_string())?;

    let kdoc_id = insert_knowledge_doc_with_status(
        &conn,
        &doc_pid,
        &effective_title,
        &effective_doc_type,
        &original_filename,
        &archived_path,
        "در انتظار پردازش توسط هوش مصنوعی برای استخراج تسک‌ها، لاین‌ها و کارت‌های دانش...",
        0,
        "UNPROCESSED",
    )
    .ok();

    let file_preview = format!("📎 {}", original_filename);
    let _ = insert_pending_queue_item(
        &conn,
        PendingQueueInput {
            project_id: &chat_pid,
            session_id: &resolved_sid,
            channel: &norm_ch,
            user_message_id: Some(user_message_id),
            system_message_id: None,
            knowledge_doc_id: kdoc_id,
            raw_event_id: Some(raw_event_id),
            title_preview: &file_preview,
            source_type: file_source_type,
            archived_path: Some(&archived_path),
            last_error: None,
        },
    );

    let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    Ok(CommittedFileTurn {
        dashboard: updated,
        session_id: resolved_sid,
        raw_event_id,
        user_message_id,
        archived_path,
        extracted_text: extracted.text,
        combined_prompt_text,
        detected_format: extracted.detected_format,
        effective_title,
        effective_filename: original_filename,
        doc_type: effective_doc_type,
        knowledge_doc_id: kdoc_id,
    })
}

#[tauri::command]
fn cmd_commit_vault_text_message(
    state: State<'_, AppState>,
    session_id: Option<String>,
    title: String,
    doc_type: String,
    original_filename: String,
    content: String,
) -> Result<CommittedFileTurn, String> {
    if content.trim().is_empty() {
        return Err("محتوای ارسالی خالی است.".into());
    }
    let effective_title = derive_title_from_content(&title, &content, &original_filename);
    let effective_filename = if original_filename.trim().is_empty() {
        format!("{}.md", vault_manager::sanitize_filename(&effective_title))
    } else {
        original_filename
    };
    let display_user_msg = format_vault_user_summary(&effective_title, None, None, &content);

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let d = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let chat_pid = resolve_chat_session_project_id(&conn, session_id.as_deref(), &d.active_project.id);
    let doc_pid = d.active_project.id.clone();
    let workspace_path = resolve_chat_archive_workspace(&d.active_project.workspace_path, &chat_pid);

    let archived_path = archive_raw_user_input(&workspace_path, "vault_text", &content)?;
    let (resolved_sid, raw_event_id, user_message_id) = commit_user_chat_turn(
        &conn,
        &chat_pid,
        session_id.as_deref(),
        "VAULT",
        &display_user_msg,
        Some(&archived_path),
        "DOC_ONBOARDING",
    )
    .map_err(|e| e.to_string())?;

    let kdoc_id = insert_knowledge_doc_with_status(
        &conn,
        &doc_pid,
        &effective_title,
        &doc_type,
        &effective_filename,
        &archived_path,
        "در انتظار پردازش توسط هوش مصنوعی برای استخراج تسک‌ها، لاین‌ها و کارت‌های دانش...",
        0,
        "UNPROCESSED",
    )
    .ok();

    let text_preview = format!("📄 {}", effective_title);
    let _ = insert_pending_queue_item(
        &conn,
        PendingQueueInput {
            project_id: &chat_pid,
            session_id: &resolved_sid,
            channel: "VAULT",
            user_message_id: Some(user_message_id),
            system_message_id: None,
            knowledge_doc_id: kdoc_id,
            raw_event_id: Some(raw_event_id),
            title_preview: &text_preview,
            source_type: "DOC_ONBOARDING",
            archived_path: Some(&archived_path),
            last_error: None,
        },
    );

    let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    Ok(CommittedFileTurn {
        dashboard: updated,
        session_id: resolved_sid,
        raw_event_id,
        user_message_id,
        archived_path,
        extracted_text: content.clone(),
        combined_prompt_text: content,
        detected_format: doc_type.clone(),
        effective_title,
        effective_filename,
        doc_type,
        knowledge_doc_id: kdoc_id,
    })
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn cmd_run_file_assistant_turn(
    state: State<'_, AppState>,
    channel: String,
    session_id: String,
    raw_event_id: i64,
    user_message_id: i64,
    effective_title: String,
    effective_filename: String,
    doc_type: String,
    extracted_text: String,
    combined_prompt_text: String,
    knowledge_doc_id: Option<i64>,
) -> Result<AssistantTurnOutcome, String> {
    let norm_ch = normalize_chat_channel(&channel).to_string();
    let (
        chat_pid,
        pid,
        pname,
        all_projects,
        workspace_path,
        settings,
        open_tasks,
        active_decisions,
        active_sops,
        knowledge_docs,
        methods_playbooks,
        copilot_history,
        vault_history,
        resolved_kdoc_id,
        portfolio_snapshot,
    ) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let resolved_doc = knowledge_doc_id.or_else(|| {
            conn.query_row(
                "SELECT knowledge_doc_id FROM pending_ai_queue WHERE user_message_id = ?1 LIMIT 1",
                rusqlite::params![user_message_id],
                |r| r.get::<_, Option<i64>>(0),
            )
            .ok()
            .flatten()
        });
        if let Some(did) = resolved_doc {
            let _ = set_knowledge_doc_processing_status(&conn, did, "PROCESSING", None, None, None);
        }
        let d = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
        let chat_pid = resolve_chat_session_project_id(&conn, Some(&session_id), &d.active_project.id);
        let portfolio_snap = build_cross_project_portfolio_snapshot(&conn);
        let filtered_copilot: Vec<ChatMessage> = d
            .chat_history
            .iter()
            .filter(|m| m.id != user_message_id)
            .cloned()
            .collect();
        let filtered_vault: Vec<ChatMessage> = d
            .vault_chat_history
            .iter()
            .filter(|m| m.id != user_message_id)
            .cloned()
            .collect();
        (
            chat_pid,
            d.active_project.id.clone(),
            d.active_project.name.clone(),
            d.projects.clone(),
            d.active_project.workspace_path.clone(),
            d.settings.clone(),
            d.open_tasks.clone(),
            d.active_decisions.clone(),
            d.active_sops.clone(),
            d.knowledge_docs.clone(),
            d.methods_playbooks.clone(),
            filtered_copilot,
            filtered_vault,
            resolved_doc,
            portfolio_snap,
        )
    };

    let base_prompt = if norm_ch == "VAULT" {
        build_document_onboarding_prompt(
            &chat_pid,
            &effective_title,
            &doc_type,
            &effective_filename,
            &extracted_text,
            &vault_history,
            &open_tasks,
            &active_decisions,
        )
    } else {
        build_copilot_chat_prompt(
            &chat_pid,
            &combined_prompt_text,
            &copilot_history,
            &vault_history,
            &open_tasks,
            &active_decisions,
            &active_sops,
            &knowledge_docs,
            &[],
        )
    };
    let enriched_base = if portfolio_snapshot.trim().is_empty() {
        base_prompt
    } else {
        format!("{base_prompt}\n\n{portfolio_snapshot}")
    };
    let prompt = append_cross_project_routing_instructions(enriched_base, &chat_pid, &all_projects);

    let (mcp_sys_prompt, mcp_user_prompt) = build_mcp_tool_turn_prompts_with_portfolio(
        &chat_pid,
        &norm_ch,
        if norm_ch == "VAULT" { &extracted_text } else { &combined_prompt_text },
        if norm_ch == "VAULT" { &vault_history } else { &copilot_history },
        &open_tasks,
        &active_decisions,
        &active_sops,
        &knowledge_docs,
        &methods_playbooks,
        &[],
        Some((&effective_title, &doc_type, &effective_filename)),
        false,
        &all_projects,
        &portfolio_snapshot,
    );

    let llm_cfg = LlmConfig::from(&settings);
    let source_origin = if norm_ch == "VAULT" { "DOC_ONBOARDING" } else { "COPILOT_FILE" };
    match run_copilot_turn(
        &state.db,
        state.app_data_dir.clone(),
        &llm_cfg,
        &chat_pid,
        Some(raw_event_id),
        source_origin,
        &mcp_sys_prompt,
        &mcp_user_prompt,
        &prompt,
    )
    .await
    {
        Ok(turn) => {
            let envelope = turn.envelope;
            let summary = envelope
                .executive_doc_summary
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| envelope.conversational_reply.clone());

            let knowledge_path = archive_onboarding_document(
                &workspace_path,
                &effective_title,
                &extracted_text,
                &summary,
            )?;

            let mut conn = state.db.lock().map_err(|e| e.to_string())?;
            let items_count = (envelope.mutations.len() + envelope.method_playbooks.len() + turn.mcp_ops_executed.len()) as i32;
            let (env_badge, env_artifacts) =
                if !envelope.mutations.is_empty()
                    || !envelope.method_playbooks.is_empty()
                    || turn.mcp_ops_executed.is_empty()
                {
                    apply_envelope_cross_project(
                        &mut conn,
                        &all_projects,
                        &pid,
                        &pname,
                        &combined_prompt_text,
                        Some(&knowledge_path),
                        source_origin,
                        &envelope,
                        Some(raw_event_id),
                    )?
                } else {
                    ("بدون تغییر گراف".to_string(), Vec::new())
                };
            let badge = format_turn_badge(&turn.mcp_ops_executed, &env_badge, &turn.transport_label);

            let inserted_doc_id = if let Some(did) = resolved_kdoc_id {
                let _ = update_knowledge_doc_analysis(
                    &conn,
                    did,
                    &summary,
                    items_count,
                    Some(&knowledge_path),
                );
                Some(did)
            } else {
                insert_knowledge_doc_with_status(
                    &conn,
                    &pid,
                    &effective_title,
                    &doc_type,
                    &effective_filename,
                    &knowledge_path,
                    &summary,
                    items_count,
                    "PROCESSED",
                )
                .ok()
            };

            let mut extra_artifacts = vec![WrittenArtifactRef {
                artifact_type: "KNOWLEDGE_DOC".to_string(),
                entity_id: inserted_doc_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| effective_title.clone()),
                title: effective_title.clone(),
                summary: summary.chars().take(180).collect(),
                file_path: Some(knowledge_path.clone()),
                action: "CREATED".to_string(),
                version: None,
                project_id: Some(pid.clone()),
                project_name: Some(pname.clone()),
            }];
            if let (Some(mtitle), Some(mbody)) = (
                envelope.meeting_title.as_deref().map(str::trim).filter(|s| !s.is_empty()),
                envelope.meeting_summary_markdown.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            ) {
                extra_artifacts.push(WrittenArtifactRef {
                    artifact_type: "MEETING_MINUTES".to_string(),
                    entity_id: format!("MTG-{}", mtitle.chars().take(24).collect::<String>()),
                    title: mtitle.to_string(),
                    summary: mbody.chars().take(160).collect(),
                    file_path: None,
                    action: "CREATED".to_string(),
                    version: None,
                    project_id: Some(pid.clone()),
                    project_name: Some(pname.clone()),
                });
            }
            let written_artifacts =
                merge_written_artifacts(&turn.mcp_written_artifacts, &env_artifacts, extra_artifacts);

            append_assistant_chat_message_with_artifacts(
                &conn,
                &chat_pid,
                &session_id,
                &norm_ch,
                "ASSISTANT",
                &envelope.conversational_reply,
                Some(&knowledge_path),
                &badge,
                &envelope.clarifying_questions,
                &written_artifacts,
            )
            .map_err(|e| e.to_string())?;

            let _ = complete_pending_items_for_user_msg(&conn, user_message_id, inserted_doc_id);

            let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            sync_touched_project_projections(
                &conn,
                &updated,
                &written_artifacts,
                envelope.meeting_title.as_deref(),
                envelope.meeting_summary_markdown.as_deref(),
            )?;

            Ok(AssistantTurnOutcome {
                dashboard: updated,
                assistant_error: None,
            })
        }
        Err(err) => {
            let fallback_summary: String = extracted_text.chars().take(260).collect();
            let knowledge_path = archive_onboarding_document(
                &workspace_path,
                &effective_title,
                &extracted_text,
                &format!("آرشیو خام بدون تحلیل مدل (خطای موقت مدل): {}", fallback_summary),
            )
            .ok();

            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let kdoc_id = if let Some(did) = resolved_kdoc_id {
                let _ = set_knowledge_doc_processing_status(
                    &conn,
                    did,
                    "FAILED",
                    Some("متن فایل استخراج و در حافظه پروژه آرشیو شد (تحلیل هوشمند به دلیل خطای مدل انجام نشد)."),
                    Some(0),
                    knowledge_path.as_deref(),
                );
                Some(did)
            } else if let Some(kp) = &knowledge_path {
                insert_knowledge_doc_with_status(
                    &conn,
                    &pid,
                    &effective_title,
                    &doc_type,
                    &effective_filename,
                    kp,
                    "متن فایل استخراج و در حافظه پروژه آرشیو شد (تحلیل هوشمند به دلیل خطای مدل انجام نشد).",
                    0,
                    "FAILED",
                )
                .ok()
            } else {
                None
            };
            let notice = format!(
                "⚠️ محتوای «{}» استخراج شد و بلافاصله در حافظه و آرشیو پروژه ذخیره گردید، اما تحلیل مدل انجام نشد:\n{}",
                effective_filename, err
            );
            let sys_id = append_assistant_chat_message(
                &conn,
                &chat_pid,
                &session_id,
                &norm_ch,
                "SYSTEM",
                &notice,
                knowledge_path.as_deref(),
                "ذخیره در حافظه • خطای موتور هوش مصنوعی",
                &[],
            )
            .ok();
            let user_arch = get_user_chat_message_by_id(&conn, user_message_id)
                .and_then(|(_, arch)| arch);
            let _ = enqueue_pending_ai_item(
                &conn,
                &chat_pid,
                &session_id,
                &norm_ch,
                Some(user_message_id),
                sys_id,
                kdoc_id,
                Some(raw_event_id),
                &format!("📎 {}", effective_filename),
                source_origin,
                user_arch.as_deref().or(knowledge_path.as_deref()),
                Some(&err),
            );
            let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            Ok(AssistantTurnOutcome {
                dashboard: updated,
                assistant_error: Some(err),
            })
        }
    }
}

#[tauri::command]
fn cmd_quick_complete_step(
    state: State<'_, AppState>,
    task_id: String,
    pomos_spent: i32,
) -> Result<DashboardState, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let owner_pid: Option<String> = conn
        .query_row(
            "SELECT project_id FROM work_entities WHERE id = ?1 LIMIT 1",
            [&task_id],
            |r| r.get(0),
        )
        .ok();
    let dash_before = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let target_pid = owner_pid.unwrap_or_else(|| dash_before.active_project.id.clone());
    quick_complete_task(&mut conn, &target_pid, &task_id, pomos_spent)
        .map_err(|e| e.to_string())?;
    let dash_after = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash_after, None, None);
    if target_pid != dash_after.active_project.id {
        if let Ok(other_dash) = get_dashboard_state_for_project(&conn, Some(&target_pid)) {
            let _ = sync_all_project_projections(&other_dash, None, None);
        }
    }
    Ok(dash_after)
}

#[tauri::command]
fn cmd_start_task_step(
    state: State<'_, AppState>,
    task_id: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let owner_pid: Option<String> = conn
        .query_row(
            "SELECT project_id FROM work_entities WHERE id = ?1 LIMIT 1",
            [&task_id],
            |r| r.get(0),
        )
        .ok();
    let dash_before = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let target_pid = owner_pid.unwrap_or_else(|| dash_before.active_project.id.clone());
    let started = start_task_step(&conn, &target_pid, &task_id)
        .map_err(|e| e.to_string())?;
    if !started {
        return Err("این گام یا در زنجیره قفل است یا از قبل در حال اجرا می‌باشد.".into());
    }
    let dash_after = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash_after, None, None);
    if target_pid != dash_after.active_project.id {
        if let Ok(other_dash) = get_dashboard_state_for_project(&conn, Some(&target_pid)) {
            let _ = sync_all_project_projections(&other_dash, None, None);
        }
    }
    Ok(dash_after)
}

#[tauri::command]
async fn cmd_send_copilot_message(
    state: State<'_, AppState>,
    message_text: String,
) -> Result<DashboardState, String> {
    let committed = cmd_commit_user_chat_message(
        state.clone(),
        "COPILOT".to_string(),
        None,
        message_text.clone(),
    )?;
    let outcome = cmd_run_assistant_turn(
        state,
        "COPILOT".to_string(),
        committed.session_id,
        message_text,
        committed.archived_path,
        Some(committed.raw_event_id),
        committed.user_message_id,
    )
    .await?;
    if let Some(err) = outcome.assistant_error {
        return Err(err);
    }
    Ok(outcome.dashboard)
}

#[tauri::command]
async fn cmd_send_copilot_file_message(
    state: State<'_, AppState>,
    original_filename: String,
    content_base64: String,
    user_note: String,
) -> Result<DashboardState, String> {
    let committed = cmd_commit_user_file_message(
        state.clone(),
        "COPILOT".to_string(),
        None,
        original_filename,
        content_base64,
        user_note,
        None,
        None,
    )?;
    let outcome = cmd_run_file_assistant_turn(
        state,
        "COPILOT".to_string(),
        committed.session_id,
        committed.raw_event_id,
        committed.user_message_id,
        committed.effective_title,
        committed.effective_filename,
        committed.doc_type,
        committed.extracted_text,
        committed.combined_prompt_text,
        committed.knowledge_doc_id,
    )
    .await?;
    if let Some(err) = outcome.assistant_error {
        return Err(err);
    }
    Ok(outcome.dashboard)
}

#[tauri::command]
async fn cmd_ingest_onboarding_document(
    state: State<'_, AppState>,
    title: String,
    doc_type: String,
    original_filename: String,
    content: String,
) -> Result<DashboardState, String> {
    let committed = cmd_commit_vault_text_message(
        state.clone(),
        None,
        title,
        doc_type,
        original_filename,
        content,
    )?;
    let outcome = cmd_run_file_assistant_turn(
        state,
        "VAULT".to_string(),
        committed.session_id,
        committed.raw_event_id,
        committed.user_message_id,
        committed.effective_title,
        committed.effective_filename,
        committed.doc_type,
        committed.extracted_text,
        committed.combined_prompt_text,
        committed.knowledge_doc_id,
    )
    .await?;
    if let Some(err) = outcome.assistant_error {
        return Err(err);
    }
    Ok(outcome.dashboard)
}

#[tauri::command]
async fn cmd_ingest_onboarding_file(
    state: State<'_, AppState>,
    title: String,
    doc_type: String,
    original_filename: String,
    content_base64: String,
) -> Result<DashboardState, String> {
    let committed = cmd_commit_user_file_message(
        state.clone(),
        "VAULT".to_string(),
        None,
        original_filename,
        content_base64,
        String::new(),
        Some(title),
        Some(doc_type),
    )?;
    let outcome = cmd_run_file_assistant_turn(
        state,
        "VAULT".to_string(),
        committed.session_id,
        committed.raw_event_id,
        committed.user_message_id,
        committed.effective_title,
        committed.effective_filename,
        committed.doc_type,
        committed.extracted_text,
        committed.combined_prompt_text,
        committed.knowledge_doc_id,
    )
    .await?;
    if let Some(err) = outcome.assistant_error {
        return Err(err);
    }
    Ok(outcome.dashboard)
}

#[tauri::command]
async fn cmd_send_vault_chat_message(
    state: State<'_, AppState>,
    message_text: String,
) -> Result<DashboardState, String> {
    let committed = cmd_commit_user_chat_message(
        state.clone(),
        "VAULT".to_string(),
        None,
        message_text.clone(),
    )?;
    let outcome = cmd_run_assistant_turn(
        state,
        "VAULT".to_string(),
        committed.session_id,
        message_text,
        committed.archived_path,
        Some(committed.raw_event_id),
        committed.user_message_id,
    )
    .await?;
    if let Some(err) = outcome.assistant_error {
        return Err(err);
    }
    Ok(outcome.dashboard)
}

#[tauri::command]
async fn cmd_fetch_live_models(
    provider: String,
    local_endpoint: String,
    gemini_api_key: String,
) -> Result<LiveModelsResponse, String> {
    Ok(fetch_live_models(&provider, &local_endpoint, &gemini_api_key).await)
}

#[tauri::command]
fn cmd_sync_mab_focus_now(
    state: State<'_, AppState>,
) -> Result<(SyncReport, DashboardState), String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let report = synchronize_with_mab_focus(&mut conn)?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    Ok((report, dash))
}

#[tauri::command]
fn cmd_save_settings(
    state: State<'_, AppState>,
    new_settings: AppSettings,
) -> Result<DashboardState, String> {
    if !new_settings.vault_root_path.trim().is_empty() {
        let _ = fs::create_dir_all(&new_settings.vault_root_path);
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    update_settings(&conn, &new_settings).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_export_excel_report(state: State<'_, AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&dash, None, None);
    let out = PathBuf::from(&dash.active_project.workspace_path)
        .join("07_client_reports")
        .join(format!("{}_Timesheet_Report.xlsx", dash.active_project.id));
    export_client_xlsx(&out, &dash.active_project.name, &dash.recent_logs)?;
    Ok(out.to_string_lossy().to_string())
}

#[tauri::command]
fn cmd_undo_last(state: State<'_, AppState>) -> Result<DashboardState, String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    undo_last_mutation(&mut conn, &dash.active_project.id).map_err(|e| e.to_string())?;
    let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    sync_all_project_projections(&updated, None, None)?;
    Ok(updated)
}

#[tauri::command]
async fn cmd_check_llm_connection(
    state: State<'_, AppState>,
) -> Result<LlmConnectionStatus, String> {
    let llm_cfg = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let settings = get_settings(&conn).map_err(|e| e.to_string())?;
        LlmConfig::from(&settings)
    };
    Ok(check_llm_connectivity(&llm_cfg).await)
}

#[tauri::command]
fn cmd_scan_unprocessed_queue(state: State<'_, AppState>) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let _ = sync_unprocessed_items_into_queue(&conn);
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_dismiss_pending_queue_item(
    state: State<'_, AppState>,
    queue_id: i64,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    dismiss_pending_queue_item(&conn, queue_id).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
async fn cmd_process_pending_queue_item(
    state: State<'_, AppState>,
    queue_id: i64,
) -> Result<PendingQueueProcessOutcome, String> {
    let prep = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let item = get_pending_queue_item(&conn, queue_id)
            .map_err(|e| format!("آیتم صف انتظار یافت نشد: {}", e))?;

        if item.status == "DONE" || item.status == "DISMISSED" {
            let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            return Ok(PendingQueueProcessOutcome {
                dashboard: dash,
                processed: false,
                error: None,
            });
        }

        let doc_row_before = item
            .knowledge_doc_id
            .and_then(|did| get_knowledge_document_by_id(&conn, did));
        let was_doc_already_processed = doc_row_before
            .as_ref()
            .map(|d| d.processing_status.eq_ignore_ascii_case("PROCESSED"))
            .unwrap_or(false);

        let _ = set_pending_item_processing(&conn, queue_id);
        if let Some(doc_id) = item.knowledge_doc_id {
            let _ = set_knowledge_doc_processing_status(&conn, doc_id, "PROCESSING", None, None, None);
        }
        let target_dash = if item.project_id == GLOBAL_CHAT_PROJECT_ID {
            get_dashboard_state(&conn).map_err(|e| e.to_string())?
        } else {
            get_dashboard_state_for_project(&conn, Some(&item.project_id))
                .map_err(|e| e.to_string())?
        };
        let portfolio_snapshot = build_cross_project_portfolio_snapshot(&conn);

        let raw_row = item
            .raw_event_id
            .and_then(|eid| get_raw_event_content_by_id(&conn, eid));
        let user_row = item
            .user_message_id
            .and_then(|uid| get_user_chat_message_by_id(&conn, uid));
        let doc_row = doc_row_before;

        let read_archived = |path_str: &str| -> Option<String> {
            let raw = fs::read_to_string(path_str).ok()?;
            let normalized = path_str.replace('\\', "/");
            let cleaned = if normalized.contains("/01_knowledge_archive/") {
                strip_knowledge_archive_header(&raw)
            } else {
                strip_raw_input_frontmatter(&raw)
            };
            if cleaned.trim().is_empty() {
                None
            } else {
                Some(cleaned)
            }
        };

        let recovered_text = item
            .archived_path
            .as_deref()
            .and_then(read_archived)
            .or_else(|| {
                user_row
                    .as_ref()
                    .and_then(|(_, arch)| arch.as_deref())
                    .and_then(read_archived)
            })
            .or_else(|| raw_row.as_ref().map(|(_, c)| c.clone()).filter(|s| !s.trim().is_empty()))
            .or_else(|| doc_row.as_ref().and_then(|d| read_archived(&d.archived_path)))
            .or_else(|| {
                user_row
                    .as_ref()
                    .map(|(txt, _)| txt.clone())
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| item.title_preview.clone());

        let is_vault_channel = item.channel.eq_ignore_ascii_case("VAULT");
        let effective_session_id = if !item.session_id.trim().is_empty() {
            item.session_id.clone()
        } else if is_vault_channel {
            target_dash.active_vault_session_id.clone()
        } else {
            target_dash.active_copilot_session_id.clone()
        };

        let session_msgs = query_messages_for_session_public(
            &conn,
            &item.project_id,
            &effective_session_id,
            is_vault_channel,
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|m| {
            Some(m.id) != item.user_message_id && Some(m.id) != item.system_message_id
        })
        .collect::<Vec<_>>();

        let (copilot_history, vault_history) = if is_vault_channel {
            (target_dash.chat_history.clone(), session_msgs)
        } else {
            (session_msgs, target_dash.vault_chat_history.clone())
        };

        let fts_pid = if item.project_id == GLOBAL_CHAT_PROJECT_ID {
            target_dash.active_project.id.as_str()
        } else {
            item.project_id.as_str()
        };
        let fts_hits = search_raw_events_fts(&conn, fts_pid, &recovered_text)
            .unwrap_or_default();

        (
            item,
            effective_session_id,
            target_dash,
            raw_row,
            doc_row,
            was_doc_already_processed,
            recovered_text,
            copilot_history,
            vault_history,
            fts_hits,
            portfolio_snapshot,
        )
    };

    let (
        item,
        effective_session_id,
        target_dash,
        raw_row,
        doc_row,
        was_doc_already_processed,
        recovered_text,
        copilot_history,
        vault_history,
        fts_hits,
        portfolio_snapshot,
    ) = prep;

    let norm_ch = normalize_chat_channel(&item.channel).to_string();
    let is_legacy_reprocess = item.source_type.starts_with("LEGACY_REPROCESS");
    let source_origin = if is_legacy_reprocess {
        item.source_type.clone()
    } else {
        raw_row
            .as_ref()
            .map(|(st, _)| st.clone())
            .unwrap_or_else(|| item.source_type.clone())
    };
    let is_doc_turn = doc_row.is_some()
        || source_origin == "DOC_ONBOARDING"
        || source_origin == "COPILOT_FILE";

    let doc_meta_owned = if is_doc_turn {
        let title = doc_row
            .as_ref()
            .map(|d| d.title.clone())
            .unwrap_or_else(|| derive_title_from_content("", &recovered_text, "سند در صف انتظار"));
        let dtype = doc_row
            .as_ref()
            .map(|d| d.doc_type.clone())
            .unwrap_or_else(|| "ONBOARDING_FILE".to_string());
        let fname = doc_row
            .as_ref()
            .map(|d| d.original_filename.clone())
            .unwrap_or_else(|| "queued_doc.md".to_string());
        Some((title, dtype, fname))
    } else {
        None
    };
    let doc_meta_ref = doc_meta_owned
        .as_ref()
        .map(|(t, dt, f)| (t.as_str(), dt.as_str(), f.as_str()));

    let prompt = if is_legacy_reprocess {
        build_legacy_reprocess_prompt(
            &item.project_id,
            &item.title_preview,
            &recovered_text,
            &target_dash.open_tasks,
            &target_dash.active_decisions,
            &target_dash.active_sops,
            &target_dash.methods_playbooks,
        )
    } else if let Some((ref t, ref dt, ref f)) = doc_meta_owned {
        if norm_ch == "VAULT" {
            build_document_onboarding_prompt(
                &item.project_id,
                t,
                dt,
                f,
                &recovered_text,
                &vault_history,
                &target_dash.open_tasks,
                &target_dash.active_decisions,
            )
        } else {
            build_copilot_chat_prompt(
                &item.project_id,
                &recovered_text,
                &copilot_history,
                &vault_history,
                &target_dash.open_tasks,
                &target_dash.active_decisions,
                &target_dash.active_sops,
                &target_dash.knowledge_docs,
                &fts_hits,
            )
        }
    } else if norm_ch == "VAULT" {
        build_vault_chat_prompt(
            &item.project_id,
            &recovered_text,
            &vault_history,
            &target_dash.open_tasks,
            &target_dash.active_decisions,
            &target_dash.active_sops,
            &target_dash.knowledge_docs,
            &fts_hits,
        )
    } else {
        build_copilot_chat_prompt(
            &item.project_id,
            &recovered_text,
            &copilot_history,
            &vault_history,
            &target_dash.open_tasks,
            &target_dash.active_decisions,
            &target_dash.active_sops,
            &target_dash.knowledge_docs,
            &fts_hits,
        )
    };

    let (mcp_sys_prompt, mcp_user_prompt) = build_mcp_tool_turn_prompts_with_portfolio(
        &item.project_id,
        &norm_ch,
        &recovered_text,
        if norm_ch == "VAULT" {
            &vault_history
        } else {
            &copilot_history
        },
        &target_dash.open_tasks,
        &target_dash.active_decisions,
        &target_dash.active_sops,
        &target_dash.knowledge_docs,
        &target_dash.methods_playbooks,
        &fts_hits,
        doc_meta_ref,
        is_legacy_reprocess,
        &target_dash.projects,
        &portfolio_snapshot,
    );

    let enriched_prompt = append_cross_project_routing_instructions(
        if portfolio_snapshot.trim().is_empty() {
            prompt
        } else {
            format!("{prompt}\n\n{portfolio_snapshot}")
        },
        &item.project_id,
        &target_dash.projects,
    );

    let llm_cfg = LlmConfig::from(&target_dash.settings);
    match run_copilot_turn(
        &state.db,
        state.app_data_dir.clone(),
        &llm_cfg,
        &item.project_id,
        item.raw_event_id,
        &source_origin,
        &mcp_sys_prompt,
        &mcp_user_prompt,
        &enriched_prompt,
    )
    .await
    {
        Ok(turn) => {
            let envelope = turn.envelope;
            let mut conn = state.db.lock().map_err(|e| e.to_string())?;
            let (env_badge, env_artifacts) =
                if !envelope.mutations.is_empty()
                    || !envelope.method_playbooks.is_empty()
                    || turn.mcp_ops_executed.is_empty()
                {
                    apply_envelope_cross_project(
                        &mut conn,
                        &target_dash.projects,
                        &target_dash.active_project.id,
                        &target_dash.active_project.name,
                        &recovered_text,
                        item.archived_path.as_deref(),
                        &source_origin,
                        &envelope,
                        item.raw_event_id,
                    )?
                } else {
                    ("بدون تغییر گراف".to_string(), Vec::new())
                };
            let badge =
                format_turn_badge(&turn.mcp_ops_executed, &env_badge, &turn.transport_label);

            let items_count = (envelope.mutations.len() + envelope.method_playbooks.len() + turn.mcp_ops_executed.len()) as i32;
            let mut extra_artifacts: Vec<WrittenArtifactRef> = Vec::new();
            if let Some(doc_id) = item.knowledge_doc_id {
                let summary = envelope
                    .executive_doc_summary
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| envelope.conversational_reply.clone());
                let title_for_doc = doc_row
                    .as_ref()
                    .map(|d| d.title.as_str())
                    .unwrap_or("سند پردازش‌شده");
                let new_kp = if is_legacy_reprocess {
                    doc_row.as_ref().map(|d| d.archived_path.clone())
                } else {
                    archive_onboarding_document(
                        &target_dash.active_project.workspace_path,
                        title_for_doc,
                        &recovered_text,
                        &summary,
                    )
                    .ok()
                };
                if !is_legacy_reprocess {
                    let _ = update_knowledge_doc_analysis(
                        &conn,
                        doc_id,
                        &summary,
                        items_count,
                        new_kp.as_deref(),
                    );
                    extra_artifacts.push(WrittenArtifactRef {
                        artifact_type: "KNOWLEDGE_DOC".to_string(),
                        entity_id: doc_id.to_string(),
                        title: title_for_doc.to_string(),
                        summary: summary.chars().take(180).collect(),
                        file_path: new_kp,
                        action: "UPDATED".to_string(),
                        version: None,
                        ..Default::default()
                    });
                } else {
                    let _ = set_knowledge_doc_processing_status(
                        &conn,
                        doc_id,
                        "PROCESSED",
                        None,
                        None,
                        None,
                    );
                    let _ = method_engine::mark_legacy_reprocessed(
                        &conn,
                        "KNOWLEDGE_DOC",
                        doc_id,
                        &item.project_id,
                        &badge,
                    );
                }
            } else if is_legacy_reprocess {
                if let Some(eid) = item.raw_event_id {
                    let _ = method_engine::mark_legacy_reprocessed(
                        &conn,
                        "RAW_EVENT",
                        eid,
                        &item.project_id,
                        &badge,
                    );
                }
            }

            if let Some(sys_id) = item.system_message_id {
                let _ = delete_system_chat_message(&conn, sys_id);
            }

            let written_artifacts =
                merge_written_artifacts(&turn.mcp_written_artifacts, &env_artifacts, extra_artifacts);

            append_assistant_chat_message_with_artifacts(
                &conn,
                &item.project_id,
                &effective_session_id,
                &norm_ch,
                "ASSISTANT",
                &envelope.conversational_reply,
                item.archived_path.as_deref(),
                &badge,
                &envelope.clarifying_questions,
                &written_artifacts,
            )
            .map_err(|e| e.to_string())?;

            let _ = mark_pending_item_done(&conn, queue_id);
            if let Some(uid) = item.user_message_id {
                let _ = complete_pending_items_for_user_msg(&conn, uid, item.knowledge_doc_id);
            }

            let _ = sync_touched_project_projections(
                &conn,
                &target_dash,
                &written_artifacts,
                envelope.meeting_title.as_deref(),
                envelope.meeting_summary_markdown.as_deref(),
            );

            let updated_dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            Ok(PendingQueueProcessOutcome {
                dashboard: updated_dash,
                processed: true,
                error: None,
            })
        }
        Err(err) => {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let _ = record_pending_item_failure(&conn, queue_id, &err);
            if let Some(doc_id) = item.knowledge_doc_id {
                let restore_status = if is_legacy_reprocess && was_doc_already_processed {
                    "PROCESSED"
                } else {
                    "FAILED"
                };
                let _ = set_knowledge_doc_processing_status(
                    &conn,
                    doc_id,
                    restore_status,
                    None,
                    None,
                    None,
                );
            }
            if let Some(sys_id) = item.system_message_id {
                let notice = format!(
                    "⚠️ پیام شما در صف انتظار و آرشیو پروژه ذخیره است، اما در ارسال خودکار به هوش مصنوعی خطا رخ داد:\n{}",
                    err
                );
                let _ = update_system_chat_message(&conn, sys_id, &notice);
            }
            let updated_dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
            Ok(PendingQueueProcessOutcome {
                dashboard: updated_dash,
                processed: false,
                error: Some(err),
            })
        }
    }
}

// ==============================================================================
// TAURI IPC COMMANDS — SEQUENTIAL CHAIN & CONTEXT EXECUTION (v3.0 MILESTONE 2)
// ==============================================================================

#[tauri::command]
fn cmd_create_work_line_chain(
    state: State<'_, AppState>,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let project_id = params
        .get("project_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            conn.query_row(
                "SELECT id FROM projects WHERE is_active = 1 LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "GERDALO".to_string())
        });

    let outcome_id = params
        .get("outcome_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty());

    let title = params
        .get("title")
        .or_else(|| params.get("line_title"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("مسیر زنجیره‌ای جدید");

    let goal_statement = params
        .get("goal_statement")
        .or_else(|| params.get("goal"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let line_mode = params
        .get("line_mode")
        .or_else(|| params.get("mode"))
        .and_then(|v| v.as_str())
        .unwrap_or("SEQUENTIAL_CHAIN");

    let steps: Vec<ChainStepInput> = if let Some(arr) = params.get("steps").and_then(|v| v.as_array()) {
        arr.iter()
            .map(|item| {
                if let Some(s) = item.as_str() {
                    ChainStepInput {
                        title: s.to_string(),
                        ..Default::default()
                    }
                } else {
                    serde_json::from_value::<ChainStepInput>(item.clone()).unwrap_or_default()
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let (mut work_line, tasks) = create_work_line_with_chain(
        &conn,
        &project_id,
        outcome_id,
        title,
        goal_statement,
        line_mode,
        steps,
    )
    .map_err(|e| e.to_string())?;

    if let Some(color) = params.get("color_hex").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
        let _ = conn.execute(
            "UPDATE work_lines SET color_hex = ?1 WHERE id = ?2",
            rusqlite::params![color.trim(), work_line.id],
        );
        work_line.color_hex = color.trim().to_string();
    }

    let updated_dash = get_dashboard_state(&conn).ok();
    Ok(serde_json::json!({
        "status": "ok",
        "work_line": work_line,
        "work_line_id": work_line.id,
        "tasks": tasks.iter().map(|t| t.id.clone()).collect::<Vec<String>>(),
        "entities": tasks,
        "dashboard": updated_dash,
    }))
}

#[tauri::command]
fn cmd_reorder_pending_queue_item(
    state: State<'_, AppState>,
    queue_id: i64,
    direction: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    reorder_pending_queue_item(&conn, queue_id, &direction).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_reset_pending_queue_order(state: State<'_, AppState>) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    reset_pending_queue_order(&conn).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_requeue_knowledge_doc(
    state: State<'_, AppState>,
    doc_id: i64,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    requeue_knowledge_document(&conn, doc_id).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn cmd_upsert_outcome(
    state: State<'_, AppState>,
    outcome_id: Option<String>,
    domain_code: String,
    title: String,
    desired_reality_change: String,
    evidence_kr: String,
    time_window: String,
    status: String,
    progress_percent: i32,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    upsert_outcome_manual(
        &conn,
        outcome_id.as_deref(),
        &domain_code,
        &title,
        &desired_reality_change,
        &evidence_kr,
        &time_window,
        &status,
        progress_percent,
    )
    .map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_delete_outcome(
    state: State<'_, AppState>,
    outcome_id: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    delete_outcome(&conn, &outcome_id).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_update_work_line(
    state: State<'_, AppState>,
    work_line_id: String,
    title: String,
    goal_statement: String,
    outcome_id: Option<String>,
    color_hex: String,
    status: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    update_work_line_manual(
        &conn,
        &work_line_id,
        &title,
        &goal_statement,
        outcome_id.as_deref(),
        &color_hex,
        &status,
    )
    .map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn cmd_assign_task_grouping(
    state: State<'_, AppState>,
    task_id: String,
    work_line_id: Option<String>,
    outcome_id: Option<String>,
    priority_score: Option<i32>,
    energy_tag: Option<String>,
    device_tag: Option<String>,
    duration_minutes: Option<i32>,
    scheduled_date: Option<String>,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    assign_task_grouping(
        &conn,
        &dash.active_project.id,
        &task_id,
        work_line_id.as_deref(),
        outcome_id.as_deref(),
        priority_score,
        energy_tag.as_deref(),
        device_tag.as_deref(),
        duration_minutes,
        scheduled_date.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&updated, None, None);
    Ok(updated)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn cmd_create_task_manual(
    state: State<'_, AppState>,
    title: String,
    category: String,
    definition_of_done: String,
    first_physical_action: String,
    work_line_id: Option<String>,
    outcome_id: Option<String>,
    energy_tag: String,
    device_tag: String,
    duration_minutes: i32,
    scheduled_date: Option<String>,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    create_task_manual(
        &conn,
        &dash.active_project.id,
        &title,
        &category,
        &definition_of_done,
        &first_physical_action,
        work_line_id.as_deref(),
        outcome_id.as_deref(),
        &energy_tag,
        &device_tag,
        duration_minutes,
        scheduled_date.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let _ = sync_all_project_projections(&updated, None, None);
    Ok(updated)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn cmd_create_personal_vault_item(
    state: State<'_, AppState>,
    realm: String,
    domain_code: String,
    item_type: String,
    title: String,
    content_markdown: String,
    tags: Vec<String>,
    attach_to_active_project: bool,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    let pid = if attach_to_active_project {
        Some(dash.active_project.id.as_str())
    } else {
        None
    };
    create_personal_vault_item(
        &conn,
        pid,
        &realm,
        &domain_code,
        &item_type,
        &title,
        &content_markdown,
        &tags,
    )
    .map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_delete_personal_vault_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    delete_personal_vault_item(&conn, &item_id).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_resolve_pending_proposal(
    state: State<'_, AppState>,
    proposal_id: String,
    approve: bool,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    resolve_pending_proposal(&conn, &proposal_id, approve).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_upsert_calendar_day(
    state: State<'_, AppState>,
    date_key: String,
    total_capacity_minutes: i32,
    deep_capacity_minutes: i32,
    day_note: String,
    is_off_day: bool,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    upsert_calendar_day(
        &conn,
        &date_key,
        total_capacity_minutes,
        deep_capacity_minutes,
        &day_note,
        is_off_day,
    )
    .map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_advance_chain_step(
    state: State<'_, AppState>,
    project_id: String,
    task_id: String,
) -> Result<Option<WorkEntity>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    advance_sequential_chain_on_complete(&conn, &project_id, &task_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_query_tasks_by_context(
    state: State<'_, AppState>,
    params: serde_json::Value,
) -> Result<Vec<WorkEntity>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let project_id = params
        .get("project_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            conn.query_row(
                "SELECT id FROM projects WHERE is_active = 1 LIMIT 1",
                [],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_default()
        });

    let device_tag = params.get("device_tag").and_then(|v| v.as_str());
    let location_tag = params.get("location_tag").and_then(|v| v.as_str());
    let energy_tag = params.get("energy_tag").and_then(|v| v.as_str());
    let max_minutes = params.get("max_minutes").and_then(|v| v.as_i64()).map(|m| m as i32);
    let only_unlocked = params.get("only_unlocked").and_then(|v| v.as_bool()).unwrap_or(true);

    query_tasks_by_context(
        &conn,
        &project_id,
        device_tag,
        location_tag,
        energy_tag,
        max_minutes,
        only_unlocked,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_upsert_method_playbook(
    state: State<'_, AppState>,
    input: MethodPlaybookInput,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    method_engine::upsert_method_playbook(
        &conn,
        &dash.active_project.id,
        &dash.active_project.workspace_path,
        input,
        Some("manual_ui"),
    )?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_delete_method_playbook(
    state: State<'_, AppState>,
    method_id: String,
) -> Result<DashboardState, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    method_engine::delete_method_playbook(&conn, &method_id).map_err(|e| e.to_string())?;
    get_dashboard_state(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_read_workspace_file(
    state: State<'_, AppState>,
    file_path: String,
) -> Result<String, String> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err("مسیر فایل خالی است.".into());
    }
    let requested = PathBuf::from(trimmed);
    let canonical_req = fs::canonicalize(&requested)
        .map_err(|e| format!("فایل در مسیر مشخص‌شده یافت نشد: {}", e))?;

    let allowed_roots: Vec<PathBuf> = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
        let mut roots = Vec::new();
        if !dash.settings.vault_root_path.trim().is_empty() {
            if let Ok(c) = fs::canonicalize(&dash.settings.vault_root_path) {
                roots.push(c);
            }
        }
        for p in &dash.projects {
            if !p.workspace_path.trim().is_empty() {
                if let Ok(c) = fs::canonicalize(&p.workspace_path) {
                    roots.push(c);
                }
            }
        }
        roots
    };

    if !allowed_roots.iter().any(|r| canonical_req.starts_with(r)) {
        return Err("دسترسی به فایل خارج از پوشه‌های Vault پروژه مجاز نیست.".into());
    }

    fs::read_to_string(&canonical_req).map_err(|e| format!("خطا در خواندن فایل: {}", e))
}

#[tauri::command]
fn cmd_list_legacy_reprocess_candidates(
    state: State<'_, AppState>,
) -> Result<Vec<LegacyReprocessCandidate>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    method_engine::list_legacy_reprocess_candidates(&conn, &dash.active_project.id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_backup_database_now(
    state: State<'_, AppState>,
) -> Result<DatabaseBackupResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let backup_dir = state.app_data_dir.join("backups");
    method_engine::backup_sqlite_database(&conn, &backup_dir)
}

#[tauri::command]
fn cmd_enqueue_legacy_reprocess(
    state: State<'_, AppState>,
    mode: String,
    selected_items: Vec<LegacyReprocessSelection>,
) -> Result<LegacyReprocessEnqueueResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let dash = get_dashboard_state(&conn).map_err(|e| e.to_string())?;

    // Mandatory automatic SQLite backup BEFORE enqueueing legacy reprocessing!
    let backup_dir = state.app_data_dir.join("backups");
    let backup = method_engine::backup_sqlite_database(&conn, &backup_dir)?;

    let enqueued_count = method_engine::enqueue_legacy_reprocess(
        &conn,
        &dash.active_project.id,
        &mode,
        &selected_items,
    )?;

    let updated = get_dashboard_state(&conn).map_err(|e| e.to_string())?;
    Ok(LegacyReprocessEnqueueResult {
        backup_path: backup.backup_path,
        backup_size_bytes: backup.backup_size_bytes,
        enqueued_count,
        dashboard: updated,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve AppData");
            fs::create_dir_all(&app_data_dir).expect("Failed to create AppData directory");
            let db_path = app_data_dir.join("mab_compass_v2.sqlite");

            let default_vault = dirs::document_dir()
                .unwrap_or_else(|| app_data_dir.clone())
                .join("MabOS_Projects_Vault");
            fs::create_dir_all(&default_vault).expect("Failed to create default vault directory");

            let mut conn =
                init_database(&db_path, &default_vault).expect("Failed to init SQLite v2");

            if let Ok(settings) = get_settings(&conn) {
                if settings.auto_sync_mab_focus {
                    let _ = synchronize_with_mab_focus(&mut conn);
                }
            }
            if let Ok(dash) = get_dashboard_state(&conn) {
                let _ = sync_all_project_projections(&dash, None, None);
            }

            let initial_core_rev = ecosystem_core::resolve_default_core_db_path()
                .map(|p| ecosystem_core::get_core_revision_at_path(&p))
                .unwrap_or(0);

            app.manage(AppState {
                db: Mutex::new(conn),
                app_data_dir,
                last_core_revision: Mutex::new(initial_core_rev),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd_get_db_data_version,
            cmd_get_dashboard,
            cmd_switch_project,
            cmd_create_project,
            cmd_update_project,
            cmd_merge_projects,
            cmd_delete_project,
            cmd_quick_complete_step,
            cmd_start_task_step,
            cmd_create_work_line_chain,
            cmd_advance_chain_step,
            cmd_query_tasks_by_context,
            cmd_create_chat_session,
            cmd_switch_chat_session,
            cmd_delete_chat_session,
            cmd_commit_user_chat_message,
            cmd_run_assistant_turn,
            cmd_retry_system_message,
            cmd_commit_user_file_message,
            cmd_commit_vault_text_message,
            cmd_run_file_assistant_turn,
            cmd_send_copilot_message,
            cmd_send_copilot_file_message,
            cmd_ingest_onboarding_document,
            cmd_ingest_onboarding_file,
            cmd_send_vault_chat_message,
            cmd_fetch_live_models,
            cmd_sync_mab_focus_now,
            cmd_save_settings,
            cmd_export_excel_report,
            cmd_undo_last,
            cmd_check_llm_connection,
            cmd_scan_unprocessed_queue,
            cmd_dismiss_pending_queue_item,
            cmd_process_pending_queue_item,
            cmd_reorder_pending_queue_item,
            cmd_reset_pending_queue_order,
            cmd_requeue_knowledge_doc,
            cmd_upsert_outcome,
            cmd_delete_outcome,
            cmd_update_work_line,
            cmd_assign_task_grouping,
            cmd_create_task_manual,
            cmd_create_personal_vault_item,
            cmd_delete_personal_vault_item,
            cmd_resolve_pending_proposal,
            cmd_upsert_calendar_day,
            cmd_upsert_method_playbook,
            cmd_delete_method_playbook,
            cmd_read_workspace_file,
            cmd_list_legacy_reprocess_candidates,
            cmd_backup_database_now,
            cmd_enqueue_legacy_reprocess
        ])
        .run(tauri::generate_context!())
        .expect("Error running Mab-Compass Tauri v2 application");
}
