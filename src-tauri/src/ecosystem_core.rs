use crate::models::Project;
use crate::vault_manager::{ensure_project_directories, sanitize_filename};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct CoreSyncStats {
    pub imported_projects: i32,
    pub exported_projects: i32,
    pub updated_projects: i32,
    pub applied_merges: i32,
    pub deleted_projects: i32,
    pub revision: i64,
}

pub fn resolve_default_core_db_path() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("MAB_ECOSYSTEM_DB_PATH") {
        let trimmed = env_path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let trimmed = appdata.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed).join("MabEcosystem").join("mab_core.db"));
        }
    }
    dirs::data_dir().map(|d| d.join("MabEcosystem").join("mab_core.db"))
}

pub fn open_core_db(core_path: &Path) -> Result<Connection, String> {
    if let Some(parent) = core_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("خطا در ساخت پوشه دیتابیس مرکزی: {}", e))?;
    }
    let conn = Connection::open(core_path)
        .map_err(|e| format!("خطا در باز کردن دیتابیس مرکزی MabEcosystem: {}", e))?;
    let _ = conn.busy_timeout(Duration::from_millis(5000));
    let _ = conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        ",
    );
    init_core_schema(&conn).map_err(|e| format!("خطا در مقداردهی اسکیمای مرکزی: {}", e))?;
    Ok(conn)
}

pub fn init_core_schema(core: &Connection) -> rusqlite::Result<()> {
    core.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS core_projects (
            id TEXT PRIMARY KEY,
            code TEXT NOT NULL DEFAULT '',
            name TEXT NOT NULL,
            parent_id TEXT NULL,
            realm TEXT NOT NULL DEFAULT 'work',
            status TEXT NOT NULL DEFAULT 'active',
            merged_into_id TEXT NULL,
            color TEXT NOT NULL DEFAULT '#3B82F6',
            daily_quota_minutes INTEGER NOT NULL DEFAULT 0,
            weekly_quota_minutes INTEGER NOT NULL DEFAULT 0,
            monthly_quota_minutes INTEGER NOT NULL DEFAULT 0,
            hourly_rate REAL NOT NULL DEFAULT 0.0,
            google_calendar_id TEXT NULL,
            domain_code TEXT NOT NULL DEFAULT 'WEALTH_FINANCE',
            primary_outcome_id TEXT NULL,
            scope_mode TEXT NOT NULL DEFAULT 'STANDARD',
            priority_weight INTEGER NOT NULL DEFAULT 5,
            pause_resume_note TEXT NOT NULL DEFAULT '',
            workspace_path TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            deleted_at TEXT NULL,
            updated_by_app TEXT NOT NULL DEFAULT 'system'
        );

        CREATE TABLE IF NOT EXISTS core_project_merges (
            id TEXT PRIMARY KEY,
            source_project_id TEXT NOT NULL,
            target_project_id TEXT NOT NULL,
            source_code TEXT NOT NULL DEFAULT '',
            source_name TEXT NOT NULL DEFAULT '',
            merged_by_app TEXT NOT NULL,
            merged_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS core_project_aliases (
            app_name TEXT NOT NULL,
            local_project_id TEXT NOT NULL,
            core_project_id TEXT NOT NULL,
            PRIMARY KEY (app_name, local_project_id)
        );

        CREATE TABLE IF NOT EXISTS core_outcomes (
            id TEXT PRIMARY KEY,
            core_project_id TEXT NULL,
            realm TEXT NOT NULL DEFAULT 'work',
            domain_code TEXT NOT NULL DEFAULT 'WEALTH_FINANCE',
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'ACTIVE',
            is_completed INTEGER NOT NULL DEFAULT 0,
            source_app TEXT NOT NULL DEFAULT 'mab_focus',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS core_time_logs (
            id TEXT PRIMARY KEY,
            core_project_id TEXT NULL,
            outcome_id TEXT NULL,
            realm TEXT NOT NULL DEFAULT 'work',
            clock_mode TEXT NOT NULL DEFAULT 'pomodoro',
            title TEXT NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT NULL,
            duration_seconds INTEGER NOT NULL DEFAULT 0,
            edit_note TEXT NULL,
            source_app TEXT NOT NULL DEFAULT 'mab_focus',
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS core_notes (
            id TEXT PRIMARY KEY,
            core_project_id TEXT NULL,
            outcome_id TEXT NULL,
            time_record_id TEXT NULL,
            realm TEXT NOT NULL DEFAULT 'work',
            note_type TEXT NOT NULL DEFAULT 'general',
            title TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL,
            tags_json TEXT NOT NULL DEFAULT '[]',
            source_app TEXT NOT NULL DEFAULT 'mab_focus',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS core_work_items (
            id TEXT PRIMARY KEY,
            core_project_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'GENERAL',
            title TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'OPEN',
            definition_of_done TEXT NOT NULL DEFAULT '',
            first_physical_action TEXT NOT NULL DEFAULT '',
            source_app TEXT NOT NULL DEFAULT 'mab_compass',
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS core_activity_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_app TEXT NOT NULL,
            event_type TEXT NOT NULL,
            core_project_id TEXT NULL,
            payload_json TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS core_sync_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS core_app_registry (
            app_id TEXT PRIMARY KEY,
            app_name TEXT NOT NULL,
            db_path TEXT NOT NULL DEFAULT '',
            last_seen_at TEXT NOT NULL,
            schema_version INTEGER NOT NULL DEFAULT 1
        );

        INSERT OR IGNORE INTO core_sync_meta (key, value) VALUES ('revision', '1');
        INSERT OR IGNORE INTO core_sync_meta (key, value) VALUES ('schema_version', '1');
        ",
    )?;
    Ok(())
}

pub fn get_core_revision_from_conn(core: &Connection) -> i64 {
    core.query_row(
        "SELECT value FROM core_sync_meta WHERE key = 'revision'",
        [],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|s| s.parse::<i64>().ok())
    .unwrap_or(1)
}

pub fn get_core_revision_at_path(core_path: &Path) -> i64 {
    if !core_path.exists() {
        return 0;
    }
    let Ok(conn) = open_core_db(core_path) else {
        return 0;
    };
    get_core_revision_from_conn(&conn)
}

fn bump_core_revision(core: &Connection) {
    let cur = get_core_revision_from_conn(core);
    let _ = core.execute(
        "INSERT OR REPLACE INTO core_sync_meta (key, value) VALUES ('revision', ?1)",
        params![(cur + 1).to_string()],
    );
}

fn now_utc_iso() -> String {
    Utc::now().to_rfc3339()
}

fn parse_ts_ms(raw: &str) -> i64 {
    DateTime::parse_from_rfc3339(raw.trim())
        .map(|dt| dt.timestamp_millis())
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%dT%H:%M:%S%.f")
                .map(|ndt| ndt.and_utc().timestamp_millis())
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%dT%H:%M:%S")
                .map(|ndt| ndt.and_utc().timestamp_millis())
        })
        .unwrap_or(0)
}

fn map_core_status_to_compass(core_status: &str) -> &'static str {
    match core_status.trim().to_ascii_lowercase().as_str() {
        "suspended" | "paused" => "PAUSED",
        "archived" => "ARCHIVED",
        _ => "ACTIVE",
    }
}

fn map_compass_status_to_core(compass_status: &str) -> &'static str {
    match compass_status.trim().to_ascii_uppercase().as_str() {
        "PAUSED" | "SUSPENDED" => "suspended",
        "ARCHIVED" => "archived",
        _ => "active",
    }
}

