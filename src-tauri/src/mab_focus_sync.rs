use crate::models::{Project, SyncReport};
use crate::vault_manager::{archive_raw_user_input, ensure_project_directories, sanitize_filename};
use chrono::Local;
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct FocusBridgeFile {
    #[serde(default)]
    pub projects: Vec<FocusProjectItem>,
    #[serde(default)]
    pub session_notes: Vec<FocusNoteItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusProjectItem {
    pub id: String,
    pub name: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusNoteItem {
    pub project_id: String,
    #[serde(default, alias = "pubproject_name")]
    pub project_name: Option<String>,
    pub task_title: String,
    pub note_text: String,
    pub duration_minutes: i32,
    pub recorded_at: String,
}

#[derive(Debug, Clone)]
struct FocusDbProject {
    id: String,
    name: String,
    parent_id: Option<String>,
}

#[derive(Debug, Clone)]
struct FocusIngestItem {
    dedup_key: String,
    focus_project_id: Option<String>,
    project_name_hint: Option<String>,
    task_title: String,
    body_text: String,
    duration_minutes: i32,
    recorded_at: String,
    create_work_log: bool,
    badge_text: String,
}

fn resolve_focus_db_and_dir(configured_path: &str) -> (Option<PathBuf>, PathBuf) {
    let trimmed = configured_path.trim();
    let p = PathBuf::from(trimmed);
    let is_db_file = p
        .extension()
        .and_then(|s| s.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "db" | "sqlite" | "sqlite3"))
        .unwrap_or(false);

    if is_db_file || p.is_file() {
        let parent = p
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let db = if p.exists() { Some(p) } else { None };
        return (db, parent);
    }

    if !trimmed.is_empty() && p.is_dir() {
        let v3 = p.join("mab_focus_v3.db");
        if v3.is_file() {
            return (Some(v3), p);
        }
        let v1 = p.join("mab_focus.db");
        if v1.is_file() {
            return (Some(v1), p);
        }
    }

    let is_build_dir = {
        let s = trimmed.to_ascii_lowercase();
        s.contains("build") || s.contains("debug") || s.contains("release") || s.contains("runner")
    };

    // بررسی مسیرهای پیش‌فرض ویندوز اپلیکیشن Mab-Focus در صورتی که مسیر خالی، ناموجود، یا پوشه بیلد باشد
    if trimmed.is_empty() || !p.exists() || is_build_dir {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let candidates = [
                PathBuf::from(&appdata).join("com.example\\mab_focus\\MabFocus"),
                PathBuf::from(&appdata).join("com.example.mab_focus\\MabFocus"),
                PathBuf::from(&appdata).join("mab_focus\\MabFocus"),
                PathBuf::from(&appdata).join("MabFocus"),
            ];

            for cand in &candidates {
                let v3 = cand.join("mab_focus_v3.db");
                if v3.is_file() {
                    return (Some(v3), cand.clone());
                }
                let v1 = cand.join("mab_focus.db");
                if v1.is_file() {
                    return (Some(v1), cand.clone());
                }
            }
        }

        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            let candidates = [
                PathBuf::from(&local_appdata).join("com.example\\mab_focus\\MabFocus"),
                PathBuf::from(&local_appdata).join("mab_focus\\MabFocus"),
                PathBuf::from(&local_appdata).join("MabFocus"),
            ];

            for cand in &candidates {
                let v3 = cand.join("mab_focus_v3.db");
                if v3.is_file() {
                    return (Some(v3), cand.clone());
                }
                let v1 = cand.join("mab_focus.db");
                if v1.is_file() {
                    return (Some(v1), cand.clone());
                }
            }
        }
    }

    (None, p)
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

fn table_columns(conn: &Connection, table: &str) -> HashSet<String> {
    let pragma = format!("PRAGMA table_info({})", table);
    let Ok(mut stmt) = conn.prepare(&pragma) else {
        return HashSet::new();
    };
    let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(1)) else {
        return HashSet::new();
    };
    rows.filter_map(|r| r.ok()).collect()
}

fn resolve_macro_project_info(
    focus_project_id: Option<&str>,
    proj_map: &HashMap<String, FocusDbProject>,
) -> (Option<String>, Option<String>, Option<String>) {
    let Some(mut cur_id) = focus_project_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, None, None);
    };
    let mut sub_name: Option<String> = None;

    for depth in 0..4 {
        let Some(node) = proj_map.get(cur_id) else {
            return (Some(cur_id.to_string()), None, sub_name);
        };
        if let Some(ref pid) = node.parent_id {
            if !pid.trim().is_empty() && proj_map.contains_key(pid.trim()) && depth < 3 {
                if sub_name.is_none() {
                    sub_name = Some(node.name.clone());
                }
                cur_id = pid.trim();
                continue;
            }
        }
        return (Some(node.id.clone()), Some(node.name.clone()), sub_name);
    }
    (Some(cur_id.to_string()), None, sub_name)
}

fn derive_compass_project_id(name: &str, existing_ids: &HashSet<String>) -> String {
    let ascii_only = name.is_ascii();
    let base = if ascii_only {
        let cleaned: String = name
            .trim()
            .to_uppercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        cleaned.trim_matches('_').chars().take(32).collect::<String>()
    } else {
        sanitize_filename(name)
    };

    let root = if base.is_empty() {
        format!("FOCUS_{}", &uuid::Uuid::new_v4().to_string()[..6].to_uppercase())
    } else {
        base
    };

    if !existing_ids.iter().any(|id| id.eq_ignore_ascii_case(&root)) {
        return root;
    }
    for n in 2..1000 {
        let cand = format!("{}_{}", root, n);
        if !existing_ids.iter().any(|id| id.eq_ignore_ascii_case(&cand)) {
            return cand;
        }
    }
    format!("FOCUS_{}", &uuid::Uuid::new_v4().to_string()[..6].to_uppercase())
}

