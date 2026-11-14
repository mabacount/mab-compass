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
    let now = Local::now().to_rfc3339();

    conn.execute("UPDATE projects SET is_active = 0", [])?;
    conn.execute(
        "INSERT OR REPLACE INTO projects (id, name, workspace_path, external_focus_id, is_active, updated_at)
         VALUES (?1, ?2, ?3, ?1, 1, ?4)",
        params![clean_id, display_name, workspace_path, now],
    )?;

    Ok(Project {
        id: clean_id.clone(),
        name: display_name,
        workspace_path,
        external_focus_id: Some(clean_id),
        is_active: true,
        updated_at: now,
    })
}

pub fn get_dashboard_state(conn: &Connection) -> Result<DashboardState> {
    get_dashboard_state_for_project(conn, None)
}

pub fn get_dashboard_state_for_project(
    conn: &Connection,
    project_id_override: Option<&str>,
) -> Result<DashboardState> {
    let settings = get_settings(conn)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, workspace_path, external_focus_id, is_active, updated_at
         FROM projects ORDER BY is_active DESC, id",
    )?;
    let projects: Vec<Project> = stmt
        .query_map([], |r| {
            Ok(Project {
                id: r.get(0)?,
                name: r.get(1)?,
                workspace_path: r.get(2)?,
                external_focus_id: r.get(3)?,
                is_active: r.get::<_, i32>(4)? == 1,
                updated_at: r.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

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

    let open_tasks = query_entities(conn, pid, "TASK", &["OPEN", "IN_PROGRESS", "BLOCKED"])?;
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
        .filter(|t| t.status != "BLOCKED")
        .cloned()
        .collect();

    let next_step_task = unblocked_tasks
        .iter()
        .find(|t| t.status == "IN_PROGRESS" && t.is_on_critical_path)
        .or_else(|| unblocked_tasks.iter().find(|t| t.status == "IN_PROGRESS"))
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

    let active_copilot_session_id = ensure_active_chat_session(conn, pid, "COPILOT")?;
    let active_vault_session_id = ensure_active_chat_session(conn, pid, "VAULT")?;

    let chat_history = query_session_messages(conn, pid, &active_copilot_session_id, false)?;
    let vault_chat_history = query_session_messages(conn, pid, &active_vault_session_id, true)?;
    let chat_sessions = query_chat_sessions(conn, Some(pid))?;
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
        "SELECT id, project_id, title, doc_type, original_filename, archived_path, executive_summary, extracted_items_count, ingested_at
         FROM knowledge_documents WHERE project_id = ?1 ORDER BY id DESC",
    )?;
    let knowledge_docs: Vec<KnowledgeDocument> = doc_stmt
        .query_map(params![pid], |r| {
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
    })
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
        conn.execute(
            "UPDATE projects SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END",
            params![pid],
        )?;
        pid.to_string()
    } else {
        conn.query_row(
            "SELECT id FROM projects WHERE is_active = 1 LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )?
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
        .unwrap_or_else(|| default_session_title(&norm_ch))
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

    conn.execute(
        "UPDATE projects SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END",
        params![pid],
    )?;
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
        let _ = ensure_active_chat_session(conn, &pid, &ch);
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
    let sql = if vault_only {
        "SELECT id, project_id, COALESCE(session_id, ''), sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at
         FROM (
            SELECT id, project_id, session_id, sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at
            FROM chat_messages
            WHERE project_id = ?1 AND channel = 'VAULT' AND (session_id = ?2 OR COALESCE(session_id, '') = '')
            ORDER BY id DESC LIMIT 80
         ) ORDER BY id ASC"
    } else {
        "SELECT id, project_id, COALESCE(session_id, ''), sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at
         FROM (
            SELECT id, project_id, session_id, sender, channel, message_text, archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at
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
                created_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(msgs)
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
                calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at
         FROM work_entities
         WHERE project_id = ?1 AND entity_type = ?2
         ORDER BY
            CASE WHEN status = 'IN_PROGRESS' THEN 0
                 WHEN status = 'OPEN' THEN 1
                 WHEN status = 'BLOCKED' THEN 2
                 ELSE 3 END ASC,
            is_on_critical_path DESC,
            updated_at DESC",
    )?;

    let rows = stmt.query_map(params![project_id, entity_type], |r| {
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
        })
    })?;

    Ok(rows
        .filter_map(|r| r.ok())
        .filter(|e| statuses.contains(&e.status.as_str()))
        .collect())
}

fn query_all_project_entities(conn: &Connection, project_id: &str) -> Result<Vec<WorkEntity>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos,
                calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at
         FROM work_entities
         WHERE project_id = ?1",
    )?;

    let rows = stmt.query_map(params![project_id], |r| {
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
        })
    })?;

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
    let task_info: Option<(String, String, f64)> = conn
        .query_row(
            "SELECT title, category, calibrated_pomos FROM work_entities WHERE id = ?1 AND project_id = ?2",
            params![task_id, project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();

    if let Some((title, cat, est)) = task_info {
        let snapshot_json = capture_undo_snapshot(conn, project_id)?;
        let pomos = pomos_spent.max(1);
        let note = format!("تکمیل و تحویل گام [{}]: {}", task_id, title);

        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE work_entities SET status = 'CLOSED', updated_at = ?1 WHERE id = ?2 AND project_id = ?3",
            params![now, task_id, project_id],
        )?;
        tx.execute(
            "INSERT INTO session_work_logs (
                project_id, task_id, category, estimated_pomos, effective_pomos,
                deliverable_summary, internal_friction_note, client_sanitized_note, source_origin, logged_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', ?6, 'QUICK_COMPLETE', ?7)",
            params![project_id, task_id, cat, est, pomos, note, now],
        )?;
        tx.execute(
            "INSERT INTO mutation_audit_logs (project_id, summary_text, snapshot_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                project_id,
                format!("تکمیل سریع گام [{}]", task_id),
                snapshot_json,
                now
            ],
        )?;
        tx.commit()?;
        Ok(format!(
            "گام [{}] مختومه شد و گام بعدی در مسیر پیش‌رو فعال گردید.",
            task_id
        ))
    } else {
        Ok("تسک موردنظر یافت نشد.".into())
    }
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
    let now = Local::now().to_rfc3339();
    let snapshot_json = capture_undo_snapshot(conn, project_id)?;

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
                                    calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at
                             FROM work_entities WHERE id = ?1 AND project_id = ?2",
                            params![tid, project_id],
                            |r| {
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
                                })
                            },
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

                        tx.execute(
                            "UPDATE work_entities SET
                                category = ?1, title = ?2, definition_of_done = ?3,
                                first_physical_action = ?4, status = 'IN_PROGRESS', is_on_critical_path = 1,
                                optimistic_pomos = ?5, most_likely_pomos = ?6, pessimistic_pomos = ?7,
                                calibrated_pomos = ?8, updated_at = ?9
                             WHERE id = ?10 AND project_id = ?11",
                            params![
                                next_cat,
                                next_title,
                                next_dod,
                                next_action,
                                o,
                                ml,
                                p,
                                calibrated,
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
                        source_event_id, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        new_id,
                        project_id,
                        etype,
                        cat,
                        m.title.clone().unwrap_or_else(|| "گام جدید".into()),
                        m.definition_of_done.clone().unwrap_or_default(),
                        m.first_physical_action.clone().unwrap_or_default(),
                        status,
                        if m.is_on_critical_path.unwrap_or(true) { 1 } else { 0 },
                        o,
                        ml,
                        p,
                        calibrated,
                        m.stakeholder_tag,
                        event_id,
                        now
                    ],
                )?;
                if etype == "DECISION" {
                    decisions += 1;
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
                                    calibrated_pomos, superseded_by_id, stakeholder_tag, updated_at
                             FROM work_entities WHERE id = ?1 AND project_id = ?2",
                            params![tid, project_id],
                            |r| {
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
                                })
                            },
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

                        tx.execute(
                            "UPDATE work_entities SET
                                entity_type = ?1, category = ?2, title = ?3, definition_of_done = ?4,
                                first_physical_action = ?5, status = ?6, is_on_critical_path = ?7,
                                optimistic_pomos = ?8, most_likely_pomos = ?9, pessimistic_pomos = ?10,
                                calibrated_pomos = ?11, stakeholder_tag = ?12, updated_at = ?13
                             WHERE id = ?14 AND project_id = ?15",
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
    Ok(badge)
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
    let norm_ch = normalize_chat_channel(channel);
    let now = Local::now().to_rfc3339();
    let cleaned_questions: Vec<String> = clarifying_questions
        .iter()
        .map(|q| q.trim().to_string())
        .filter(|q| !q.is_empty())
        .collect();
    let questions_json = serde_json::to_string(&cleaned_questions).unwrap_or_else(|_| "[]".into());

    let effective_sid = if session_id.trim().is_empty() {
        ensure_active_chat_session(conn, project_id, norm_ch)?
    } else {
        session_id.to_string()
    };

    conn.execute(
        "INSERT INTO chat_messages (
            project_id, session_id, sender, channel, message_text,
            archived_file_path, applied_mutations_badge, clarifying_questions_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            project_id,
            effective_sid,
            sender,
            norm_ch,
            assistant_reply,
            archived_path,
            badge,
            questions_json,
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
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO knowledge_documents (
            project_id, title, doc_type, original_filename, archived_path,
            executive_summary, extracted_items_count, ingested_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            project_id,
            title,
            doc_type,
            original_filename,
            archived_path,
            summary,
            items_count,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_knowledge_doc_analysis(
    conn: &Connection,
    doc_id: i64,
    new_summary: &str,
    items_count: i32,
    new_archived_path: Option<&str>,
) -> Result<()> {
    let now = Local::now().to_rfc3339();
    if let Some(kp) = new_archived_path {
        conn.execute(
            "UPDATE knowledge_documents
             SET executive_summary = ?1, extracted_items_count = ?2, archived_path = ?3, ingested_at = ?4
             WHERE id = ?5",
            params![new_summary, items_count, kp, now, doc_id],
        )?;
    } else {
        conn.execute(
            "UPDATE knowledge_documents
             SET executive_summary = ?1, extracted_items_count = ?2, ingested_at = ?3
             WHERE id = ?4",
            params![new_summary, items_count, now, doc_id],
        )?;
    }
    Ok(())
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
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'PENDING', 0, ?11, ?12, ?12)",
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

    // Auto-close pending knowledge_doc items whose executive summary is no longer the failure placeholder
    let like_prefix = format!("{}%", UNANALYZED_DOC_SUMMARY_PREFIX);
    conn.execute(
        "UPDATE pending_ai_queue
         SET status = 'DONE', updated_at = ?1
         WHERE status IN ('PENDING', 'PROCESSING')
           AND user_message_id IS NULL
           AND knowledge_doc_id IS NOT NULL
           AND NOT EXISTS (
               SELECT 1 FROM knowledge_documents k
               WHERE k.id = pending_ai_queue.knowledge_doc_id
                 AND k.executive_summary LIKE ?2
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
         WHERE executive_summary LIKE ?1
           AND id NOT IN (SELECT knowledge_doc_id FROM pending_ai_queue WHERE knowledge_doc_id IS NOT NULL)
         ORDER BY id ASC",
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

    Ok(inserted)
}

pub fn query_pending_ai_queue(conn: &Connection) -> Result<Vec<PendingQueueItem>> {
    let mut stmt = conn.prepare(
        "SELECT q.id, q.project_id, COALESCE(p.name, q.project_id), q.session_id, q.channel,
                q.user_message_id, q.system_message_id, q.knowledge_doc_id, q.raw_event_id,
                q.title_preview, q.source_type, q.archived_path, q.status, q.attempts,
                q.last_error, q.created_at, q.updated_at
         FROM pending_ai_queue q
         LEFT JOIN projects p ON p.id = q.project_id
         WHERE q.status IN ('PENDING', 'PROCESSING')
         ORDER BY q.id ASC",
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
                created_at: r.get(15)?,
                updated_at: r.get(16)?,
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
                q.last_error, q.created_at, q.updated_at
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
                created_at: r.get(15)?,
                updated_at: r.get(16)?,
            })
        },
    )
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
}