fn map_core_realm_to_compass(core_realm: &str) -> &'static str {
    if core_realm.trim().eq_ignore_ascii_case("life") {
        "LIFE"
    } else {
        "WORK"
    }
}

fn map_compass_realm_to_core(compass_realm: &str) -> &'static str {
    if compass_realm.trim().eq_ignore_ascii_case("LIFE") {
        "life"
    } else {
        "work"
    }
}

fn derive_unique_compass_id(code_hint: &str, name: &str, used_ids: &mut HashSet<String>) -> String {
    let raw_source = if !code_hint.trim().is_empty() {
        code_hint.trim()
    } else {
        name.trim()
    };
    let base = if raw_source.is_ascii() {
        let cleaned: String = raw_source
            .to_uppercase()
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        cleaned.trim_matches('_').chars().take(32).collect::<String>()
    } else {
        sanitize_filename(raw_source)
    };

    let root = if base.is_empty() {
        format!("PRJ_{}", &uuid::Uuid::new_v4().to_string()[..6].to_uppercase())
    } else {
        base
    };

    if !used_ids.iter().any(|id| id.eq_ignore_ascii_case(&root)) {
        used_ids.insert(root.clone());
        return root;
    }
    for n in 2..1000 {
        let cand = format!("{}_{}", root, n);
        if !used_ids.iter().any(|id| id.eq_ignore_ascii_case(&cand)) {
            used_ids.insert(cand.clone());
            return cand;
        }
    }
    let fallback = format!("PRJ_{}", &uuid::Uuid::new_v4().to_string()[..8].to_uppercase());
    used_ids.insert(fallback.clone());
    fallback
}

#[derive(Debug, Clone)]
struct CoreProjectRow {
    id: String,
    code: String,
    name: String,
    parent_id: Option<String>,
    realm: String,
    status: String,
    color: String,
    daily_quota_minutes: i32,
    weekly_quota_minutes: i32,
    monthly_quota_minutes: i32,
    hourly_rate: f64,
    google_calendar_id: Option<String>,
    domain_code: String,
    primary_outcome_id: Option<String>,
    scope_mode: String,
    priority_weight: i32,
    pause_resume_note: String,
    workspace_path: String,
    created_at: String,
    updated_at: String,
    deleted_at: Option<String>,
}

fn load_core_projects(core: &Connection) -> Result<Vec<CoreProjectRow>, String> {
    let mut stmt = core
        .prepare(
            "SELECT id, code, name, parent_id, realm, status, color,
                    daily_quota_minutes, weekly_quota_minutes, monthly_quota_minutes,
                    hourly_rate, google_calendar_id, domain_code, primary_outcome_id,
                    scope_mode, priority_weight, pause_resume_note, workspace_path,
                    created_at, updated_at, deleted_at
             FROM core_projects",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(CoreProjectRow {
                id: r.get(0)?,
                code: r.get(1)?,
                name: r.get(2)?,
                parent_id: r.get(3)?,
                realm: r.get(4)?,
                status: r.get(5)?,
                color: r.get(6)?,
                daily_quota_minutes: r.get(7)?,
                weekly_quota_minutes: r.get(8)?,
                monthly_quota_minutes: r.get(9)?,
                hourly_rate: r.get(10)?,
                google_calendar_id: r.get(11)?,
                domain_code: r.get(12)?,
                primary_outcome_id: r.get(13)?,
                scope_mode: r.get(14)?,
                priority_weight: r.get(15)?,
                pause_resume_note: r.get(16)?,
                workspace_path: r.get(17)?,
                created_at: r.get(18)?,
                updated_at: r.get(19)?,
                deleted_at: r.get(20)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn ensure_compass_project_columns(conn: &Connection) {
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "projects",
        "parent_id",
        "ALTER TABLE projects ADD COLUMN parent_id TEXT NULL",
    );
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "projects",
        "color",
        "ALTER TABLE projects ADD COLUMN color TEXT NOT NULL DEFAULT '#3B82F6'",
    );
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "projects",
        "daily_quota_minutes",
        "ALTER TABLE projects ADD COLUMN daily_quota_minutes INTEGER NOT NULL DEFAULT 0",
    );
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "projects",
        "weekly_quota_minutes",
        "ALTER TABLE projects ADD COLUMN weekly_quota_minutes INTEGER NOT NULL DEFAULT 0",
    );
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "projects",
        "monthly_quota_minutes",
        "ALTER TABLE projects ADD COLUMN monthly_quota_minutes INTEGER NOT NULL DEFAULT 0",
    );
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "projects",
        "hourly_rate",
        "ALTER TABLE projects ADD COLUMN hourly_rate REAL NOT NULL DEFAULT 0.0",
    );
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "projects",
        "google_calendar_id",
        "ALTER TABLE projects ADD COLUMN google_calendar_id TEXT NULL",
    );
    let _ = crate::db::schema::ensure_column_exists(
        conn,
        "projects",
        "created_at",
        "ALTER TABLE projects ADD COLUMN created_at TEXT NOT NULL DEFAULT ''",
    );
}

