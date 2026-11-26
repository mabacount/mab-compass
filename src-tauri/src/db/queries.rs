use crate::models::*;
use crate::vault_manager::ensure_project_directories;
use chrono::Local;
use rusqlite::{params, Connection, Result};
use std::path::Path;

pub fn get_settings(conn: &Connection) -> Result<AppSettings> {
    conn.query_row(
        "SELECT vault_root_path, mab_focus_data_path, auto_sync_mab_focus, active_llm_provider, agy_model, gemini_model, local_model, gemini_api_key, local_endpoint
         FROM app_settings WHERE id = 1",
        [],
        |r| {
            Ok(AppSettings {
                vault_root_path: r.get(0)?,
                mab_focus_data_path: r.get(1)?,
                auto_sync_mab_focus: r.get::<_, i32>(2)? == 1,
                active_llm_provider: r.get(3)?,
                agy_model: r.get(4)?,
                gemini_model: r.get(5)?,
                local_model: r.get(6)?,
                gemini_api_key: r.get(7)?,
                local_endpoint: r.get(8)?,
            })
        },
    )
}

pub fn update_settings(conn: &Connection, s: &AppSettings) -> Result<()> {
    conn.execute(
        "UPDATE app_settings SET
            vault_root_path = ?1,
            mab_focus_data_path = ?2,
            auto_sync_mab_focus = ?3,
            active_llm_provider = ?4,
            agy_model = ?5,
            gemini_model = ?6,
            local_model = ?7,
            gemini_api_key = ?8,
            local_endpoint = ?9
         WHERE id = 1",
        params![
            s.vault_root_path,
            s.mab_focus_data_path,
            if s.auto_sync_mab_focus { 1 } else { 0 },
            s.active_llm_provider,
            s.agy_model,
            s.gemini_model,
            s.local_model,
            s.gemini_api_key,
            s.local_endpoint
        ],
    )?;
    Ok(())
}

pub fn create_new_project(conn: &Connection, id_code: &str, name: &str) -> Result<Project> {
    create_new_project_with_options(conn, id_code, name, None, None, None, None)
}

pub fn create_new_project_with_options(
    conn: &Connection,
    id_code: &str,
    name: &str,
    parent_id: Option<&str>,
    realm: Option<&str>,
    color: Option<&str>,
    lifecycle_status: Option<&str>,
) -> Result<Project> {
    let settings = get_settings(conn)?;
    let raw_code: String = id_code
        .trim()
        .to_uppercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let clean_id = if raw_code.trim_matches('_').is_empty() {
        format!("PRJ_{}", &uuid::Uuid::new_v4().to_string()[..4].to_uppercase())
    } else {
        raw_code.trim_matches('_').to_string()
    };

    let clean_name = name.trim();
    let display_name = if clean_name.is_empty() {
        clean_id.clone()
    } else {
        clean_name.to_string()
    };

    let workspace_path = Path::new(&settings.vault_root_path)
        .join(&clean_id)
        .to_string_lossy()
        .to_string();

    let _ = ensure_project_directories(&workspace_path);
    let now = chrono::Utc::now().to_rfc3339();
    let ext_uuid = uuid::Uuid::new_v4().to_string();
    let norm_realm = if realm.unwrap_or("WORK").trim().eq_ignore_ascii_case("LIFE") {
        "LIFE"
    } else {
        "WORK"
    };
    let norm_color = color
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(if norm_realm == "LIFE" { "#10B981" } else { "#3B82F6" });
    let norm_status = match lifecycle_status
        .unwrap_or("ACTIVE")
        .trim()
        .to_ascii_uppercase()
        .as_str()
    {
        "PAUSED" | "SUSPENDED" => "PAUSED",
        "ARCHIVED" => "ARCHIVED",
        _ => "ACTIVE",
    };
    let clean_parent = parent_id
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != clean_id)
        .map(str::to_string);

    conn.execute("UPDATE projects SET is_active = 0", [])?;
    conn.execute(
        "INSERT OR REPLACE INTO projects (
            id, name, workspace_path, external_focus_id, is_active, updated_at,
            realm, lifecycle_status, parent_id, color, created_at
         ) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8, ?9, ?5)",
        params![
            clean_id,
            display_name,
            workspace_path,
            ext_uuid,
            now,
            norm_realm,
            norm_status,
            clean_parent,
            norm_color,
        ],
    )?;

    Ok(Project {
        id: clean_id,
        name: display_name,
        workspace_path,
        external_focus_id: Some(ext_uuid),
        is_active: true,
        updated_at: now.clone(),
        realm: norm_realm.to_string(),
        lifecycle_status: norm_status.to_string(),
        parent_id: clean_parent,
        color: norm_color.to_string(),
        created_at: now,
        ..Default::default()
    })
}

#[allow(clippy::too_many_arguments)]
pub fn update_project_details(
    conn: &Connection,
    project_id: &str,
    name: Option<&str>,
    parent_id: Option<&str>,
    clear_parent: bool,
    realm: Option<&str>,
    lifecycle_status: Option<&str>,
    color: Option<&str>,
    daily_quota_minutes: Option<i32>,
    weekly_quota_minutes: Option<i32>,
    monthly_quota_minutes: Option<i32>,
    hourly_rate: Option<f64>,
) -> Result<Project> {
    let existing = crate::ecosystem_core::load_compass_projects(conn)
        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?
        .into_iter()
        .find(|p| p.id == project_id)
        .ok_or(rusqlite::Error::QueryReturnedNoRows)?;

    let next_name = name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&existing.name)
        .to_string();
    let next_parent = if clear_parent {
        None
    } else if let Some(pid) = parent_id {
        let trimmed = pid.trim();
        if trimmed.is_empty() || trimmed == project_id {
            None
        } else {
            Some(trimmed.to_string())
        }
    } else {
        existing.parent_id.clone()
    };
    let next_realm = match realm.map(str::trim) {
        Some(r) if r.eq_ignore_ascii_case("LIFE") => "LIFE".to_string(),
        Some(r) if r.eq_ignore_ascii_case("WORK") => "WORK".to_string(),
        _ => existing.realm.clone(),
    };
    let next_status = match lifecycle_status.map(|s| s.trim().to_ascii_uppercase()) {
        Some(s) if s == "PAUSED" || s == "SUSPENDED" => "PAUSED".to_string(),
        Some(s) if s == "ARCHIVED" => "ARCHIVED".to_string(),
        Some(s) if s == "ACTIVE" => "ACTIVE".to_string(),
        _ => existing.lifecycle_status.clone(),
    };
    let next_color = color
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&existing.color)
        .to_string();
    let next_daily = daily_quota_minutes.unwrap_or(existing.daily_quota_minutes);
    let next_weekly = weekly_quota_minutes.unwrap_or(existing.weekly_quota_minutes);
    let next_monthly = monthly_quota_minutes.unwrap_or(existing.monthly_quota_minutes);
    let next_rate = hourly_rate.unwrap_or(existing.hourly_rate);
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE projects SET
            name = ?1, parent_id = ?2, realm = ?3, lifecycle_status = ?4,
            color = ?5, daily_quota_minutes = ?6, weekly_quota_minutes = ?7,
            monthly_quota_minutes = ?8, hourly_rate = ?9, updated_at = ?10
         WHERE id = ?11",
        params![
            next_name,
            next_parent,
            next_realm,
            next_status,
            next_color,
            next_daily,
            next_weekly,
            next_monthly,
            next_rate,
            now,
            project_id,
        ],
    )?;

    crate::ecosystem_core::load_compass_projects(conn)
        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?
        .into_iter()
        .find(|p| p.id == project_id)
        .ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn delete_project_local(conn: &mut Connection, project_id: &str) -> Result<Vec<String>> {
    let all = crate::ecosystem_core::load_compass_projects(conn)
        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
    if all.len() <= 1 {
        return Ok(Vec::new());
    }
    let mut to_delete = vec![project_id.to_string()];
    let mut i = 0;
    while i < to_delete.len() {
        let cur = to_delete[i].clone();
        for p in &all {
            if p.parent_id.as_deref() == Some(cur.as_str()) && !to_delete.contains(&p.id) {
                to_delete.push(p.id.clone());
            }
        }
        i += 1;
    }

    let mut deleted_core_ids = Vec::new();
    for pid in &to_delete {
        if let Some(proj) = all.iter().find(|p| p.id == *pid) {
            if let Some(ref ext) = proj.external_focus_id {
                deleted_core_ids.push(ext.clone());
            }
        }
        conn.execute("DELETE FROM projects WHERE id = ?1", params![pid])?;
    }

    let active_cnt: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects WHERE is_active = 1", [], |r| r.get(0))
        .unwrap_or(0);
    if active_cnt == 0 {
        let _ = conn.execute(
            "UPDATE projects SET is_active = 1 WHERE id = (SELECT id FROM projects ORDER BY id LIMIT 1)",
            [],
        );
    }
    Ok(deleted_core_ids)
}

pub fn get_dashboard_state(conn: &Connection) -> Result<DashboardState> {
    get_dashboard_state_for_project(conn, None)
}