fn load_focus_phase_tags(conn: &Connection) -> HashMap<String, String> {
    if !table_exists(conn, "phase_tags") {
        return HashMap::new();
    }
    let Ok(mut stmt) = conn.prepare("SELECT id, name FROM phase_tags") else {
        return HashMap::new();
    };
    let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))) else {
        return HashMap::new();
    };
    rows.filter_map(|r| r.ok()).collect()
}

fn format_tag_labels(tag_ids_json: &str, tags_map: &HashMap<String, String>) -> String {
    let Ok(ids) = serde_json::from_str::<Vec<String>>(tag_ids_json) else {
        return String::new();
    };
    let names: Vec<String> = ids
        .into_iter()
        .filter_map(|id| tags_map.get(&id).cloned())
        .collect();
    names.join("، ")
}

fn ingest_item_into_compass(
    conn: &Connection,
    local_projects: &[Project],
    default_proj: &Project,
    item: &FocusIngestItem,
    now: &str,
) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(item.dedup_key.as_bytes());
    let fingerprint = format!("{:x}", hasher.finalize());

    let already_synced: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM mab_focus_synced_notes WHERE fingerprint = ?1",
            params![fingerprint],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if already_synced > 0 {
        return false;
    }

    let target_proj = local_projects
        .iter()
        .find(|p| {
            if let Some(ref fid) = item.focus_project_id {
                if p.external_focus_id.as_deref() == Some(fid.as_str())
                    || p.id.eq_ignore_ascii_case(fid)
                {
                    return true;
                }
            }
            if let Some(ref pname) = item.project_name_hint {
                if p.name.trim() == pname.trim() {
                    return true;
                }
            }
            false
        })
        .unwrap_or(default_proj);

    let formatted_note = if item.duration_minutes > 0 {
        format!(
            "[Mab-Focus Sync] تسک: {} ({} دقیقه)\n{}",
            item.task_title, item.duration_minutes, item.body_text
        )
    } else {
        format!(
            "[Mab-Focus Sync] تسک: {}\n{}",
            item.task_title, item.body_text
        )
    };

    let archived_path = archive_raw_user_input(
        &target_proj.workspace_path,
        "mabfocus_note",
        &formatted_note,
    )
    .ok();

    let _ = conn.execute(
        "INSERT INTO mab_focus_synced_notes (fingerprint, project_id, raw_note, synced_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![fingerprint, target_proj.id, formatted_note, now],
    );

    let _ = conn.execute(
        "INSERT INTO raw_events (project_id, raw_content, archived_path, source_type, created_at)
         VALUES (?1, ?2, ?3, 'MAB_FOCUS_SYNC', ?4)",
        params![target_proj.id, formatted_note, archived_path, now],
    );

    if item.create_work_log {
        let pomos = ((item.duration_minutes + 12) / 25).max(1);
        let log_summary = if item.body_text.trim().is_empty() {
            item.task_title.clone()
        } else {
            format!("{}: {}", item.task_title, item.body_text)
        };
        let _ = conn.execute(
            "INSERT INTO session_work_logs (
                project_id, task_id, category, estimated_pomos, effective_pomos,
                deliverable_summary, internal_friction_note, client_sanitized_note,
                source_origin, logged_at
            ) VALUES (?1, NULL, 'GENERAL', ?2, ?3, ?4, '', ?4, 'MAB_FOCUS_SYNC', ?5)",
            params![
                target_proj.id,
                pomos as f64,
                pomos,
                log_summary,
                item.recorded_at
            ],
        );
    }

    let copilot_sid =
        crate::db::queries::ensure_active_chat_session(conn, &target_proj.id, "COPILOT")
            .unwrap_or_default();

    let _ = conn.execute(
        "INSERT INTO chat_messages (project_id, session_id, sender, channel, message_text, archived_file_path, applied_mutations_badge, created_at)
         VALUES (?1, ?2, 'MAB_FOCUS_SYNC', 'COPILOT', ?3, ?4, ?5, ?6)",
        params![
            target_proj.id,
            copilot_sid,
            formatted_note,
            archived_path,
            item.badge_text,
            now
        ],
    );
    let _ = conn.execute(
        "UPDATE chat_sessions SET updated_at = ?1 WHERE id = ?2",
        params![now, copilot_sid],
    );

    true
}