pub fn load_compass_projects(conn: &Connection) -> Result<Vec<Project>, String> {
    ensure_compass_project_columns(conn);
    let mut stmt = conn
        .prepare(
            "SELECT id, name, workspace_path, external_focus_id, is_active, updated_at,
                    COALESCE(domain_code, 'WEALTH_FINANCE'), primary_outcome_id,
                    COALESCE(lifecycle_status, 'ACTIVE'), COALESCE(scope_mode, 'STANDARD'),
                    COALESCE(realm, 'WORK'), COALESCE(priority_weight, 5),
                    COALESCE(pause_resume_note, ''), parent_id,
                    COALESCE(color, '#3B82F6'), COALESCE(daily_quota_minutes, 0),
                    COALESCE(weekly_quota_minutes, 0), COALESCE(monthly_quota_minutes, 0),
                    COALESCE(hourly_rate, 0.0), google_calendar_id, COALESCE(created_at, '')
             FROM projects ORDER BY is_active DESC, id",
        )
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
                domain_code: r.get(6)?,
                primary_outcome_id: r.get(7)?,
                lifecycle_status: r.get(8)?,
                scope_mode: r.get(9)?,
                realm: r.get(10)?,
                priority_weight: r.get(11)?,
                pause_resume_note: r.get(12)?,
                parent_id: r.get(13)?,
                color: r.get(14)?,
                daily_quota_minutes: r.get(15)?,
                weekly_quota_minutes: r.get(16)?,
                monthly_quota_minutes: r.get(17)?,
                hourly_rate: r.get(18)?,
                google_calendar_id: r.get(19)?,
                created_at: r.get(20)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

/// ادغام اتمیک دو پروژه داخل دیتابیس Mab-Compass بدون از دست رفتن هیچ تسک، چت، متد، سند یا حافظه‌ای
pub fn merge_compass_projects_atomic(
    conn: &mut Connection,
    source_local_id: &str,
    target_local_id: &str,
) -> Result<(), String> {
    if source_local_id == target_local_id {
        return Ok(());
    }
    let now = now_utc_iso();
    let source_was_active: bool = conn
        .query_row(
            "SELECT is_active FROM projects WHERE id = ?1",
            params![source_local_id],
            |r| r.get::<_, i32>(0),
        )
        .map(|v| v == 1)
        .unwrap_or(false);

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let tables = [
        "work_entities",
        "work_lines",
        "outcomes",
        "sop_rules",
        "learned_antipatterns",
        "session_work_logs",
        "chat_sessions",
        "chat_messages",
        "knowledge_documents",
        "pending_ai_queue",
        "personal_vault_items",
        "methods_playbooks",
        "raw_events",
        "mab_focus_synced_notes",
        "mutation_audit_logs",
        "pending_proposals",
        "legacy_reprocess_history",
        "calendar_events",
    ];

    for table in tables {
        let sql = format!("UPDATE {} SET project_id = ?1 WHERE project_id = ?2", table);
        let _ = tx.execute(&sql, params![target_local_id, source_local_id]);
    }

    let _ = tx.execute(
        "UPDATE projects SET parent_id = ?1, updated_at = ?2 WHERE parent_id = ?3",
        params![target_local_id, now, source_local_id],
    );

    if source_was_active {
        let _ = tx.execute(
            "UPDATE projects SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END",
            params![target_local_id],
        );
    }

    tx.execute("DELETE FROM projects WHERE id = ?1", params![source_local_id])
        .map_err(|e| e.to_string())?;

    let _ = tx.execute(
        "INSERT INTO raw_events_fts(raw_events_fts) VALUES('rebuild')",
        [],
    );

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn ensure_active_project_exists(conn: &Connection) {
    let active_cnt: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects WHERE is_active = 1", [], |r| r.get(0))
        .unwrap_or(0);
    if active_cnt == 0 {
        let _ = conn.execute(
            "UPDATE projects SET is_active = 1 WHERE id = (SELECT id FROM projects ORDER BY created_at ASC, id ASC LIMIT 1)",
            [],
        );
    }
}

pub fn reconcile_compass_with_core_at_path(
    conn: &mut Connection,
    core_path: &Path,
    vault_root: &str,
) -> Result<CoreSyncStats, String> {
    let core = open_core_db(core_path)?;
    let mut stats = CoreSyncStats::default();
    let now = now_utc_iso();

    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS applied_core_merges (
            merge_id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    );

    let _ = core.execute(
        "INSERT OR REPLACE INTO core_app_registry (app_id, app_name, db_path, last_seen_at, schema_version)
         VALUES ('mab_compass', 'Mab-Compass', ?1, ?2, 4)",
        params![vault_root, now],
    );

    let mut core_changed = false;

    // Load aliases
    let mut alias_to_core: HashMap<String, String> = HashMap::new();
    if let Ok(mut stmt) = core.prepare(
        "SELECT local_project_id, core_project_id FROM core_project_aliases WHERE app_name = 'mab_compass'",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))) {
            for (loc_id, core_id) in rows.filter_map(|r| r.ok()) {
                alias_to_core.insert(loc_id, core_id);
            }
        }
    }

    // ۱. اعمال ادغام‌های ثبت‌شده در مرکز که هنوز در Mab-Compass اجرا نشده‌اند
    let applied_merges: HashSet<String> = {
        let mut s = HashSet::new();
        if let Ok(mut stmt) = conn.prepare("SELECT merge_id FROM applied_core_merges") {
            if let Ok(rows) = stmt.query_map([], |r| r.get::<_, String>(0)) {
                for id in rows.filter_map(|r| r.ok()) {
                    s.insert(id);
                }
            }
        }
        s
    };

    let core_merges: Vec<(String, String, String)> = {
        let mut out = Vec::new();
        if let Ok(mut stmt) = core.prepare(
            "SELECT id, source_project_id, target_project_id FROM core_project_merges ORDER BY merged_at ASC",
        ) {
            if let Ok(rows) = stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
            }) {
                for item in rows.filter_map(|r| r.ok()) {
                    out.push(item);
                }
            }
        }
        out
    };

    let merged_source_core_ids: HashSet<String> =
        core_merges.iter().map(|(_, s, _)| s.clone()).collect();

    for (merge_id, src_core_id, tgt_core_id) in &core_merges {
        if applied_merges.contains(merge_id) {
            continue;
        }
        let local_projects = load_compass_projects(conn)?;
        let src_local = local_projects.iter().find(|p| {
            p.external_focus_id.as_deref() == Some(src_core_id.as_str())
                || p.id == *src_core_id
                || alias_to_core.get(&p.id).map(String::as_str) == Some(src_core_id.as_str())
        });

        if let Some(src_p) = src_local {
            let mut tgt_local_id = local_projects
                .iter()
                .find(|p| {
                    p.external_focus_id.as_deref() == Some(tgt_core_id.as_str())
                        || p.id == *tgt_core_id
                        || alias_to_core.get(&p.id).map(String::as_str) == Some(tgt_core_id.as_str())
                })
                .map(|p| p.id.clone());

            if tgt_local_id.is_none() {
                // ساخت پروژه مقصد از روی core_projects پیش از ادغام
                let core_projs = load_core_projects(&core)?;
                if let Some(cp) = core_projs.iter().find(|c| c.id == *tgt_core_id) {
                    let mut used_ids: HashSet<String> =
                        local_projects.iter().map(|p| p.id.clone()).collect();
                    let new_cid = derive_unique_compass_id(&cp.code, &cp.name, &mut used_ids);
                    let wpath = if !cp.workspace_path.trim().is_empty() {
                        cp.workspace_path.clone()
                    } else {
                        Path::new(vault_root).join(&new_cid).to_string_lossy().to_string()
                    };
                    let _ = ensure_project_directories(&wpath);
                    let _ = conn.execute(
                        "INSERT INTO projects (
                            id, name, workspace_path, external_focus_id, is_active, updated_at,
                            domain_code, primary_outcome_id, lifecycle_status, scope_mode, realm,
                            priority_weight, pause_resume_note, parent_id, color,
                            daily_quota_minutes, weekly_quota_minutes, monthly_quota_minutes,
                            hourly_rate, google_calendar_id, created_at
                        ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                        params![
                            new_cid,
                            cp.name,
                            wpath,
                            cp.id,
                            cp.updated_at,
                            cp.domain_code,
                            cp.primary_outcome_id,
                            map_core_status_to_compass(&cp.status),
                            cp.scope_mode,
                            map_core_realm_to_compass(&cp.realm),
                            cp.priority_weight,
                            cp.pause_resume_note,
                            cp.color,
                            cp.daily_quota_minutes,
                            cp.weekly_quota_minutes,
                            cp.monthly_quota_minutes,
                            cp.hourly_rate,
                            cp.google_calendar_id,
                            cp.created_at,
                        ],
                    );
                    tgt_local_id = Some(new_cid);
                    stats.imported_projects += 1;
                }
            }

            if let Some(tgt_id) = tgt_local_id {
                merge_compass_projects_atomic(conn, &src_p.id, &tgt_id)?;
                stats.applied_merges += 1;
            }
        }

        let _ = conn.execute(
            "INSERT OR IGNORE INTO applied_core_merges (merge_id, applied_at) VALUES (?1, ?2)",
            params![merge_id, now],
        );
    }

    // ۲. تطبیق و همگام‌سازی پروژه‌های مرکزی به Mab-Compass
    let mut local_projects = load_compass_projects(conn)?;
    let mut used_ids: HashSet<String> = local_projects.iter().map(|p| p.id.clone()).collect();
    let mut core_projs = load_core_projects(&core)?;

    // ساخت نقشه core_id -> local_id برای پروژه‌های موجود
    let mut core_to_local_id: HashMap<String, String> = HashMap::new();
    for cp in &core_projs {
        if let Some(lp) = local_projects.iter().find(|p| {
            p.external_focus_id.as_deref() == Some(cp.id.as_str())
                || alias_to_core.get(&p.id).map(String::as_str) == Some(cp.id.as_str())
                || (!cp.code.trim().is_empty() && p.id.eq_ignore_ascii_case(cp.code.trim()))
                || p.name.trim().eq_ignore_ascii_case(cp.name.trim())
        }) {
            core_to_local_id.insert(cp.id.clone(), lp.id.clone());
        }
    }

    // الف) حذف پروژه‌های حذف‌شده در مرکز و ایمپورت پروژه‌های جدید (ریشه و زیرپروژه)
    for cp in &core_projs {
        let is_deleted = cp.status == "deleted"
            || cp.status == "merged"
            || cp.deleted_at.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);

        if is_deleted {
            if let Some(local_id) = core_to_local_id.get(&cp.id) {
                let _ = conn.execute(
                    "UPDATE projects SET parent_id = NULL WHERE parent_id = ?1",
                    params![local_id],
                );
                let _ = conn.execute("DELETE FROM projects WHERE id = ?1", params![local_id]);
                stats.deleted_projects += 1;
            }
            continue;
        }

        if let Some(local_id) = core_to_local_id.get(&cp.id).cloned() {
            if let Some(lp) = local_projects.iter().find(|p| p.id == local_id) {
                if lp.external_focus_id.as_deref() != Some(cp.id.as_str()) {
                    let _ = conn.execute(
                        "UPDATE projects SET external_focus_id = ?1 WHERE id = ?2",
                        params![cp.id, lp.id],
                    );
                }
                let _ = core.execute(
                    "INSERT OR REPLACE INTO core_project_aliases (app_name, local_project_id, core_project_id)
                     VALUES ('mab_compass', ?1, ?2)",
                    params![lp.id, cp.id],
                );
            }
        } else {
            let new_cid = derive_unique_compass_id(&cp.code, &cp.name, &mut used_ids);
            let wpath = if !cp.workspace_path.trim().is_empty() {
                cp.workspace_path.clone()
            } else {
                Path::new(vault_root).join(&new_cid).to_string_lossy().to_string()
            };
            let _ = ensure_project_directories(&wpath);
            let _ = conn.execute(
                "INSERT INTO projects (
                    id, name, workspace_path, external_focus_id, is_active, updated_at,
                    domain_code, primary_outcome_id, lifecycle_status, scope_mode, realm,
                    priority_weight, pause_resume_note, parent_id, color,
                    daily_quota_minutes, weekly_quota_minutes, monthly_quota_minutes,
                    hourly_rate, google_calendar_id, created_at
                ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    new_cid,
                    cp.name,
                    wpath,
                    cp.id,
                    cp.updated_at,
                    cp.domain_code,
                    cp.primary_outcome_id,
                    map_core_status_to_compass(&cp.status),
                    cp.scope_mode,
                    map_core_realm_to_compass(&cp.realm),
                    cp.priority_weight,
                    cp.pause_resume_note,
                    cp.color,
                    cp.daily_quota_minutes,
                    cp.weekly_quota_minutes,
                    cp.monthly_quota_minutes,
                    cp.hourly_rate,
                    cp.google_calendar_id,
                    cp.created_at,
                ],
            );
            let _ = core.execute(
                "INSERT OR REPLACE INTO core_project_aliases (app_name, local_project_id, core_project_id)
                 VALUES ('mab_compass', ?1, ?2)",
                params![new_cid, cp.id],
            );
            core_to_local_id.insert(cp.id.clone(), new_cid);
            stats.imported_projects += 1;
        }
    }

    // ب) ارسال پروژه‌های محلی کامپس که هنوز در مرکز ثبت نشده‌اند (و مرج هم نشده‌اند)
    local_projects = load_compass_projects(conn)?;
    let local_to_core_id: HashMap<String, String> = core_to_local_id
        .iter()
        .map(|(c, l)| (l.clone(), c.clone()))
        .collect();

    for lp in &local_projects {
        if local_to_core_id.contains_key(&lp.id) {
            continue;
        }
        let maybe_ext = lp
            .external_focus_id
            .as_deref()
            .filter(|s| uuid::Uuid::parse_str(s).is_ok());
        if let Some(ext) = maybe_ext {
            if merged_source_core_ids.contains(ext) {
                continue;
            }
        }
        let new_core_id = maybe_ext
            .map(str::to_string)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let parent_core_id = lp
            .parent_id
            .as_deref()
            .and_then(|pid| local_to_core_id.get(pid).cloned());
        let created = if lp.created_at.trim().is_empty() {
            now.clone()
        } else {
            lp.created_at.clone()
        };
        let updated = if lp.updated_at.trim().is_empty() {
            now.clone()
        } else {
            lp.updated_at.clone()
        };

        let _ = core.execute(
            "INSERT OR IGNORE INTO core_projects (
                id, code, name, parent_id, realm, status, merged_into_id, color,
                daily_quota_minutes, weekly_quota_minutes, monthly_quota_minutes,
                hourly_rate, google_calendar_id, domain_code, primary_outcome_id,
                scope_mode, priority_weight, pause_resume_note, workspace_path,
                created_at, updated_at, deleted_at, updated_by_app
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, NULL, 'mab_compass')",
            params![
                new_core_id,
                lp.id,
                lp.name,
                parent_core_id,
                map_compass_realm_to_core(&lp.realm),
                map_compass_status_to_core(&lp.lifecycle_status),
                lp.color,
                lp.daily_quota_minutes,
                lp.weekly_quota_minutes,
                lp.monthly_quota_minutes,
                lp.hourly_rate,
                lp.google_calendar_id,
                lp.domain_code,
                lp.primary_outcome_id,
                lp.scope_mode,
                lp.priority_weight,
                lp.pause_resume_note,
                lp.workspace_path,
                created,
                updated,
            ],
        );
        let _ = core.execute(
            "INSERT OR REPLACE INTO core_project_aliases (app_name, local_project_id, core_project_id)
             VALUES ('mab_compass', ?1, ?2)",
            params![lp.id, new_core_id],
        );
        let _ = conn.execute(
            "UPDATE projects SET external_focus_id = ?1 WHERE id = ?2",
            params![new_core_id, lp.id],
        );
        core_to_local_id.insert(lp.id.clone(), new_core_id);
        stats.exported_projects += 1;
        core_changed = true;
    }

    // ج) بروزرسانی دوطرفه فیلدها و روابط والد-فرزند (parent_id) بر اساس آخرین timestamp
    core_projs = load_core_projects(&core)?;
    local_projects = load_compass_projects(conn)?;
    for cp in &core_projs {
        let is_deleted = cp.status == "deleted"
            || cp.status == "merged"
            || cp.deleted_at.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        if is_deleted {
            continue;
        }
        let Some(local_id) = core_to_local_id.get(&cp.id) else {
            continue;
        };
        let Some(lp) = local_projects.iter().find(|p| p.id == *local_id) else {
            continue;
        };

        let expected_local_parent = cp
            .parent_id
            .as_deref()
            .and_then(|pid| core_to_local_id.get(pid).cloned());
        let expected_status = map_core_status_to_compass(&cp.status);
        let expected_realm = map_core_realm_to_compass(&cp.realm);

        let core_ts = parse_ts_ms(&cp.updated_at);
        let local_ts = parse_ts_ms(&lp.updated_at);

        let fields_differ = lp.name != cp.name
            || lp.parent_id != expected_local_parent
            || lp.lifecycle_status != expected_status
            || lp.realm != expected_realm
            || lp.color != cp.color
            || lp.daily_quota_minutes != cp.daily_quota_minutes
            || lp.weekly_quota_minutes != cp.weekly_quota_minutes
            || lp.monthly_quota_minutes != cp.monthly_quota_minutes
            || (lp.hourly_rate - cp.hourly_rate).abs() > 0.01
            || lp.google_calendar_id != cp.google_calendar_id;

        if fields_differ {
            if core_ts >= local_ts {
                let _ = conn.execute(
                    "UPDATE projects SET
                        name = ?1, parent_id = ?2, lifecycle_status = ?3, realm = ?4,
                        color = ?5, daily_quota_minutes = ?6, weekly_quota_minutes = ?7,
                        monthly_quota_minutes = ?8, hourly_rate = ?9, google_calendar_id = ?10,
                        updated_at = ?11
                     WHERE id = ?12",
                    params![
                        cp.name,
                        expected_local_parent,
                        expected_status,
                        expected_realm,
                        cp.color,
                        cp.daily_quota_minutes,
                        cp.weekly_quota_minutes,
                        cp.monthly_quota_minutes,
                        cp.hourly_rate,
                        cp.google_calendar_id,
                        cp.updated_at,
                        lp.id,
                    ],
                );
                stats.updated_projects += 1;
            } else {
                let local_to_core: HashMap<String, String> = core_to_local_id
                    .iter()
                    .map(|(c, l)| (l.clone(), c.clone()))
                    .collect();
                let parent_core = lp
                    .parent_id
                    .as_deref()
                    .and_then(|pid| local_to_core.get(pid).cloned());
                let _ = core.execute(
                    "UPDATE core_projects SET
                        code = ?1, name = ?2, parent_id = ?3, realm = ?4, status = ?5,
                        color = ?6, daily_quota_minutes = ?7, weekly_quota_minutes = ?8,
                        monthly_quota_minutes = ?9, hourly_rate = ?10, google_calendar_id = ?11,
                        domain_code = ?12, primary_outcome_id = ?13, scope_mode = ?14,
                        priority_weight = ?15, pause_resume_note = ?16, workspace_path = ?17,
                        updated_at = ?18, updated_by_app = 'mab_compass'
                     WHERE id = ?19",
                    params![
                        lp.id,
                        lp.name,
                        parent_core,
                        map_compass_realm_to_core(&lp.realm),
                        map_compass_status_to_core(&lp.lifecycle_status),
                        lp.color,
                        lp.daily_quota_minutes,
                        lp.weekly_quota_minutes,
                        lp.monthly_quota_minutes,
                        lp.hourly_rate,
                        lp.google_calendar_id,
                        lp.domain_code,
                        lp.primary_outcome_id,
                        lp.scope_mode,
                        lp.priority_weight,
                        lp.pause_resume_note,
                        lp.workspace_path,
                        lp.updated_at,
                        cp.id,
                    ],
                );
                core_changed = true;
            }
        }
    }

    ensure_active_project_exists(conn);

    // ۳. آینه‌سازی داده‌های کلان کامپس در هسته مرکزی (برای اکوسیستم یکپارچه)
    let local_to_core_map: HashMap<String, String> = core_to_local_id
        .into_iter()
        .map(|(c, l)| (l, c))
        .collect();
    mirror_compass_entities_to_core(conn, &core, &local_to_core_map, &now);

    if core_changed {
        bump_core_revision(&core);
    }
    stats.revision = get_core_revision_from_conn(&core);

    Ok(stats)
}