pub fn get_dashboard_state_for_project(
    conn: &Connection,
    project_id_override: Option<&str>,
) -> Result<DashboardState> {
    let settings = get_settings(conn)?;
    let projects = crate::ecosystem_core::load_compass_projects(conn)
        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

    let active_project = if let Some(req_id) = project_id_override.map(str::trim).filter(|s| !s.is_empty()) {
        projects
            .iter()
            .find(|p| p.id.eq_ignore_ascii_case(req_id) || p.name.trim() == req_id)
            .cloned()
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?
    } else {
        projects
            .iter()
            .find(|p| p.is_active)
            .cloned()
            .or_else(|| projects.first().cloned())
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?
    };

    let _ = ensure_project_directories(&active_project.workspace_path);
    let pid = &active_project.id;

    let domains = query_all_domains(conn).unwrap_or_default();
    let outcomes = query_all_outcomes(conn).unwrap_or_default();
    let work_lines = query_all_work_lines(conn, pid).unwrap_or_default();

    let mut open_tasks = query_entities(
        conn,
        pid,
        "TASK",
        &["OPEN", "IN_PROGRESS", "BLOCKED", "QUEUED_IN_CHAIN"],
    )?;

    // Enrich tasks with project_name, work_line_title, inherited outcome_id, and live chain lock state
    for task in &mut open_tasks {
        task.project_name = active_project.name.clone();
        if let Some(ref wl_id) = task.work_line_id {
            if let Some(wl) = work_lines.iter().find(|w| w.id == *wl_id) {
                task.work_line_title = Some(wl.title.clone());
                if task.outcome_id.is_none() {
                    task.outcome_id = wl.outcome_id.clone();
                }
            }
            if task.status == "QUEUED_IN_CHAIN"
                || is_locked_by_chain(conn, &task.id).unwrap_or(false)
            {
                task.is_locked_by_chain = true;
            }
        } else if task.status == "QUEUED_IN_CHAIN" {
            task.is_locked_by_chain = true;
        }
    }

    let closed_tasks_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM work_entities WHERE project_id = ?1 AND entity_type = 'TASK' AND status = 'CLOSED'",
            params![pid],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let active_decisions = query_entities(conn, pid, "DECISION", &["OPEN", "IN_PROGRESS"])?;

    let unblocked_tasks: Vec<WorkEntity> = open_tasks
        .iter()
        .filter(|t| {
            t.status != "BLOCKED"
                && t.status != "QUEUED_IN_CHAIN"
                && !t.is_locked_by_chain
        })
        .cloned()
        .collect();

    // Select up to 3 top priority steps across distinct WorkLines (1 active/next unlocked task per WorkLine)
    let mut top_priority_steps: Vec<WorkEntity> = Vec::new();
    let mut used_task_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut used_line_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Gather candidate task per active WorkLine
    let mut line_candidates: Vec<(bool, i32, WorkEntity)> = work_lines
        .iter()
        .filter(|wl| wl.status != "COMPLETED" && wl.status != "ARCHIVED")
        .filter_map(|wl| {
            let candidate = open_tasks
                .iter()
                .filter(|t| t.work_line_id.as_deref() == Some(wl.id.as_str()))
                .filter(|t| {
                    t.status != "BLOCKED"
                        && t.status != "QUEUED_IN_CHAIN"
                        && !t.is_locked_by_chain
                })
                .find(|t| t.status == "IN_PROGRESS")
                .or_else(|| {
                    open_tasks
                        .iter()
                        .filter(|t| t.work_line_id.as_deref() == Some(wl.id.as_str()))
                        .filter(|t| {
                            t.status != "BLOCKED"
                                && t.status != "QUEUED_IN_CHAIN"
                                && !t.is_locked_by_chain
                        })
                        .min_by_key(|t| t.sequence_order)
                })
                .cloned()?;
            let is_in_progress = candidate.status == "IN_PROGRESS";
            let score = candidate.priority_score + if candidate.is_on_critical_path { 20 } else { 0 };
            Some((is_in_progress, score, candidate))
        })
        .collect();

    line_candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));

    for (_, _, task) in line_candidates {
        if top_priority_steps.len() >= 3 {
            break;
        }
        if let Some(ref wl_id) = task.work_line_id {
            used_line_ids.insert(wl_id.clone());
        }
        used_task_ids.insert(task.id.clone());
        top_priority_steps.push(task);
    }

    // Fill remaining priority slots (up to 3) from standalone or other unblocked tasks
    if top_priority_steps.len() < 3 {
        for t in &unblocked_tasks {
            if top_priority_steps.len() >= 3 {
                break;
            }
            if used_task_ids.contains(&t.id) {
                continue;
            }
            if let Some(ref wl_id) = t.work_line_id {
                if used_line_ids.contains(wl_id) {
                    continue;
                }
                used_line_ids.insert(wl_id.clone());
            }
            used_task_ids.insert(t.id.clone());
            top_priority_steps.push(t.clone());
        }
    }

    let next_step_task = unblocked_tasks
        .iter()
        .find(|t| t.status == "IN_PROGRESS" && t.is_on_critical_path)
        .or_else(|| unblocked_tasks.iter().find(|t| t.status == "IN_PROGRESS"))
        .or_else(|| top_priority_steps.first())
        .or_else(|| unblocked_tasks.first())
        .cloned();

    let horizon_tasks: Vec<WorkEntity> = unblocked_tasks
        .into_iter()
        .filter(|t| next_step_task.as_ref().map(|ns| ns.id != t.id).unwrap_or(true))
        .take(3)
        .collect();

    let active_cat = next_step_task
        .as_ref()
        .map(|t| t.category.clone())
        .unwrap_or_else(|| "GENERAL".to_string());

    let mut anti_stmt = conn.prepare(
        "SELECT id, project_id, trigger_category, mistake_pattern, preventive_gate_question, severity_score, times_triggered
         FROM learned_antipatterns
         WHERE project_id = ?1 AND is_active = 1 AND (trigger_category = ?2 OR trigger_category = 'GENERAL')
         ORDER BY severity_score DESC, times_triggered DESC LIMIT 3",
    )?;
    let active_guardrails: Vec<LearnedAntipattern> = anti_stmt
        .query_map(params![pid, active_cat], |r| {
            Ok(LearnedAntipattern {
                id: r.get(0)?,
                project_id: r.get(1)?,
                trigger_category: r.get(2)?,
                mistake_pattern: r.get(3)?,
                preventive_gate_question: r.get(4)?,
                severity_score: r.get(5)?,
                times_triggered: r.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut sop_stmt = conn.prepare(
        "SELECT id, project_id, category, rule_statement, updated_at
         FROM sop_rules WHERE project_id = ?1 AND is_active = 1 ORDER BY category, id DESC",
    )?;
    let active_sops: Vec<SopRule> = sop_stmt
        .query_map(params![pid], |r| {
            Ok(SopRule {
                id: r.get(0)?,
                project_id: r.get(1)?,
                category: r.get(2)?,
                rule_statement: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut log_stmt = conn.prepare(
        "SELECT id, project_id, task_id, category, estimated_pomos, effective_pomos, deliverable_summary, internal_friction_note, client_sanitized_note, source_origin, logged_at
         FROM session_work_logs WHERE project_id = ?1 ORDER BY id DESC LIMIT 20",
    )?;
    let recent_logs: Vec<SessionWorkLog> = log_stmt
        .query_map(params![pid], |r| {
            Ok(SessionWorkLog {
                id: r.get(0)?,
                project_id: r.get(1)?,
                task_id: r.get(2)?,
                category: r.get(3)?,
                estimated_pomos: r.get(4)?,
                effective_pomos: r.get(5)?,
                deliverable_summary: r.get(6)?,
                internal_friction_note: r.get(7)?,
                client_sanitized_note: r.get(8)?,
                source_origin: r.get(9)?,
                logged_at: r.get(10)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let active_chat_scope: String = if project_id_override.is_some() {
        "PROJECT".to_string()
    } else {
        get_active_chat_scope(conn)
    };
    let chat_pid = if active_chat_scope == "GLOBAL" {
        GLOBAL_CHAT_PROJECT_ID
    } else {
        pid.as_str()
    };

    let active_copilot_session_id = ensure_active_chat_session(conn, chat_pid, "COPILOT")?;
    let active_vault_session_id = ensure_active_chat_session(conn, chat_pid, "VAULT")?;

    let chat_history = query_session_messages(conn, chat_pid, &active_copilot_session_id, false)?;
    let vault_chat_history = query_session_messages(conn, chat_pid, &active_vault_session_id, true)?;
    let chat_sessions = query_chat_sessions(conn, Some(chat_pid))?;
    let all_project_chat_sessions = query_chat_sessions(conn, None)?;

    let raw_events_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM raw_events WHERE project_id = ?1",
            params![pid],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut raw_stmt = conn.prepare(
        "SELECT id, source_type, raw_content, archived_path, created_at
         FROM raw_events WHERE project_id = ?1 ORDER BY id DESC LIMIT 15",
    )?;
    let recent_raw_events: Vec<RawEventPreview> = raw_stmt
        .query_map(params![pid], |r| {
            let full: String = r.get(2)?;
            let first_clean: String = full.trim().chars().take(160).collect();
            let snippet = if full.trim().chars().count() > 160 {
                format!("{}…", first_clean.trim())
            } else {
                first_clean
            };
            Ok(RawEventPreview {
                id: r.get(0)?,
                source_type: r.get(1)?,
                snippet,
                archived_path: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut doc_stmt = conn.prepare(
        "SELECT id, project_id, title, doc_type, original_filename, archived_path,
                executive_summary, extracted_items_count, ingested_at,
                binary_original_path, tags_json, realm, domain_code,
                processing_status, processed_at
         FROM knowledge_documents WHERE project_id = ?1 ORDER BY id DESC",
    )?;
    let knowledge_docs: Vec<KnowledgeDocument> = doc_stmt
        .query_map(params![pid], |r| {
            let tags_json: String = r.get(10).unwrap_or_else(|_| "[]".to_string());
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(KnowledgeDocument {
                id: r.get(0)?,
                project_id: r.get(1)?,
                title: r.get(2)?,
                doc_type: r.get(3)?,
                original_filename: r.get(4)?,
                archived_path: r.get(5)?,
                executive_summary: r.get(6)?,
                extracted_items_count: r.get(7)?,
                ingested_at: r.get(8)?,
                binary_original_path: r.get(9).ok(),
                tags_json,
                tags,
                realm: r.get(11).unwrap_or_else(|_| "WORK".to_string()),
                domain_code: r.get(12).unwrap_or_else(|_| "WEALTH_FINANCE".to_string()),
                processing_status: r.get(13).unwrap_or_else(|_| "UNPROCESSED".to_string()),
                processed_at: r.get(14).ok(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let last_undo: Option<String> = conn
        .query_row(
            "SELECT summary_text FROM mutation_audit_logs WHERE project_id = ?1 ORDER BY id DESC LIMIT 1",
            params![pid],
            |r| r.get(0),
        )
        .ok();

    let pending_queue = query_pending_ai_queue(conn).unwrap_or_default();

    let calendar_days = query_all_calendar_days(conn).unwrap_or_default();
    let pending_proposals = query_all_pending_proposals(conn).unwrap_or_default();
    let personal_vault_items = query_all_personal_vault_items(conn).unwrap_or_default();
    let methods_playbooks =
        crate::method_engine::query_all_methods_playbooks(conn, pid).unwrap_or_default();
    let (portfolio_projects, cross_project_priorities) = if project_id_override.is_none() {
        query_portfolio_path_summary(conn, &projects).unwrap_or_default()
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(DashboardState {
        projects,
        active_project,
        next_step_task,
        horizon_tasks,
        active_guardrails,
        active_sops,
        open_tasks,
        closed_tasks_count,
        active_decisions,
        recent_logs,
        chat_history,
        vault_chat_history,
        chat_sessions,
        all_project_chat_sessions,
        active_copilot_session_id,
        active_vault_session_id,
        raw_events_count,
        recent_raw_events,
        knowledge_docs,
        pending_queue,
        settings,
        can_undo: last_undo.is_some(),
        last_undo_summary: last_undo,
        domains,
        outcomes,
        work_lines,
        top_priority_steps,
        calendar_days,
        pending_proposals,
        personal_vault_items,
        methods_playbooks,
        active_chat_scope,
        portfolio_projects,
        cross_project_priorities,
    })
}

pub fn query_portfolio_path_summary(
    conn: &Connection,
    projects: &[Project],
) -> Result<(Vec<ProjectPathSummary>, Vec<WorkEntity>)> {
    let mut summaries = Vec::new();
    let mut global_candidates: Vec<(bool, bool, i32, String, String, WorkEntity)> = Vec::new();

    for proj in projects {
        let is_archived = proj.lifecycle_status.eq_ignore_ascii_case("ARCHIVED");
        if is_archived && !proj.is_active {
            continue;
        }
        let pid = &proj.id;
        let work_lines = query_all_work_lines(conn, pid).unwrap_or_default();
        let active_wl_count = work_lines
            .iter()
            .filter(|w| !w.status.eq_ignore_ascii_case("COMPLETED") && !w.status.eq_ignore_ascii_case("ARCHIVED"))
            .count() as i32;

        let mut open_tasks = query_entities(
            conn,
            pid,
            "TASK",
            &["OPEN", "IN_PROGRESS", "BLOCKED", "QUEUED_IN_CHAIN"],
        )
        .unwrap_or_default();

        for task in &mut open_tasks {
            task.project_name = proj.name.clone();
            if let Some(ref wl_id) = task.work_line_id {
                if let Some(wl) = work_lines.iter().find(|w| w.id == *wl_id) {
                    task.work_line_title = Some(wl.title.clone());
                    if task.outcome_id.is_none() {
                        task.outcome_id = wl.outcome_id.clone();
                    }
                }
                if task.status == "QUEUED_IN_CHAIN"
                    || is_locked_by_chain(conn, &task.id).unwrap_or(false)
                {
                    task.is_locked_by_chain = true;
                }
            } else if task.status == "QUEUED_IN_CHAIN" {
                task.is_locked_by_chain = true;
            }
        }

        let unblocked: Vec<WorkEntity> = open_tasks
            .iter()
            .filter(|t| {
                t.status != "BLOCKED"
                    && t.status != "QUEUED_IN_CHAIN"
                    && !t.is_locked_by_chain
            })
            .cloned()
            .collect();

        let next_step_task = unblocked
            .iter()
            .find(|t| t.status == "IN_PROGRESS" && t.is_on_critical_path)
            .or_else(|| unblocked.iter().find(|t| t.status == "IN_PROGRESS"))
            .or_else(|| unblocked.first())
            .cloned();

        let horizon_tasks: Vec<WorkEntity> = unblocked
            .iter()
            .filter(|t| next_step_task.as_ref().map(|ns| ns.id != t.id).unwrap_or(true))
            .take(3)
            .cloned()
            .collect();

        let mut blockers = query_entities(conn, pid, "BLOCKER", &["OPEN", "IN_PROGRESS", "BLOCKED"])
            .unwrap_or_default();
        for b in &mut blockers {
            b.project_name = proj.name.clone();
        }

        let open_tasks_count = open_tasks.len() as i32;
        let in_progress_count = open_tasks.iter().filter(|t| t.status == "IN_PROGRESS").count() as i32;

        if !proj.lifecycle_status.eq_ignore_ascii_case("ARCHIVED")
            && !proj.lifecycle_status.eq_ignore_ascii_case("PAUSED")
        {
            for task in &unblocked {
                let is_ip = task.status == "IN_PROGRESS";
                let sched = task.scheduled_date.clone().unwrap_or_else(|| "9999-99-99".to_string());
                let due = task.due_date.clone().unwrap_or_else(|| "9999-99-99".to_string());
                global_candidates.push((
                    is_ip,
                    task.is_on_critical_path,
                    task.priority_score,
                    sched,
                    due,
                    task.clone(),
                ));
            }
        }

        summaries.push(ProjectPathSummary {
            project: proj.clone(),
            next_step_task,
            horizon_tasks,
            blockers,
            open_tasks_count,
            in_progress_count,
            active_work_lines_count: active_wl_count,
        });
    }

    global_candidates.sort_by(|a, b| {
        b.2.cmp(&a.2) // priority_score DESC
            .then_with(|| b.0.cmp(&a.0)) // IN_PROGRESS DESC
            .then_with(|| b.1.cmp(&a.1)) // is_on_critical_path DESC
            .then_with(|| a.3.cmp(&b.3)) // earliest scheduled_date ASC
            .then_with(|| a.4.cmp(&b.4)) // earliest due_date ASC
    });

    let cross_project_priorities = global_candidates
        .into_iter()
        .take(30)
        .map(|(_, _, _, _, _, t)| t)
        .collect();

    Ok((summaries, cross_project_priorities))
}

pub fn query_all_domains(conn: &Connection) -> Result<Vec<LifeDomain>> {
    let mut stmt = conn.prepare(
        "SELECT code, title, description, maturity_state, maintenance_floor, desired_state, is_growth_focus, updated_at
         FROM life_domains ORDER BY code",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(LifeDomain {
            code: r.get(0)?,
            title: r.get(1)?,
            description: r.get(2).unwrap_or_default(),
            maturity_state: r.get(3).unwrap_or(2),
            maintenance_floor: r.get(4).unwrap_or_default(),
            desired_state: r.get(5).unwrap_or_default(),
            is_growth_focus: r.get::<_, i32>(6).unwrap_or(0) == 1,
            updated_at: r.get(7).unwrap_or_default(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn query_all_outcomes(conn: &Connection) -> Result<Vec<Outcome>> {
    let mut stmt = conn.prepare(
        "SELECT id, domain_code, title, desired_reality_change, evidence_kr, time_window, status, progress_percent, created_at, updated_at
         FROM outcomes ORDER BY id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Outcome {
            id: r.get(0)?,
            domain_code: r.get(1)?,
            title: r.get(2)?,
            desired_reality_change: r.get(3).unwrap_or_default(),
            evidence_kr: r.get(4).unwrap_or_default(),
            time_window: r.get(5).unwrap_or_default(),
            status: r.get(6).unwrap_or_else(|_| "ACTIVE".to_string()),
            progress_percent: r.get(7).unwrap_or(0),
            linked_projects_count: 0,
            created_at: r.get(8).unwrap_or_default(),
            updated_at: r.get(9).unwrap_or_default(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn query_all_work_lines(conn: &Connection, project_id: &str) -> Result<Vec<WorkLine>> {
    let mut stmt = conn.prepare(
        "SELECT wl.id,
                wl.project_id,
                COALESCE(p.name, wl.project_id) AS project_name,
                wl.outcome_id,
                o.title AS outcome_title,
                wl.title,
                wl.goal_statement,
                wl.line_mode,
                wl.status,
                wl.color_hex,
                (SELECT COUNT(*) FROM work_entities w
                  WHERE w.work_line_id = wl.id AND w.entity_type = 'TASK' AND w.status != 'SUPERSEDED') AS total_tasks,
                (SELECT COUNT(*) FROM work_entities w
                  WHERE w.work_line_id = wl.id AND w.entity_type = 'TASK' AND w.status = 'CLOSED') AS completed_tasks,
                (SELECT w.id FROM work_entities w
                  WHERE w.work_line_id = wl.id AND w.entity_type = 'TASK' AND w.status IN ('IN_PROGRESS', 'OPEN')
                  ORDER BY CASE WHEN w.status = 'IN_PROGRESS' THEN 0 ELSE 1 END ASC, w.sequence_order ASC
                  LIMIT 1) AS active_step_id,
                wl.created_at,
                wl.updated_at
         FROM work_lines wl
         LEFT JOIN projects p ON p.id = wl.project_id
         LEFT JOIN outcomes o ON o.id = wl.outcome_id
         WHERE wl.project_id = ?1
         ORDER BY wl.created_at ASC, wl.id ASC",
    )?;
    let rows = stmt.query_map(params![project_id], |r| {
        Ok(WorkLine {
            id: r.get(0)?,
            project_id: r.get(1)?,
            project_name: r.get(2).unwrap_or_default(),
            outcome_id: r.get(3).ok(),
            outcome_title: r.get(4).ok(),
            title: r.get(5)?,
            goal_statement: r.get(6).unwrap_or_default(),
            line_mode: r.get(7).unwrap_or_else(|_| "SEQUENTIAL_CHAIN".to_string()),
            status: r.get(8).unwrap_or_else(|_| "ACTIVE".to_string()),
            color_hex: r.get(9).unwrap_or_else(|_| "#3B82F6".to_string()),
            total_tasks: r.get(10).unwrap_or(0),
            completed_tasks: r.get(11).unwrap_or(0),
            active_step_id: r.get(12).ok(),
            created_at: r.get(13).unwrap_or_default(),
            updated_at: r.get(14).unwrap_or_default(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn query_all_calendar_days(conn: &Connection) -> Result<Vec<CalendarDay>> {
    let mut stmt = conn.prepare(
        "SELECT c.date_key,
                c.total_capacity_minutes,
                c.deep_capacity_minutes,
                c.day_note,
                c.is_off_day,
                (SELECT COUNT(*) FROM work_entities w
                  WHERE w.scheduled_date = c.date_key AND w.entity_type = 'TASK' AND w.status NOT IN ('CLOSED', 'SUPERSEDED')) AS assigned_tasks_count,
                (SELECT COALESCE(SUM(w.duration_minutes), 0) FROM work_entities w
                  WHERE w.scheduled_date = c.date_key AND w.entity_type = 'TASK' AND w.status NOT IN ('CLOSED', 'SUPERSEDED')) AS assigned_minutes,
                c.updated_at
         FROM calendar_days c
         ORDER BY c.date_key DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(CalendarDay {
            date_key: r.get(0)?,
            total_capacity_minutes: r.get(1).unwrap_or(300),
            deep_capacity_minutes: r.get(2).unwrap_or(150),
            day_note: r.get(3).unwrap_or_default(),
            is_off_day: r.get::<_, i32>(4).unwrap_or(0) == 1,
            assigned_tasks_count: r.get(5).unwrap_or(0),
            assigned_minutes: r.get(6).unwrap_or(0),
            updated_at: r.get(7).unwrap_or_default(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn query_all_pending_proposals(conn: &Connection) -> Result<Vec<PendingProposal>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at, resolved_at
         FROM pending_proposals ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(PendingProposal {
            id: r.get(0)?,
            project_id: r.get(1).ok(),
            proposal_type: r.get(2)?,
            title: r.get(3)?,
            rationale_markdown: r.get(4).unwrap_or_default(),
            payload_json: r.get(5).unwrap_or_default(),
            status: r.get(6).unwrap_or_else(|_| "PENDING".to_string()),
            created_at: r.get(7).unwrap_or_default(),
            resolved_at: r.get(8).ok(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn query_all_personal_vault_items(conn: &Connection) -> Result<Vec<PersonalVaultItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, realm, domain_code, project_id, item_type, title, content_markdown, tags_json, archived_path, created_at, updated_at
         FROM personal_vault_items ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        let tags_str: String = r.get(7).unwrap_or_else(|_| "[]".to_string());
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        Ok(PersonalVaultItem {
            id: r.get(0)?,
            realm: r.get(1).unwrap_or_else(|_| "LIFE".to_string()),
            domain_code: r.get(2).unwrap_or_else(|_| "MIND_LEARNING".to_string()),
            project_id: r.get(3).ok(),
            item_type: r.get(4)?,
            title: r.get(5)?,
            content_markdown: r.get(6).unwrap_or_default(),
            tags_json: tags_str,
            tags,
            archived_path: r.get(8).ok(),
            created_at: r.get(9).unwrap_or_default(),
            updated_at: r.get(10).unwrap_or_default(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn normalize_chat_channel(channel: &str) -> &'static str {
    if channel.trim().eq_ignore_ascii_case("VAULT") {
        "VAULT"
    } else {
        "COPILOT"
    }
}

fn default_session_title(channel: &str) -> &'static str {
    if normalize_chat_channel(channel) == "VAULT" {
        "گفتگوی جدید"
    } else {
        "چت جدید"
    }
}

fn derive_session_title(text: &str, fallback: &str) -> String {
    let first_line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    if first_line.is_empty() {
        return fallback.to_string();
    }
    let clipped: String = first_line.chars().take(42).collect();
    if first_line.chars().count() > 42 {
        format!("{}…", clipped.trim())
    } else {
        clipped
    }
}

pub const GLOBAL_CHAT_PROJECT_ID: &str = "GLOBAL";

pub fn get_active_chat_scope(conn: &Connection) -> String {
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "app_settings",
        "active_chat_scope",
        "ALTER TABLE app_settings ADD COLUMN active_chat_scope TEXT NOT NULL DEFAULT 'GLOBAL'",
    );
    conn.query_row(
        "SELECT COALESCE(active_chat_scope, 'GLOBAL') FROM app_settings WHERE id = 1",
        [],
        |r| r.get::<_, String>(0),
    )
    .unwrap_or_else(|_| "GLOBAL".to_string())
}

pub fn set_active_chat_scope(conn: &Connection, scope: &str) {
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "app_settings",
        "active_chat_scope",
        "ALTER TABLE app_settings ADD COLUMN active_chat_scope TEXT NOT NULL DEFAULT 'GLOBAL'",
    );
    let norm = if scope.eq_ignore_ascii_case("GLOBAL") {
        "GLOBAL"
    } else {
        "PROJECT"
    };
    let _ = conn.execute(
        "UPDATE app_settings SET active_chat_scope = ?1 WHERE id = 1",
        params![norm],
    );
}

pub fn ensure_active_chat_session(
    conn: &Connection,
    project_id: &str,
    channel: &str,
) -> Result<String> {
    let norm_ch = normalize_chat_channel(channel);
    if let Ok(sid) = conn.query_row(
        "SELECT id FROM chat_sessions
         WHERE project_id = ?1 AND channel = ?2 AND is_active = 1
         ORDER BY updated_at DESC LIMIT 1",
        params![project_id, norm_ch],
        |r| r.get::<_, String>(0),
    ) {
        return Ok(sid);
    }

    if let Ok(sid) = conn.query_row(
        "SELECT id FROM chat_sessions
         WHERE project_id = ?1 AND channel = ?2
         ORDER BY updated_at DESC LIMIT 1",
        params![project_id, norm_ch],
        |r| r.get::<_, String>(0),
    ) {
        conn.execute(
            "UPDATE chat_sessions SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END
             WHERE project_id = ?2 AND channel = ?3",
            params![sid, project_id, norm_ch],
        )?;
        return Ok(sid);
    }

    let now = Local::now().to_rfc3339();
    let sid = format!(
        "SES-{}",
        &uuid::Uuid::new_v4().to_string()[..8].to_uppercase()
    );
    let title = default_session_title(norm_ch);
    conn.execute(
        "INSERT INTO chat_sessions (id, project_id, channel, title, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
        params![sid, project_id, norm_ch, title, now],
    )?;
    Ok(sid)
}

pub fn create_chat_session(
    conn: &Connection,
    project_id: Option<&str>,
    channel: &str,
    title: Option<&str>,
) -> Result<ChatSession> {
    let target_pid = if let Some(pid) = project_id.map(str::trim).filter(|s| !s.is_empty()) {
        if pid.eq_ignore_ascii_case(GLOBAL_CHAT_PROJECT_ID) {
            set_active_chat_scope(conn, "GLOBAL");
            GLOBAL_CHAT_PROJECT_ID.to_string()
        } else {
            conn.execute(
                "UPDATE projects SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END",
                params![pid],
            )?;
            set_active_chat_scope(conn, "PROJECT");
            pid.to_string()
        }
    } else {
        set_active_chat_scope(conn, "GLOBAL");
        GLOBAL_CHAT_PROJECT_ID.to_string()
    };

    let norm_ch = normalize_chat_channel(channel).to_string();
    let now = Local::now().to_rfc3339();
    let sid = format!(
        "SES-{}",
        &uuid::Uuid::new_v4().to_string()[..8].to_uppercase()
    );
    let session_title = title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if target_pid == GLOBAL_CHAT_PROJECT_ID {
                "گفتگوی آزاد (همه پروژه‌ها)"
            } else {
                default_session_title(&norm_ch)
            }
        })
        .to_string();

    conn.execute(
        "UPDATE chat_sessions SET is_active = 0 WHERE project_id = ?1 AND channel = ?2",
        params![target_pid, norm_ch],
    )?;
    conn.execute(
        "INSERT INTO chat_sessions (id, project_id, channel, title, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
        params![sid, target_pid, norm_ch, session_title, now],
    )?;

    Ok(ChatSession {
        id: sid,
        project_id: target_pid,
        channel: norm_ch,
        title: session_title,
        is_active: true,
        message_count: 0,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn switch_chat_session(conn: &Connection, session_id: &str) -> Result<(String, String)> {
    let (pid, channel): (String, String) = conn.query_row(
        "SELECT project_id, channel FROM chat_sessions WHERE id = ?1",
        params![session_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;

    if pid == GLOBAL_CHAT_PROJECT_ID {
        set_active_chat_scope(conn, "GLOBAL");
    } else {
        conn.execute(
            "UPDATE projects SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END",
            params![pid],
        )?;
        set_active_chat_scope(conn, "PROJECT");
    }
    conn.execute(
        "UPDATE chat_sessions
         SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END
         WHERE project_id = ?2 AND channel = ?3",
        params![session_id, pid, channel],
    )?;
    Ok((pid, channel))
}

pub fn delete_chat_session(conn: &Connection, session_id: &str) -> Result<()> {
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT project_id, channel FROM chat_sessions WHERE id = ?1",
            params![session_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    if let Some((pid, ch)) = row {
        conn.execute(
            "DELETE FROM pending_ai_queue WHERE session_id = ?1",
            params![session_id],
        )?;
        conn.execute(
            "DELETE FROM chat_messages WHERE session_id = ?1",
            params![session_id],
        )?;
        conn.execute(
            "DELETE FROM chat_sessions WHERE id = ?1",
            params![session_id],
        )?;
        if pid == GLOBAL_CHAT_PROJECT_ID {
            let _ = ensure_active_chat_session(conn, &pid, &ch);
        } else {
            let _ = ensure_active_chat_session(conn, &pid, &ch);
        }
    }
    Ok(())
}

pub fn query_chat_sessions(
    conn: &Connection,
    project_filter: Option<&str>,
) -> Result<Vec<ChatSession>> {
    let map_row = |r: &rusqlite::Row| -> Result<ChatSession> {
        Ok(ChatSession {
            id: r.get(0)?,
            project_id: r.get(1)?,
            channel: r.get(2)?,
            title: r.get(3)?,
            is_active: r.get::<_, i32>(4)? == 1,
            message_count: r.get(5)?,
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
        })
    };

    if let Some(pid) = project_filter {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.project_id, s.channel, s.title, s.is_active,
                    (SELECT COUNT(*) FROM chat_messages m WHERE m.session_id = s.id) AS msg_cnt,
                    s.created_at, s.updated_at
             FROM chat_sessions s
             WHERE s.project_id = ?1
             ORDER BY s.is_active DESC, s.updated_at DESC",
        )?;
        let rows: Vec<ChatSession> = stmt
            .query_map(params![pid], map_row)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    } else {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.project_id, s.channel, s.title, s.is_active,
                    (SELECT COUNT(*) FROM chat_messages m WHERE m.session_id = s.id) AS msg_cnt,
                    s.created_at, s.updated_at
             FROM chat_sessions s
             ORDER BY s.is_active DESC, s.updated_at DESC",
        )?;
        let rows: Vec<ChatSession> = stmt
            .query_map([], map_row)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }
}

fn query_session_messages(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
    vault_only: bool,
) -> Result<Vec<ChatMessage>> {
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "chat_messages",
        "written_artifacts_json",
        "ALTER TABLE chat_messages ADD COLUMN written_artifacts_json TEXT NOT NULL DEFAULT '[]'",
    );
    let sql = if vault_only {
        "SELECT id, project_id, COALESCE(session_id, ''), sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at, COALESCE(written_artifacts_json, '[]')
         FROM (
            SELECT id, project_id, session_id, sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at, written_artifacts_json
            FROM chat_messages
            WHERE project_id = ?1 AND channel = 'VAULT' AND (session_id = ?2 OR COALESCE(session_id, '') = '')
            ORDER BY id DESC LIMIT 80
         ) ORDER BY id ASC"
    } else {
        "SELECT id, project_id, COALESCE(session_id, ''), sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at, COALESCE(written_artifacts_json, '[]')
         FROM (
            SELECT id, project_id, session_id, sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at, written_artifacts_json
            FROM chat_messages
            WHERE project_id = ?1 AND channel != 'VAULT' AND (session_id = ?2 OR COALESCE(session_id, '') = '')
            ORDER BY id DESC LIMIT 80
         ) ORDER BY id ASC"
    };
    let mut stmt = conn.prepare(sql)?;
    let msgs: Vec<ChatMessage> = stmt
        .query_map(params![project_id, session_id], |r| {
            let q_json: String = r.get(8)?;
            let clarifying_questions: Vec<String> =
                serde_json::from_str(&q_json).unwrap_or_default();
            let art_json: String = r.get(10).unwrap_or_else(|_| "[]".into());
            let written_artifacts: Vec<WrittenArtifactRef> =
                serde_json::from_str(&art_json).unwrap_or_default();
            Ok(ChatMessage {
                id: r.get(0)?,
                project_id: r.get(1)?,
                session_id: r.get(2)?,
                sender: r.get(3)?,
                channel: r.get(4)?,
                message_text: r.get(5)?,
                archived_file_path: r.get(6)?,
                applied_mutations_badge: r.get(7)?,
                clarifying_questions,
                written_artifacts,
                created_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(msgs)
}

fn map_work_entity_from_row(r: &rusqlite::Row) -> rusqlite::Result<WorkEntity> {
    let loc_str: String = r.get(23).unwrap_or_else(|_| "[\"DESK\"]".to_string());
    let location_tags: Vec<String> =
        serde_json::from_str(&loc_str).unwrap_or_else(|_| vec!["DESK".to_string()]);
    let is_locked: bool = r
        .get::<_, i32>(28)
        .map(|v| v == 1)
        .unwrap_or_else(|_| r.get::<_, String>(7).map(|s| s == "QUEUED_IN_CHAIN").unwrap_or(false));
    Ok(WorkEntity {
        id: r.get(0)?,
        project_id: r.get(1)?,
        entity_type: r.get(2)?,
        category: r.get(3)?,
        title: r.get(4)?,
        definition_of_done: r.get(5)?,
        first_physical_action: r.get(6)?,
        status: r.get(7)?,
        is_on_critical_path: r.get::<_, i32>(8)? == 1,
        optimistic_pomos: r.get(9)?,
        most_likely_pomos: r.get(10)?,
        pessimistic_pomos: r.get(11)?,
        calibrated_pomos: r.get(12)?,
        superseded_by_id: r.get(13)?,
        stakeholder_tag: r.get(14)?,
        updated_at: r.get(15)?,
        project_name: String::new(),
        work_line_id: r.get(16).ok(),
        work_line_title: None,
        outcome_id: r.get(17).ok(),
        sequence_order: r.get(18).unwrap_or(1),
        depends_on_task_id: r.get(19).ok(),
        is_locked_by_chain: is_locked,
        energy_tag: r.get(20).unwrap_or_else(|_| "MEDIUM_NORMAL".to_string()),
        duration_minutes: r.get(21).unwrap_or(25),
        device_tag: r.get(22).unwrap_or_else(|_| "LAPTOP_ONLY".to_string()),
        location_tags,
        scheduled_date: r.get(24).ok(),
        scheduled_slot: r.get(25).ok(),
        due_date: r.get(26).ok(),
        priority_score: r.get(27).unwrap_or(50),
    })
}

pub fn query_entities(
    conn: &Connection,
    project_id: &str,
    entity_type: &str,
    statuses: &[&str],
) -> Result<Vec<WorkEntity>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos,
                calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at,
                work_line_id, outcome_id, sequence_order, depends_on_task_id, energy_tag, duration_minutes,
                device_tag, location_tags_json, scheduled_date, scheduled_slot, due_date, priority_score
         FROM work_entities
         WHERE project_id = ?1 AND entity_type = ?2
         ORDER BY
            CASE WHEN status = 'IN_PROGRESS' THEN 0
                 WHEN status = 'OPEN' THEN 1
                 WHEN status = 'BLOCKED' THEN 2
                 ELSE 3 END ASC,
            is_on_critical_path DESC,
            priority_score DESC,
            updated_at DESC",
    )?;

    let rows = stmt.query_map(params![project_id, entity_type], map_work_entity_from_row)?;

    Ok(rows
        .filter_map(|r| r.ok())
        .filter(|e| statuses.contains(&e.status.as_str()))
        .collect())
}

fn query_all_project_entities(conn: &Connection, project_id: &str) -> Result<Vec<WorkEntity>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos,
                calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at,
                work_line_id, outcome_id, sequence_order, depends_on_task_id, energy_tag, duration_minutes,
                device_tag, location_tags_json, scheduled_date, scheduled_slot, due_date, priority_score
         FROM work_entities
         WHERE project_id = ?1",
    )?;

    let rows = stmt.query_map(params![project_id], map_work_entity_from_row)?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

fn capture_undo_snapshot(conn: &Connection, project_id: &str) -> Result<String> {
    let entities = query_all_project_entities(conn, project_id)?;
    let max_sop_id: i64 = conn
        .query_row("SELECT COALESCE(MAX(id), 0) FROM sop_rules", [], |r| r.get(0))
        .unwrap_or(0);
    let max_anti_id: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(id), 0) FROM learned_antipatterns",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let max_log_id: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(id), 0) FROM session_work_logs",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let snapshot = UndoSnapshot {
        entities,
        max_sop_id,
        max_anti_id,
        max_log_id,
    };
    Ok(serde_json::to_string(&snapshot).unwrap_or_default())
}

pub fn calculate_category_bias(conn: &Connection, project_id: &str, category: &str) -> f64 {
    let res: Result<(f64, f64)> = conn.query_row(
        "SELECT COALESCE(SUM(effective_pomos), 0.0), COALESCE(SUM(estimated_pomos), 0.0)
         FROM session_work_logs WHERE project_id = ?1 AND category = ?2",
        params![project_id, category],
        |r| Ok((r.get(0)?, r.get(1)?)),
    );
    if let Ok((eff, est)) = res {
        if est > 0.5 {
            return (eff / est).clamp(0.8, 3.0);
        }
    }
    1.0
}

/// تکمیل سریع گام فعلی با یک کلیک از روی کارت مسیر پیش‌رو
pub fn quick_complete_task(
    conn: &mut Connection,
    project_id: &str,
    task_id: &str,
    pomos_spent: i32,
) -> Result<String> {
    let now = Local::now().to_rfc3339();
    let direct_info: Option<(String, String, String, f64)> = conn
        .query_row(
            "SELECT project_id, title, category, calibrated_pomos FROM work_entities WHERE id = ?1 AND project_id = ?2",
            params![task_id, project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    let task_info = direct_info.or_else(|| {
        conn.query_row(
            "SELECT project_id, title, category, calibrated_pomos FROM work_entities WHERE id = ?1 LIMIT 1",
            params![task_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok()
    });

    if let Some((owner_pid, title, cat, est)) = task_info {
        let snapshot_json = capture_undo_snapshot(conn, &owner_pid)?;
        let pomos = pomos_spent.max(1);
        let note = format!("تکمیل و تحویل گام [{}]: {}", task_id, title);

        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE work_entities SET status = 'CLOSED', updated_at = ?1 WHERE id = ?2 AND project_id = ?3",
            params![now, task_id, owner_pid],
        )?;
        tx.execute(
            "INSERT INTO session_work_logs (
                project_id, task_id, category, estimated_pomos, effective_pomos,
                deliverable_summary, internal_friction_note, client_sanitized_note, source_origin, logged_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', ?6, 'QUICK_COMPLETE', ?7)",
            params![owner_pid, task_id, cat, est, pomos, note, now],
        )?;
        tx.execute(
            "INSERT INTO mutation_audit_logs (project_id, summary_text, snapshot_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                owner_pid,
                format!("تکمیل سریع گام [{}]", task_id),
                snapshot_json,
                now
            ],
        )?;
        tx.commit()?;
        // Domino advancement: if this task belonged to a SEQUENTIAL_CHAIN, unlock the next step
        let _ = advance_sequential_chain_on_complete(conn, &owner_pid, task_id);
        Ok(format!(
            "گام [{}] مختومه شد و گام بعدی در مسیر پیش‌رو فعال گردید.",
            task_id
        ))
    } else {
        Ok("تسک موردنظر یافت نشد.".into())
    }
}

/// تغییر وضعیت یک تسک باز (غیرقفل) به IN_PROGRESS برای شروع فوری اجرا
pub fn start_task_step(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
) -> Result<bool> {
    let locked = is_locked_by_chain(conn, task_id).unwrap_or(false);
    if locked {
        return Ok(false);
    }
    let owner_pid: String = conn
        .query_row(
            "SELECT project_id FROM work_entities WHERE id = ?1 AND project_id = ?2",
            params![task_id, project_id],
            |r| r.get(0),
        )
        .or_else(|_| {
            conn.query_row(
                "SELECT project_id FROM work_entities WHERE id = ?1 LIMIT 1",
                params![task_id],
                |r| r.get(0),
            )
        })
        .unwrap_or_else(|_| project_id.to_string());
    let now = Local::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE work_entities
         SET status = 'IN_PROGRESS', updated_at = ?1
         WHERE id = ?2 AND project_id = ?3 AND status = 'OPEN'",
        params![now, task_id, owner_pid],
    )?;
    Ok(affected > 0)
}

pub fn apply_copilot_envelope(
    conn: &mut Connection,
    project_id: &str,
    raw_text: &str,
    archived_path: Option<&str>,
    source_origin: &str,
    envelope: &CopilotEnvelope,
) -> Result<String> {
    apply_copilot_envelope_with_event(
        conn,
        project_id,
        raw_text,
        archived_path,
        source_origin,
        envelope,
        None,
    )
}

pub fn apply_copilot_envelope_with_event(
    conn: &mut Connection,
    project_id: &str,
    raw_text: &str,
    archived_path: Option<&str>,
    source_origin: &str,
    envelope: &CopilotEnvelope,
    existing_event_id: Option<i64>,
) -> Result<String> {
    let (badge, _artifacts) = apply_copilot_envelope_with_artifacts(
        conn,
        project_id,
        raw_text,
        archived_path,
        source_origin,
        envelope,
        existing_event_id,
    )?;
    Ok(badge)
}

pub fn apply_copilot_envelope_with_artifacts(
    conn: &mut Connection,
    project_id: &str,
    raw_text: &str,
    archived_path: Option<&str>,
    source_origin: &str,
    envelope: &CopilotEnvelope,
    existing_event_id: Option<i64>,
) -> Result<(String, Vec<WrittenArtifactRef>)> {
    let now = Local::now().to_rfc3339();
    let snapshot_json = capture_undo_snapshot(conn, project_id)?;
    let workspace_path: String = conn
        .query_row(
            "SELECT workspace_path FROM projects WHERE id = ?1",
            params![project_id],
            |r| r.get(0),
        )
        .unwrap_or_default();
    let is_legacy_reprocess = source_origin.starts_with("LEGACY_REPROCESS");
    let mut written_artifacts: Vec<WrittenArtifactRef> = Vec::new();

    let tx = conn.transaction()?;

    let event_id = if let Some(eid) = existing_event_id {
        if let Some(arch) = archived_path {
            let _ = tx.execute(
                "UPDATE raw_events SET archived_path = COALESCE(?1, archived_path) WHERE id = ?2",
                params![arch, eid],
            );
        }
        eid
    } else {
        tx.execute(
            "INSERT INTO raw_events (project_id, raw_content, archived_path, source_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![project_id, raw_text, archived_path, source_origin, now],
        )?;
        tx.last_insert_rowid()
    };

    let mut resolved = 0;
    let mut created = 0;
    let mut updated = 0;
    let mut deleted = 0;
    let mut decisions = 0;
    let mut sops = 0;
    let mut total_applied = 0;

    for m in &envelope.mutations {
        let cat = m
            .category
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("GENERAL")
            .to_string();

        match m.op.as_str() {
            "RESOLVE_ENTITY" => {
                if let Some(tid) = &m.target_id {
                    let changed = tx.execute(
                        "UPDATE work_entities SET status = 'CLOSED', updated_at = ?1 WHERE id = ?2 AND project_id = ?3",
                        params![now, tid, project_id],
                    )?;
                    if changed > 0 {
                        resolved += 1;
                        total_applied += 1;
                    }
                }
            }
            "DELETE_ENTITY" => {
                if let Some(tid) = m.target_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    let changed = tx.execute(
                        "DELETE FROM work_entities WHERE id = ?1 AND project_id = ?2",
                        params![tid, project_id],
                    )?;
                    if changed > 0 {
                        deleted += 1;
                        total_applied += 1;
                    }
                }
            }
            "SET_NEXT_STEP" => {
                if let Some(tid) = m.target_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    let existing: Option<WorkEntity> = tx
                        .query_row(
                            "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                                    status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos,
                                    calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at,
                                    work_line_id, outcome_id, sequence_order, depends_on_task_id, energy_tag, duration_minutes,
                                    device_tag, location_tags_json, scheduled_date, scheduled_slot, due_date, priority_score
                             FROM work_entities WHERE id = ?1 AND project_id = ?2",
                            params![tid, project_id],
                            map_work_entity_from_row,
                        )
                        .ok();

                    if let Some(cur) = existing {
                        tx.execute(
                            "UPDATE work_entities SET status = 'OPEN'
                             WHERE project_id = ?1 AND entity_type = 'TASK' AND status = 'IN_PROGRESS' AND id != ?2",
                            params![project_id, tid],
                        )?;

                        let next_cat = m
                            .category
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&cur.category)
                            .to_string();
                        let next_title = m
                            .title
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&cur.title)
                            .to_string();
                        let next_dod = m
                            .definition_of_done
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&cur.definition_of_done)
                            .to_string();
                        let next_action = m
                            .first_physical_action
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&cur.first_physical_action)
                            .to_string();
                        let o = m.optimistic_pomos.unwrap_or(cur.optimistic_pomos).max(0.2);
                        let ml = m.most_likely_pomos.unwrap_or(cur.most_likely_pomos).max(o);
                        let p = m.pessimistic_pomos.unwrap_or(cur.pessimistic_pomos).max(ml);
                        let pert_base = (o + 4.0 * ml + p) / 6.0;
                        let bias = calculate_category_bias(&tx, project_id, &next_cat);
                        let calibrated = ((pert_base * bias) * 10.0).round() / 10.0;

                        let next_duration = m
                            .duration_minutes
                            .unwrap_or(cur.duration_minutes)
                            .max(5);
                        let next_scheduled_date = m.scheduled_date.clone().or(cur.scheduled_date);
                        let next_scheduled_slot = m.scheduled_slot.clone().or(cur.scheduled_slot);
                        let next_due_date = m.due_date.clone().or(cur.due_date);
                        let next_priority = m
                            .priority_score
                            .unwrap_or(cur.priority_score)
                            .clamp(0, 100);

                        tx.execute(
                            "UPDATE work_entities SET
                                category = ?1, title = ?2, definition_of_done = ?3,
                                first_physical_action = ?4, status = 'IN_PROGRESS', is_on_critical_path = 1,
                                optimistic_pomos = ?5, most_likely_pomos = ?6, pessimistic_pomos = ?7,
                                calibrated_pomos = ?8, duration_minutes = ?9, scheduled_date = ?10,
                                scheduled_slot = ?11, due_date = ?12, priority_score = ?13, updated_at = ?14
                             WHERE id = ?15 AND project_id = ?16",
                            params![
                                next_cat,
                                next_title,
                                next_dod,
                                next_action,
                                o,
                                ml,
                                p,
                                calibrated,
                                next_duration,
                                next_scheduled_date,
                                next_scheduled_slot,
                                next_due_date,
                                next_priority,
                                now,
                                tid,
                                project_id
                            ],
                        )?;
                        updated += 1;
                        total_applied += 1;
                    }
                }
            }
            "CREATE_ENTITY" => {
                let etype = m
                    .entity_type
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("TASK")
                    .to_string();
                let cand_title = m.title.clone().unwrap_or_else(|| "گام جدید".into());
                if is_legacy_reprocess
                    && etype == "TASK"
                    && crate::method_engine::is_duplicate_task_title(&tx, project_id, &cand_title)
                {
                    continue;
                }
                let prefix = match etype.as_str() {
                    "DECISION" => "ADR",
                    "BLOCKER" => "BLK",
                    _ => "TSK",
                };
                let new_id = format!(
                    "{}-{}",
                    prefix,
                    &uuid::Uuid::new_v4().to_string()[..4].to_uppercase()
                );
                let o = m.optimistic_pomos.unwrap_or(1.0).max(0.2);
                let ml = m.most_likely_pomos.unwrap_or(2.0).max(o);
                let p = m.pessimistic_pomos.unwrap_or(4.0).max(ml);
                let pert_base = (o + 4.0 * ml + p) / 6.0;
                let bias = calculate_category_bias(&tx, project_id, &cat);
                let calibrated = ((pert_base * bias) * 10.0).round() / 10.0;
                let status = if etype == "BLOCKER" {
                    "BLOCKED"
                } else {
                    match m.status.as_deref() {
                        Some("IN_PROGRESS") => "IN_PROGRESS",
                        Some("BLOCKED") => "BLOCKED",
                        Some("CLOSED") => "CLOSED",
                        _ => "OPEN",
                    }
                };

                if etype == "TASK" && status == "IN_PROGRESS" {
                    tx.execute(
                        "UPDATE work_entities SET status = 'OPEN'
                         WHERE project_id = ?1 AND entity_type = 'TASK' AND status = 'IN_PROGRESS'",
                        params![project_id],
                    )?;
                }

                tx.execute(
                    "INSERT INTO work_entities (
                        id, project_id, entity_type, category, title, definition_of_done,
                        first_physical_action, status, is_on_critical_path, optimistic_pomos,
                        most_likely_pomos, pessimistic_pomos, calibrated_pomos, stakeholder_tag,
                        duration_minutes, scheduled_date, scheduled_slot, due_date, priority_score,
                        source_event_id, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                    params![
                        new_id,
                        project_id,
                        etype,
                        cat,
                        cand_title,
                        m.definition_of_done.clone().unwrap_or_default(),
                        m.first_physical_action.clone().unwrap_or_default(),
                        status,
                        if m.is_on_critical_path.unwrap_or(true) { 1 } else { 0 },
                        o,
                        ml,
                        p,
                        calibrated,
                        m.stakeholder_tag,
                        m.duration_minutes.unwrap_or(25).max(5),
                        m.scheduled_date,
                        m.scheduled_slot,
                        m.due_date,
                        m.priority_score.unwrap_or(if m.is_on_critical_path.unwrap_or(true) { 75 } else { 50 }).clamp(0, 100),
                        event_id,
                        now
                    ],
                )?;
                if etype == "DECISION" {
                    decisions += 1;
                    written_artifacts.push(WrittenArtifactRef {
                        artifact_type: "DECISION_ADR".to_string(),
                        entity_id: new_id,
                        title: cand_title,
                        summary: m.definition_of_done.clone().unwrap_or_else(|| "تصمیم معماری ثبت‌شده در ADR Registry".into()),
                        file_path: if workspace_path.is_empty() {
                            None
                        } else {
                            Some(
                                Path::new(&workspace_path)
                                    .join("03_decisions_adr")
                                    .join("ADR_REGISTRY.md")
                                    .to_string_lossy()
                                    .to_string(),
                            )
                        },
                        action: "CREATED".to_string(),
                        version: None,
                        ..Default::default()
                    });
                } else {
                    created += 1;
                }
                total_applied += 1;
            }
            "UPDATE_ENTITY" => {
                if let Some(tid) = m.target_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    let existing: Option<WorkEntity> = tx
                        .query_row(
                            "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                                    status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos,
                                    calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at,
                                    work_line_id, outcome_id, sequence_order, depends_on_task_id, energy_tag, duration_minutes,
                                    device_tag, location_tags_json, scheduled_date, scheduled_slot, due_date, priority_score
                             FROM work_entities WHERE id = ?1 AND project_id = ?2",
                            params![tid, project_id],
                            map_work_entity_from_row,
                        )
                        .ok();

                    if let Some(cur) = existing {
                        let next_etype = match m.entity_type.as_deref().map(str::trim) {
                            Some(et @ ("TASK" | "DECISION" | "BLOCKER")) => et.to_string(),
                            _ => cur.entity_type.clone(),
                        };
                        let next_cat = m
                            .category
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&cur.category)
                            .to_string();
                        let next_title = m
                            .title
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&cur.title)
                            .to_string();
                        let next_dod = m
                            .definition_of_done
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&cur.definition_of_done)
                            .to_string();
                        let next_action = m
                            .first_physical_action
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&cur.first_physical_action)
                            .to_string();
                        let next_status = match m.status.as_deref().map(str::trim) {
                            Some(s @ ("OPEN" | "IN_PROGRESS" | "BLOCKED" | "CLOSED" | "SUPERSEDED")) => s.to_string(),
                            _ => cur.status.clone(),
                        };
                        if next_etype == "TASK" && next_status == "IN_PROGRESS" {
                            tx.execute(
                                "UPDATE work_entities SET status = 'OPEN'
                                 WHERE project_id = ?1 AND entity_type = 'TASK' AND status = 'IN_PROGRESS' AND id != ?2",
                                params![project_id, tid],
                            )?;
                        }
                        let next_crit = m.is_on_critical_path.unwrap_or(cur.is_on_critical_path);
                        let o = m.optimistic_pomos.unwrap_or(cur.optimistic_pomos).max(0.2);
                        let ml = m.most_likely_pomos.unwrap_or(cur.most_likely_pomos).max(o);
                        let p = m.pessimistic_pomos.unwrap_or(cur.pessimistic_pomos).max(ml);
                        let calibrated = if next_etype == "DECISION" {
                            0.0
                        } else {
                            let pert_base = (o + 4.0 * ml + p) / 6.0;
                            let bias = calculate_category_bias(&tx, project_id, &next_cat);
                            ((pert_base * bias) * 10.0).round() / 10.0
                        };
                        let next_stakeholder = m
                            .stakeholder_tag
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(str::to_string)
                            .or(cur.stakeholder_tag);
                        let next_duration = m
                            .duration_minutes
                            .unwrap_or(cur.duration_minutes)
                            .max(5);
                        let next_scheduled_date = m.scheduled_date.clone().or(cur.scheduled_date);
                        let next_scheduled_slot = m.scheduled_slot.clone().or(cur.scheduled_slot);
                        let next_due_date = m.due_date.clone().or(cur.due_date);
                        let next_priority = m
                            .priority_score
                            .unwrap_or(cur.priority_score)
                            .clamp(0, 100);

                        tx.execute(
                            "UPDATE work_entities SET
                                entity_type = ?1, category = ?2, title = ?3, definition_of_done = ?4,
                                first_physical_action = ?5, status = ?6, is_on_critical_path = ?7,
                                optimistic_pomos = ?8, most_likely_pomos = ?9, pessimistic_pomos = ?10,
                                calibrated_pomos = ?11, stakeholder_tag = ?12,
                                duration_minutes = ?13, scheduled_date = ?14, scheduled_slot = ?15,
                                due_date = ?16, priority_score = ?17, updated_at = ?18
                             WHERE id = ?19 AND project_id = ?20",
                            params![
                                next_etype,
                                next_cat,
                                next_title,
                                next_dod,
                                next_action,
                                next_status,
                                if next_crit { 1 } else { 0 },
                                o,
                                ml,
                                p,
                                calibrated,
                                next_stakeholder,
                                next_duration,
                                next_scheduled_date,
                                next_scheduled_slot,
                                next_due_date,
                                next_priority,
                                now,
                                tid,
                                project_id
                            ],
                        )?;
                        updated += 1;
                        total_applied += 1;
                    }
                }
            }
            "SUPERSEDE_DECISION" => {
                let new_id = format!(
                    "ADR-{}",
                    &uuid::Uuid::new_v4().to_string()[..4].to_uppercase()
                );
                if let Some(old_id) = &m.target_id {
                    tx.execute(
                        "UPDATE work_entities SET status = 'SUPERSEDED', superseded_by_id = ?1, updated_at = ?2 WHERE id = ?3 AND project_id = ?4",
                        params![new_id, now, old_id, project_id],
                    )?;
                }
                tx.execute(
                    "INSERT INTO work_entities (
                        id, project_id, entity_type, category, title, definition_of_done,
                        first_physical_action, status, is_on_critical_path, calibrated_pomos,
                        stakeholder_tag, source_event_id, updated_at
                    ) VALUES (?1, ?2, 'DECISION', ?3, ?4, ?5, '', 'OPEN', 0, 0.0, ?6, ?7, ?8)",
                    params![
                        new_id,
                        project_id,
                        cat,
                        m.title.clone().unwrap_or_default(),
                        m.definition_of_done.clone().unwrap_or_default(),
                        m.stakeholder_tag,
                        event_id,
                        now
                    ],
                )?;
                decisions += 1;
                total_applied += 1;
            }
            "UPSERT_SOP" => {
                if let Some(rule) = &m.sop_rule_statement {
                    if !rule.trim().is_empty() {
                        tx.execute(
                            "INSERT INTO sop_rules (project_id, category, rule_statement, is_active, updated_at)
                             VALUES (?1, ?2, ?3, 1, ?4)",
                            params![project_id, cat, rule, now],
                        )?;
                        sops += 1;
                        total_applied += 1;
                    }
                }
            }
            "REGISTER_ANTIPATTERN" => {
                if let (Some(pat), Some(gate)) = (&m.mistake_pattern, &m.preventive_gate_question) {
                    if !pat.trim().is_empty() && !gate.trim().is_empty() {
                        tx.execute(
                            "INSERT INTO learned_antipatterns (
                                project_id, trigger_category, mistake_pattern, preventive_gate_question,
                                severity_score, times_triggered, is_active, created_at
                             ) VALUES (?1, ?2, ?3, ?4, ?5, 1, 1, ?6)",
                            params![
                                project_id,
                                cat,
                                pat,
                                gate,
                                m.severity_score.unwrap_or(7),
                                now
                            ],
                        )?;
                        total_applied += 1;
                    }
                }
            }
            "LOG_WORK" => {
                let estimated_pomos = m
                    .target_id
                    .as_ref()
                    .and_then(|tid| {
                        tx.query_row(
                            "SELECT calibrated_pomos FROM work_entities WHERE id = ?1 AND project_id = ?2",
                            params![tid, project_id],
                            |r| r.get::<_, f64>(0),
                        )
                        .ok()
                    })
                    .filter(|v| *v > 0.1)
                    .unwrap_or(2.0);

                let deliverable = m.deliverable_summary.clone().unwrap_or_default();
                let sanitized = m
                    .client_sanitized_note
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| deliverable.clone());

                tx.execute(
                    "INSERT INTO session_work_logs (
                        project_id, task_id, category, estimated_pomos, effective_pomos,
                        deliverable_summary, internal_friction_note, client_sanitized_note,
                        source_origin, logged_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        project_id,
                        m.target_id,
                        cat,
                        estimated_pomos,
                        m.effective_pomos.unwrap_or(1).max(1),
                        deliverable,
                        m.internal_friction_note.clone().unwrap_or_default(),
                        sanitized,
                        source_origin,
                        now
                    ],
                )?;
                total_applied += 1;
            }
            _ => {}
        }
    }

    let badge = if total_applied > 0 {
        let mut parts = Vec::new();
        if created > 0 {
            parts.push(format!("تسک جدید: {}", created));
        }
        if updated > 0 {
            parts.push(format!("تدقیق/اصلاح: {}", updated));
        }
        if resolved > 0 {
            parts.push(format!("مختومه: {}", resolved));
        }
        if deleted > 0 {
            parts.push(format!("حذف: {}", deleted));
        }
        if decisions > 0 {
            parts.push(format!("تصمیم: {}", decisions));
        }
        if sops > 0 {
            parts.push(format!("SOP: {}", sops));
        }
        if parts.is_empty() {
            format!("ثبت فعالیت/گزارش ({})", total_applied)
        } else {
            parts.join(" | ")
        }
    } else if !envelope.clarifying_questions.is_empty() {
        "نیازمند شفاف‌سازی (گفتگوی تعاملی)".to_string()
    } else {
        "پرسش/پاسخ (بدون تغییر گراف)".to_string()
    };

    if total_applied > 0 {
        tx.execute(
            "INSERT INTO mutation_audit_logs (project_id, summary_text, snapshot_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![project_id, badge, snapshot_json, now],
        )?;
    }

    tx.commit()?;

    // Apply any extracted Method / Playbooks + BPMN 2.0 workflows outside the tx so files + DB sync cleanly
    let mut methods_count = 0usize;
    let source_ref_str = format!("event:{}", event_id);
    for method_in in &envelope.method_playbooks {
        if method_in.title.trim().is_empty() {
            continue;
        }
        if let Ok((_saved, _is_new, art_ref)) = crate::method_engine::upsert_method_playbook(
            conn,
            project_id,
            &workspace_path,
            method_in.clone(),
            Some(&source_ref_str),
        ) {
            written_artifacts.push(art_ref);
            methods_count += 1;
        }
    }

    let final_badge = if methods_count > 0 {
        if badge == "پرسش/پاسخ (بدون تغییر گراف)" {
            format!("متد/دستورالعمل BPMN: {}", methods_count)
        } else {
            format!("{} | متد/BPMN: {}", badge, methods_count)
        }
    } else {
        badge
    };

    Ok((final_badge, written_artifacts))
}

pub fn commit_user_chat_turn(
    conn: &Connection,
    project_id: &str,
    requested_session_id: Option<&str>,
    channel: &str,
    user_text: &str,
    archived_path: Option<&str>,
    source_type: &str,
) -> Result<(String, i64, i64)> {
    let norm_ch = normalize_chat_channel(channel);
    let now = Local::now().to_rfc3339();
    if project_id.eq_ignore_ascii_case(GLOBAL_CHAT_PROJECT_ID) {
        set_active_chat_scope(conn, "GLOBAL");
    } else {
        set_active_chat_scope(conn, "PROJECT");
    }

    let session_id = if let Some(req_sid) = requested_session_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let valid: Option<String> = conn
            .query_row(
                "SELECT id FROM chat_sessions WHERE id = ?1 AND project_id = ?2 AND channel = ?3",
                params![req_sid, project_id, norm_ch],
                |r| r.get(0),
            )
            .ok();
        if let Some(sid) = valid {
            conn.execute(
                "UPDATE chat_sessions SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END
                 WHERE project_id = ?2 AND channel = ?3",
                params![sid, project_id, norm_ch],
            )?;
            sid
        } else {
            ensure_active_chat_session(conn, project_id, norm_ch)?
        }
    } else {
        ensure_active_chat_session(conn, project_id, norm_ch)?
    };

    conn.execute(
        "INSERT INTO raw_events (project_id, raw_content, archived_path, source_type, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![project_id, user_text, archived_path, source_type, now],
    )?;
    let event_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO chat_messages (
            project_id, session_id, sender, channel, message_text,
            archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at
         ) VALUES (?1, ?2, 'USER', ?3, ?4, ?5, 'ثبت فوری در حافظه پروژه', '[]', ?6)",
        params![project_id, session_id, norm_ch, user_text, archived_path, now],
    )?;
    let msg_id = conn.last_insert_rowid();

    let cur_title: String = conn
        .query_row(
            "SELECT title FROM chat_sessions WHERE id = ?1",
            params![session_id],
            |r| r.get(0),
        )
        .unwrap_or_default();

    let next_title = if cur_title == "چت جدید"
        || cur_title == "گفتگوی جدید"
        || cur_title == "گفتگوی اصلی پروژه"
        || cur_title == "گفتگوی آرشیو دانش"
        || cur_title == "گفتگوی آزاد (همه پروژه‌ها)"
        || cur_title.trim().is_empty()
    {
        derive_session_title(user_text, default_session_title(norm_ch))
    } else {
        cur_title
    };

    conn.execute(
        "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![next_title, now, session_id],
    )?;

    Ok((session_id, event_id, msg_id))
}

pub fn append_assistant_chat_message(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
    channel: &str,
    sender: &str,
    assistant_reply: &str,
    archived_path: Option<&str>,
    badge: &str,
    clarifying_questions: &[String],
) -> Result<i64> {
    append_assistant_chat_message_with_artifacts(
        conn,
        project_id,
        session_id,
        channel,
        sender,
        assistant_reply,
        archived_path,
        badge,
        clarifying_questions,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn append_assistant_chat_message_with_artifacts(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
    channel: &str,
    sender: &str,
    assistant_reply: &str,
    archived_path: Option<&str>,
    badge: &str,
    clarifying_questions: &[String],
    written_artifacts: &[WrittenArtifactRef],
) -> Result<i64> {
    let norm_ch = normalize_chat_channel(channel);
    let now = Local::now().to_rfc3339();
    let cleaned_questions: Vec<String> = clarifying_questions
        .iter()
        .map(|q| q.trim().to_string())
        .filter(|q| !q.is_empty())
        .collect();
    let questions_json = serde_json::to_string(&cleaned_questions).unwrap_or_else(|_| "[]".into());
    let artifacts_json = serde_json::to_string(written_artifacts).unwrap_or_else(|_| "[]".into());

    let effective_sid = if session_id.trim().is_empty() {
        ensure_active_chat_session(conn, project_id, norm_ch)?
    } else {
        session_id.to_string()
    };

    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "chat_messages",
        "written_artifacts_json",
        "ALTER TABLE chat_messages ADD COLUMN written_artifacts_json TEXT NOT NULL DEFAULT '[]'",
    );
    conn.execute(
        "INSERT INTO chat_messages (
            project_id, session_id, sender, channel, message_text,
            archived_file_path, applied_mutations_badge, clarifying_questions_json, written_artifacts_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            project_id,
            effective_sid,
            sender,
            norm_ch,
            assistant_reply,
            archived_path,
            badge,
            questions_json,
            artifacts_json,
            now
        ],
    )?;
    let msg_id = conn.last_insert_rowid();

    conn.execute(
        "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
        params![now, effective_sid],
    )?;

    Ok(msg_id)
}

pub struct RetrySystemContext {
    pub project_id: String,
    pub session_id: String,
    pub channel: String,
    pub user_message_id: i64,
    pub user_message_text: String,
    pub user_archived_path: Option<String>,
    pub system_archived_path: Option<String>,
    pub raw_event_id: Option<i64>,
    pub raw_source_type: Option<String>,
    pub raw_content: Option<String>,
}

pub fn get_retry_context_for_system_message(
    conn: &Connection,
    system_message_id: i64,
) -> Result<RetrySystemContext> {
    let (project_id, session_id, raw_channel, system_archived_path): (
        String,
        String,
        String,
        Option<String>,
    ) = conn.query_row(
        "SELECT project_id, COALESCE(session_id, ''), channel, archived_file_path
         FROM chat_messages
         WHERE id = ?1 AND sender = 'SYSTEM'",
        params![system_message_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;

    let channel = normalize_chat_channel(&raw_channel).to_string();

    let (user_message_id, user_message_text, user_archived_path): (i64, String, Option<String>) =
        if !session_id.is_empty() {
            conn.query_row(
                "SELECT id, message_text, archived_file_path
                 FROM chat_messages
                 WHERE project_id = ?1 AND session_id = ?2 AND channel = ?3
                   AND sender = 'USER' AND id < ?4
                 ORDER BY id DESC LIMIT 1",
                params![project_id, session_id, channel, system_message_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?
        } else {
            conn.query_row(
                "SELECT id, message_text, archived_file_path
                 FROM chat_messages
                 WHERE project_id = ?1 AND channel = ?2
                   AND sender = 'USER' AND id < ?3
                 ORDER BY id DESC LIMIT 1",
                params![project_id, channel, system_message_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?
        };

    let raw_match: Option<(i64, String, String)> = if let Some(ref arch) = user_archived_path {
        conn.query_row(
            "SELECT id, source_type, raw_content
             FROM raw_events
             WHERE project_id = ?1 AND archived_path = ?2
             ORDER BY id DESC LIMIT 1",
            params![project_id, arch],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok()
    } else {
        None
    }
    .or_else(|| {
        conn.query_row(
            "SELECT id, source_type, raw_content
             FROM raw_events
             WHERE project_id = ?1 AND raw_content = ?2
             ORDER BY id DESC LIMIT 1",
            params![project_id, user_message_text],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok()
    });

    let (raw_event_id, raw_source_type, raw_content) = match raw_match {
        Some((eid, st, rc)) => (Some(eid), Some(st), Some(rc)),
        None => (None, None, None),
    };

    Ok(RetrySystemContext {
        project_id,
        session_id,
        channel,
        user_message_id,
        user_message_text,
        user_archived_path,
        system_archived_path,
        raw_event_id,
        raw_source_type,
        raw_content,
    })
}

pub fn get_knowledge_document_by_id(conn: &Connection, doc_id: i64) -> Option<KnowledgeDocument> {
    conn.query_row(
        "SELECT id, project_id, title, doc_type, original_filename, archived_path,
                executive_summary, extracted_items_count, ingested_at
         FROM knowledge_documents WHERE id = ?1",
        params![doc_id],
        |r| {
            Ok(KnowledgeDocument {
                id: r.get(0)?,
                project_id: r.get(1)?,
                title: r.get(2)?,
                doc_type: r.get(3)?,
                original_filename: r.get(4)?,
                archived_path: r.get(5)?,
                executive_summary: r.get(6)?,
                extracted_items_count: r.get(7)?,
                ingested_at: r.get(8)?,
                ..Default::default()
            })
        },
    )
    .ok()
}

pub fn get_raw_event_content_by_id(conn: &Connection, event_id: i64) -> Option<(String, String)> {
    conn.query_row(
        "SELECT source_type, raw_content FROM raw_events WHERE id = ?1",
        params![event_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .ok()
}

pub fn get_user_chat_message_by_id(
    conn: &Connection,
    msg_id: i64,
) -> Option<(String, Option<String>)> {
    conn.query_row(
        "SELECT message_text, archived_file_path FROM chat_messages WHERE id = ?1",
        params![msg_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .ok()
}

pub fn query_messages_for_session_public(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
    vault_only: bool,
) -> Result<Vec<ChatMessage>> {
    query_session_messages(conn, project_id, session_id, vault_only)
}

pub fn delete_system_chat_message(conn: &Connection, system_message_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM chat_messages WHERE id = ?1 AND sender = 'SYSTEM'",
        params![system_message_id],
    )?;
    Ok(())
}

pub fn update_system_chat_message(
    conn: &Connection,
    system_message_id: i64,
    new_text: &str,
) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE chat_messages
         SET message_text = ?1, created_at = ?2
         WHERE id = ?3 AND sender = 'SYSTEM'",
        params![new_text, now, system_message_id],
    )?;
    Ok(())
}

pub fn insert_chat_turn(
    conn: &Connection,
    project_id: &str,
    user_text: &str,
    archived_path: Option<&str>,
    assistant_reply: &str,
    badge: &str,
) -> Result<()> {
    insert_chat_turn_with_details(
        conn,
        project_id,
        "COPILOT",
        user_text,
        archived_path,
        assistant_reply,
        badge,
        &[],
    )
}

pub fn insert_chat_turn_with_details(
    conn: &Connection,
    project_id: &str,
    channel: &str,
    user_text: &str,
    archived_path: Option<&str>,
    assistant_reply: &str,
    badge: &str,
    clarifying_questions: &[String],
) -> Result<()> {
    let norm_ch = normalize_chat_channel(channel);
    let session_id = ensure_active_chat_session(conn, project_id, norm_ch)?;
    let now = Local::now().to_rfc3339();
    let cleaned_questions: Vec<String> = clarifying_questions
        .iter()
        .map(|q| q.trim().to_string())
        .filter(|q| !q.is_empty())
        .collect();
    let questions_json = serde_json::to_string(&cleaned_questions).unwrap_or_else(|_| "[]".into());

    conn.execute(
        "INSERT INTO chat_messages (project_id, session_id, sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at)
         VALUES (?1, ?2, 'USER', ?3, ?4, ?5, 'ثبت فوری در حافظه پروژه', '[]', ?6)",
        params![project_id, session_id, norm_ch, user_text, archived_path, now],
    )?;
    conn.execute(
        "INSERT INTO chat_messages (project_id, session_id, sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at)
         VALUES (?1, ?2, 'ASSISTANT', ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project_id,
            session_id,
            norm_ch,
            assistant_reply,
            archived_path,
            badge,
            questions_json,
            now
        ],
    )?;

    let cur_title: String = conn
        .query_row(
            "SELECT title FROM chat_sessions WHERE id = ?1",
            params![session_id],
            |r| r.get(0),
        )
        .unwrap_or_default();
    let next_title = if cur_title == "چت جدید" || cur_title == "گفتگوی جدید" || cur_title.trim().is_empty() {
        derive_session_title(user_text, default_session_title(norm_ch))
    } else {
        cur_title
    };
    conn.execute(
        "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![next_title, now, session_id],
    )?;

    Ok(())
}

pub const UNANALYZED_DOC_SUMMARY_PREFIX: &str =
    "متن فایل استخراج و در حافظه پروژه آرشیو شد (تحلیل هوشمند";

pub const WAITING_DOC_SUMMARY_PREFIX: &str = "در انتظار پردازش توسط هوش مصنوعی";

#[allow(clippy::too_many_arguments)]
pub fn insert_knowledge_doc_with_status(
    conn: &Connection,
    project_id: &str,
    title: &str,
    doc_type: &str,
    original_filename: &str,
    archived_path: &str,
    summary: &str,
    items_count: i32,
    processing_status: &str,
) -> Result<i64> {
    let now = Local::now().to_rfc3339();
    let processed_at: Option<&str> = if processing_status == "PROCESSED" {
        Some(&now)
    } else {
        None
    };
    conn.execute(
        "INSERT INTO knowledge_documents (
            project_id, title, doc_type, original_filename, archived_path,
            executive_summary, extracted_items_count, ingested_at, processing_status, processed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            project_id,
            title,
            doc_type,
            original_filename,
            archived_path,
            summary,
            items_count,
            now,
            processing_status,
            processed_at
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[allow(clippy::too_many_arguments)]
pub fn insert_knowledge_doc_record(
    conn: &Connection,
    project_id: &str,
    title: &str,
    doc_type: &str,
    original_filename: &str,
    archived_path: &str,
    summary: &str,
    items_count: i32,
) -> Result<i64> {
    let status = if summary.starts_with(UNANALYZED_DOC_SUMMARY_PREFIX)
        || summary.starts_with(WAITING_DOC_SUMMARY_PREFIX)
    {
        "UNPROCESSED"
    } else {
        "PROCESSED"
    };
    insert_knowledge_doc_with_status(
        conn,
        project_id,
        title,
        doc_type,
        original_filename,
        archived_path,
        summary,
        items_count,
        status,
    )
}

pub fn set_knowledge_doc_processing_status(
    conn: &Connection,
    doc_id: i64,
    status: &str,
    summary_opt: Option<&str>,
    items_count_opt: Option<i32>,
    new_archived_path: Option<&str>,
) -> Result<()> {
    let now = Local::now().to_rfc3339();
    let processed_at: Option<&str> = if status == "PROCESSED" {
        Some(&now)
    } else {
        None
    };
    conn.execute(
        "UPDATE knowledge_documents
         SET processing_status = ?1,
             processed_at = CASE WHEN ?1 = 'PROCESSED' THEN ?2 ELSE processed_at END,
             executive_summary = COALESCE(?3, executive_summary),
             extracted_items_count = COALESCE(?4, extracted_items_count),
             archived_path = COALESCE(?5, archived_path)
         WHERE id = ?6",
        params![
            status,
            processed_at,
            summary_opt,
            items_count_opt,
            new_archived_path,
            doc_id
        ],
    )?;
    Ok(())
}

pub fn update_knowledge_doc_analysis(
    conn: &Connection,
    doc_id: i64,
    new_summary: &str,
    items_count: i32,
    new_archived_path: Option<&str>,
) -> Result<()> {
    set_knowledge_doc_processing_status(
        conn,
        doc_id,
        "PROCESSED",
        Some(new_summary),
        Some(items_count),
        new_archived_path,
    )
}

pub fn requeue_knowledge_document(conn: &Connection, doc_id: i64) -> Result<i64> {
    let (pid, title, orig_fname, arch_path): (String, String, String, String) = conn.query_row(
        "SELECT project_id, title, original_filename, archived_path
         FROM knowledge_documents WHERE id = ?1",
        params![doc_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;

    conn.execute(
        "UPDATE knowledge_documents
         SET processing_status = 'UNPROCESSED', processed_at = NULL
         WHERE id = ?1",
        params![doc_id],
    )?;

    let preview = if orig_fname.trim().is_empty() {
        format!("📄 {}", title)
    } else {
        format!("📄 {} ({})", title, orig_fname)
    };
    let sid = ensure_active_chat_session(conn, &pid, "VAULT").unwrap_or_default();

    // If an old DONE/DISMISSED queue item exists for this knowledge_doc_id, re-activate it
    let updated = conn.execute(
        "UPDATE pending_ai_queue
         SET status = 'PENDING', last_error = NULL, updated_at = ?1
         WHERE knowledge_doc_id = ?2",
        params![Local::now().to_rfc3339(), doc_id],
    )?;
    if updated > 0 {
        let qid: i64 = conn.query_row(
            "SELECT id FROM pending_ai_queue WHERE knowledge_doc_id = ?1 ORDER BY id ASC LIMIT 1",
            params![doc_id],
            |r| r.get(0),
        )?;
        return Ok(qid);
    }

    enqueue_pending_ai_item(
        conn,
        &pid,
        &sid,
        "VAULT",
        None,
        None,
        Some(doc_id),
        None,
        &build_queue_preview(&preview),
        "DOC_ONBOARDING",
        Some(&arch_path),
        None,
    )
}

pub fn find_unanalyzed_knowledge_doc_id(
    conn: &Connection,
    project_id: &str,
    archived_path_hint: Option<&str>,
    text_hint: &str,
) -> Option<i64> {
    let like_prefix = format!("{}%", UNANALYZED_DOC_SUMMARY_PREFIX);
    if let Some(arch) = archived_path_hint.map(str::trim).filter(|s| !s.is_empty()) {
        if let Ok(doc_id) = conn.query_row(
            "SELECT id FROM knowledge_documents
             WHERE project_id = ?1 AND archived_path = ?2 AND executive_summary LIKE ?3
             ORDER BY id DESC LIMIT 1",
            params![project_id, arch, like_prefix],
            |r| r.get::<_, i64>(0),
        ) {
            return Some(doc_id);
        }
    }

    if text_hint.trim().is_empty() {
        return None;
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, original_filename, title
             FROM knowledge_documents
             WHERE project_id = ?1 AND executive_summary LIKE ?2
             ORDER BY id DESC",
        )
        .ok()?;
    let candidates: Vec<(i64, String, String)> = stmt
        .query_map(params![project_id, like_prefix], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .ok()?
        .filter_map(|r| r.ok())
        .collect();

    for (doc_id, fname, title) in candidates {
        let f_trim = fname.trim();
        let t_trim = title.trim();
        if (!f_trim.is_empty() && text_hint.contains(f_trim))
            || (!t_trim.is_empty() && text_hint.contains(t_trim))
        {
            return Some(doc_id);
        }
    }
    None
}

fn build_queue_preview(text: &str) -> String {
    let first_line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or(text.trim());
    let clipped: String = first_line.chars().take(72).collect();
    if first_line.chars().count() > 72 {
        format!("{}…", clipped.trim())
    } else if clipped.is_empty() {
        "ورودی بدون عنوان".to_string()
    } else {
        clipped
    }
}

pub struct PendingQueueInput<'a> {
    pub project_id: &'a str,
    pub session_id: &'a str,
    pub channel: &'a str,
    pub user_message_id: Option<i64>,
    pub system_message_id: Option<i64>,
    pub knowledge_doc_id: Option<i64>,
    pub raw_event_id: Option<i64>,
    pub title_preview: &'a str,
    pub source_type: &'a str,
    pub archived_path: Option<&'a str>,
    pub last_error: Option<&'a str>,
}

pub fn insert_pending_queue_item(
    conn: &Connection,
    input: PendingQueueInput<'_>,
) -> Result<i64> {
    enqueue_pending_ai_item(
        conn,
        input.project_id,
        input.session_id,
        input.channel,
        input.user_message_id,
        input.system_message_id,
        input.knowledge_doc_id,
        input.raw_event_id,
        input.title_preview,
        input.source_type,
        input.archived_path,
        input.last_error,
    )
}

fn resolve_original_created_at(
    conn: &Connection,
    user_message_id: Option<i64>,
    raw_event_id: Option<i64>,
    knowledge_doc_id: Option<i64>,
    fallback: &str,
) -> String {
    if let Some(uid) = user_message_id {
        if let Ok(ts) = conn.query_row(
            "SELECT created_at FROM chat_messages WHERE id = ?1",
            params![uid],
            |r| r.get::<_, String>(0),
        ) {
            if !ts.trim().is_empty() {
                return ts;
            }
        }
    }
    if let Some(eid) = raw_event_id {
        if let Ok(ts) = conn.query_row(
            "SELECT created_at FROM raw_events WHERE id = ?1",
            params![eid],
            |r| r.get::<_, String>(0),
        ) {
            if !ts.trim().is_empty() {
                return ts;
            }
        }
    }
    if let Some(kid) = knowledge_doc_id {
        if let Ok(ts) = conn.query_row(
            "SELECT ingested_at FROM knowledge_documents WHERE id = ?1",
            params![kid],
            |r| r.get::<_, String>(0),
        ) {
            if !ts.trim().is_empty() {
                return ts;
            }
        }
    }
    fallback.to_string()
}

#[allow(clippy::too_many_arguments)]
pub fn enqueue_pending_ai_item(
    conn: &Connection,
    project_id: &str,
    session_id: &str,
    channel: &str,
    user_message_id: Option<i64>,
    system_message_id: Option<i64>,
    knowledge_doc_id: Option<i64>,
    raw_event_id: Option<i64>,
    title_preview: &str,
    source_type: &str,
    archived_path: Option<&str>,
    last_error: Option<&str>,
) -> Result<i64> {
    let norm_ch = normalize_chat_channel(channel);
    let now = Local::now().to_rfc3339();
    let original_created_at = resolve_original_created_at(
        conn,
        user_message_id,
        raw_event_id,
        knowledge_doc_id,
        &now,
    );
    let existing_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM pending_ai_queue
             WHERE (?1 IS NOT NULL AND user_message_id = ?1)
                OR (?2 IS NOT NULL AND knowledge_doc_id = ?2)
             ORDER BY id ASC LIMIT 1",
            params![user_message_id, knowledge_doc_id],
            |r| r.get(0),
        )
        .ok();

    if let Some(qid) = existing_id {
        conn.execute(
            "UPDATE pending_ai_queue
             SET system_message_id = COALESCE(?1, system_message_id),
                 knowledge_doc_id = COALESCE(?2, knowledge_doc_id),
                 raw_event_id = COALESCE(?3, raw_event_id),
                 archived_path = COALESCE(?4, archived_path),
                 last_error = COALESCE(?5, last_error),
                 status = CASE WHEN status = 'DONE' THEN 'DONE' WHEN status = 'DISMISSED' THEN 'DISMISSED' ELSE 'PENDING' END,
                 updated_at = ?6
             WHERE id = ?7",
            params![
                system_message_id,
                knowledge_doc_id,
                raw_event_id,
                archived_path,
                last_error,
                now,
                qid
            ],
        )?;
        return Ok(qid);
    }

    conn.execute(
        "INSERT INTO pending_ai_queue (
            project_id, session_id, channel, user_message_id, system_message_id,
            knowledge_doc_id, raw_event_id, title_preview, source_type, archived_path,
            status, attempts, last_error, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'PENDING', 0, ?11, ?12, ?13)",
        params![
            project_id,
            session_id,
            norm_ch,
            user_message_id,
            system_message_id,
            knowledge_doc_id,
            raw_event_id,
            title_preview,
            source_type,
            archived_path,
            last_error,
            original_created_at,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn sync_unprocessed_items_into_queue(conn: &Connection) -> Result<usize> {
    let now = Local::now().to_rfc3339();
    // 1. Recover any stuck PROCESSING items from abrupt shutdown
    conn.execute(
        "UPDATE pending_ai_queue SET status = 'PENDING', updated_at = ?1 WHERE status = 'PROCESSING'",
        params![now],
    )?;

    // 2. Auto-close pending user_message items if a later ASSISTANT reply already exists in the same group
    conn.execute(
        "UPDATE pending_ai_queue
         SET status = 'DONE', updated_at = ?1
         WHERE status IN ('PENDING', 'PROCESSING')
           AND user_message_id IS NOT NULL
           AND EXISTS (
               SELECT 1 FROM chat_messages u
               JOIN chat_messages a ON a.project_id = u.project_id
                 AND COALESCE(a.session_id, '') = COALESCE(u.session_id, '')
                 AND (CASE WHEN a.channel = 'VAULT' THEN 'VAULT' ELSE 'COPILOT' END)
                   = (CASE WHEN u.channel = 'VAULT' THEN 'VAULT' ELSE 'COPILOT' END)
                 AND a.sender = 'ASSISTANT'
                 AND a.id > u.id
               WHERE u.id = pending_ai_queue.user_message_id
           )",
        params![now],
    )?;

    // Auto-close pending knowledge_doc items that are now marked PROCESSED (except explicit LEGACY_REPROCESS jobs)
    let like_prefix = format!("{}%", UNANALYZED_DOC_SUMMARY_PREFIX);
    conn.execute(
        "UPDATE pending_ai_queue
         SET status = 'DONE', updated_at = ?1
         WHERE status IN ('PENDING', 'PROCESSING')
           AND user_message_id IS NULL
           AND knowledge_doc_id IS NOT NULL
           AND source_type NOT LIKE 'LEGACY_REPROCESS%'
           AND EXISTS (
               SELECT 1 FROM knowledge_documents k
               WHERE k.id = pending_ai_queue.knowledge_doc_id
                 AND k.processing_status = 'PROCESSED'
                 AND k.executive_summary NOT LIKE ?2
           )",
        params![now, like_prefix],
    )?;

    let mut inserted = 0usize;

    // 3. Scan trailing USER messages (after the latest ASSISTANT message in each project/session/channel)
    let mut grp_stmt = conn.prepare(
        "SELECT project_id, COALESCE(session_id, ''),
                CASE WHEN channel = 'VAULT' THEN 'VAULT' ELSE 'COPILOT' END AS norm_ch
         FROM chat_messages
         WHERE sender = 'USER'
         GROUP BY project_id, COALESCE(session_id, ''), norm_ch",
    )?;
    let groups: Vec<(String, String, String)> = grp_stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    for (pid, sid, norm_ch) in groups {
        let last_assistant_id: i64 = if norm_ch == "VAULT" {
            conn.query_row(
                "SELECT COALESCE(MAX(id), 0) FROM chat_messages
                 WHERE project_id = ?1 AND COALESCE(session_id, '') = ?2
                   AND channel = 'VAULT' AND sender = 'ASSISTANT'",
                params![pid, sid],
                |r| r.get(0),
            )
            .unwrap_or(0)
        } else {
            conn.query_row(
                "SELECT COALESCE(MAX(id), 0) FROM chat_messages
                 WHERE project_id = ?1 AND COALESCE(session_id, '') = ?2
                   AND channel != 'VAULT' AND sender = 'ASSISTANT'",
                params![pid, sid],
                |r| r.get(0),
            )
            .unwrap_or(0)
        };

        let user_sql = if norm_ch == "VAULT" {
            "SELECT id, message_text, archived_file_path
             FROM chat_messages
             WHERE project_id = ?1 AND COALESCE(session_id, '') = ?2
               AND channel = 'VAULT' AND sender = 'USER' AND id > ?3
             ORDER BY id ASC"
        } else {
            "SELECT id, message_text, archived_file_path
             FROM chat_messages
             WHERE project_id = ?1 AND COALESCE(session_id, '') = ?2
               AND channel != 'VAULT' AND sender = 'USER' AND id > ?3
             ORDER BY id ASC"
        };
        let mut u_stmt = conn.prepare(user_sql)?;
        let pending_users: Vec<(i64, String, Option<String>)> = u_stmt
            .query_map(params![pid, sid, last_assistant_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for idx in 0..pending_users.len() {
            let (uid, ref utext, ref uarch) = pending_users[idx];
            let next_uid = pending_users.get(idx + 1).map(|(id, _, _)| *id).unwrap_or(i64::MAX);

            let exists_in_queue: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pending_ai_queue WHERE user_message_id = ?1",
                    params![uid],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap_or(0)
                > 0;
            if exists_in_queue {
                continue;
            }

            let sys_row: Option<(i64, String, Option<String>)> = if norm_ch == "VAULT" {
                conn.query_row(
                    "SELECT id, message_text, archived_file_path FROM chat_messages
                     WHERE project_id = ?1 AND COALESCE(session_id, '') = ?2
                       AND channel = 'VAULT' AND sender = 'SYSTEM' AND id > ?3 AND id < ?4
                     ORDER BY id ASC LIMIT 1",
                    params![pid, sid, uid, next_uid],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .ok()
            } else {
                conn.query_row(
                    "SELECT id, message_text, archived_file_path FROM chat_messages
                     WHERE project_id = ?1 AND COALESCE(session_id, '') = ?2
                       AND channel != 'VAULT' AND sender = 'SYSTEM' AND id > ?3 AND id < ?4
                     ORDER BY id ASC LIMIT 1",
                    params![pid, sid, uid, next_uid],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .ok()
            };

            let (sys_id, sys_err, sys_arch) = match sys_row {
                Some((id, msg, arch)) => (Some(id), Some(msg), arch),
                None => (None, None, None),
            };

            let raw_match: Option<(i64, String)> = if let Some(ref arch) = uarch {
                conn.query_row(
                    "SELECT id, source_type FROM raw_events
                     WHERE project_id = ?1 AND archived_path = ?2 ORDER BY id DESC LIMIT 1",
                    params![pid, arch],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .ok()
            } else {
                None
            }
            .or_else(|| {
                conn.query_row(
                    "SELECT id, source_type FROM raw_events
                     WHERE project_id = ?1 AND raw_content = ?2 ORDER BY id DESC LIMIT 1",
                    params![pid, utext],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .ok()
            });

            let (raw_event_id, source_type) = match raw_match {
                Some((eid, st)) => (Some(eid), st),
                None => (
                    None,
                    if norm_ch == "VAULT" {
                        "VAULT_CHAT".to_string()
                    } else {
                        "USER_CHAT".to_string()
                    },
                ),
            };

            let kdoc_id = find_unanalyzed_knowledge_doc_id(
                conn,
                &pid,
                sys_arch.as_deref(),
                utext,
            );

            enqueue_pending_ai_item(
                conn,
                &pid,
                &sid,
                &norm_ch,
                Some(uid),
                sys_id,
                kdoc_id,
                raw_event_id,
                &build_queue_preview(utext),
                &source_type,
                uarch.as_deref().or(sys_arch.as_deref()),
                sys_err.as_deref(),
            )?;
            inserted += 1;
        }
    }

    // 4. Scan any unanalyzed knowledge_documents not yet represented in pending_ai_queue
    let mut doc_stmt = conn.prepare(
        "SELECT id, project_id, title, original_filename, archived_path, executive_summary
         FROM knowledge_documents
         WHERE (processing_status IN ('UNPROCESSED', 'FAILED') OR executive_summary LIKE ?1)
           AND id NOT IN (SELECT knowledge_doc_id FROM pending_ai_queue WHERE knowledge_doc_id IS NOT NULL)
         ORDER BY ingested_at ASC, id ASC",
    )?;
    let orphan_docs: Vec<(i64, String, String, String, String, String)> = doc_stmt
        .query_map(params![like_prefix], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (doc_id, pid, title, orig_fname, arch_path, summary) in orphan_docs {
        let preview = if orig_fname.trim().is_empty() {
            format!("📄 {}", title)
        } else {
            format!("📄 {} ({})", title, orig_fname)
        };
        let sid = ensure_active_chat_session(conn, &pid, "VAULT").unwrap_or_default();
        enqueue_pending_ai_item(
            conn,
            &pid,
            &sid,
            "VAULT",
            None,
            None,
            Some(doc_id),
            None,
            &build_queue_preview(&preview),
            "DOC_ONBOARDING",
            Some(&arch_path),
            Some(&summary),
        )?;
        inserted += 1;
    }

    // 5. Scan any historical MAB_FOCUS_SYNC raw_events not yet represented in pending_ai_queue
    let mut focus_stmt = conn.prepare(
        "SELECT id, project_id, raw_content, archived_path, created_at
         FROM raw_events
         WHERE source_type = 'MAB_FOCUS_SYNC'
           AND id NOT IN (SELECT raw_event_id FROM pending_ai_queue WHERE raw_event_id IS NOT NULL)
         ORDER BY created_at ASC, id ASC",
    )?;
    let orphan_focus: Vec<(i64, String, String, Option<String>, String)> = focus_stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (raw_id, pid, content, arch_path, created_at) in orphan_focus {
        let first_line = build_queue_preview(&content);
        let title = format!("گزارش Mab-Focus: {}", first_line);
        let doc_fname = format!("mabfocus_ev_{}.md", raw_id);

        let existing_kdoc: Option<i64> = if let Some(ref ap) = arch_path {
            conn.query_row(
                "SELECT id FROM knowledge_documents WHERE project_id = ?1 AND archived_path = ?2 ORDER BY id DESC LIMIT 1",
                params![pid, ap],
                |r| r.get(0),
            )
            .ok()
        } else {
            None
        };

        let kdoc_id = match existing_kdoc {
            Some(id) => Some(id),
            None => conn
                .execute(
                    "INSERT INTO knowledge_documents (
                        project_id, title, doc_type, original_filename, archived_path,
                        executive_summary, extracted_items_count, ingested_at, processing_status
                     ) VALUES (?1, ?2, 'MAB_FOCUS_REPORT', ?3, ?4, ?5, 0, ?6, 'UNPROCESSED')",
                    params![
                        pid,
                        title,
                        doc_fname,
                        arch_path.as_deref().unwrap_or(""),
                        "در انتظار پردازش توسط هوش مصنوعی (گزارش دریافت‌شده از Mab-Focus)...",
                        created_at
                    ],
                )
                .ok()
                .map(|_| conn.last_insert_rowid()),
        };

        let sid = ensure_active_chat_session(conn, &pid, "COPILOT").unwrap_or_default();
        enqueue_pending_ai_item(
            conn,
            &pid,
            &sid,
            "COPILOT",
            None,
            None,
            kdoc_id,
            Some(raw_id),
            &format!("⏱️ {}", first_line),
            "MAB_FOCUS_SYNC",
            arch_path.as_deref(),
            None,
        )?;
        inserted += 1;
    }

    Ok(inserted)
}

pub fn query_pending_ai_queue(conn: &Connection) -> Result<Vec<PendingQueueItem>> {
    let mut stmt = conn.prepare(
        "SELECT q.id, q.project_id, COALESCE(p.name, q.project_id), q.session_id, q.channel,
                q.user_message_id, q.system_message_id, q.knowledge_doc_id, q.raw_event_id,
                q.title_preview, q.source_type, q.archived_path, q.status, q.attempts,
                q.last_error, q.priority_order,
                COALESCE(cm.created_at, re.created_at, kd.ingested_at, q.created_at) AS effective_created_at,
                q.updated_at
         FROM pending_ai_queue q
         LEFT JOIN projects p ON p.id = q.project_id
         LEFT JOIN chat_messages cm ON cm.id = q.user_message_id
         LEFT JOIN raw_events re ON re.id = q.raw_event_id
         LEFT JOIN knowledge_documents kd ON kd.id = q.knowledge_doc_id
         WHERE q.status IN ('PENDING', 'PROCESSING')
         ORDER BY q.priority_order DESC, effective_created_at ASC, q.id ASC",
    )?;
    let items: Vec<PendingQueueItem> = stmt
        .query_map([], |r| {
            Ok(PendingQueueItem {
                id: r.get(0)?,
                project_id: r.get(1)?,
                project_name: r.get(2)?,
                session_id: r.get(3)?,
                channel: r.get(4)?,
                user_message_id: r.get(5)?,
                system_message_id: r.get(6)?,
                knowledge_doc_id: r.get(7)?,
                raw_event_id: r.get(8)?,
                title_preview: r.get(9)?,
                source_type: r.get(10)?,
                archived_path: r.get(11)?,
                status: r.get(12)?,
                attempts: r.get(13)?,
                last_error: r.get(14)?,
                priority_order: r.get(15).unwrap_or(0),
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(items)
}

pub fn get_pending_queue_item(conn: &Connection, queue_id: i64) -> Result<PendingQueueItem> {
    conn.query_row(
        "SELECT q.id, q.project_id, COALESCE(p.name, q.project_id), q.session_id, q.channel,
                q.user_message_id, q.system_message_id, q.knowledge_doc_id, q.raw_event_id,
                q.title_preview, q.source_type, q.archived_path, q.status, q.attempts,
                q.last_error, q.priority_order, q.created_at, q.updated_at
         FROM pending_ai_queue q
         LEFT JOIN projects p ON p.id = q.project_id
         WHERE q.id = ?1",
        params![queue_id],
        |r| {
            Ok(PendingQueueItem {
                id: r.get(0)?,
                project_id: r.get(1)?,
                project_name: r.get(2)?,
                session_id: r.get(3)?,
                channel: r.get(4)?,
                user_message_id: r.get(5)?,
                system_message_id: r.get(6)?,
                knowledge_doc_id: r.get(7)?,
                raw_event_id: r.get(8)?,
                title_preview: r.get(9)?,
                source_type: r.get(10)?,
                archived_path: r.get(11)?,
                status: r.get(12)?,
                attempts: r.get(13)?,
                last_error: r.get(14)?,
                priority_order: r.get(15).unwrap_or(0),
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        },
    )
}

/// Reorders a pending queue item ("TOP" | "UP" | "DOWN") by normalizing priority_order across active queue items
pub fn reorder_pending_queue_item(
    conn: &Connection,
    queue_id: i64,
    direction: &str,
) -> Result<()> {
    let items = query_pending_ai_queue(conn)?;
    if items.len() <= 1 {
        return Ok(());
    }
    let Some(idx) = items.iter().position(|it| it.id == queue_id) else {
        return Ok(());
    };

    let mut ordered_ids: Vec<i64> = items.iter().map(|it| it.id).collect();
    match direction.trim().to_uppercase().as_str() {
        "TOP" => {
            if idx == 0 {
                return Ok(());
            }
            let id = ordered_ids.remove(idx);
            ordered_ids.insert(0, id);
        }
        "UP" => {
            if idx == 0 {
                return Ok(());
            }
            ordered_ids.swap(idx - 1, idx);
        }
        "DOWN" => {
            if idx + 1 >= ordered_ids.len() {
                return Ok(());
            }
            ordered_ids.swap(idx, idx + 1);
        }
        _ => return Ok(()),
    }

    let total = ordered_ids.len() as i32;
    let now = Local::now().to_rfc3339();
    for (pos, id) in ordered_ids.into_iter().enumerate() {
        let rank = total - pos as i32;
        conn.execute(
            "UPDATE pending_ai_queue SET priority_order = ?1, updated_at = ?2 WHERE id = ?3",
            params![rank, now, id],
        )?;
    }
    Ok(())
}

/// Resets manual priority overrides so the queue falls back strictly to chronological creation time
pub fn reset_pending_queue_order(conn: &Connection) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE pending_ai_queue SET priority_order = 0, updated_at = ?1 WHERE status IN ('PENDING', 'PROCESSING')",
        params![now],
    )?;
    Ok(())
}

// ============================================================================
// Manual 5-Layer Ecosystem Management (Outcomes, WorkLines, Tasks, Cards, Proposals, Calendar)
// ============================================================================

#[allow(clippy::too_many_arguments)]
pub fn upsert_outcome_manual(
    conn: &Connection,
    outcome_id: Option<&str>,
    domain_code: &str,
    title: &str,
    desired_reality_change: &str,
    evidence_kr: &str,
    time_window: &str,
    status: &str,
    progress_percent: i32,
) -> Result<String> {
    let now = Local::now().to_rfc3339();
    let clean_title = title.trim();
    if clean_title.is_empty() {
        return Err(rusqlite::Error::InvalidParameterName("عنوان Outcome نمی‌تواند خالی باشد".into()));
    }
    let clean_domain = if domain_code.trim().is_empty() {
        "WEALTH_FINANCE"
    } else {
        domain_code.trim()
    };
    let clean_status = if status.trim().is_empty() { "ACTIVE" } else { status.trim() };
    let clamped_progress = progress_percent.clamp(0, 100);

    let target_id = outcome_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("OUT-{}", &uuid::Uuid::new_v4().to_string()[..4].to_uppercase()));

    let exists: bool = conn
        .query_row("SELECT COUNT(*) FROM outcomes WHERE id = ?1", params![target_id], |r| r.get::<_, i64>(0))
        .unwrap_or(0)
        > 0;

    if exists {
        conn.execute(
            "UPDATE outcomes
             SET domain_code = ?1, title = ?2, desired_reality_change = ?3, evidence_kr = ?4,
                 time_window = ?5, status = ?6, progress_percent = ?7, updated_at = ?8
             WHERE id = ?9",
            params![
                clean_domain,
                clean_title,
                desired_reality_change.trim(),
                evidence_kr.trim(),
                time_window.trim(),
                clean_status,
                clamped_progress,
                now,
                target_id
            ],
        )?;
    } else {
        conn.execute(
            "INSERT INTO outcomes (
                id, domain_code, title, desired_reality_change, evidence_kr, time_window,
                status, progress_percent, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![
                target_id,
                clean_domain,
                clean_title,
                desired_reality_change.trim(),
                evidence_kr.trim(),
                time_window.trim(),
                clean_status,
                clamped_progress,
                now
            ],
        )?;
    }
    Ok(target_id)
}

pub fn delete_outcome(conn: &Connection, outcome_id: &str) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE work_lines SET outcome_id = NULL, updated_at = ?1 WHERE outcome_id = ?2",
        params![now, outcome_id],
    )?;
    conn.execute(
        "UPDATE work_entities SET outcome_id = NULL, updated_at = ?1 WHERE outcome_id = ?2",
        params![now, outcome_id],
    )?;
    conn.execute("DELETE FROM outcomes WHERE id = ?1", params![outcome_id])?;
    Ok(())
}

pub fn update_work_line_manual(
    conn: &Connection,
    work_line_id: &str,
    title: &str,
    goal_statement: &str,
    outcome_id: Option<&str>,
    color_hex: &str,
    status: &str,
) -> Result<()> {
    let now = Local::now().to_rfc3339();
    let norm_outcome = outcome_id.map(str::trim).filter(|s| !s.is_empty());
    let clean_color = if color_hex.trim().is_empty() { "#3B82F6" } else { color_hex.trim() };
    let clean_status = if status.trim().is_empty() { "ACTIVE" } else { status.trim() };

    conn.execute(
        "UPDATE work_lines
         SET title = ?1, goal_statement = ?2, outcome_id = ?3, color_hex = ?4, status = ?5, updated_at = ?6
         WHERE id = ?7",
        params![title.trim(), goal_statement.trim(), norm_outcome, clean_color, clean_status, now, work_line_id],
    )?;

    // Propagate outcome change to member tasks of this work line
    conn.execute(
        "UPDATE work_entities SET outcome_id = ?1, updated_at = ?2 WHERE work_line_id = ?3",
        params![norm_outcome, now, work_line_id],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn assign_task_grouping(
    conn: &Connection,
    project_id: &str,
    task_id: &str,
    work_line_id: Option<&str>,
    outcome_id: Option<&str>,
    priority_score: Option<i32>,
    energy_tag: Option<&str>,
    device_tag: Option<&str>,
    duration_minutes: Option<i32>,
    scheduled_date: Option<&str>,
) -> Result<()> {
    let _ = ensure_chain_columns(conn);
    let now = Local::now().to_rfc3339();
    let norm_wl = work_line_id.map(str::trim).filter(|s| !s.is_empty());
    let mut norm_out = outcome_id.map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);

    // If assigned to a work_line and no explicit outcome_id given, inherit the work_line's outcome_id
    if let Some(wl_id) = norm_wl {
        if norm_out.is_none() {
            norm_out = conn
                .query_row("SELECT outcome_id FROM work_lines WHERE id = ?1", params![wl_id], |r| r.get::<_, Option<String>>(0))
                .ok()
                .flatten();
        }
    }

    // Compute sequence_order & lock status if moved into a SEQUENTIAL_CHAIN
    let mut next_seq: i32 = 1;
    let mut next_depends: Option<String> = None;
    let mut should_lock = false;
    let mut next_status_override: Option<&str> = None;

    if let Some(wl_id) = norm_wl {
        let line_mode: String = conn
            .query_row("SELECT line_mode FROM work_lines WHERE id = ?1", params![wl_id], |r| r.get(0))
            .unwrap_or_else(|_| "GOAL_CLUSTER".to_string());
        let max_seq: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(sequence_order), 0) FROM work_entities WHERE work_line_id = ?1 AND id != ?2",
                params![wl_id, task_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        next_seq = max_seq + 1;
        if line_mode == "SEQUENTIAL_CHAIN" && max_seq > 0 {
            let open_prev: Option<String> = conn
                .query_row(
                    "SELECT id FROM work_entities
                     WHERE work_line_id = ?1 AND id != ?2 AND status != 'CLOSED'
                     ORDER BY sequence_order DESC LIMIT 1",
                    params![wl_id, task_id],
                    |r| r.get(0),
                )
                .ok();
            if let Some(prev_id) = open_prev {
                next_depends = Some(prev_id);
                should_lock = true;
                next_status_override = Some("QUEUED_IN_CHAIN");
            }
        }
    } else {
        // Standalone task cannot remain QUEUED_IN_CHAIN
        let cur_status: String = conn
            .query_row("SELECT status FROM work_entities WHERE id = ?1", params![task_id], |r| r.get(0))
            .unwrap_or_else(|_| "OPEN".to_string());
        if cur_status == "QUEUED_IN_CHAIN" {
            next_status_override = Some("OPEN");
        }
    }

    conn.execute(
        "UPDATE work_entities
         SET work_line_id = ?1,
             outcome_id = ?2,
             sequence_order = ?3,
             depends_on_task_id = ?4,
             is_locked_by_chain = ?5,
             status = COALESCE(?6, status),
             priority_score = COALESCE(?7, priority_score),
             energy_tag = COALESCE(?8, energy_tag),
             device_tag = COALESCE(?9, device_tag),
             duration_minutes = COALESCE(?10, duration_minutes),
             scheduled_date = CASE WHEN ?11 IS NOT NULL THEN NULLIF(?11, '') ELSE scheduled_date END,
             updated_at = ?12
         WHERE id = ?13 AND project_id = ?14",
        params![
            norm_wl,
            norm_out,
            next_seq,
            next_depends,
            if should_lock { 1 } else { 0 },
            next_status_override,
            priority_score,
            energy_tag.map(str::trim).filter(|s| !s.is_empty()),
            device_tag.map(str::trim).filter(|s| !s.is_empty()),
            duration_minutes,
            scheduled_date,
            now,
            task_id,
            project_id
        ],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn create_task_manual(
    conn: &Connection,
    project_id: &str,
    title: &str,
    category: &str,
    definition_of_done: &str,
    first_physical_action: &str,
    work_line_id: Option<&str>,
    outcome_id: Option<&str>,
    energy_tag: &str,
    device_tag: &str,
    duration_minutes: i32,
    scheduled_date: Option<&str>,
) -> Result<String> {
    let _ = ensure_chain_columns(conn);
    let now = Local::now().to_rfc3339();
    let task_id = format!("TSK-{}", &uuid::Uuid::new_v4().to_string()[..4].to_uppercase());
    let clean_cat = if category.trim().is_empty() { "GENERAL" } else { category.trim() };
    let clean_energy = if energy_tag.trim().is_empty() { "MEDIUM_NORMAL" } else { energy_tag.trim() };
    let clean_device = if device_tag.trim().is_empty() { "LAPTOP_ONLY" } else { device_tag.trim() };
    let dur = duration_minutes.max(5);
    let pomos = (dur as f64 / 25.0).max(0.5);

    conn.execute(
        "INSERT INTO work_entities (
            id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
            status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos, calibrated_pomos,
            energy_tag, duration_minutes, device_tag, scheduled_date, priority_score, updated_at
         ) VALUES (?1, ?2, 'TASK', ?3, ?4, ?5, ?6, 'OPEN', 0, ?7, ?7, ?7, ?7, ?8, ?9, ?10, ?11, 50, ?12)",
        params![
            task_id,
            project_id,
            clean_cat,
            title.trim(),
            definition_of_done.trim(),
            first_physical_action.trim(),
            pomos,
            clean_energy,
            dur,
            clean_device,
            scheduled_date.map(str::trim).filter(|s| !s.is_empty()),
            now
        ],
    )?;

    if work_line_id.is_some() || outcome_id.is_some() {
        assign_task_grouping(
            conn,
            project_id,
            &task_id,
            work_line_id,
            outcome_id,
            None,
            None,
            None,
            None,
            None,
        )?;
    }
    Ok(task_id)
}

#[allow(clippy::too_many_arguments)]
pub fn create_personal_vault_item(
    conn: &Connection,
    project_id: Option<&str>,
    realm: &str,
    domain_code: &str,
    item_type: &str,
    title: &str,
    content_markdown: &str,
    tags: &[String],
) -> Result<String> {
    let now = Local::now().to_rfc3339();
    let id = format!("PVI-{}", &uuid::Uuid::new_v4().to_string()[..4].to_uppercase());
    let tags_json = serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string());
    let clean_realm = if realm.trim().is_empty() { "WORK" } else { realm.trim() };
    let clean_domain = if domain_code.trim().is_empty() { "MIND_LEARNING" } else { domain_code.trim() };
    let clean_type = if item_type.trim().is_empty() { "MEMORY_CARD" } else { item_type.trim() };

    conn.execute(
        "INSERT INTO personal_vault_items (
            id, realm, domain_code, project_id, item_type, title, content_markdown, tags_json, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            id,
            clean_realm,
            clean_domain,
            project_id.map(str::trim).filter(|s| !s.is_empty()),
            clean_type,
            title.trim(),
            content_markdown.trim(),
            tags_json,
            now
        ],
    )?;
    Ok(id)
}

pub fn delete_personal_vault_item(conn: &Connection, item_id: &str) -> Result<()> {
    conn.execute("DELETE FROM personal_vault_items WHERE id = ?1", params![item_id])?;
    Ok(())
}

pub fn resolve_pending_proposal(conn: &Connection, proposal_id: &str, approve: bool) -> Result<()> {
    let now = Local::now().to_rfc3339();
    let status = if approve { "APPROVED" } else { "REJECTED" };
    conn.execute(
        "UPDATE pending_proposals SET status = ?1, resolved_at = ?2 WHERE id = ?3",
        params![status, now, proposal_id],
    )?;
    Ok(())
}

pub fn upsert_calendar_day(
    conn: &Connection,
    date_key: &str,
    total_capacity_minutes: i32,
    deep_capacity_minutes: i32,
    day_note: &str,
    is_off_day: bool,
) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO calendar_days (date_key, total_capacity_minutes, deep_capacity_minutes, day_note, is_off_day, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(date_key) DO UPDATE SET
            total_capacity_minutes = excluded.total_capacity_minutes,
            deep_capacity_minutes = excluded.deep_capacity_minutes,
            day_note = excluded.day_note,
            is_off_day = excluded.is_off_day,
            updated_at = excluded.updated_at",
        params![
            date_key.trim(),
            total_capacity_minutes.max(0),
            deep_capacity_minutes.max(0),
            day_note.trim(),
            if is_off_day { 1 } else { 0 },
            now
        ],
    )?;
    Ok(())
}

pub fn set_pending_item_processing(conn: &Connection, queue_id: i64) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE pending_ai_queue SET status = 'PROCESSING', updated_at = ?1 WHERE id = ?2",
        params![now, queue_id],
    )?;
    Ok(())
}

pub fn record_pending_item_failure(conn: &Connection, queue_id: i64, err: &str) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE pending_ai_queue
         SET status = 'PENDING', attempts = attempts + 1, last_error = ?1, updated_at = ?2
         WHERE id = ?3",
        params![err, now, queue_id],
    )?;
    Ok(())
}

pub fn mark_pending_item_done(conn: &Connection, queue_id: i64) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE pending_ai_queue SET status = 'DONE', last_error = NULL, updated_at = ?1 WHERE id = ?2",
        params![now, queue_id],
    )?;
    Ok(())
}

pub fn complete_pending_items_for_user_msg(
    conn: &Connection,
    user_message_id: i64,
    knowledge_doc_id: Option<i64>,
) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE pending_ai_queue
         SET status = 'DONE', last_error = NULL, updated_at = ?1
         WHERE user_message_id = ?2
            OR (?3 IS NOT NULL AND knowledge_doc_id = ?3)",
        params![now, user_message_id, knowledge_doc_id],
    )?;
    Ok(())
}

pub fn dismiss_pending_queue_item(conn: &Connection, queue_id: i64) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE pending_ai_queue SET status = 'DISMISSED', updated_at = ?1 WHERE id = ?2",
        params![now, queue_id],
    )?;
    Ok(())
}

pub fn search_raw_events_fts(
    conn: &Connection,
    project_id: &str,
    keyword: &str,
) -> Result<Vec<(String, String)>> {
    let trimmed = keyword.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let tokens: Vec<String> = trimmed
        .split_whitespace()
        .filter(|t| !matches!(t.to_uppercase().as_str(), "OR" | "AND" | "NOT"))
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric() && c < '\u{0600}'))
        .filter(|t| !t.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect();

    let mut fts_hits: Vec<(String, String)> = Vec::new();
    if !tokens.is_empty() {
        let safe_fts_query = tokens.join(" OR ");
        let mut stmt = conn.prepare(
            "SELECT r.created_at, r.raw_content
             FROM raw_events_fts f
             JOIN raw_events r ON f.rowid = r.id
             WHERE f.project_id = ?1 AND raw_events_fts MATCH ?2
             ORDER BY rank LIMIT 8",
        )?;
        let res = stmt.query_map(params![project_id, safe_fts_query], |r| {
            Ok((r.get(0)?, r.get(1)?))
        });
        if let Ok(rows) = res {
            fts_hits = rows.filter_map(|r| r.ok()).collect();
        }
    }

    if !fts_hits.is_empty() {
        return Ok(fts_hits);
    }

    let pattern = format!("%{}%", trimmed);
    let mut fallback = conn.prepare(
        "SELECT created_at, raw_content
         FROM raw_events
         WHERE project_id = ?1 AND raw_content LIKE ?2
         ORDER BY id DESC LIMIT 8",
    )?;
    let rows = fallback.query_map(params![project_id, pattern], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn undo_last_mutation(conn: &mut Connection, project_id: &str) -> Result<String> {
    let row: Option<(i64, String)> = conn
        .query_row(
            "SELECT id, snapshot_json FROM mutation_audit_logs WHERE project_id = ?1 ORDER BY id DESC LIMIT 1",
            params![project_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();

    if let Some((audit_id, snapshot_json)) = row {
        let tx = conn.transaction()?;

        if let Ok(snap) = serde_json::from_str::<UndoSnapshot>(&snapshot_json) {
            tx.execute(
                "DELETE FROM work_entities WHERE project_id = ?1",
                params![project_id],
            )?;
            for e in snap.entities {
                tx.execute(
                    "INSERT INTO work_entities (
                        id, project_id, entity_type, category, title, definition_of_done,
                        first_physical_action, status, is_on_critical_path, optimistic_pomos,
                        most_likely_pomos, pessimistic_pomos, calibrated_pomos, superseded_by_id,
                        stakeholder_tag, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        e.id,
                        e.project_id,
                        e.entity_type,
                        e.category,
                        e.title,
                        e.definition_of_done,
                        e.first_physical_action,
                        e.status,
                        if e.is_on_critical_path { 1 } else { 0 },
                        e.optimistic_pomos,
                        e.most_likely_pomos,
                        e.pessimistic_pomos,
                        e.calibrated_pomos,
                        e.superseded_by_id,
                        e.stakeholder_tag,
                        e.updated_at
                    ],
                )?;
            }
            tx.execute(
                "DELETE FROM sop_rules WHERE project_id = ?1 AND id > ?2",
                params![project_id, snap.max_sop_id],
            )?;
            tx.execute(
                "DELETE FROM learned_antipatterns WHERE project_id = ?1 AND id > ?2",
                params![project_id, snap.max_anti_id],
            )?;
            tx.execute(
                "DELETE FROM session_work_logs WHERE project_id = ?1 AND id > ?2",
                params![project_id, snap.max_log_id],
            )?;
        } else {
            let restored: Vec<WorkEntity> =
                serde_json::from_str(&snapshot_json).unwrap_or_default();
            tx.execute(
                "DELETE FROM work_entities WHERE project_id = ?1 AND entity_type = 'TASK'",
                params![project_id],
            )?;
            for e in restored {
                tx.execute(
                    "INSERT INTO work_entities (
                        id, project_id, entity_type, category, title, definition_of_done,
                        first_physical_action, status, is_on_critical_path, optimistic_pomos,
                        most_likely_pomos, pessimistic_pomos, calibrated_pomos, stakeholder_tag, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                    params![
                        e.id,
                        e.project_id,
                        e.entity_type,
                        e.category,
                        e.title,
                        e.definition_of_done,
                        e.first_physical_action,
                        e.status,
                        if e.is_on_critical_path { 1 } else { 0 },
                        e.optimistic_pomos,
                        e.most_likely_pomos,
                        e.pessimistic_pomos,
                        e.calibrated_pomos,
                        e.stakeholder_tag,
                        e.updated_at
                    ],
                )?;
            }
        }

        tx.execute(
            "DELETE FROM mutation_audit_logs WHERE id = ?1",
            params![audit_id],
        )?;
        tx.commit()?;
        Ok("آخرین تغییر به حالت قبل بازگردانده شد.".into())
    } else {
        Ok("تراکنشی برای بازگردانی وجود ندارد.".into())
    }
}

// ==============================================================================
// SEQUENTIAL CHAIN ENGINE & CONTEXT-AWARE TASK EXECUTION (v3.0 MILESTONE 2)
// ==============================================================================

/// Ensures `is_locked_by_chain` column exists on `work_entities`
pub fn ensure_chain_columns(conn: &Connection) -> Result<()> {
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(work_entities)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        rows.filter_map(|r| r.ok()).collect()
    };
    if !pragma_cols.iter().any(|c| c == "is_locked_by_chain") {
        let _ = conn.execute(
            "ALTER TABLE work_entities ADD COLUMN is_locked_by_chain INTEGER NOT NULL DEFAULT 0",
            [],
        );
    }
    Ok(())
}

/// Helper to load a single WorkEntity by its ID with all v3.0 columns
pub fn query_work_entity_by_id(conn: &Connection, task_id: &str) -> Result<Option<WorkEntity>> {
    let _ = ensure_chain_columns(conn);
    let mut stmt = conn.prepare(
        "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos,
                calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at,
                work_line_id, outcome_id, sequence_order, depends_on_task_id, energy_tag, duration_minutes,
                device_tag, location_tags_json, scheduled_date, scheduled_slot, due_date, priority_score,
                is_locked_by_chain
         FROM work_entities WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![task_id])?;
    if let Some(r) = rows.next()? {
        let mut entity = map_work_entity_from_row(r)?;
        entity.is_locked_by_chain = is_locked_by_chain(conn, &entity.id).unwrap_or(false);
        Ok(Some(entity))
    } else {
        Ok(None)
    }
}

/// Atomically creates a WorkLine and sequential chain of tasks.
/// - Creates a `work_lines` record (ID prefix `WLN-`, status = 'ACTIVE', line_mode = 'SEQUENTIAL_CHAIN' or 'GOAL_CLUSTER').
/// - Atomically creates `work_entities` for each step:
///   - Step 1 (idx 0): sequence_order = 1, status = "IN_PROGRESS", depends_on_task_id = None, is_locked_by_chain = false, is_on_critical_path = true.
///   - Step 2..N (idx 1..N-1): sequence_order = idx + 1, status = "QUEUED_IN_CHAIN", depends_on_task_id = Some(previous_task_id), is_locked_by_chain = true, is_on_critical_path = false.
///   - Populates 4D context tags from ChainStepInput (energy_tag, duration_minutes, device_tag, location_tags_json).
/// Returns `(WorkLine, Vec<WorkEntity>)`.
pub fn create_work_line_with_chain(
    conn: &Connection,
    project_id: &str,
    outcome_id: Option<&str>,
    title: &str,
    goal_statement: &str,
    line_mode: &str,
    steps: Vec<ChainStepInput>,
) -> Result<(WorkLine, Vec<WorkEntity>)> {
    let _ = ensure_chain_columns(conn);

    let line_id = format!("WLN-{}", &uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
    let now = Local::now().to_rfc3339();
    let mode = if line_mode.trim().is_empty() {
        "SEQUENTIAL_CHAIN"
    } else {
        line_mode.trim()
    };

    conn.execute(
        "INSERT INTO work_lines (
            id, project_id, outcome_id, title, goal_statement, line_mode, status, color_hex,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'ACTIVE', '#3B82F6', ?7, ?7)",
        params![line_id, project_id, outcome_id, title, goal_statement, mode, now],
    )?;

    let mut created_entities = Vec::with_capacity(steps.len());
    let mut prev_task_id: Option<String> = None;

    for (idx, step) in steps.into_iter().enumerate() {
        let task_id = format!("TSK-{}", &uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
        let seq_order = (idx + 1) as i32;
        let is_step_one = idx == 0;
        let status = if is_step_one {
            "IN_PROGRESS"
        } else {
            "QUEUED_IN_CHAIN"
        };
        let is_locked = !is_step_one;
        let is_on_critical_path = is_step_one;
        let depends_on = if is_step_one {
            None
        } else {
            prev_task_id.clone()
        };

        let energy_tag = if step.energy_tag.trim().is_empty() {
            "MEDIUM_NORMAL".to_string()
        } else {
            step.energy_tag.trim().to_string()
        };
        let duration_minutes = if step.duration_minutes <= 0 {
            25
        } else {
            step.duration_minutes
        };
        let device_tag = if step.device_tag.trim().is_empty() {
            "LAPTOP_ONLY".to_string()
        } else {
            step.device_tag.trim().to_string()
        };
        let location_tags = if step.location_tags.is_empty() {
            vec!["DESK".to_string()]
        } else {
            step.location_tags
        };
        let location_tags_json =
            serde_json::to_string(&location_tags).unwrap_or_else(|_| "[\"DESK\"]".to_string());
        let category = if step.category.trim().is_empty() {
            "GENERAL".to_string()
        } else {
            step.category.trim().to_string()
        };
        let priority_score = 50;

        conn.execute(
            "INSERT INTO work_entities (
                id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos, calibrated_pomos,
                work_line_id, outcome_id, sequence_order, depends_on_task_id, is_locked_by_chain,
                energy_tag, duration_minutes, device_tag, location_tags_json, scheduled_date, scheduled_slot, due_date, priority_score, updated_at
            ) VALUES (
                ?1, ?2, 'TASK', ?3, ?4, ?5, ?6,
                ?7, ?8, 1.0, 2.0, 3.0, 2.0,
                ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
            )",
            params![
                task_id,
                project_id,
                category,
                step.title,
                step.definition_of_done,
                step.first_physical_action,
                status,
                if is_on_critical_path { 1 } else { 0 },
                line_id,
                outcome_id,
                seq_order,
                depends_on,
                if is_locked { 1 } else { 0 },
                energy_tag,
                duration_minutes,
                device_tag,
                location_tags_json,
                step.scheduled_date,
                step.scheduled_slot,
                step.due_date,
                priority_score,
                now
            ],
        )?;

        let entity = WorkEntity {
            id: task_id.clone(),
            project_id: project_id.to_string(),
            project_name: String::new(),
            entity_type: "TASK".to_string(),
            category,
            title: step.title,
            definition_of_done: step.definition_of_done,
            first_physical_action: step.first_physical_action,
            status: status.to_string(),
            is_on_critical_path,
            optimistic_pomos: 1.0,
            most_likely_pomos: 2.0,
            pessimistic_pomos: 3.0,
            calibrated_pomos: 2.0,
            superseded_by_id: None,
            stakeholder_tag: None,
            updated_at: now.clone(),
            work_line_id: Some(line_id.clone()),
            work_line_title: Some(title.to_string()),
            outcome_id: outcome_id.map(str::to_string),
            sequence_order: seq_order,
            depends_on_task_id: depends_on,
            is_locked_by_chain: is_locked,
            energy_tag,
            duration_minutes,
            device_tag,
            location_tags,
            scheduled_date: step.scheduled_date,
            scheduled_slot: step.scheduled_slot,
            due_date: step.due_date,
            priority_score,
        };

        prev_task_id = Some(task_id);
        created_entities.push(entity);
    }

    let work_line = WorkLine {
        id: line_id,
        project_id: project_id.to_string(),
        project_name: String::new(),
        outcome_id: outcome_id.map(str::to_string),
        outcome_title: None,
        title: title.to_string(),
        goal_statement: goal_statement.to_string(),
        line_mode: mode.to_string(),
        status: "ACTIVE".to_string(),
        color_hex: "#3B82F6".to_string(),
        total_tasks: created_entities.len() as i32,
        completed_tasks: 0,
        active_step_id: created_entities.first().map(|e| e.id.clone()),
        created_at: now.clone(),
        updated_at: now,
    };

    Ok((work_line, created_entities))
}

/// Evaluates whether a task is locked by its sequential chain:
/// - If task has no `work_line_id`, returns `false`.
/// - If parent `work_lines.line_mode != 'SEQUENTIAL_CHAIN'`, returns `false`.
/// - If task status is `'CLOSED'` or `'SUPERSEDED'`, returns `false`.
/// - Checks if any preceding step in the same chain (with `sequence_order < current_task.sequence_order`) is not `'CLOSED'`.
/// - Returns `true` if any preceding step is unclosed; otherwise `false`.
pub fn is_locked_by_chain(conn: &Connection, task_id: &str) -> Result<bool> {
    let row: Option<(Option<String>, i32, String)> = conn
        .query_row(
            "SELECT work_line_id, sequence_order, status FROM work_entities WHERE id = ?1",
            params![task_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();

    let (work_line_id, seq_order, status) = match row {
        Some((Some(wln), seq, st)) if !wln.is_empty() => (wln, seq, st),
        _ => return Ok(false),
    };

    if status == "CLOSED" || status == "SUPERSEDED" {
        return Ok(false);
    }

    let line_mode: Option<String> = conn
        .query_row(
            "SELECT line_mode FROM work_lines WHERE id = ?1",
            params![work_line_id],
            |r| r.get(0),
        )
        .ok();

    if let Some(mode) = line_mode {
        if mode != "SEQUENTIAL_CHAIN" {
            return Ok(false);
        }
    } else {
        return Ok(false);
    }

    if seq_order <= 1 {
        return Ok(false);
    }

    let unclosed_prior: i64 = conn.query_row(
        "SELECT COUNT(*) FROM work_entities WHERE work_line_id = ?1 AND sequence_order < ?2 AND status != 'CLOSED'",
        params![work_line_id, seq_order],
        |r| r.get(0),
    )?;

    Ok(unclosed_prior > 0)
}

/// Domino auto-advancement:
/// - Updates the completed task: status = 'CLOSED', is_locked_by_chain = false.
/// - Checks if the task belongs to a SEQUENTIAL_CHAIN WorkLine.
/// - If so, finds the immediate next step (`sequence_order > completed.sequence_order` and status in `["QUEUED_IN_CHAIN", "OPEN"]` ordered by `sequence_order ASC LIMIT 1`).
/// - If a next step exists: updates it to `status = 'IN_PROGRESS'`, `is_locked_by_chain = false`, `is_on_critical_path = true`, and returns `Some(WorkEntity)`.
/// - If no further uncompleted steps exist in the WorkLine (all steps are `'CLOSED'`), marks the parent `work_lines.status = 'COMPLETED'` and returns `None`.
pub fn advance_sequential_chain_on_complete(
    conn: &Connection,
    project_id: &str,
    completed_task_id: &str,
) -> Result<Option<WorkEntity>> {
    let _ = ensure_chain_columns(conn);
    let now = Local::now().to_rfc3339();

    // 1. Mark completed task as CLOSED and unlocked
    conn.execute(
        "UPDATE work_entities SET status = 'CLOSED', is_locked_by_chain = 0, updated_at = ?1
         WHERE id = ?2 AND project_id = ?3",
        params![now, completed_task_id, project_id],
    )?;

    // 2. Fetch work_line_id and sequence_order of completed task
    let meta: Option<(Option<String>, i32)> = conn
        .query_row(
            "SELECT work_line_id, sequence_order FROM work_entities WHERE id = ?1",
            params![completed_task_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();

    if let Some((Some(line_id), seq_order)) = meta {
        // Check if parent work_line is SEQUENTIAL_CHAIN
        let line_mode: Option<String> = conn
            .query_row(
                "SELECT line_mode FROM work_lines WHERE id = ?1",
                params![line_id],
                |r| r.get(0),
            )
            .ok();

        if line_mode.as_deref() == Some("SEQUENTIAL_CHAIN") {
            // Find next step in chain
            let next_task: Option<String> = conn
                .query_row(
                    "SELECT id FROM work_entities
                     WHERE work_line_id = ?1 AND status IN ('QUEUED_IN_CHAIN', 'OPEN') AND sequence_order > ?2
                     ORDER BY sequence_order ASC LIMIT 1",
                    params![line_id, seq_order],
                    |r| r.get(0),
                )
                .ok();

            if let Some(next_id) = next_task {
                conn.execute(
                    "UPDATE work_entities
                     SET status = 'IN_PROGRESS', is_locked_by_chain = 0, is_on_critical_path = 1, updated_at = ?1
                     WHERE id = ?2",
                    params![now, next_id],
                )?;
                return query_work_entity_by_id(conn, &next_id);
            } else {
                // Check if all tasks in the work_line are CLOSED
                let open_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM work_entities WHERE work_line_id = ?1 AND status != 'CLOSED'",
                    params![line_id],
                    |r| r.get(0),
                )?;

                if open_count == 0 {
                    conn.execute(
                        "UPDATE work_lines SET status = 'COMPLETED', updated_at = ?1 WHERE id = ?2",
                        params![now, line_id],
                    )?;
                }
            }
        }
    }

    Ok(None)
}

/// Queries work_entities for actionable tasks matching user execution context:
/// - Filters by entity_type = 'TASK' and status NOT IN ('CLOSED', 'SUPERSEDED').
/// - If `only_unlocked == true`, filters out any task where `is_locked_by_chain` is true or `is_locked_by_chain(conn, &task.id)?` is true.
/// - Matches `device_tag` (if provided, matches or if task device_tag is `ANY_DEVICE`).
/// - Matches `location_tag` (if provided, checks if `location_tags_json` contains the tag or `ANY_LOCATION`).
/// - Matches `energy_tag` (if provided, matches energy level).
/// - Matches `max_minutes` (if provided, duration_minutes <= max_minutes).
/// - Orders by priority_score DESC, sequence_order ASC, updated_at DESC.
pub fn query_tasks_by_context(
    conn: &Connection,
    project_id: &str,
    device_tag: Option<&str>,
    location_tag: Option<&str>,
    energy_tag: Option<&str>,
    max_minutes: Option<i32>,
    only_unlocked: bool,
) -> Result<Vec<WorkEntity>> {
    let _ = ensure_chain_columns(conn);

    let mut sql = String::from(
        "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos,
                calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at,
                work_line_id, outcome_id, sequence_order, depends_on_task_id, energy_tag, duration_minutes,
                device_tag, location_tags_json, scheduled_date, scheduled_slot, due_date, priority_score,
                is_locked_by_chain
         FROM work_entities
         WHERE entity_type = 'TASK' AND status NOT IN ('CLOSED', 'SUPERSEDED')"
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if !project_id.trim().is_empty() {
        sql.push_str(" AND project_id = ?");
        params_vec.push(Box::new(project_id.trim().to_string()));
    }

    if only_unlocked {
        sql.push_str(" AND status != 'QUEUED_IN_CHAIN' AND is_locked_by_chain = 0");
    }

    if let Some(dev) = device_tag.map(str::trim).filter(|s| !s.is_empty() && *s != "ALL") {
        sql.push_str(" AND (device_tag = ? OR device_tag = 'ANY_DEVICE')");
        params_vec.push(Box::new(dev.to_string()));
    }

    if let Some(loc) = location_tag.map(str::trim).filter(|s| !s.is_empty() && *s != "ALL") {
        sql.push_str(" AND (location_tags_json LIKE ? OR location_tags_json LIKE '%\"ANY_LOCATION\"%')");
        params_vec.push(Box::new(format!("%\"{}\"%", loc)));
    }

    if let Some(eng) = energy_tag.map(str::trim).filter(|s| !s.is_empty() && *s != "ALL") {
        sql.push_str(" AND energy_tag = ?");
        params_vec.push(Box::new(eng.to_string()));
    }

    if let Some(max_m) = max_minutes.filter(|m| *m > 0) {
        sql.push_str(" AND duration_minutes <= ?");
        params_vec.push(Box::new(max_m));
    }

    sql.push_str(" ORDER BY priority_score DESC, sequence_order ASC, updated_at DESC");

    let params_slice: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_slice.as_slice(), map_work_entity_from_row)?;

    let mut result = Vec::new();
    for item in rows {
        let mut task = item?;
        let locked = is_locked_by_chain(conn, &task.id)?;
        task.is_locked_by_chain = locked;

        if only_unlocked && locked {
            continue;
        }

        if let Some(loc) = location_tag.map(str::trim).filter(|s| !s.is_empty() && *s != "ALL") {
            if !task.location_tags.iter().any(|t| t == loc || t == "ANY_LOCATION") {
                continue;
            }
        }

        result.push(task);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::init_database;
    use std::path::PathBuf;

    #[test]
    fn test_v2_copilot_apply_quick_complete_fts_and_undo() {
        let dir = std::env::temp_dir().join(format!("mab_compass_q_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test_v2.sqlite");
        let mut conn = init_database(&db_path, &dir).unwrap();

        let env = CopilotEnvelope {
            conversational_reply: "گام جدید و تصمیم معماری ثبت شدند.".into(),
            clarifying_questions: vec![],
            needs_user_clarification: false,
            meeting_detected: true,
            meeting_title: Some("جلسه طراحی معماری".into()),
            meeting_summary_markdown: Some("توافق روی پایگاه‌داده انجام شد.".into()),
            executive_doc_summary: None,
            method_playbooks: vec![],
            mutations: vec![
                AtomicMutation {
                    op: "CREATE_ENTITY".into(),
                    target_id: None,
                    entity_type: Some("TASK".into()),
                    category: Some("BACKEND".into()),
                    title: Some("ساخت اندپوینت پرداخت".into()),
                    definition_of_done: Some("تست پرداخت موفق".into()),
                    first_physical_action: Some("باز کردن payment.rs".into()),
                    status: None,
                    is_on_critical_path: Some(true),
                    optimistic_pomos: Some(1.0),
                    most_likely_pomos: Some(2.0),
                    pessimistic_pomos: Some(3.0),
                    stakeholder_tag: Some("@Mohandes".into()),
                    sop_rule_statement: None,
                    mistake_pattern: None,
                    preventive_gate_question: None,
                    severity_score: None,
                    effective_pomos: None,
                    deliverable_summary: None,
                    internal_friction_note: None,
                    client_sanitized_note: None,
                    ..Default::default()
                },
                AtomicMutation {
                    op: "CREATE_ENTITY".into(),
                    target_id: None,
                    entity_type: Some("TASK".into()),
                    category: Some("UI_UX".into()),
                    title: Some("طراحی صفحه تایید".into()),
                    definition_of_done: Some("رندر بدون خطا".into()),
                    first_physical_action: Some("باز کردن Checkout.tsx".into()),
                    status: None,
                    is_on_critical_path: Some(false),
                    optimistic_pomos: Some(1.0),
                    most_likely_pomos: Some(1.0),
                    pessimistic_pomos: Some(2.0),
                    stakeholder_tag: None,
                    sop_rule_statement: None,
                    mistake_pattern: None,
                    preventive_gate_question: None,
                    severity_score: None,
                    effective_pomos: None,
                    deliverable_summary: None,
                    internal_friction_note: None,
                    client_sanitized_note: None,
                    ..Default::default()
                },
            ],
        };

        let badge = apply_copilot_envelope(
            &mut conn,
            "GERDALO",
            "جلسه با @Mohandes برای اندپوینت پرداخت",
            Some("00_raw_inputs/test.md"),
            "USER_CHAT",
            &env,
        )
        .unwrap();
        assert!(badge.contains("تسک جدید: 2"));

        insert_chat_turn(
            &conn,
            "GERDALO",
            "جلسه با @Mohandes",
            None,
            &env.conversational_reply,
            &badge,
        )
        .unwrap();

        let dash = get_dashboard_state(&conn).unwrap();
        assert_eq!(dash.open_tasks.len(), 2);
        assert_eq!(dash.horizon_tasks.len(), 1);
        assert!(dash.next_step_task.is_some());
        assert_eq!(dash.chat_history.len(), 2);

        // Quick complete the critical path task
        let first_id = dash.next_step_task.as_ref().unwrap().id.clone();
        quick_complete_task(&mut conn, "GERDALO", &first_id, 2).unwrap();
        let dash2 = get_dashboard_state(&conn).unwrap();
        assert_eq!(dash2.open_tasks.len(), 1);
        assert_eq!(dash2.closed_tasks_count, 1);

        // Undo quick complete
        undo_last_mutation(&mut conn, "GERDALO").unwrap();
        let dash3 = get_dashboard_state(&conn).unwrap();
        assert_eq!(dash3.open_tasks.len(), 2);
        assert_eq!(dash3.closed_tasks_count, 0);

        // FTS query with OR & symbol should never panic/error
        let hits = search_raw_events_fts(&conn, "GERDALO", "@Mohandes OR پرداخت").unwrap();
        assert_eq!(hits.len(), 1);

        // Undo initial copilot transaction completely
        undo_last_mutation(&mut conn, "GERDALO").unwrap();
        let dash4 = get_dashboard_state(&conn).unwrap();
        assert_eq!(dash4.open_tasks.len(), 0);
        assert!(!dash4.can_undo);

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_multi_turn_update_entity_and_vault_channels_and_migration() {
        let dir = std::env::temp_dir().join(format!("mab_compass_mt_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("legacy_v2.sqlite");

        // Simulate legacy v2 DB without new columns first
        {
            let legacy_conn = Connection::open(&db_path).unwrap();
            legacy_conn
                .execute_batch(
                    "
                    CREATE TABLE app_settings (
                        id INTEGER PRIMARY KEY CHECK (id = 1),
                        vault_root_path TEXT NOT NULL,
                        mab_focus_data_path TEXT NOT NULL,
                        auto_sync_mab_focus INTEGER NOT NULL DEFAULT 1,
                        active_llm_provider TEXT NOT NULL DEFAULT 'AGY_CLI',
                        agy_model TEXT NOT NULL DEFAULT 'Gemini 3.1 Pro (High)',
                        gemini_api_key TEXT NOT NULL DEFAULT '',
                        local_endpoint TEXT NOT NULL DEFAULT 'http://127.0.0.1:1234/v1/chat/completions'
                    );
                    CREATE TABLE chat_messages (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        project_id TEXT NOT NULL,
                        sender TEXT NOT NULL,
                        message_text TEXT NOT NULL,
                        archived_file_path TEXT,
                        applied_mutations_badge TEXT,
                        created_at TEXT NOT NULL
                    );
                    ",
                )
                .unwrap();
        }

        let mut conn = init_database(&db_path, &dir).unwrap();

        // Turn 1 in VAULT: create initial task + ask clarifying questions
        let turn1 = CopilotEnvelope {
            conversational_reply: "پیش‌نویس تسک ثبت شد؛ برای دقیق‌تر شدن دو سوال دارم.".into(),
            clarifying_questions: vec![
                "کدام درگاه پرداخت مدنظر است؟".into(),
                "آیا این کار روی مسیر بحرانی قرار گیرد؟".into(),
            ],
            needs_user_clarification: true,
            ..Default::default()
        };
        let mut env1 = turn1;
        env1.mutations.push(AtomicMutation {
            op: "CREATE_ENTITY".into(),
            target_id: None,
            entity_type: Some("TASK".into()),
            category: Some("BACKEND".into()),
            title: Some("اتصال درگاه پرداخت".into()),
            definition_of_done: Some("تکمیل اتصال".into()),
            first_physical_action: Some("بررسی مستندات درگاه".into()),
            status: None,
            is_on_critical_path: Some(false),
            optimistic_pomos: Some(1.0),
            most_likely_pomos: Some(2.0),
            pessimistic_pomos: Some(3.0),
            stakeholder_tag: None,
            sop_rule_statement: None,
            mistake_pattern: None,
            preventive_gate_question: None,
            severity_score: None,
            effective_pomos: None,
            deliverable_summary: None,
            internal_friction_note: None,
            client_sanitized_note: None,
            ..Default::default()
        });

        let badge1 = apply_copilot_envelope(
            &mut conn,
            "GERDALO",
            "سند اولیه پرداخت",
            None,
            "DOC_ONBOARDING",
            &env1,
        )
        .unwrap();
        insert_chat_turn_with_details(
            &conn,
            "GERDALO",
            "VAULT",
            "سند اولیه پرداخت",
            None,
            &env1.conversational_reply,
            &badge1,
            &env1.clarifying_questions,
        )
        .unwrap();

        let dash1 = get_dashboard_state(&conn).unwrap();
        assert_eq!(dash1.open_tasks.len(), 1);
        assert_eq!(dash1.vault_chat_history.len(), 2);
        assert_eq!(dash1.chat_history.len(), 0);
        assert_eq!(dash1.vault_chat_history[1].clarifying_questions.len(), 2);

        let tid = dash1.open_tasks[0].id.clone();

        // Turn 2 in VAULT: user answers questions -> UPDATE_ENTITY promotes task to IN_PROGRESS & critical path
        let env2 = CopilotEnvelope {
            conversational_reply: "عالی، تسک با درگاه زرین‌پال دقیق شد و در صدر مسیر پیش‌رو قرار گرفت.".into(),
            clarifying_questions: vec![],
            needs_user_clarification: false,
            mutations: vec![AtomicMutation {
                op: "UPDATE_ENTITY".into(),
                target_id: Some(tid.clone()),
                entity_type: None,
                category: Some("BACKEND".into()),
                title: Some("اتصال درگاه زرین‌پال با تایید دو مرحله‌ای".into()),
                definition_of_done: Some("پرداخت تستی سندباکس و کال‌بک موفق".into()),
                first_physical_action: Some("باز کردن payment.rs و ثبت کلید مرچنت".into()),
                status: Some("IN_PROGRESS".into()),
                is_on_critical_path: Some(true),
                optimistic_pomos: Some(2.0),
                most_likely_pomos: Some(3.0),
                pessimistic_pomos: Some(4.0),
                stakeholder_tag: Some("@Client".into()),
                sop_rule_statement: None,
                mistake_pattern: None,
                preventive_gate_question: None,
                severity_score: None,
                effective_pomos: None,
                deliverable_summary: None,
                internal_friction_note: None,
                client_sanitized_note: None,
                ..Default::default()
            }],
            ..Default::default()
        };

        let badge2 = apply_copilot_envelope(
            &mut conn,
            "GERDALO",
            "پاسخ: زرین‌پال و روی مسیر بحرانی",
            None,
            "VAULT_CHAT",
            &env2,
        )
        .unwrap();
        assert!(badge2.contains("تدقیق/اصلاح: 1"));

        let dash2 = get_dashboard_state(&conn).unwrap();
        assert_eq!(dash2.open_tasks.len(), 1);
        let ns = dash2.next_step_task.as_ref().unwrap();
        assert_eq!(ns.id, tid);
        assert_eq!(ns.status, "IN_PROGRESS");
        assert!(ns.is_on_critical_path);
        assert_eq!(ns.calibrated_pomos, 3.0);
        assert!(ns.title.contains("زرین‌پال"));

        // Undo reverts UPDATE_ENTITY cleanly
        undo_last_mutation(&mut conn, "GERDALO").unwrap();
        let dash3 = get_dashboard_state(&conn).unwrap();
        assert_eq!(dash3.open_tasks[0].status, "OPEN");
        assert!(!dash3.open_tasks[0].is_on_critical_path);

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_sessions_immediate_commit_set_next_step_and_delete() {
        let dir = std::env::temp_dir().join(format!("mab_compass_sess_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("sessions.sqlite");
        let mut conn = init_database(&db_path, &dir).unwrap();

        // 1. Legacy orphan message check -> ensure clean immediate commit creates active session and auto-titles it
        let (sid1, eid1, _mid1) = commit_user_chat_turn(
            &conn,
            "GERDALO",
            None,
            "COPILOT",
            "ساخت پنل گزارش مالی برای مشتری",
            Some("00_raw_inputs/chat1.md"),
            "USER_CHAT",
        )
        .unwrap();

        let d1 = get_dashboard_state(&conn).unwrap();
        assert_eq!(d1.active_copilot_session_id, sid1);
        assert_eq!(d1.chat_history.len(), 1);
        assert_eq!(d1.raw_events_count, 1);
        assert_eq!(d1.recent_raw_events.len(), 1);
        assert!(d1.chat_sessions.iter().any(|s| s.id == sid1 && s.title.contains("گزارش مالی")));

        // 2. Apply mutations reusing existing event id + create two tasks (first IN_PROGRESS)
        let m1 = AtomicMutation {
            op: "CREATE_ENTITY".into(),
            target_id: None,
            entity_type: Some("TASK".into()),
            category: Some("BACKEND".into()),
            title: Some("تسک اول".into()),
            definition_of_done: Some("DoD1".into()),
            first_physical_action: Some("Act1".into()),
            status: Some("IN_PROGRESS".into()),
            is_on_critical_path: Some(true),
            optimistic_pomos: Some(1.0),
            most_likely_pomos: Some(2.0),
            pessimistic_pomos: Some(3.0),
            stakeholder_tag: None,
            sop_rule_statement: None,
            mistake_pattern: None,
            preventive_gate_question: None,
            severity_score: None,
            effective_pomos: None,
            deliverable_summary: None,
            internal_friction_note: None,
            client_sanitized_note: None,
            ..Default::default()
        };
        let mut m2 = m1.clone();
        m2.title = Some("تسک دوم".into());
        m2.status = Some("OPEN".into());
        m2.is_on_critical_path = Some(false);

        let env_create = CopilotEnvelope {
            conversational_reply: "دو تسک ساخته شد".into(),
            mutations: vec![m1, m2],
            ..Default::default()
        };
        apply_copilot_envelope_with_event(
            &mut conn,
            "GERDALO",
            "ساخت پنل گزارش مالی برای مشتری",
            None,
            "USER_CHAT",
            &env_create,
            Some(eid1),
        )
        .unwrap();

        let d2 = get_dashboard_state(&conn).unwrap();
        // raw_events_count MUST still be 1 (no duplicate raw_event from Step B)
        assert_eq!(d2.raw_events_count, 1);
        assert_eq!(d2.open_tasks.len(), 2);
        let t1_id = d2.open_tasks.iter().find(|t| t.title == "تسک اول").unwrap().id.clone();
        let t2_id = d2.open_tasks.iter().find(|t| t.title == "تسک دوم").unwrap().id.clone();
        assert_eq!(d2.next_step_task.as_ref().unwrap().id, t1_id);

        // 3. SET_NEXT_STEP switches Path Ahead cleanly to Task 2 and demotes Task 1 to OPEN
        let env_promote = CopilotEnvelope {
            conversational_reply: "مسیر پیش‌رو به تسک دوم تغییر یافت".into(),
            mutations: vec![AtomicMutation {
                op: "SET_NEXT_STEP".into(),
                target_id: Some(t2_id.clone()),
                entity_type: None,
                category: None,
                title: Some("تسک دوم اصلاح‌شده".into()),
                definition_of_done: None,
                first_physical_action: None,
                status: None,
                is_on_critical_path: None,
                optimistic_pomos: None,
                most_likely_pomos: None,
                pessimistic_pomos: None,
                stakeholder_tag: None,
                sop_rule_statement: None,
                mistake_pattern: None,
                preventive_gate_question: None,
                severity_score: None,
                effective_pomos: None,
                deliverable_summary: None,
                internal_friction_note: None,
                client_sanitized_note: None,
                ..Default::default()
            }],
            ..Default::default()
        };
        apply_copilot_envelope(&mut conn, "GERDALO", "اول تسک دوم رو انجام بدیم", None, "USER_CHAT", &env_promote).unwrap();
        let d3 = get_dashboard_state(&conn).unwrap();
        assert_eq!(d3.next_step_task.as_ref().unwrap().id, t2_id);
        assert_eq!(d3.next_step_task.as_ref().unwrap().title, "تسک دوم اصلاح‌شده");
        assert_eq!(d3.open_tasks.iter().find(|t| t.id == t1_id).unwrap().status, "OPEN");

        // 4. DELETE_ENTITY removes Task 1 and Undo restores it
        let env_del = CopilotEnvelope {
            conversational_reply: "تسک اول حذف شد".into(),
            mutations: vec![AtomicMutation {
                op: "DELETE_ENTITY".into(),
                target_id: Some(t1_id.clone()),
                entity_type: None,
                category: None,
                title: None,
                definition_of_done: None,
                first_physical_action: None,
                status: None,
                is_on_critical_path: None,
                optimistic_pomos: None,
                most_likely_pomos: None,
                pessimistic_pomos: None,
                stakeholder_tag: None,
                sop_rule_statement: None,
                mistake_pattern: None,
                preventive_gate_question: None,
                severity_score: None,
                effective_pomos: None,
                deliverable_summary: None,
                internal_friction_note: None,
                client_sanitized_note: None,
                ..Default::default()
            }],
            ..Default::default()
        };
        let del_badge = apply_copilot_envelope(&mut conn, "GERDALO", "حذف تسک اول", None, "USER_CHAT", &env_del).unwrap();
        assert!(del_badge.contains("حذف: 1"));
        assert_eq!(get_dashboard_state(&conn).unwrap().open_tasks.len(), 1);
        undo_last_mutation(&mut conn, "GERDALO").unwrap();
        assert_eq!(get_dashboard_state(&conn).unwrap().open_tasks.len(), 2);

        // 5. Start New Chat session under GERDALO & switch back to session 1
        let s2 = create_chat_session(&conn, Some("GERDALO"), "COPILOT", None).unwrap();
        let d4 = get_dashboard_state(&conn).unwrap();
        assert_eq!(d4.active_copilot_session_id, s2.id);
        assert_eq!(d4.chat_history.len(), 0);
        assert!(d4.all_project_chat_sessions.iter().any(|s| s.id == sid1));

        switch_chat_session(&conn, &sid1).unwrap();
        let d5 = get_dashboard_state(&conn).unwrap();
        assert_eq!(d5.active_copilot_session_id, sid1);
        assert_eq!(d5.chat_history.len(), 1);

        // 6. Failed SYSTEM message retry context, update, and delete
        append_assistant_chat_message(
            &conn,
            "GERDALO",
            &sid1,
            "COPILOT",
            "SYSTEM",
            "⚠️ خطای موقت مدل",
            Some("00_raw_inputs/chat1.md"),
            "ثبت در حافظه • خطای موتور هوش مصنوعی",
            &[],
        )
        .unwrap();
        let d6 = get_dashboard_state(&conn).unwrap();
        let sys_id = d6
            .chat_history
            .iter()
            .find(|m| m.sender == "SYSTEM")
            .unwrap()
            .id;

        let retry_ctx = get_retry_context_for_system_message(&conn, sys_id).unwrap();
        assert_eq!(retry_ctx.project_id, "GERDALO");
        assert_eq!(retry_ctx.session_id, sid1);
        assert_eq!(retry_ctx.user_message_text, "ساخت پنل گزارش مالی برای مشتری");
        assert_eq!(retry_ctx.raw_event_id, Some(eid1));

        update_system_chat_message(&conn, sys_id, "⚠️ خطای جدید").unwrap();
        assert!(get_dashboard_state(&conn)
            .unwrap()
            .chat_history
            .iter()
            .any(|m| m.id == sys_id && m.message_text == "⚠️ خطای جدید"));

        delete_system_chat_message(&conn, sys_id).unwrap();
        assert!(!get_dashboard_state(&conn)
            .unwrap()
            .chat_history
            .iter()
            .any(|m| m.id == sys_id));

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_pending_queue_discovery_and_lifecycle() {
        let dir = std::env::temp_dir().join(format!("mab_compass_pq_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("pq.sqlite");
        let conn = init_database(&db_path, &dir).unwrap();

        // Older session turn already succeeded later (USER -> SYSTEM -> ASSISTANT) => MUST NOT be queued
        let (sid, _eid_old, u_old) = commit_user_chat_turn(
            &conn,
            "IRANOIL",
            None,
            "COPILOT",
            "پیام قدیمی که بعدا پاسخ گرفته",
            Some("00_raw_inputs/old.md"),
            "USER_CHAT",
        )
        .unwrap();
        append_assistant_chat_message(
            &conn,
            "IRANOIL",
            &sid,
            "COPILOT",
            "SYSTEM",
            "خطای موقت قدیمی",
            Some("00_raw_inputs/old.md"),
            "خطا",
            &[],
        )
        .unwrap();
        append_assistant_chat_message(
            &conn,
            "IRANOIL",
            &sid,
            "COPILOT",
            "ASSISTANT",
            "پاسخ مدل ثبت شد.",
            None,
            "⚡ پاسخ مستقیم مدل",
            &[],
        )
        .unwrap();

        // Two trailing unprocessed turns: #1 file attachment with failed doc record, #2 user chat
        let (_, eid_file, u_file) = commit_user_chat_turn(
            &conn,
            "IRANOIL",
            Some(&sid),
            "COPILOT",
            "📎 فایل ضمیمه: «code_artifact (7).md» (MD • 7973 کاراکتر)",
            Some("00_raw_inputs/copilot_file.md"),
            "COPILOT_FILE",
        )
        .unwrap();
        let kdoc_id = insert_knowledge_doc_record(
            &conn,
            "IRANOIL",
            "code_artifact (7).md",
            "MD",
            "code_artifact (7).md",
            "01_knowledge_archive/code_artifact_7.md",
            "متن فایل استخراج و در حافظه پروژه آرشیو شد (تحلیل هوشمند به دلیل خطای مدل انجام نشد).",
            0,
        )
        .unwrap();
        append_assistant_chat_message(
            &conn,
            "IRANOIL",
            &sid,
            "COPILOT",
            "SYSTEM",
            "خطا در پردازش فایل",
            Some("01_knowledge_archive/code_artifact_7.md"),
            "خطا",
            &[],
        )
        .unwrap();

        let (_, _eid_chat, u_chat) = commit_user_chat_turn(
            &conn,
            "IRANOIL",
            Some(&sid),
            "COPILOT",
            "من سعی کردم که با گوگل فلو اجرا کنم",
            Some("00_raw_inputs/user_chat.md"),
            "USER_CHAT",
        )
        .unwrap();
        append_assistant_chat_message(
            &conn,
            "IRANOIL",
            &sid,
            "COPILOT",
            "SYSTEM",
            "خطا در ارسال به مدل",
            Some("00_raw_inputs/user_chat.md"),
            "خطا",
            &[],
        )
        .unwrap();

        // Orphan failed knowledge doc with no chat message
        let orphan_doc_id = insert_knowledge_doc_record(
            &conn,
            "GERDALO",
            "orphan_spec",
            "PDF",
            "orphan_spec.pdf",
            "01_knowledge_archive/orphan_spec.md",
            "متن فایل استخراج و در حافظه پروژه آرشیو شد (تحلیل هوشمند به دلیل خطای مدل انجام نشد).",
            0,
        )
        .unwrap();

        let inserted = sync_unprocessed_items_into_queue(&conn).unwrap();
        assert_eq!(inserted, 3);

        // Running discovery twice MUST remain idempotent
        let inserted2 = sync_unprocessed_items_into_queue(&conn).unwrap();
        assert_eq!(inserted2, 0);

        let queue = query_pending_ai_queue(&conn).unwrap();
        assert_eq!(queue.len(), 3);
        assert!(!queue.iter().any(|q| q.user_message_id == Some(u_old)));

        let file_item = queue.iter().find(|q| q.user_message_id == Some(u_file)).unwrap();
        assert_eq!(file_item.raw_event_id, Some(eid_file));
        assert_eq!(file_item.knowledge_doc_id, Some(kdoc_id));
        assert!(file_item.system_message_id.is_some());

        let chat_item = queue.iter().find(|q| q.user_message_id == Some(u_chat)).unwrap();
        assert!(chat_item.system_message_id.is_some());

        let orphan_item = queue
            .iter()
            .find(|q| q.knowledge_doc_id == Some(orphan_doc_id))
            .unwrap();
        assert!(orphan_item.user_message_id.is_none());

        // Completing user message item + updating knowledge doc clears it cleanly
        update_knowledge_doc_analysis(&conn, kdoc_id, "خلاصه کامل سند تحلیل شد", 3, None).unwrap();
        complete_pending_items_for_user_msg(&conn, u_file, Some(kdoc_id)).unwrap();
        let queue_after = query_pending_ai_queue(&conn).unwrap();
        assert_eq!(queue_after.len(), 2);

        // Dismissing removes from active queue and re-sync does not resurrect it
        dismiss_pending_queue_item(&conn, chat_item.id).unwrap();
        sync_unprocessed_items_into_queue(&conn).unwrap();
        let queue_final = query_pending_ai_queue(&conn).unwrap();
        assert_eq!(queue_final.len(), 1);
        assert_eq!(queue_final[0].knowledge_doc_id, Some(orphan_doc_id));

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_m2_sequential_chain_engine_and_context_queries() {
        let dir = std::env::temp_dir().join(format!("mab_compass_m2_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test_m2.sqlite");
        let conn = init_database(&db_path, &dir).unwrap();

        // 1. Test create_work_line_with_chain
        let steps = vec![
            ChainStepInput {
                title: "مرحله ۱: طراحی ساختار".into(),
                definition_of_done: "مستندات آماده باشد".into(),
                first_physical_action: "باز کردن ادیتور".into(),
                energy_tag: "DEEP_HIGH".into(),
                duration_minutes: 50,
                device_tag: "LAPTOP_ONLY".into(),
                location_tags: vec!["DESK".into()],
                category: "ARCHITECTURE".into(),
                ..Default::default()
            },
            ChainStepInput {
                title: "مرحله ۲: پیاده‌سازی سرویس".into(),
                definition_of_done: "تست‌ها پاس شوند".into(),
                first_physical_action: "کدنویسی متدها".into(),
                energy_tag: "MEDIUM_NORMAL".into(),
                duration_minutes: 25,
                device_tag: "LAPTOP_ONLY".into(),
                location_tags: vec!["DESK".into()],
                category: "BACKEND".into(),
                ..Default::default()
            },
            ChainStepInput {
                title: "مرحله ۳: بازبینی نهایی روی موبایل".into(),
                definition_of_done: "تایید کیفیت".into(),
                first_physical_action: "نصب روی گوشی".into(),
                energy_tag: "LOW_ZOMBIE".into(),
                duration_minutes: 15,
                device_tag: "PHONE_OK".into(),
                location_tags: vec!["METRO_COMMUTE".into()],
                category: "GENERAL".into(),
                ..Default::default()
            },
        ];

        let (work_line, entities) = create_work_line_with_chain(
            &conn,
            "GERDALO",
            None,
            "زنجیره تست M2",
            "اتمام کامل تسک‌ها",
            "SEQUENTIAL_CHAIN",
            steps,
        )
        .expect("create_work_line_with_chain failed");

        assert!(work_line.id.starts_with("WLN-"));
        assert_eq!(work_line.line_mode, "SEQUENTIAL_CHAIN");
        assert_eq!(work_line.status, "ACTIVE");
        assert_eq!(entities.len(), 3);

        // Step 1: active, unlocked, critical path
        assert_eq!(entities[0].sequence_order, 1);
        assert_eq!(entities[0].status, "IN_PROGRESS");
        assert_eq!(entities[0].is_locked_by_chain, false);
        assert_eq!(entities[0].is_on_critical_path, true);
        assert!(entities[0].depends_on_task_id.is_none());

        // Step 2: queued, locked, depends on step 1
        assert_eq!(entities[1].sequence_order, 2);
        assert_eq!(entities[1].status, "QUEUED_IN_CHAIN");
        assert_eq!(entities[1].is_locked_by_chain, true);
        assert_eq!(entities[1].is_on_critical_path, false);
        assert_eq!(entities[1].depends_on_task_id.as_deref(), Some(entities[0].id.as_str()));

        // Step 3: queued, locked, depends on step 2
        assert_eq!(entities[2].sequence_order, 3);
        assert_eq!(entities[2].status, "QUEUED_IN_CHAIN");
        assert_eq!(entities[2].is_locked_by_chain, true);
        assert_eq!(entities[2].depends_on_task_id.as_deref(), Some(entities[1].id.as_str()));

        // 2. Test is_locked_by_chain
        assert_eq!(is_locked_by_chain(&conn, &entities[0].id).unwrap(), false);
        assert_eq!(is_locked_by_chain(&conn, &entities[1].id).unwrap(), true);
        assert_eq!(is_locked_by_chain(&conn, &entities[2].id).unwrap(), true);

        // Unchained standalone task
        conn.execute(
            "INSERT INTO work_entities (id, project_id, entity_type, category, title, status, is_locked_by_chain, updated_at)
             VALUES ('TSK-SOLO-M2', 'GERDALO', 'TASK', 'GENERAL', 'Standalone', 'OPEN', 0, '2026-11-15T00:00:00Z')",
            [],
        ).unwrap();
        assert_eq!(is_locked_by_chain(&conn, "TSK-SOLO-M2").unwrap(), false);

        // Cluster task
        conn.execute(
            "INSERT INTO work_lines (id, project_id, title, line_mode, status, created_at, updated_at)
             VALUES ('WLN-CLUSTER-M2', 'GERDALO', 'Cluster', 'GOAL_CLUSTER', 'ACTIVE', '2026-11-15T00:00:00Z', '2026-11-15T00:00:00Z')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO work_entities (id, project_id, entity_type, category, title, work_line_id, sequence_order, status, is_locked_by_chain, updated_at)
             VALUES ('TSK-CLUS-M2', 'GERDALO', 'TASK', 'GENERAL', 'Cluster Step', 'WLN-CLUSTER-M2', 2, 'OPEN', 0, '2026-11-15T00:00:00Z')",
            [],
        ).unwrap();
        assert_eq!(is_locked_by_chain(&conn, "TSK-CLUS-M2").unwrap(), false);

        // 3. Test query_tasks_by_context
        // Query unlocked tasks for laptop: should match Step 1 and Standalone, but NOT Step 2 (locked)
        let laptop_tasks = query_tasks_by_context(
            &conn,
            "GERDALO",
            Some("LAPTOP_ONLY"),
            None,
            None,
            None,
            true, // only_unlocked
        ).unwrap();
        let laptop_ids: Vec<String> = laptop_tasks.iter().map(|t| t.id.clone()).collect();
        assert!(laptop_ids.contains(&entities[0].id));
        assert!(!laptop_ids.contains(&entities[1].id)); // step 2 locked!

        // Query with only_unlocked = false: step 2 IS included
        let all_laptop = query_tasks_by_context(
            &conn,
            "GERDALO",
            Some("LAPTOP_ONLY"),
            None,
            None,
            None,
            false,
        ).unwrap();
        let all_laptop_ids: Vec<String> = all_laptop.iter().map(|t| t.id.clone()).collect();
        assert!(all_laptop_ids.contains(&entities[0].id));
        assert!(all_laptop_ids.contains(&entities[1].id));

        // 4. Test advance_sequential_chain_on_complete (Domino effect)
        // Complete Step 1 -> advances Step 2 to IN_PROGRESS
        let promoted_1 = advance_sequential_chain_on_complete(&conn, "GERDALO", &entities[0].id).unwrap();
        assert!(promoted_1.is_some());
        let p1 = promoted_1.unwrap();
        assert_eq!(p1.id, entities[1].id);
        assert_eq!(p1.status, "IN_PROGRESS");
        assert_eq!(p1.is_locked_by_chain, false);

        // Step 2 is now unlocked; Step 3 remains locked
        assert_eq!(is_locked_by_chain(&conn, &entities[1].id).unwrap(), false);
        assert_eq!(is_locked_by_chain(&conn, &entities[2].id).unwrap(), true);

        // Complete Step 2 -> advances Step 3 to IN_PROGRESS
        let promoted_2 = advance_sequential_chain_on_complete(&conn, "GERDALO", &entities[1].id).unwrap();
        assert!(promoted_2.is_some());
        let p2 = promoted_2.unwrap();
        assert_eq!(p2.id, entities[2].id);
        assert_eq!(p2.status, "IN_PROGRESS");
        assert_eq!(p2.is_locked_by_chain, false);
        assert_eq!(is_locked_by_chain(&conn, &entities[2].id).unwrap(), false);

        // Complete Step 3 (final step) -> returns None, marks parent work_lines COMPLETED
        let final_promoted = advance_sequential_chain_on_complete(&conn, "GERDALO", &entities[2].id).unwrap();
        assert!(final_promoted.is_none());

        let line_status: String = conn
            .query_row("SELECT status FROM work_lines WHERE id = ?1", params![work_line.id], |r| r.get(0))
            .unwrap();
        assert_eq!(line_status, "COMPLETED");

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_top_3_priority_steps_work_lines_progress_and_domino_quick_complete() {
        let dir = format!("target/tmp/test_top3_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = PathBuf::from(&dir).join("compass.sqlite");
        let vault_path = PathBuf::from(&dir).join("vault");
        let mut conn = init_database(&db_path, &vault_path).expect("init db failed");

        // Create an outcome
        conn.execute(
            "INSERT INTO outcomes (id, domain_code, title, desired_reality_change, evidence_kr, time_window, status, progress_percent, created_at, updated_at)
             VALUES ('OUT-101', 'WEALTH_FINANCE', 'راه‌اندازی سیستم پایدار', 'تحول اجرایی', '۳ لاین تکمیل', 'Q4', 'ACTIVE', 25, '2026-11-10T10:00:00Z', '2026-11-10T10:00:00Z')",
            [],
        ).unwrap();

        // Create 4 distinct WorkLines (3 sequential + 1 cluster) to verify top_priority_steps picks max 3 from distinct lines
        let (wl1, e1) = create_work_line_with_chain(
            &conn,
            "GERDALO",
            Some("OUT-101"),
            "لاین ۱ - زیرساخت",
            "ساخت لایه هسته",
            "SEQUENTIAL_CHAIN",
            vec![
                ChainStepInput { title: "گام ۱-۱".into(), ..Default::default() },
                ChainStepInput { title: "گام ۱-۲".into(), ..Default::default() },
            ],
        ).unwrap();

        let (wl2, e2) = create_work_line_with_chain(
            &conn,
            "GERDALO",
            Some("OUT-101"),
            "لاین ۲ - فرانت‌اند",
            "ساخت رابط کاربری",
            "SEQUENTIAL_CHAIN",
            vec![
                ChainStepInput { title: "گام ۲-۱".into(), ..Default::default() },
                ChainStepInput { title: "گام ۲-۲".into(), ..Default::default() },
            ],
        ).unwrap();

        let (wl3, e3) = create_work_line_with_chain(
            &conn,
            "GERDALO",
            None,
            "لاین ۳ - مستندسازی",
            "نوشتن راهنما",
            "GOAL_CLUSTER",
            vec![
                ChainStepInput { title: "گام ۳-۱".into(), ..Default::default() },
            ],
        ).unwrap();

        let (_wl4, _e4) = create_work_line_with_chain(
            &conn,
            "GERDALO",
            None,
            "لاین ۴ - اضافی",
            "کار چهارم",
            "GOAL_CLUSTER",
            vec![
                ChainStepInput { title: "گام ۴-۱".into(), ..Default::default() },
            ],
        ).unwrap();

        let dash = get_dashboard_state(&conn).expect("get_dashboard_state failed");

        // 1. All QUEUED_IN_CHAIN tasks must be visible in open_tasks with is_locked_by_chain = true
        let step1_2 = dash.open_tasks.iter().find(|t| t.id == e1[1].id).expect("step 1-2 must appear in open_tasks for WBS");
        assert!(step1_2.is_locked_by_chain);
        assert_eq!(step1_2.work_line_title.as_deref(), Some("لاین ۱ - زیرساخت"));
        assert_eq!(step1_2.outcome_id.as_deref(), Some("OUT-101"));

        // 2. query_all_work_lines computes real task counts & outcome_title
        let wl1_state = dash.work_lines.iter().find(|w| w.id == wl1.id).unwrap();
        assert_eq!(wl1_state.total_tasks, 2);
        assert_eq!(wl1_state.completed_tasks, 0);
        assert_eq!(wl1_state.outcome_title.as_deref(), Some("راه‌اندازی سیستم پایدار"));
        assert_eq!(wl1_state.active_step_id.as_deref(), Some(e1[0].id.as_str()));

        // 3. top_priority_steps has exactly 3 items from 3 distinct work lines, none locked
        assert_eq!(dash.top_priority_steps.len(), 3);
        let distinct_lines: std::collections::HashSet<&str> = dash
            .top_priority_steps
            .iter()
            .filter_map(|t| t.work_line_id.as_deref())
            .collect();
        assert_eq!(distinct_lines.len(), 3);
        assert!(dash.top_priority_steps.iter().all(|t| !t.is_locked_by_chain));
        let top_ids: Vec<&str> = dash.top_priority_steps.iter().map(|t| t.id.as_str()).collect();
        assert!(top_ids.contains(&e1[0].id.as_str()));
        assert!(top_ids.contains(&e2[0].id.as_str()));
        assert!(!top_ids.contains(&e1[1].id.as_str()));
        assert!(!top_ids.contains(&e2[1].id.as_str()));

        // 4. start_task_step rejects locked chain step, accepts unlocked OPEN step
        assert_eq!(start_task_step(&conn, "GERDALO", &e1[1].id).unwrap(), false);
        // Make e3[0] OPEN first to test start_task_step
        conn.execute("UPDATE work_entities SET status = 'OPEN' WHERE id = ?1", params![e3[0].id]).unwrap();
        assert_eq!(start_task_step(&conn, "GERDALO", &e3[0].id).unwrap(), true);

        // 5. quick_complete_task triggers domino advancement: completing e1[0] unlocks e1[1] and updates work_line progress!
        quick_complete_task(&mut conn, "GERDALO", &e1[0].id, 2).expect("quick_complete_task failed");
        let dash_after = get_dashboard_state(&conn).expect("get_dashboard_state after complete failed");
        let step1_2_after = dash_after.open_tasks.iter().find(|t| t.id == e1[1].id).unwrap();
        assert_eq!(step1_2_after.status, "IN_PROGRESS");
        assert!(!step1_2_after.is_locked_by_chain);

        let wl1_after = dash_after.work_lines.iter().find(|w| w.id == wl1.id).unwrap();
        assert_eq!(wl1_after.total_tasks, 2);
        assert_eq!(wl1_after.completed_tasks, 1);
        assert_eq!(wl1_after.active_step_id.as_deref(), Some(e1[1].id.as_str()));
        let _ = (wl2, wl3);

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_pending_queue_chronological_creation_order() {
        let dir = format!("target/tmp/test_queue_order_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = PathBuf::from(&dir).join("compass.sqlite");
        let vault_path = PathBuf::from(&dir).join("vault");
        let conn = init_database(&db_path, &vault_path).expect("init db failed");
        let sid = ensure_active_chat_session(&conn, "GERDALO", "COPILOT").unwrap();

        // Insert a NEWER message first, and an OLDER message second (simulating backfilled/offline earlier creation)
        conn.execute(
            "INSERT INTO chat_messages (project_id, session_id, sender, channel, message_text, created_at)
             VALUES ('GERDALO', ?1, 'USER', 'COPILOT', 'پیام جدیدتر', '2026-11-17T12:00:00+03:30')",
            params![sid],
        ).unwrap();
        let newer_msg_id = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO chat_messages (project_id, session_id, sender, channel, message_text, created_at)
             VALUES ('GERDALO', ?1, 'USER', 'COPILOT', 'پیام قدیمی‌تر', '2026-11-16T08:00:00+03:30')",
            params![sid],
        ).unwrap();
        let older_msg_id = conn.last_insert_rowid();

        // Enqueue newer FIRST, then older SECOND
        insert_pending_queue_item(
            &conn,
            PendingQueueInput {
                project_id: "GERDALO",
                session_id: &sid,
                channel: "COPILOT",
                user_message_id: Some(newer_msg_id),
                system_message_id: None,
                knowledge_doc_id: None,
                raw_event_id: None,
                title_preview: "پیام جدیدتر",
                source_type: "USER_CHAT",
                archived_path: None,
                last_error: None,
            },
        ).unwrap();

        insert_pending_queue_item(
            &conn,
            PendingQueueInput {
                project_id: "GERDALO",
                session_id: &sid,
                channel: "COPILOT",
                user_message_id: Some(older_msg_id),
                system_message_id: None,
                knowledge_doc_id: None,
                raw_event_id: None,
                title_preview: "پیام قدیمی‌تر",
                source_type: "USER_CHAT",
                archived_path: None,
                last_error: None,
            },
        ).unwrap();

        // Query must return the OLDER creation timestamp first!
        let queue = query_pending_ai_queue(&conn).unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0].user_message_id, Some(older_msg_id));
        assert_eq!(queue[0].created_at, "2026-11-16T08:00:00+03:30");
        assert_eq!(queue[1].user_message_id, Some(newer_msg_id));
        assert_eq!(queue[1].created_at, "2026-11-17T12:00:00+03:30");

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_document_processing_status_lifecycle() {
        let dir = format!("target/tmp/test_doc_lifecycle_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = PathBuf::from(&dir).join("compass.sqlite");
        let vault_path = PathBuf::from(&dir).join("vault");
        let conn = init_database(&db_path, &vault_path).expect("init db failed");

        let doc_id = insert_knowledge_doc_with_status(
            &conn,
            "GERDALO",
            "گزارش معماری",
            "TECHNICAL_DOC",
            "arch.md",
            "vault/arch.md",
            "در انتظار پردازش توسط هوش مصنوعی",
            0,
            "UNPROCESSED",
        )
        .unwrap();

        let dash1 = get_dashboard_state(&conn).unwrap();
        let doc1 = dash1.knowledge_docs.iter().find(|d| d.id == doc_id).unwrap();
        assert_eq!(doc1.processing_status, "UNPROCESSED");
        assert!(doc1.processed_at.is_none());

        // Move to PROCESSING
        set_knowledge_doc_processing_status(&conn, doc_id, "PROCESSING", None, None, None).unwrap();
        let dash2 = get_dashboard_state(&conn).unwrap();
        assert_eq!(
            dash2.knowledge_docs.iter().find(|d| d.id == doc_id).unwrap().processing_status,
            "PROCESSING"
        );

        // Complete processing -> PROCESSED with extracted count & processed_at
        update_knowledge_doc_analysis(&conn, doc_id, "خلاصه نهایی تحلیل‌شده", 5, None).unwrap();
        let dash3 = get_dashboard_state(&conn).unwrap();
        let doc3 = dash3.knowledge_docs.iter().find(|d| d.id == doc_id).unwrap();
        assert_eq!(doc3.processing_status, "PROCESSED");
        assert_eq!(doc3.extracted_items_count, 5);
        assert!(doc3.processed_at.is_some());

        // Requeue -> resets to UNPROCESSED and puts back into pending_ai_queue
        let qid = requeue_knowledge_document(&conn, doc_id).unwrap();
        assert!(qid > 0);
        let dash4 = get_dashboard_state(&conn).unwrap();
        let doc4 = dash4.knowledge_docs.iter().find(|d| d.id == doc_id).unwrap();
        assert_eq!(doc4.processing_status, "UNPROCESSED");
        assert!(dash4.pending_queue.iter().any(|q| q.knowledge_doc_id == Some(doc_id)));

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_pending_queue_manual_reordering_and_reset() {
        let dir = format!("target/tmp/test_queue_reorder_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = PathBuf::from(&dir).join("compass.sqlite");
        let vault_path = PathBuf::from(&dir).join("vault");
        let conn = init_database(&db_path, &vault_path).expect("init db failed");
        let sid = ensure_active_chat_session(&conn, "GERDALO", "COPILOT").unwrap();

        let mut qids = Vec::new();
        for (i, ts) in [
            "2026-11-15T08:00:00+03:30",
            "2026-11-16T08:00:00+03:30",
            "2026-11-17T08:00:00+03:30",
        ]
        .iter()
        .enumerate()
        {
            conn.execute(
                "INSERT INTO chat_messages (project_id, session_id, sender, channel, message_text, created_at)
                 VALUES ('GERDALO', ?1, 'USER', 'COPILOT', ?2, ?3)",
                params![sid, format!("msg-{}", i + 1), ts],
            )
            .unwrap();
            let mid = conn.last_insert_rowid();
            let qid = insert_pending_queue_item(
                &conn,
                PendingQueueInput {
                    project_id: "GERDALO",
                    session_id: &sid,
                    channel: "COPILOT",
                    user_message_id: Some(mid),
                    system_message_id: None,
                    knowledge_doc_id: None,
                    raw_event_id: None,
                    title_preview: &format!("msg-{}", i + 1),
                    source_type: "USER_CHAT",
                    archived_path: None,
                    last_error: None,
                },
            )
            .unwrap();
            qids.push(qid);
        }

        // Default chronological order: [0, 1, 2]
        let q0: Vec<i64> = query_pending_ai_queue(&conn).unwrap().iter().map(|q| q.id).collect();
        assert_eq!(q0, vec![qids[0], qids[1], qids[2]]);

        // Move item 2 to TOP -> [2, 0, 1]
        reorder_pending_queue_item(&conn, qids[2], "TOP").unwrap();
        let q1: Vec<i64> = query_pending_ai_queue(&conn).unwrap().iter().map(|q| q.id).collect();
        assert_eq!(q1, vec![qids[2], qids[0], qids[1]]);

        // Move item 1 UP -> [2, 1, 0]
        reorder_pending_queue_item(&conn, qids[1], "UP").unwrap();
        let q2: Vec<i64> = query_pending_ai_queue(&conn).unwrap().iter().map(|q| q.id).collect();
        assert_eq!(q2, vec![qids[2], qids[1], qids[0]]);

        // Move item 2 DOWN -> [1, 2, 0]
        reorder_pending_queue_item(&conn, qids[2], "DOWN").unwrap();
        let q3: Vec<i64> = query_pending_ai_queue(&conn).unwrap().iter().map(|q| q.id).collect();
        assert_eq!(q3, vec![qids[1], qids[2], qids[0]]);

        // Reset -> back to chronological order [0, 1, 2]
        reset_pending_queue_order(&conn).unwrap();
        let q4: Vec<i64> = query_pending_ai_queue(&conn).unwrap().iter().map(|q| q.id).collect();
        assert_eq!(q4, vec![qids[0], qids[1], qids[2]]);

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }

    #[test]
    fn test_manual_outcome_workline_task_assignment_and_cards() {
        let dir = format!("target/tmp/test_manual_5layer_{}", uuid::Uuid::new_v4());
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = PathBuf::from(&dir).join("compass.sqlite");
        let vault_path = PathBuf::from(&dir).join("vault");
        let conn = init_database(&db_path, &vault_path).expect("init db failed");

        // 1. Create Outcome
        let out_id = upsert_outcome_manual(
            &conn,
            None,
            "WEALTH_FINANCE",
            "انتشار نسخه ۳",
            "سیستم کاملاً خودکار کار کند",
            "۰ خطا در تولید",
            "۳۰ روز",
            "ACTIVE",
            30,
        )
        .unwrap();
        assert!(out_id.starts_with("OUT-"));

        // 2. Create WorkLine linked to Outcome
        let (wl, _) = create_work_line_with_chain(
            &conn,
            "GERDALO",
            Some(&out_id),
            "لاین هسته",
            "تحویل هسته پایدار",
            "SEQUENTIAL_CHAIN",
            vec![ChainStepInput { title: "گام اول".into(), ..Default::default() }],
        )
        .unwrap();

        // 3. Create manual standalone task, then assign it to WorkLine (should become step #2 locked in sequential chain!)
        let t_id = create_task_manual(
            &conn,
            "GERDALO",
            "تسک دوم دستی",
            "BACKEND",
            "پاس شدن تست",
            "باز کردن فایل",
            None,
            None,
            "HIGH_DEEP",
            "LAPTOP_ONLY",
            45,
            Some("2026-11-20"),
        )
        .unwrap();

        assign_task_grouping(
            &conn,
            "GERDALO",
            &t_id,
            Some(&wl.id),
            None,
            Some(85),
            None,
            None,
            None,
            None,
        )
        .unwrap();

        // 4. Create Knowledge / Memory Card & Calendar Day
        let card_id = create_personal_vault_item(
            &conn,
            Some("GERDALO"),
            "WORK",
            "MIND_LEARNING",
            "MEMORY_CARD",
            "قانون Full-Stack Gate چیست؟",
            "هر قابلیت باید همزمان در Rust و TypeScript و UI کامل شود.",
            &["معماری".into(), "کیفیت".into()],
        )
        .unwrap();

        upsert_calendar_day(&conn, "2026-11-20", 360, 180, "روز تمرکز عمیق", false).unwrap();

        let dash = get_dashboard_state(&conn).unwrap();
        let assigned_task = dash.open_tasks.iter().find(|t| t.id == t_id).unwrap();
        assert_eq!(assigned_task.work_line_id.as_deref(), Some(wl.id.as_str()));
        assert_eq!(assigned_task.outcome_id.as_deref(), Some(out_id.as_str()));
        assert_eq!(assigned_task.sequence_order, 2);
        assert!(assigned_task.is_locked_by_chain);

        assert!(dash.personal_vault_items.iter().any(|c| c.id == card_id));
        let cal = dash.calendar_days.iter().find(|d| d.date_key == "2026-11-20").unwrap();
        assert_eq!(cal.assigned_tasks_count, 1);
        assert_eq!(cal.assigned_minutes, 45);

        // Cleanup delete tests
        delete_personal_vault_item(&conn, &card_id).unwrap();
        delete_outcome(&conn, &out_id).unwrap();
        let dash_after = get_dashboard_state(&conn).unwrap();
        assert!(!dash_after.personal_vault_items.iter().any(|c| c.id == card_id));
        assert!(!dash_after.outcomes.iter().any(|o| o.id == out_id));

        drop(conn);
        let _ = std::fs::remove_dir_all(PathBuf::from(dir));
    }
}
