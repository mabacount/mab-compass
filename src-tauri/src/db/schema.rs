use crate::vault_manager::ensure_project_directories;
use rusqlite::{Connection, Result};
use std::path::Path;

pub fn init_database(db_path: &Path, default_vault_root: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA busy_timeout = 5000;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS app_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            vault_root_path TEXT NOT NULL,
            mab_focus_data_path TEXT NOT NULL,
            auto_sync_mab_focus INTEGER NOT NULL DEFAULT 1,
            active_llm_provider TEXT NOT NULL DEFAULT 'AGY_CLI',
            agy_model TEXT NOT NULL DEFAULT 'Gemini 3.1 Pro (High)',
            gemini_model TEXT NOT NULL DEFAULT 'gemini-2.5-flash',
            local_model TEXT NOT NULL DEFAULT '',
            gemini_api_key TEXT NOT NULL DEFAULT '',
            local_endpoint TEXT NOT NULL DEFAULT 'http://127.0.0.1:1234/v1/chat/completions'
        );

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            workspace_path TEXT NOT NULL,
            external_focus_id TEXT,
            is_active INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS raw_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            raw_content TEXT NOT NULL,
            archived_path TEXT,
            source_type TEXT NOT NULL DEFAULT 'USER_CHAT',
            created_at TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS raw_events_fts USING fts5(
            project_id,
            raw_content,
            content='raw_events',
            content_rowid='id'
        );

        CREATE TRIGGER IF NOT EXISTS raw_events_ai AFTER INSERT ON raw_events BEGIN
            INSERT INTO raw_events_fts(rowid, project_id, raw_content)
            VALUES (new.id, new.project_id, new.raw_content);
        END;

        CREATE TABLE IF NOT EXISTS work_entities (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            category TEXT NOT NULL,
            title TEXT NOT NULL,
            definition_of_done TEXT NOT NULL DEFAULT '',
            first_physical_action TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'OPEN',
            is_on_critical_path INTEGER NOT NULL DEFAULT 0,
            optimistic_pomos REAL NOT NULL DEFAULT 1.0,
            most_likely_pomos REAL NOT NULL DEFAULT 2.0,
            pessimistic_pomos REAL NOT NULL DEFAULT 3.0,
            calibrated_pomos REAL NOT NULL DEFAULT 2.0,
            superseded_by_id TEXT,
            stakeholder_tag TEXT,
            source_event_id INTEGER,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sop_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            category TEXT NOT NULL,
            rule_statement TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS learned_antipatterns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            trigger_category TEXT NOT NULL,
            mistake_pattern TEXT NOT NULL,
            preventive_gate_question TEXT NOT NULL,
            severity_score INTEGER NOT NULL DEFAULT 5,
            times_triggered INTEGER NOT NULL DEFAULT 1,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS session_work_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            task_id TEXT,
            category TEXT NOT NULL,
            estimated_pomos REAL NOT NULL DEFAULT 2.0,
            effective_pomos INTEGER NOT NULL,
            deliverable_summary TEXT NOT NULL,
            internal_friction_note TEXT NOT NULL DEFAULT '',
            client_sanitized_note TEXT NOT NULL,
            source_origin TEXT NOT NULL DEFAULT 'USER_CHAT',
            logged_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chat_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            channel TEXT NOT NULL DEFAULT 'COPILOT',
            title TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            session_id TEXT NOT NULL DEFAULT '',
            sender TEXT NOT NULL,
            channel TEXT NOT NULL DEFAULT 'COPILOT',
            message_text TEXT NOT NULL,
            archived_file_path TEXT,
            applied_mutations_badge TEXT,
            clarifying_questions_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS knowledge_documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            doc_type TEXT NOT NULL,
            original_filename TEXT NOT NULL,
            archived_path TEXT NOT NULL,
            executive_summary TEXT NOT NULL,
            extracted_items_count INTEGER NOT NULL DEFAULT 0,
            ingested_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS mab_focus_synced_notes (
            fingerprint TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            raw_note TEXT NOT NULL,
            synced_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS mutation_audit_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            summary_text TEXT NOT NULL,
            snapshot_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS pending_ai_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id TEXT NOT NULL,
            session_id TEXT NOT NULL DEFAULT '',
            channel TEXT NOT NULL DEFAULT 'COPILOT',
            user_message_id INTEGER,
            system_message_id INTEGER,
            knowledge_doc_id INTEGER,
            raw_event_id INTEGER,
            title_preview TEXT NOT NULL DEFAULT '',
            source_type TEXT NOT NULL DEFAULT 'USER_CHAT',
            archived_path TEXT,
            status TEXT NOT NULL DEFAULT 'PENDING',
            attempts INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_pending_ai_queue_user_msg
            ON pending_ai_queue(user_message_id) WHERE user_message_id IS NOT NULL;

        CREATE UNIQUE INDEX IF NOT EXISTS idx_pending_ai_queue_kdoc
            ON pending_ai_queue(knowledge_doc_id) WHERE knowledge_doc_id IS NOT NULL;
        ",
    )?;

    ensure_column_exists(
        &conn,
        "app_settings",
        "gemini_model",
        "ALTER TABLE app_settings ADD COLUMN gemini_model TEXT NOT NULL DEFAULT 'gemini-2.5-flash'",
    )?;
    ensure_column_exists(
        &conn,
        "app_settings",
        "local_model",
        "ALTER TABLE app_settings ADD COLUMN local_model TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column_exists(
        &conn,
        "chat_messages",
        "channel",
        "ALTER TABLE chat_messages ADD COLUMN channel TEXT NOT NULL DEFAULT 'COPILOT'",
    )?;
    ensure_column_exists(
        &conn,
        "chat_messages",
        "clarifying_questions_json",
        "ALTER TABLE chat_messages ADD COLUMN clarifying_questions_json TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column_exists(
        &conn,
        "chat_messages",
        "session_id",
        "ALTER TABLE chat_messages ADD COLUMN session_id TEXT NOT NULL DEFAULT ''",
    )?;

    let vault_str = default_vault_root.to_string_lossy().to_string();
    let legacy_focus_dir = dirs::document_dir()
        .unwrap_or_else(|| default_vault_root.to_path_buf())
        .join("MabFocus");
    let appdata_focus_dir = dirs::data_dir()
        .map(|d| d.join("com.example").join("mab_focus").join("MabFocus"))
        .unwrap_or_else(|| legacy_focus_dir.clone());

    let default_focus_dir = appdata_focus_dir.to_string_lossy().to_string();

    conn.execute(
        "INSERT OR IGNORE INTO app_settings (id, vault_root_path, mab_focus_data_path) VALUES (1, ?1, ?2)",
        rusqlite::params![vault_str, default_focus_dir],
    )?;

    // Migrate legacy Documents/MabFocus default path to real %APPDATA%/com.example/mab_focus/MabFocus when present
    if let Ok(cur_focus_path) = conn.query_row(
        "SELECT mab_focus_data_path FROM app_settings WHERE id = 1",
        [],
        |r| r.get::<_, String>(0),
    ) {
        let cur_p = Path::new(&cur_focus_path);
        let is_legacy_default = cur_p == legacy_focus_dir.as_path();
        let legacy_has_data = legacy_focus_dir.join("mab_focus_v3.db").exists()
            || legacy_focus_dir.join("mab_focus.db").exists()
            || legacy_focus_dir.join("mab_ecosystem_sync.json").exists();
        let appdata_has_db = appdata_focus_dir.join("mab_focus_v3.db").exists()
            || appdata_focus_dir.join("mab_focus.db").exists();
        if is_legacy_default && !legacy_has_data && appdata_has_db {
            let _ = conn.execute(
                "UPDATE app_settings SET mab_focus_data_path = ?1 WHERE id = 1",
                rusqlite::params![default_focus_dir],
            );
        }
    }

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;
    if count == 0 {
        let now = chrono::Local::now().to_rfc3339();
        let p1_path = default_vault_root.join("GERDALO").to_string_lossy().to_string();
        let p2_path = default_vault_root.join("IRANOIL").to_string_lossy().to_string();

        let _ = ensure_project_directories(&p1_path);
        let _ = ensure_project_directories(&p2_path);

        conn.execute(
            "INSERT INTO projects (id, name, workspace_path, external_focus_id, is_active, updated_at)
             VALUES ('GERDALO', 'پروژه گردالو (Gerdalo)', ?1, 'GERDALO', 1, ?2)",
            rusqlite::params![p1_path, now],
        )?;
        conn.execute(
            "INSERT INTO projects (id, name, workspace_path, external_focus_id, is_active, updated_at)
             VALUES ('IRANOIL', 'پروژه ایران‌اویل‌جاب / زیگهوش', ?1, 'IRANOIL', 0, ?2)",
            rusqlite::params![p2_path, now],
        )?;
    }

    migrate_orphan_chat_messages(&conn)?;
    crate::db::queries::sync_unprocessed_items_into_queue(&conn)?;

    Ok(conn)
}