fn mirror_compass_entities_to_core(
    conn: &Connection,
    core: &Connection,
    local_to_core: &HashMap<String, String>,
    now: &str,
) {
    // Mirror work_entities -> core_work_items
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, project_id, entity_type, category, title, status, definition_of_done, first_physical_action, updated_at
         FROM work_entities",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
            ))
        }) {
            for (id, pid, etype, cat, title, status, dod, fpa, updated_at) in
                rows.filter_map(|r| r.ok())
            {
                let core_pid = local_to_core
                    .get(&pid)
                    .cloned()
                    .unwrap_or(pid);
                let _ = core.execute(
                    "INSERT OR REPLACE INTO core_work_items (
                        id, core_project_id, entity_type, category, title, status,
                        definition_of_done, first_physical_action, source_app, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'mab_compass', ?9)",
                    params![id, core_pid, etype, cat, title, status, dod, fpa, updated_at],
                );
            }
        }
    }

    // Mirror personal_vault_items -> core_notes
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, project_id, realm, item_type, title, content_markdown, tags_json, created_at, updated_at
         FROM personal_vault_items",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
            ))
        }) {
            for (id, pid_opt, realm, itype, title, content, tags_json, created_at, updated_at) in
                rows.filter_map(|r| r.ok())
            {
                let core_pid = pid_opt.map(|p| local_to_core.get(&p).cloned().unwrap_or(p));
                let _ = core.execute(
                    "INSERT OR REPLACE INTO core_notes (
                        id, core_project_id, outcome_id, time_record_id, realm, note_type,
                        title, content, tags_json, source_app, created_at, updated_at
                     ) VALUES (?1, ?2, NULL, NULL, ?3, ?4, ?5, ?6, ?7, 'mab_compass', ?8, ?9)",
                    params![
                        id,
                        core_pid,
                        map_compass_realm_to_core(&realm),
                        itype,
                        title,
                        content,
                        tags_json,
                        created_at,
                        if updated_at.is_empty() { now } else { updated_at.as_str() },
                    ],
                );
            }
        }
    }
}