fn sync_with_focus_sqlite(
    conn: &mut Connection,
    db_path: &Path,
    vault_root: &str,
    now: &str,
) -> Result<(i32, i32, i32), String> {
    let (focus_conn, can_write) = match Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE,
    ) {
        Ok(c) => (c, true),
        Err(_) => {
            let c = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|e| format!("خطا در باز کردن دیتابیس Mab-Focus: {}", e))?;
            (c, false)
        }
    };
    let _ = focus_conn.busy_timeout(Duration::from_millis(3000));

    let mut imported_projects = 0;
    let mut exported_projects = 0;
    let mut ingested_notes = 0;

    let proj_cols = table_columns(&focus_conn, "projects");
    if proj_cols.is_empty() {
        return Ok((0, 0, 0));
    }

    let has_archived = proj_cols.contains("archived");
    let has_status = proj_cols.contains("status");
    let has_realm = proj_cols.contains("realm");
    let has_created_at = proj_cols.contains("created_at");

    let proj_sql = if has_archived {
        "SELECT id, name, parent_id FROM projects WHERE COALESCE(archived, 0) = 0"
    } else if has_status {
        "SELECT id, name, parent_id FROM projects WHERE COALESCE(status, 'active') != 'archived'"
    } else {
        "SELECT id, name, parent_id FROM projects"
    };

    let focus_projects: Vec<FocusDbProject> = {
        let mut stmt = focus_conn.prepare(proj_sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(FocusDbProject {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    parent_id: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let proj_map: HashMap<String, FocusDbProject> = focus_projects
        .iter()
        .map(|p| (p.id.clone(), p.clone()))
        .collect();

    // ۱. وارد کردن پروژه‌های کلان جدید از SQLite Mab-Focus به Mab-Compass
    let existing_local = load_local_projects(conn)?;
    let mut existing_ids: HashSet<String> =
        existing_local.iter().map(|p| p.id.clone()).collect();

    for fp in focus_projects
        .iter()
        .filter(|p| p.parent_id.as_deref().map(str::trim).unwrap_or("").is_empty())
    {
        if fp.name.trim().is_empty() {
            continue;
        }
        let matched_local = existing_local.iter().find(|lp| {
            lp.external_focus_id.as_deref() == Some(fp.id.as_str())
                || lp.name.trim() == fp.name.trim()
                || lp.id.eq_ignore_ascii_case(fp.name.trim())
        });

        if let Some(lp) = matched_local {
            if lp.external_focus_id.as_deref() != Some(fp.id.as_str()) {
                let _ = conn.execute(
                    "UPDATE projects SET external_focus_id = ?1 WHERE id = ?2",
                    params![fp.id, lp.id],
                );
            }
        } else {
            let new_cid = derive_compass_project_id(&fp.name, &existing_ids);
            let proj_folder = Path::new(vault_root)
                .join(&new_cid)
                .to_string_lossy()
                .to_string();
            let _ = ensure_project_directories(&proj_folder);
            if conn
                .execute(
                    "INSERT INTO projects (id, name, workspace_path, external_focus_id, is_active, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 0, ?5)",
                    params![new_cid, fp.name.trim(), proj_folder, fp.id, now],
                )
                .is_ok()
            {
                existing_ids.insert(new_cid);
                imported_projects += 1;
            }
        }
    }

    // ۲. صادر کردن پروژه‌های Mab-Compass به جدول projects در Mab-Focus (با UUID v4 استاندارد)
    let local_projects = load_local_projects(conn)?;
    for lp in &local_projects {
        let _ = ensure_project_directories(&lp.workspace_path);

        let matched_focus = focus_projects.iter().find(|fp| {
            lp.external_focus_id.as_deref() == Some(fp.id.as_str())
                || fp.name.trim() == lp.name.trim()
                || fp.name.trim().eq_ignore_ascii_case(lp.name.trim())
        });

        if let Some(fp) = matched_focus {
            if lp.external_focus_id.as_deref() != Some(fp.id.as_str()) {
                let _ = conn.execute(
                    "UPDATE projects SET external_focus_id = ?1 WHERE id = ?2",
                    params![fp.id, lp.id],
                );
            }
            continue;
        }

        if !can_write {
            continue;
        }

        let focus_uuid = if let Some(ref ext) = lp.external_focus_id {
            if uuid::Uuid::parse_str(ext).is_ok() {
                ext.clone()
            } else {
                uuid::Uuid::new_v4().to_string()
            }
        } else {
            uuid::Uuid::new_v4().to_string()
        };

        let utc_now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
        let insert_res = if has_status && has_realm && has_created_at {
            focus_conn.execute(
                "INSERT INTO projects (id, name, parent_id, status, color, count_short_breaks_as_work, count_long_breaks_as_work, realm, created_at)
                 VALUES (?1, ?2, NULL, 'active', '#3B82F6', 0, 0, 'work', ?3)",
                params![focus_uuid, lp.name, utc_now],
            )
        } else if has_status {
            focus_conn.execute(
                "INSERT INTO projects (id, name, parent_id, status, color, count_short_breaks_as_work, count_long_breaks_as_work)
                 VALUES (?1, ?2, NULL, 'active', '#3B82F6', 0, 0)",
                params![focus_uuid, lp.name],
            )
        } else {
            focus_conn.execute(
                "INSERT INTO projects (id, name, parent_id, count_short_breaks_as_work, count_long_breaks_as_work, color, archived)
                 VALUES (?1, ?2, NULL, 0, 0, '#3B82F6', 0)",
                params![focus_uuid, lp.name],
            )
        };

        if insert_res.is_ok() {
            let _ = conn.execute(
                "UPDATE projects SET external_focus_id = ?1 WHERE id = ?2",
                params![focus_uuid, lp.id],
            );
            exported_projects += 1;
        }
    }

    // ۳. دریافت نشست‌های تکمیل‌شده و یادداشت‌ها از جداول واقعی Mab-Focus
    let refreshed_local = load_local_projects(conn)?;
    let Some(default_proj) = refreshed_local
        .iter()
        .find(|p| p.is_active)
        .or_else(|| refreshed_local.first())
        .cloned()
    else {
        return Ok((imported_projects, exported_projects, 0));
    };

    let tags_map = load_focus_phase_tags(&focus_conn);
    let mut items: Vec<FocusIngestItem> = Vec::new();

    if table_exists(&focus_conn, "time_records") {
        // نسخه v3: جدول time_records شامل سوابق کامل نشست‌ها است
        let tr_cols = table_columns(&focus_conn, "time_records");
        let has_tr_realm = tr_cols.contains("realm");
        let sql = if has_tr_realm {
            "SELECT id, project_id, title_snapshot, tag_ids_json, start_time, end_time, duration_seconds, edit_note
             FROM time_records
             WHERE COALESCE(realm, 'work') = 'work' AND end_time IS NOT NULL AND duration_seconds >= 60
             ORDER BY start_time ASC"
        } else {
            "SELECT id, project_id, title_snapshot, tag_ids_json, start_time, end_time, duration_seconds, edit_note
             FROM time_records
             WHERE end_time IS NOT NULL AND duration_seconds >= 60
             ORDER BY start_time ASC"
        };

        if let Ok(mut stmt) = focus_conn.prepare(sql) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            }) {
                for row in rows.filter_map(|r| r.ok()) {
                    let (rec_id, raw_pid, title, tag_json, _start, end_t, dur_secs, edit_note) = row;
                    let (macro_id, macro_name, sub_name) =
                        resolve_macro_project_info(raw_pid.as_deref(), &proj_map);
                    let tags_str = format_tag_labels(&tag_json, &tags_map);
                    let mut meta_parts = Vec::new();
                    if let Some(sub) = sub_name {
                        meta_parts.push(format!("زیرپروژه: {}", sub));
                    }
                    if !tags_str.is_empty() {
                        meta_parts.push(format!("فاز: {}", tags_str));
                    }
                    if let Some(en) = edit_note.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                        meta_parts.push(format!("توضیح ویرایش: {}", en));
                    }
                    let body = if meta_parts.is_empty() {
                        "ثبت نشست تمرکز تکمیل‌شده از دیتابیس Mab-Focus".to_string()
                    } else {
                        meta_parts.join(" • ")
                    };

                    items.push(FocusIngestItem {
                        dedup_key: format!("focus_v3_time_record|{}", rec_id),
                        focus_project_id: macro_id,
                        project_name_hint: macro_name,
                        task_title: title,
                        body_text: body,
                        duration_minutes: ((dur_secs + 30) / 60).max(1) as i32,
                        recorded_at: end_t,
                        create_work_log: true,
                        badge_text: "سینک نشست کارکرد از Mab-Focus (SQLite v3)".into(),
                    });
                }
            }
        }
    } else if table_exists(&focus_conn, "task_logs") {
        // نسخه v1: جدول task_logs
        let tl_cols = table_columns(&focus_conn, "task_logs");
        let has_entry_type = tl_cols.contains("entry_type");
        let sql = if has_entry_type {
            "SELECT id, title, macro_project_id, sub_project_id, tag_ids_json, end_time, duration_seconds, edit_note
             FROM task_logs
             WHERE COALESCE(entry_type, 'work') = 'work' AND end_time IS NOT NULL
             ORDER BY start_time ASC"
        } else {
            "SELECT id, title, macro_project_id, sub_project_id, tag_ids_json, end_time, duration_seconds, edit_note
             FROM task_logs
             WHERE end_time IS NOT NULL
             ORDER BY start_time ASC"
        };

        if let Ok(mut stmt) = focus_conn.prepare(sql) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            }) {
                for row in rows.filter_map(|r| r.ok()) {
                    let (tid, title, macro_pid, sub_pid, tag_json, end_t, dur_secs, edit_note) = row;
                    let note_trimmed = edit_note.as_deref().map(str::trim).unwrap_or("");
                    if dur_secs < 60 && note_trimmed.is_empty() {
                        continue;
                    }
                    let effective_pid = macro_pid.as_deref().or(sub_pid.as_deref());
                    let (macro_id, macro_name, sub_from_macro) =
                        resolve_macro_project_info(effective_pid, &proj_map);
                    let sub_name = sub_pid
                        .as_deref()
                        .and_then(|sid| proj_map.get(sid))
                        .map(|p| p.name.clone())
                        .or(sub_from_macro);

                    let tags_str = format_tag_labels(&tag_json, &tags_map);
                    let mut parts = Vec::new();
                    if let Some(sub) = sub_name {
                        parts.push(format!("زیرپروژه: {}", sub));
                    }
                    if !tags_str.is_empty() {
                        parts.push(format!("فاز: {}", tags_str));
                    }
                    if !note_trimmed.is_empty() {
                        parts.push(format!("یادداشت: {}", note_trimmed));
                    }
                    let body = if parts.is_empty() {
                        "ثبت نشست تمرکز تکمیل‌شده از دیتابیس Mab-Focus".to_string()
                    } else {
                        parts.join("\n")
                    };

                    items.push(FocusIngestItem {
                        dedup_key: format!("focus_v1_task_log|{}", tid),
                        focus_project_id: macro_id.clone(),
                        project_name_hint: macro_name.clone(),
                        task_title: title.clone(),
                        body_text: body,
                        duration_minutes: ((dur_secs + 30) / 60).max(1) as i32,
                        recorded_at: end_t.clone(),
                        create_work_log: dur_secs >= 60,
                        badge_text: "سینک تسک از Mab-Focus (SQLite) + آرشیو در 00_raw_inputs".into(),
                    });

                    // اگر کاربر بعداً یادداشت ویرایش جدیدی روی تسک ثبت کرد، یادداشت جدید هم سینک شود
                    if !note_trimmed.is_empty() {
                        items.push(FocusIngestItem {
                            dedup_key: format!("focus_v1_task_edit_note|{}|{}", tid, note_trimmed),
                            focus_project_id: macro_id,
                            project_name_hint: macro_name,
                            task_title: format!("یادداشت ویرایش: {}", title),
                            body_text: format!("یادداشت: {}", note_trimmed),
                            duration_minutes: 0,
                            recorded_at: end_t,
                            create_work_log: false,
                            badge_text: "سینک یادداشت از Mab-Focus + آرشیو در 00_raw_inputs".into(),
                        });
                    }
                }
            }
        }
    }

    if table_exists(&focus_conn, "session_notes") {
        let sn_cols = table_columns(&focus_conn, "session_notes");
        let has_sn_realm = sn_cols.contains("realm");
        let has_tr_table = table_exists(&focus_conn, "time_records");

        let sql = match (has_tr_table, has_sn_realm) {
            (true, true) => {
                "SELECT sn.id, sn.note_type, sn.project_id, sn.content, sn.created_at, tr.title_snapshot
                 FROM session_notes sn
                 LEFT JOIN time_records tr ON sn.time_record_id = tr.id
                 WHERE COALESCE(sn.realm, 'work') = 'work' AND TRIM(sn.content) != ''
                 ORDER BY sn.created_at ASC"
            }
            (true, false) => {
                "SELECT sn.id, sn.note_type, sn.project_id, sn.content, sn.created_at, tr.title_snapshot
                 FROM session_notes sn
                 LEFT JOIN time_records tr ON sn.time_record_id = tr.id
                 WHERE TRIM(sn.content) != ''
                 ORDER BY sn.created_at ASC"
            }
            (false, true) => {
                "SELECT sn.id, sn.note_type, sn.project_id, sn.content, sn.created_at, NULL
                 FROM session_notes sn
                 WHERE COALESCE(sn.realm, 'work') = 'work' AND TRIM(sn.content) != ''
                 ORDER BY sn.created_at ASC"
            }
            (false, false) => {
                "SELECT sn.id, sn.note_type, sn.project_id, sn.content, sn.created_at, NULL
                 FROM session_notes sn
                 WHERE TRIM(sn.content) != ''
                 ORDER BY sn.created_at ASC"
            }
        };

        if let Ok(mut stmt) = focus_conn.prepare(sql) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            }) {
                for row in rows.filter_map(|r| r.ok()) {
                    let (nid, note_type, raw_pid, content, created_at, title_snap) = row;
                    let (macro_id, macro_name, sub_name) =
                        resolve_macro_project_info(raw_pid.as_deref(), &proj_map);
                    let mut body = format!("یادداشت ({}): {}", note_type, content.trim());
                    if let Some(sub) = sub_name {
                        body = format!("زیرپروژه: {}\n{}", sub, body);
                    }
                    items.push(FocusIngestItem {
                        dedup_key: format!("focus_v3_session_note|{}", nid),
                        focus_project_id: macro_id,
                        project_name_hint: macro_name,
                        task_title: title_snap
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or_else(|| "یادداشت نشست در Mab-Focus".into()),
                        body_text: body,
                        duration_minutes: 0,
                        recorded_at: created_at,
                        create_work_log: false,
                        badge_text: "سینک یادداشت نشست از Mab-Focus + آرشیو در 00_raw_inputs".into(),
                    });
                }
            }
        }
    } else if table_exists(&focus_conn, "work_notes") {
        if let Ok(mut stmt) = focus_conn.prepare(
            "SELECT id, project_id, task_title, content, created_at
             FROM work_notes WHERE TRIM(content) != '' ORDER BY created_at ASC",
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            }) {
                for row in rows.filter_map(|r| r.ok()) {
                    let (nid, raw_pid, task_title, content, created_at) = row;
                    let (macro_id, macro_name, sub_name) =
                        resolve_macro_project_info(raw_pid.as_deref(), &proj_map);
                    let body = if let Some(sub) = sub_name {
                        format!("زیرپروژه: {}\nیادداشت: {}", sub, content.trim())
                    } else {
                        format!("یادداشت: {}", content.trim())
                    };
                    items.push(FocusIngestItem {
                        dedup_key: format!("focus_work_note|{}", nid),
                        focus_project_id: macro_id,
                        project_name_hint: macro_name,
                        task_title,
                        body_text: body,
                        duration_minutes: 0,
                        recorded_at: created_at,
                        create_work_log: false,
                        badge_text: "سینک یادداشت از Mab-Focus + آرشیو در 00_raw_inputs".into(),
                    });
                }
            }
        }
    }

    for item in &items {
        if ingest_item_into_compass(conn, &refreshed_local, &default_proj, item, now) {
            ingested_notes += 1;
        }
    }

    Ok((imported_projects, exported_projects, ingested_notes))
}