fn migrate_orphan_chat_messages(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT project_id, CASE WHEN channel = 'VAULT' THEN 'VAULT' ELSE 'COPILOT' END AS norm_ch
         FROM chat_messages WHERE COALESCE(session_id, '') = ''",
    )?;
    let groups: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    if groups.is_empty() {
        return Ok(());
    }

    let now = chrono::Local::now().to_rfc3339();
    for (pid, norm_ch) in groups {
        let existing_active: Option<String> = conn
            .query_row(
                "SELECT id FROM chat_sessions WHERE project_id = ?1 AND channel = ?2 AND is_active = 1 ORDER BY updated_at DESC LIMIT 1",
                rusqlite::params![pid, norm_ch],
                |r| r.get(0),
            )
            .ok();

        let target_session_id = if let Some(sid) = existing_active {
            sid
        } else {
            let sid = format!(
                "SES-{}",
                &uuid::Uuid::new_v4().to_string()[..8].to_uppercase()
            );
            let first_user_msg: Option<String> = conn
                .query_row(
                    "SELECT message_text FROM chat_messages
                     WHERE project_id = ?1 AND sender = 'USER'
                     AND (CASE WHEN channel = 'VAULT' THEN 'VAULT' ELSE 'COPILOT' END) = ?2
                     ORDER BY id ASC LIMIT 1",
                    rusqlite::params![pid, norm_ch],
                    |r| r.get(0),
                )
                .ok();
            let title = first_user_msg
                .as_deref()
                .and_then(|txt| {
                    txt.lines()
                        .map(str::trim)
                        .find(|l| !l.is_empty())
                        .map(|l| {
                            let c: String = l.chars().take(40).collect();
                            if l.chars().count() > 40 {
                                format!("{}…", c.trim())
                            } else {
                                c
                            }
                        })
                })
                .unwrap_or_else(|| {
                    if norm_ch == "VAULT" {
                        "گفتگوی آرشیو دانش".to_string()
                    } else {
                        "گفتگوی اصلی پروژه".to_string()
                    }
                });

            conn.execute(
                "INSERT INTO chat_sessions (id, project_id, channel, title, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5)",
                rusqlite::params![sid, pid, norm_ch, title, now],
            )?;
            sid
        };

        if norm_ch == "VAULT" {
            conn.execute(
                "UPDATE chat_messages SET session_id = ?1 WHERE project_id = ?2 AND channel = 'VAULT' AND COALESCE(session_id, '') = ''",
                rusqlite::params![target_session_id, pid],
            )?;
        } else {
            conn.execute(
                "UPDATE chat_messages SET session_id = ?1 WHERE project_id = ?2 AND channel != 'VAULT' AND COALESCE(session_id, '') = ''",
                rusqlite::params![target_session_id, pid],
            )?;
        }
    }

    Ok(())
}

fn ensure_column_exists(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<()> {
    let pragma = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma)?;
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    if !cols.iter().any(|c| c == column) {
        conn.execute(alter_sql, [])?;
    }
    Ok(())
}