/// انتشار آنی ایجاد یا ویرایش یک پروژه از Mab-Compass به دیتابیس مرکزی
pub fn push_compass_project_upsert(conn: &Connection, core_path: &Path, proj: &Project) -> Result<(), String> {
    let core = open_core_db(core_path)?;
    let now = now_utc_iso();
    let core_id = proj
        .external_focus_id
        .as_deref()
        .filter(|s| uuid::Uuid::parse_str(s).is_ok())
        .map(str::to_string)
        .unwrap_or_else(|| {
            let new_id = uuid::Uuid::new_v4().to_string();
            let _ = conn.execute(
                "UPDATE projects SET external_focus_id = ?1 WHERE id = ?2",
                params![new_id, proj.id],
            );
            new_id
        });

    let parent_core_id: Option<String> = proj.parent_id.as_deref().and_then(|local_pid| {
        conn.query_row(
            "SELECT external_focus_id FROM projects WHERE id = ?1",
            params![local_pid],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| Some(local_pid.to_string()))
    });

    let created = if proj.created_at.trim().is_empty() {
        now.clone()
    } else {
        proj.created_at.clone()
    };
    let updated = if proj.updated_at.trim().is_empty() {
        now.clone()
    } else {
        proj.updated_at.clone()
    };

    core.execute(
        "INSERT OR REPLACE INTO core_projects (
            id, code, name, parent_id, realm, status, merged_into_id, color,
            daily_quota_minutes, weekly_quota_minutes, monthly_quota_minutes,
            hourly_rate, google_calendar_id, domain_code, primary_outcome_id,
            scope_mode, priority_weight, pause_resume_note, workspace_path,
            created_at, updated_at, deleted_at, updated_by_app
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, NULL, 'mab_compass')",
        params![
            core_id,
            proj.id,
            proj.name,
            parent_core_id,
            map_compass_realm_to_core(&proj.realm),
            map_compass_status_to_core(&proj.lifecycle_status),
            proj.color,
            proj.daily_quota_minutes,
            proj.weekly_quota_minutes,
            proj.monthly_quota_minutes,
            proj.hourly_rate,
            proj.google_calendar_id,
            proj.domain_code,
            proj.primary_outcome_id,
            proj.scope_mode,
            proj.priority_weight,
            proj.pause_resume_note,
            proj.workspace_path,
            created,
            updated,
        ],
    )
    .map_err(|e| e.to_string())?;

    let _ = core.execute(
        "INSERT OR REPLACE INTO core_project_aliases (app_name, local_project_id, core_project_id)
         VALUES ('mab_compass', ?1, ?2)",
        params![proj.id, core_id],
    );
    let _ = core.execute(
        "INSERT INTO core_activity_events (source_app, event_type, core_project_id, payload_json, created_at)
         VALUES ('mab_compass', 'PROJECT_UPSERT', ?1, ?2, ?3)",
        params![
            core_id,
            json!({ "name": proj.name, "code": proj.id, "parent_id": parent_core_id }).to_string(),
            now,
        ],
    );
    bump_core_revision(&core);
    Ok(())
}