fn load_local_projects(conn: &Connection) -> Result<Vec<Project>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, workspace_path, external_focus_id, is_active, updated_at FROM projects")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Project {
                id: r.get(0)?,
                name: r.get(1)?,
                workspace_path: r.get(2)?,
                external_focus_id: r.get(3)?,
                is_active: r.get::<_, i32>(4)? == 1,
                updated_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn synchronize_with_mab_focus(conn: &mut Connection) -> Result<SyncReport, String> {
    let (vault_root, configured_focus_path): (String, String) = conn
        .query_row(
            "SELECT vault_root_path, mab_focus_data_path FROM app_settings WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let (focus_db_opt, focus_dir) = resolve_focus_db_and_dir(&configured_focus_path);
    let now = Local::now().to_rfc3339();

    let mut imported_projects = 0;
    let mut exported_projects = 0;
    let mut ingested_notes = 0;

    // اولویت ۱: اتصال مستقیم به دیتابیس واقعی SQLite اپلیکیشن Mab-Focus (mab_focus_v3.db / mab_focus.db)
    if let Some(ref db_path) = focus_db_opt {
        if let Some(p_str) = focus_dir.to_str() {
            if configured_focus_path.trim() != p_str {
                let _ = conn.execute(
                    "UPDATE app_settings SET mab_focus_data_path = ?1 WHERE id = 1",
                    params![p_str],
                );
            }
        }
        let (imp, exp, ing) = sync_with_focus_sqlite(conn, db_path, &vault_root, &now)?;
        imported_projects += imp;
        exported_projects += exp;
        ingested_notes += ing;
    }

    // اولویت ۲ (پشتیبانی تکمیلی / پل اختیاری JSON و TXT در صورت وجود در پوشه)
    let bridge_file_path = focus_dir.join("mab_ecosystem_sync.json");
    let txt_report_path = focus_dir.join("mab_focus_report.txt");

    if bridge_file_path.exists() || txt_report_path.exists() {
        let mut bridge_data = if bridge_file_path.exists() {
            let raw = fs::read_to_string(&bridge_file_path).unwrap_or_default();
            serde_json::from_str::<FocusBridgeFile>(&raw).unwrap_or(FocusBridgeFile {
                projects: vec![],
                session_notes: vec![],
            })
        } else {
            FocusBridgeFile {
                projects: vec![],
                session_notes: vec![],
            }
        };

        for fp in &bridge_data.projects {
            let clean_fp_id = fp.id.trim().to_uppercase().replace(' ', "_");
            if clean_fp_id.is_empty() || fp.name.trim().is_empty() {
                continue;
            }
            let exists: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM projects WHERE id = ?1 OR name = ?2",
                    params![clean_fp_id, fp.name],
                    |r| r.get(0),
                )
                .unwrap_or(0);

            if exists == 0 {
                let proj_folder = Path::new(&vault_root)
                    .join(&clean_fp_id)
                    .to_string_lossy()
                    .to_string();
                let _ = ensure_project_directories(&proj_folder);
                let _ = conn.execute(
                    "INSERT INTO projects (id, name, workspace_path, external_focus_id, is_active, updated_at)
                     VALUES (?1, ?2, ?3, ?4, 0, ?5)",
                    params![clean_fp_id, fp.name, proj_folder, fp.id, now],
                );
                imported_projects += 1;
            }
        }

        let local_projects = load_local_projects(conn)?;
        for lp in &local_projects {
            let _ = ensure_project_directories(&lp.workspace_path);
            let in_bridge = bridge_data
                .projects
                .iter()
                .any(|bp| bp.id.eq_ignore_ascii_case(&lp.id) || bp.name == lp.name);
            if !in_bridge && bridge_file_path.exists() {
                bridge_data.projects.push(FocusProjectItem {
                    id: lp.id.clone(),
                    name: lp.name.clone(),
                    updated_at: lp.updated_at.clone(),
                });
                exported_projects += 1;
            }
        }

        let mut notes_to_process = bridge_data.session_notes.clone();
        if txt_report_path.exists() {
            if let Ok(txt_content) = fs::read_to_string(&txt_report_path) {
                for line in txt_content.lines() {
                    let trimmed = line.trim();
                    if trimmed.chars().count() > 10
                        && !trimmed.starts_with('#')
                        && !trimmed.starts_with('=')
                    {
                        notes_to_process.push(FocusNoteItem {
                            project_id: "ACTIVE".into(),
                            project_name: None,
                            task_title: "نشست ثبت‌شده در Mab-Focus".into(),
                            note_text: trimmed.to_string(),
                            duration_minutes: 25,
                            recorded_at: Local::now().to_rfc3339(),
                        });
                    }
                }
            }
        }

        if let Some(default_proj) = local_projects
            .iter()
            .find(|p| p.is_active)
            .or_else(|| local_projects.first())
            .cloned()
        {
            for note in &notes_to_process {
                let item = FocusIngestItem {
                    dedup_key: format!("{}|{}|{}", note.project_id, note.task_title, note.note_text),
                    focus_project_id: Some(note.project_id.clone()),
                    project_name_hint: note.project_name.clone(),
                    task_title: note.task_title.clone(),
                    body_text: format!("یادداشت: {}", note.note_text),
                    duration_minutes: note.duration_minutes,
                    recorded_at: note.recorded_at.clone(),
                    create_work_log: true,
                    badge_text: "سینک یک‌طرفه از Mab-Focus + آرشیو در 00_raw_inputs".into(),
                };
                if ingest_item_into_compass(conn, &local_projects, &default_proj, &item, &now) {
                    ingested_notes += 1;
                }
            }
        }

        if bridge_file_path.exists() {
            if let Ok(serialized) = serde_json::to_string_pretty(&bridge_data) {
                let _ = fs::write(&bridge_file_path, serialized);
            }
        }
    }

    Ok(SyncReport {
        projects_imported_from_focus: imported_projects,
        projects_exported_to_focus: exported_projects,
        notes_ingested_from_focus: ingested_notes,
        last_synced_at: Local::now().format("%H:%M:%S").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{get_dashboard_state, get_settings, update_settings};
    use crate::db::schema::init_database;

    #[test]
    fn test_bidirectional_project_and_deduped_note_sync() {
        let root = std::env::temp_dir().join(format!("mab_focus_sync_test_{}", uuid::Uuid::new_v4()));
        let vault_dir = root.join("vault");
        let focus_dir = root.join("MabFocus");
        fs::create_dir_all(&vault_dir).unwrap();
        fs::create_dir_all(&focus_dir).unwrap();

        let db_path = root.join("test.sqlite");
        let mut conn = init_database(&db_path, &vault_dir).unwrap();

        let mut settings = get_settings(&conn).unwrap();
        settings.mab_focus_data_path = focus_dir.to_string_lossy().to_string();
        update_settings(&conn, &settings).unwrap();

        let bridge = FocusBridgeFile {
            projects: vec![FocusProjectItem {
                id: "ZIGHOOSH".into(),
                name: "پلتفرم زیگهوش".into(),
                updated_at: "2026-11-11T00:00:00+03:30".into(),
            }],
            session_notes: vec![FocusNoteItem {
                project_id: "GERDALO".into(),
                project_name: None,
                task_title: "بهینه‌سازی موتور کش".into(),
                note_text: "تست ردیس انجام شد و سرعت پاسخ ۲ برابر شد".into(),
                duration_minutes: 50,
                recorded_at: "2026-11-11T00:15:00+03:30".into(),
            }],
        };
        fs::write(
            focus_dir.join("mab_ecosystem_sync.json"),
            serde_json::to_string_pretty(&bridge).unwrap(),
        )
        .unwrap();

        fs::write(
            focus_dir.join("mab_focus_report.txt"),
            "# گزارش روزانه\nیادداشت پومودورو برای تست API جدید سرور انجام شد.\n",
        )
        .unwrap();

        let rep1 = synchronize_with_mab_focus(&mut conn).unwrap();
        assert_eq!(rep1.projects_imported_from_focus, 1);
        assert_eq!(rep1.projects_exported_to_focus, 2); // GERDALO + IRANOIL exported
        assert_eq!(rep1.notes_ingested_from_focus, 2); // 1 JSON note + 1 txt line

        // Second sync should ingest zero duplicate notes and import/export zero duplicate projects
        let rep2 = synchronize_with_mab_focus(&mut conn).unwrap();
        assert_eq!(rep2.projects_imported_from_focus, 0);
        assert_eq!(rep2.projects_exported_to_focus, 0);
        assert_eq!(rep2.notes_ingested_from_focus, 0);

        let dash = get_dashboard_state(&conn).unwrap();
        assert!(dash.projects.iter().any(|p| p.id == "ZIGHOOSH"));
        assert_eq!(dash.recent_logs.len(), 2);
        assert_eq!(dash.chat_history.len(), 2);
        assert!(vault_dir.join("ZIGHOOSH").join("00_raw_inputs").is_dir());

        drop(conn);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_sync_with_real_mab_focus_v1_and_v3_sqlite_schemas() {
        // 1. Test v1 schema (mab_focus.db)
        let root_v1 = std::env::temp_dir().join(format!("mab_focus_v1_{}", uuid::Uuid::new_v4()));
        let vault_v1 = root_v1.join("vault");
        let focus_v1 = root_v1.join("MabFocus");
        fs::create_dir_all(&vault_v1).unwrap();
        fs::create_dir_all(&focus_v1).unwrap();

        let fdb_v1 = Connection::open(focus_v1.join("mab_focus.db")).unwrap();
        fdb_v1
            .execute_batch(
                "
                CREATE TABLE projects (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    parent_id TEXT NULL,
                    count_short_breaks_as_work INTEGER NOT NULL DEFAULT 0,
                    count_long_breaks_as_work INTEGER NOT NULL DEFAULT 0,
                    color TEXT NOT NULL DEFAULT '#3B82F6',
                    archived INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE phase_tags (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    category TEXT NOT NULL DEFAULT 'phase',
                    archived INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE task_logs (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    is_untitled_draft INTEGER NOT NULL DEFAULT 0,
                    macro_project_id TEXT NULL,
                    sub_project_id TEXT NULL,
                    tag_ids_json TEXT NOT NULL DEFAULT '[]',
                    start_time TEXT NOT NULL,
                    end_time TEXT NULL,
                    duration_seconds INTEGER NOT NULL DEFAULT 0,
                    overtime_seconds INTEGER NOT NULL DEFAULT 0,
                    pomodoro_set_index INTEGER NOT NULL DEFAULT 1,
                    is_edited INTEGER NOT NULL DEFAULT 0,
                    original_start_time TEXT NULL,
                    original_end_time TEXT NULL,
                    original_duration_seconds INTEGER NULL,
                    edited_at TEXT NULL,
                    edit_note TEXT NULL,
                    sync_status TEXT NOT NULL DEFAULT 'pending',
                    calendar_event_id TEXT NULL
                );
                INSERT INTO phase_tags (id, name) VALUES ('phase_execution', 'اجرا و توسعه');
                INSERT INTO projects (id, name, parent_id, archived) VALUES
                    ('11111111-1111-4111-8111-111111111111', 'سامانه مالی کارفرما', NULL, 0),
                    ('22222222-2222-4222-8222-222222222222', 'ماژول فاکتور', '11111111-1111-4111-8111-111111111111', 0);
                INSERT INTO task_logs (id, title, macro_project_id, sub_project_id, tag_ids_json, start_time, end_time, duration_seconds, edit_note)
                VALUES (
                    '33333333-3333-4333-8333-333333333333',
                    'پیاده‌سازی API صدور فاکتور',
                    '11111111-1111-4111-8111-111111111111',
                    '22222222-2222-4222-8222-222222222222',
                    '[\"phase_execution\"]',
                    '2026-11-12T09:00:00.000',
                    '2026-11-12T09:50:00.000',
                    3000,
                    'تست پرداخت موفق شد'
                );
                ",
            )
            .unwrap();
        drop(fdb_v1);

        let mut c1 = init_database(&root_v1.join("compass.sqlite"), &vault_v1).unwrap();
        let mut s1 = get_settings(&c1).unwrap();
        s1.mab_focus_data_path = focus_v1.to_string_lossy().to_string();
        update_settings(&c1, &s1).unwrap();

        let rep_v1 = synchronize_with_mab_focus(&mut c1).unwrap();
        assert_eq!(rep_v1.projects_imported_from_focus, 1); // macro project only
        assert_eq!(rep_v1.projects_exported_to_focus, 2); // GERDALO + IRANOIL exported as UUIDs
        assert_eq!(rep_v1.notes_ingested_from_focus, 2); // 1 completed task_log + 1 edit_note

        let fdb_v1_check = Connection::open(focus_v1.join("mab_focus.db")).unwrap();
        let focus_proj_cnt: i64 = fdb_v1_check
            .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
            .unwrap();
        assert_eq!(focus_proj_cnt, 4);
        drop(fdb_v1_check);

        // Repeat sync -> zero duplicates
        let rep_v1_again = synchronize_with_mab_focus(&mut c1).unwrap();
        assert_eq!(rep_v1_again.projects_imported_from_focus, 0);
        assert_eq!(rep_v1_again.projects_exported_to_focus, 0);
        assert_eq!(rep_v1_again.notes_ingested_from_focus, 0);
        drop(c1);
        let _ = fs::remove_dir_all(root_v1);

        // 2. Test v3 schema (mab_focus_v3.db) ensuring time_records + session_notes don't double-count
        let root_v3 = std::env::temp_dir().join(format!("mab_focus_v3_{}", uuid::Uuid::new_v4()));
        let vault_v3 = root_v3.join("vault");
        let focus_v3 = root_v3.join("MabFocus");
        fs::create_dir_all(&vault_v3).unwrap();
        fs::create_dir_all(&focus_v3).unwrap();

        let fdb_v3 = Connection::open(focus_v3.join("mab_focus_v3.db")).unwrap();
        fdb_v3
            .execute_batch(
                "
                CREATE TABLE projects (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    parent_id TEXT NULL,
                    status TEXT NOT NULL DEFAULT 'active',
                    color TEXT NOT NULL DEFAULT '#3B82F6',
                    count_short_breaks_as_work INTEGER NOT NULL DEFAULT 0,
                    count_long_breaks_as_work INTEGER NOT NULL DEFAULT 0,
                    realm TEXT NOT NULL DEFAULT 'work',
                    created_at TEXT NOT NULL DEFAULT ''
                );
                CREATE TABLE phase_tags (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    category TEXT NOT NULL DEFAULT 'phase',
                    archived INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE time_records (
                    id TEXT PRIMARY KEY,
                    realm TEXT NOT NULL DEFAULT 'work',
                    clock_mode TEXT NOT NULL DEFAULT 'pomodoro',
                    outcome_id TEXT NULL,
                    project_id TEXT NULL,
                    title_snapshot TEXT NOT NULL DEFAULT 'تسک بدون عنوان',
                    tag_ids_json TEXT NOT NULL DEFAULT '[]',
                    start_time TEXT NOT NULL,
                    end_time TEXT NULL,
                    duration_seconds INTEGER NOT NULL DEFAULT 0,
                    edit_note TEXT NULL
                );
                CREATE TABLE session_notes (
                    id TEXT PRIMARY KEY,
                    realm TEXT NOT NULL DEFAULT 'work',
                    note_type TEXT NOT NULL DEFAULT 'report',
                    time_record_id TEXT NULL,
                    project_id TEXT NULL,
                    content TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                INSERT INTO projects (id, name, parent_id, status, realm) VALUES
                    ('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', 'زیگهوش', NULL, 'active', 'work');
                INSERT INTO time_records (id, realm, project_id, title_snapshot, start_time, end_time, duration_seconds)
                VALUES
                    ('bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb', 'work', 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', 'ساخت ویدئو معرفی', '2026-11-11T17:00:00', '2026-11-11T17:55:00', 3300),
                    ('cccccccc-cccc-4ccc-8ccc-cccccccccccc', 'life', NULL, 'استراحت کوتاه', '2026-11-11T18:00:00', '2026-11-11T18:05:00', 300);
                INSERT INTO session_notes (id, realm, note_type, time_record_id, project_id, content, created_at)
                VALUES
                    ('dddddddd-dddd-4ddd-8ddd-dddddddddddd', 'work', 'report', 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb', 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', 'ایده اول تست شد', '2026-11-11T17:20:00'),
                    ('eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee', 'work', 'report', 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb', 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', 'نکته دوم ضبط شد', '2026-11-11T17:45:00');
                ",
            )
            .unwrap();
        drop(fdb_v3);

        let mut c3 = init_database(&root_v3.join("compass.sqlite"), &vault_v3).unwrap();
        let mut s3 = get_settings(&c3).unwrap();
        s3.mab_focus_data_path = focus_v3.to_string_lossy().to_string();
        update_settings(&c3, &s3).unwrap();

        let rep_v3 = synchronize_with_mab_focus(&mut c3).unwrap();
        assert_eq!(rep_v3.projects_imported_from_focus, 1); // زیگهوش
        assert_eq!(rep_v3.projects_exported_to_focus, 2); // GERDALO + IRANOIL
        assert_eq!(rep_v3.notes_ingested_from_focus, 3); // 1 work time_record + 2 session_notes (life break skipped!)

        // Switch to زیگهوش and verify work log count is 1 (2 pomos for 55m), NOT 3!
        c3.execute("UPDATE projects SET is_active = CASE WHEN name = 'زیگهوش' THEN 1 ELSE 0 END", [])
            .unwrap();
        let d3 = get_dashboard_state(&c3).unwrap();
        assert_eq!(d3.recent_logs.len(), 1);
        assert_eq!(d3.recent_logs[0].effective_pomos, 2);
        assert_eq!(d3.chat_history.len(), 3);

        drop(c3);
        let _ = fs::remove_dir_all(root_v3);
    }
}