/// ثبت ادغام دو پروژه از Mab-Compass در دیتابیس مرکزی (برای همگام‌سازی خودکار به Mab-Focus)
pub fn push_compass_project_merge(
    conn: &Connection,
    core_path: &Path,
    source_core_id: &str,
    target_core_id: &str,
    source_code: &str,
    source_name: &str,
) -> Result<(), String> {
    let core = open_core_db(core_path)?;
    let now = now_utc_iso();
    let merge_id = uuid::Uuid::new_v4().to_string();

    core.execute(
        "INSERT INTO core_project_merges (id, source_project_id, target_project_id, source_code, source_name, merged_by_app, merged_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'mab_compass', ?6)",
        params![merge_id, source_core_id, target_core_id, source_code, source_name, now],
    )
    .map_err(|e| e.to_string())?;

    let _ = core.execute(
        "UPDATE core_projects SET parent_id = ?1, updated_at = ?2, updated_by_app = 'mab_compass' WHERE parent_id = ?3",
        params![target_core_id, now, source_core_id],
    );
    let _ = core.execute(
        "UPDATE core_projects SET status = 'merged', merged_into_id = ?1, deleted_at = ?2, updated_at = ?2, updated_by_app = 'mab_compass' WHERE id = ?3",
        params![target_core_id, now, source_core_id],
    );

    for table in ["core_outcomes", "core_time_logs", "core_notes", "core_work_items"] {
        let sql = format!("UPDATE {} SET core_project_id = ?1 WHERE core_project_id = ?2", table);
        let _ = core.execute(&sql, params![target_core_id, source_core_id]);
    }
    let _ = core.execute(
        "UPDATE core_project_aliases SET core_project_id = ?1 WHERE core_project_id = ?2",
        params![target_core_id, source_core_id],
    );
    let _ = core.execute(
        "INSERT INTO core_activity_events (source_app, event_type, core_project_id, payload_json, created_at)
         VALUES ('mab_compass', 'PROJECT_MERGE', ?1, ?2, ?3)",
        params![
            target_core_id,
            json!({ "merge_id": merge_id, "source_id": source_core_id, "target_id": target_core_id }).to_string(),
            now,
        ],
    );
    bump_core_revision(&core);

    let _ = conn.execute(
        "INSERT OR IGNORE INTO applied_core_merges (merge_id, applied_at) VALUES (?1, ?2)",
        params![merge_id, now],
    );
    Ok(())
}

/// ثبت حذف پروژه از Mab-Compass در دیتابیس مرکزی
pub fn push_compass_project_delete(core_path: &Path, deleted_core_ids: &[String]) -> Result<(), String> {
    if deleted_core_ids.is_empty() {
        return Ok(());
    }
    let core = open_core_db(core_path)?;
    let now = now_utc_iso();
    for cid in deleted_core_ids {
        let _ = core.execute(
            "UPDATE core_projects SET status = 'deleted', deleted_at = ?1, updated_at = ?1, updated_by_app = 'mab_compass' WHERE id = ?2",
            params![now, cid],
        );
        let _ = core.execute(
            "INSERT INTO core_activity_events (source_app, event_type, core_project_id, payload_json, created_at)
             VALUES ('mab_compass', 'PROJECT_DELETE', ?1, ?2, ?3)",
            params![cid, json!({ "deleted_id": cid }).to_string(), now],
        );
    }
    bump_core_revision(&core);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::{create_new_project_with_options, delete_project_local, update_project_details};
    use crate::db::schema::init_database;

    #[test]
    fn test_unified_core_tree_sync_edit_delete_and_merge() {
        let root = std::env::temp_dir().join(format!("mab_core_test_{}", uuid::Uuid::new_v4()));
        let vault = root.join("vault");
        fs::create_dir_all(&vault).unwrap();
        let compass_db = root.join("compass.sqlite");
        let core_db = root.join("mab_core.db");

        let mut conn = init_database(&compass_db, &vault).unwrap();
        let core = open_core_db(&core_db).unwrap();
        let now = now_utc_iso();

        let root_uuid = uuid::Uuid::new_v4().to_string();
        let sub_uuid = uuid::Uuid::new_v4().to_string();
        let old_uuid = uuid::Uuid::new_v4().to_string();

        // شبیه‌سازی ایجاد درخت پروژه توسط Mab-Focus در دیتابیس مرکزی
        core.execute(
            "INSERT INTO core_projects (id, code, name, parent_id, realm, status, color, hourly_rate, created_at, updated_at, updated_by_app)
             VALUES (?1, '', 'هولدینگ مرکزی', NULL, 'work', 'active', '#8B5CF6', 750000.0, ?2, ?2, 'mab_focus')",
            params![root_uuid, now],
        ).unwrap();
        core.execute(
            "INSERT INTO core_projects (id, code, name, parent_id, realm, status, color, hourly_rate, created_at, updated_at, updated_by_app)
             VALUES (?1, '', 'اپلیکیشن موبایل', ?2, 'work', 'active', '#10B981', 0.0, ?3, ?3, 'mab_focus')",
            params![sub_uuid, root_uuid, now],
        ).unwrap();
        core.execute(
            "INSERT INTO core_projects (id, code, name, parent_id, realm, status, color, hourly_rate, created_at, updated_at, updated_by_app)
             VALUES (?1, '', 'پروژه قدیمی برای ادغام', NULL, 'work', 'active', '#F59E0B', 0.0, ?2, ?2, 'mab_focus')",
            params![old_uuid, now],
        ).unwrap();
        drop(core);

        // ۱. آشتی اولیه کامپس با هسته مرکزی -> ایمپورت ریشه، زیرپروژه، و اکسپورت پروژه‌های کامپس
        let stats1 = reconcile_compass_with_core_at_path(&mut conn, &core_db, vault.to_str().unwrap()).unwrap();
        assert_eq!(stats1.imported_projects, 3);
        assert_eq!(stats1.exported_projects, 2); // GERDALO + IRANOIL

        let projs1 = load_compass_projects(&conn).unwrap();
        let root_c = projs1.iter().find(|p| p.external_focus_id.as_deref() == Some(root_uuid.as_str())).unwrap().clone();
        let sub_c = projs1.iter().find(|p| p.external_focus_id.as_deref() == Some(sub_uuid.as_str())).unwrap().clone();
        let old_c = projs1.iter().find(|p| p.external_focus_id.as_deref() == Some(old_uuid.as_str())).unwrap().clone();
        assert_eq!(sub_c.parent_id.as_deref(), Some(root_c.id.as_str()));
        assert_eq!(root_c.color, "#8B5CF6");
        assert!((root_c.hourly_rate - 750000.0).abs() < 0.01);

        // ثبت داده داخل پروژه قدیمی در کامپس پیش از ادغام
        conn.execute(
            "INSERT INTO work_entities (id, project_id, entity_type, category, title, updated_at)
             VALUES ('TSK-OLD1', ?1, 'TASK', 'BACKEND', 'تسک پروژه قدیمی', ?2)",
            params![old_c.id, now],
        ).unwrap();
        conn.execute(
            "INSERT INTO raw_events (project_id, raw_content, source_type, created_at)
             VALUES (?1, 'نکته مهم پروژه قدیمی', 'USER_CHAT', ?2)",
            params![old_c.id, now],
        ).unwrap();

        // ۲. شبیه‌سازی ادغام old_uuid -> root_uuid از سمت Mab-Focus در هسته مرکزی
        let core2 = open_core_db(&core_db).unwrap();
        let merge_id = uuid::Uuid::new_v4().to_string();
        let later = (Utc::now() + chrono::Duration::seconds(5)).to_rfc3339();
        core2.execute(
            "INSERT INTO core_project_merges (id, source_project_id, target_project_id, source_name, merged_by_app, merged_at)
             VALUES (?1, ?2, ?3, 'پروژه قدیمی برای ادغام', 'mab_focus', ?4)",
            params![merge_id, old_uuid, root_uuid, later],
        ).unwrap();
        core2.execute(
            "UPDATE core_projects SET status = 'merged', merged_into_id = ?1, deleted_at = ?2, updated_at = ?2 WHERE id = ?3",
            params![root_uuid, later, old_uuid],
        ).unwrap();
        bump_core_revision(&core2);
        drop(core2);

        // ۳. همگام‌سازی مجدد کامپس -> ادغام خودکار در کامپس و انتقال تمام داده‌ها بدون تلفات
        let stats2 = reconcile_compass_with_core_at_path(&mut conn, &core_db, vault.to_str().unwrap()).unwrap();
        assert_eq!(stats2.applied_merges, 1);

        let projs2 = load_compass_projects(&conn).unwrap();
        assert!(!projs2.iter().any(|p| p.id == old_c.id));
        let task_pid: String = conn.query_row(
            "SELECT project_id FROM work_entities WHERE id = 'TSK-OLD1'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(task_pid, root_c.id);

        // ۴. ایجاد زیرپروژه، ویرایش، و حذف از سمت کامپس -> انتشار در هسته مرکزی
        let created_sub = create_new_project_with_options(
            &conn,
            "MOB_API",
            "سرویس API موبایل",
            Some(&sub_c.id),
            Some("WORK"),
            Some("#06B6D4"),
            Some("ACTIVE"),
        ).unwrap();
        push_compass_project_upsert(&conn, &core_db, &created_sub).unwrap();

        let updated_sub = update_project_details(
            &conn,
            &created_sub.id,
            Some("سرویس API موبایل نسخه ۲"),
            None,
            false,
            None,
            Some("PAUSED"),
            None,
            Some(90),
            None,
            None,
            None,
        ).unwrap();
        push_compass_project_upsert(&conn, &core_db, &updated_sub).unwrap();

        let core3 = open_core_db(&core_db).unwrap();
        let (core_name, core_status, core_quota): (String, String, i32) = core3.query_row(
            "SELECT name, status, daily_quota_minutes FROM core_projects WHERE id = ?1",
            params![updated_sub.external_focus_id.as_deref().unwrap()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ).unwrap();
        assert_eq!(core_name, "سرویس API موبایل نسخه ۲");
        assert_eq!(core_status, "suspended");
        assert_eq!(core_quota, 90);
        drop(core3);

        // تست حذف با Tombstone
        let deleted_cores = delete_project_local(&mut conn, &updated_sub.id).unwrap();
        push_compass_project_delete(&core_db, &deleted_cores).unwrap();
        let stats3 = reconcile_compass_with_core_at_path(&mut conn, &core_db, vault.to_str().unwrap()).unwrap();
        assert_eq!(stats3.imported_projects, 0); // پروژه حذف‌شده نباید دوباره برگردد

        drop(conn);
        let _ = fs::remove_dir_all(root);
    }
}
