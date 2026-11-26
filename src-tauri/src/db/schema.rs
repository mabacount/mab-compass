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
            agy_model TEXT NOT NULL DEFAULT 'gemini-3.8-flash-high',
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

        -- 1. Life & Work Durable Domains (8 CADM Domains)
        CREATE TABLE IF NOT EXISTS life_domains (
            code TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            maturity_state INTEGER NOT NULL DEFAULT 2,
            maintenance_floor TEXT NOT NULL DEFAULT '',
            desired_state TEXT NOT NULL DEFAULT '',
            is_growth_focus INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );

        -- 2. Desired Reality Outcomes (Growth Outcomes OUT-XXXX)
        CREATE TABLE IF NOT EXISTS outcomes (
            id TEXT PRIMARY KEY,
            domain_code TEXT NOT NULL,
            title TEXT NOT NULL,
            desired_reality_change TEXT NOT NULL DEFAULT '',
            evidence_kr TEXT NOT NULL DEFAULT '',
            time_window TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'ACTIVE',
            progress_percent INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_outcomes_domain ON outcomes(domain_code);

        -- 3. WorkLines & Sequential Chains (WLN-XXXX)
        CREATE TABLE IF NOT EXISTS work_lines (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            outcome_id TEXT,
            title TEXT NOT NULL,
            goal_statement TEXT NOT NULL DEFAULT '',
            line_mode TEXT NOT NULL DEFAULT 'SEQUENTIAL_CHAIN',
            status TEXT NOT NULL DEFAULT 'ACTIVE',
            color_hex TEXT NOT NULL DEFAULT '#3B82F6',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_work_lines_project ON work_lines(project_id);
        CREATE INDEX IF NOT EXISTS idx_work_lines_outcome ON work_lines(outcome_id);

        -- 4. Calendar Days Capacity Planner
        CREATE TABLE IF NOT EXISTS calendar_days (
            date_key TEXT PRIMARY KEY,
            total_capacity_minutes INTEGER NOT NULL DEFAULT 300,
            deep_capacity_minutes INTEGER NOT NULL DEFAULT 150,
            day_note TEXT NOT NULL DEFAULT '',
            is_off_day INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );

        -- 5. Human-in-the-Loop Lifecycle Proposals Gate (PRP-XXXX)
        CREATE TABLE IF NOT EXISTS pending_proposals (
            id TEXT PRIMARY KEY,
            project_id TEXT,
            proposal_type TEXT NOT NULL,
            title TEXT NOT NULL,
            rationale_markdown TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'PENDING',
            created_at TEXT NOT NULL,
            resolved_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_pending_proposals_status ON pending_proposals(status);

        -- 6. Personal Knowledge Vault Items (PVI-XXXX)
        CREATE TABLE IF NOT EXISTS personal_vault_items (
            id TEXT PRIMARY KEY,
            realm TEXT NOT NULL DEFAULT 'LIFE',
            domain_code TEXT NOT NULL DEFAULT 'MIND_LEARNING',
            project_id TEXT,
            item_type TEXT NOT NULL,
            title TEXT NOT NULL,
            content_markdown TEXT NOT NULL,
            tags_json TEXT NOT NULL DEFAULT '[]',
            archived_path TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_personal_vault_items_domain ON personal_vault_items(domain_code);
        CREATE INDEX IF NOT EXISTS idx_personal_vault_items_realm ON personal_vault_items(realm);

        -- 7. Methods, Playbooks & BPMN 2.0 Workflows (MTH-XXXX)
        CREATE TABLE IF NOT EXISTS methods_playbooks (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            slug TEXT NOT NULL,
            title TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT 'GENERAL',
            summary TEXT NOT NULL DEFAULT '',
            objective TEXT NOT NULL DEFAULT '',
            prerequisites_json TEXT NOT NULL DEFAULT '[]',
            steps_json TEXT NOT NULL DEFAULT '[]',
            checklist_json TEXT NOT NULL DEFAULT '[]',
            notes_markdown TEXT NOT NULL DEFAULT '',
            markdown_content TEXT NOT NULL DEFAULT '',
            bpmn_xml TEXT NOT NULL DEFAULT '',
            parent_method_id TEXT,
            parent_element_id TEXT,
            version INTEGER NOT NULL DEFAULT 1,
            archived_path TEXT NOT NULL DEFAULT '',
            bpmn_file_path TEXT NOT NULL DEFAULT '',
            source_refs_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_methods_playbooks_project ON methods_playbooks(project_id);
        CREATE INDEX IF NOT EXISTS idx_methods_playbooks_slug ON methods_playbooks(project_id, slug);

        -- 8. Legacy Memory Reprocessing Audit History
        CREATE TABLE IF NOT EXISTS legacy_reprocess_history (
            source_kind TEXT NOT NULL,
            source_id INTEGER NOT NULL,
            project_id TEXT NOT NULL,
            reprocessed_at TEXT NOT NULL,
            extracted_summary TEXT NOT NULL DEFAULT '',
            PRIMARY KEY (source_kind, source_id)
        );
        CREATE INDEX IF NOT EXISTS idx_legacy_reprocess_project ON legacy_reprocess_history(project_id);
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
        "app_settings",
        "active_chat_scope",
        "ALTER TABLE app_settings ADD COLUMN active_chat_scope TEXT NOT NULL DEFAULT 'PROJECT'",
    )?;
    ensure_column_exists(
        &conn,
        "app_settings",
        "chat_decoupled_v1",
        "ALTER TABLE app_settings ADD COLUMN chat_decoupled_v1 INTEGER NOT NULL DEFAULT 0",
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
    ensure_column_exists(
        &conn,
        "chat_messages",
        "written_artifacts_json",
        "ALTER TABLE chat_messages ADD COLUMN written_artifacts_json TEXT NOT NULL DEFAULT '[]'",
    )?;

    // Projects alterations
    ensure_column_exists(
        &conn,
        "projects",
        "domain_code",
        "ALTER TABLE projects ADD COLUMN domain_code TEXT NOT NULL DEFAULT 'WEALTH_FINANCE'",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "primary_outcome_id",
        "ALTER TABLE projects ADD COLUMN primary_outcome_id TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "lifecycle_status",
        "ALTER TABLE projects ADD COLUMN lifecycle_status TEXT NOT NULL DEFAULT 'ACTIVE'",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "scope_mode",
        "ALTER TABLE projects ADD COLUMN scope_mode TEXT NOT NULL DEFAULT 'STANDARD'",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "realm",
        "ALTER TABLE projects ADD COLUMN realm TEXT NOT NULL DEFAULT 'WORK'",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "priority_weight",
        "ALTER TABLE projects ADD COLUMN priority_weight INTEGER NOT NULL DEFAULT 5",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "pause_resume_note",
        "ALTER TABLE projects ADD COLUMN pause_resume_note TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "parent_id",
        "ALTER TABLE projects ADD COLUMN parent_id TEXT NULL",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "color",
        "ALTER TABLE projects ADD COLUMN color TEXT NOT NULL DEFAULT '#3B82F6'",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "daily_quota_minutes",
        "ALTER TABLE projects ADD COLUMN daily_quota_minutes INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "weekly_quota_minutes",
        "ALTER TABLE projects ADD COLUMN weekly_quota_minutes INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "monthly_quota_minutes",
        "ALTER TABLE projects ADD COLUMN monthly_quota_minutes INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "hourly_rate",
        "ALTER TABLE projects ADD COLUMN hourly_rate REAL NOT NULL DEFAULT 0.0",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "google_calendar_id",
        "ALTER TABLE projects ADD COLUMN google_calendar_id TEXT NULL",
    )?;
    ensure_column_exists(
        &conn,
        "projects",
        "created_at",
        "ALTER TABLE projects ADD COLUMN created_at TEXT NOT NULL DEFAULT ''",
    )?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS applied_core_merges (
            merge_id TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
        );
        ",
    )?;

    // Work entities alterations
    ensure_column_exists(
        &conn,
        "work_entities",
        "work_line_id",
        "ALTER TABLE work_entities ADD COLUMN work_line_id TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "outcome_id",
        "ALTER TABLE work_entities ADD COLUMN outcome_id TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "sequence_order",
        "ALTER TABLE work_entities ADD COLUMN sequence_order INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "depends_on_task_id",
        "ALTER TABLE work_entities ADD COLUMN depends_on_task_id TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "energy_tag",
        "ALTER TABLE work_entities ADD COLUMN energy_tag TEXT NOT NULL DEFAULT 'MEDIUM_NORMAL'",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "duration_minutes",
        "ALTER TABLE work_entities ADD COLUMN duration_minutes INTEGER NOT NULL DEFAULT 25",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "device_tag",
        "ALTER TABLE work_entities ADD COLUMN device_tag TEXT NOT NULL DEFAULT 'LAPTOP_ONLY'",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "location_tags_json",
        "ALTER TABLE work_entities ADD COLUMN location_tags_json TEXT NOT NULL DEFAULT '[\"DESK\"]'",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "scheduled_date",
        "ALTER TABLE work_entities ADD COLUMN scheduled_date TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "scheduled_slot",
        "ALTER TABLE work_entities ADD COLUMN scheduled_slot TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "due_date",
        "ALTER TABLE work_entities ADD COLUMN due_date TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "work_entities",
        "priority_score",
        "ALTER TABLE work_entities ADD COLUMN priority_score INTEGER NOT NULL DEFAULT 50",
    )?;

    // Knowledge documents alterations
    ensure_column_exists(
        &conn,
        "knowledge_documents",
        "binary_original_path",
        "ALTER TABLE knowledge_documents ADD COLUMN binary_original_path TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "knowledge_documents",
        "tags_json",
        "ALTER TABLE knowledge_documents ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column_exists(
        &conn,
        "knowledge_documents",
        "realm",
        "ALTER TABLE knowledge_documents ADD COLUMN realm TEXT NOT NULL DEFAULT 'WORK'",
    )?;
    ensure_column_exists(
        &conn,
        "knowledge_documents",
        "domain_code",
        "ALTER TABLE knowledge_documents ADD COLUMN domain_code TEXT NOT NULL DEFAULT 'WEALTH_FINANCE'",
    )?;
    ensure_column_exists(
        &conn,
        "knowledge_documents",
        "processing_status",
        "ALTER TABLE knowledge_documents ADD COLUMN processing_status TEXT NOT NULL DEFAULT 'PROCESSED'",
    )?;
    ensure_column_exists(
        &conn,
        "knowledge_documents",
        "processed_at",
        "ALTER TABLE knowledge_documents ADD COLUMN processed_at TEXT",
    )?;
    ensure_column_exists(
        &conn,
        "pending_ai_queue",
        "priority_order",
        "ALTER TABLE pending_ai_queue ADD COLUMN priority_order INTEGER NOT NULL DEFAULT 0",
    )?;

    // Ensure legacy unanalyzed placeholder docs are explicitly marked UNPROCESSED
    let _ = conn.execute(
        "UPDATE knowledge_documents
         SET processing_status = 'UNPROCESSED'
         WHERE processing_status = 'PROCESSED'
           AND (executive_summary LIKE 'متن فایل استخراج و در حافظه پروژه آرشیو شد%'
                OR executive_summary LIKE 'در انتظار پردازش%')",
        [],
    );

    // Migrate legacy/invalid AGY CLI model names to the current active Gemini 3.8 Flash
    let _ = conn.execute(
        "UPDATE app_settings
         SET agy_model = 'gemini-3.8-flash-high'
         WHERE agy_model = 'Gemini 3.1 Pro (High)' OR agy_model = ''",
        [],
    );

    seed_life_domains_if_empty(&conn)?;

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

pub fn ensure_column_exists(
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

pub fn seed_life_domains_if_empty(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM life_domains", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let now = chrono::Local::now().to_rfc3339();
    let domains = [
        (
            "MIND_LEARNING",
            "ذهن و یادگیری (Mind & Learning)",
            "توسعه مهارت‌های شناختی، زبان انگلیسی، مطالعه عمیق و تسلط بر ابزارهای نوین هوش مصنوعی",
            2,
            "حداقل ۲۰ دقیقه مطالعه متمرکز یا تمرین زبان در روز",
            "تسلط کامل به مکالمه فنی انگلیسی و تسلط بر معماری‌های نوین نرم‌افزاری و هوش مصنوعی",
            1, // is_growth_focus: true
        ),
        (
            "HEALTH_ENERGY",
            "سلامت و انرژی (Health & Energy)",
            "سلامت جسمانی، الگوی خواب منظم، تغذیه باکیفیت و سطح انرژی پایدار روزانه",
            2,
            "حداقل ۷ ساعت خواب شبانه و پیاده‌روی یا تحرک روزانه",
            "انرژی پایدار، روتین منظم بدنسازی و خواب باکیفیت بدون افت انرژی عصرگاهی",
            0,
        ),
        (
            "WEALTH_FINANCE",
            "ثروت و درآمد (Wealth & Finance)",
            "جریان‌های درآمدی پایدار، پلتفرم‌های درآمدزا، مدیریت سرمایه‌گذاری و انضباط مالی",
            2,
            "ثبت و پایش درآمد/هزینه ماهانه و مدیریت بهنگام تعهدات مالی پروژه‌ها",
            "جریان درآمدی خودکار و پایدار ارزی و آزادی عمل مالی در انتخاب پروژه‌ها",
            1, // is_growth_focus: true
        ),
        (
            "RELATIONS",
            "روابط و خانواده (Relations & Family)",
            "ارتباطات عاطفی، پیوندهای خانوادگی، شبکه دوستان صمیمی و ارتباطات حرفه‌ای موثر",
            2,
            "تماس هفتگی با خانواده و اختصاص زمان باکیفیت و بدون حواس‌پرتی به نزدیکان",
            "روابط عمیق، امن، پایدار و الهام‌بخش با دایره اول اطرافیان و شبکه همکاران",
            0,
        ),
        (
            "RESILIENCE",
            "تاب‌آوری و بازیابی (Resilience & Recovery)",
            "مدیریت استرس، پیشگیری از فرسودگی، تاب‌آوری روانی و پروتکل‌های مهار چالش‌های ADHD",
            2,
            "استراحت منظم بین شیفت‌های کاری عمیق و پرهیز از کار در وضعیت خستگی شدید",
            "ثبات خلقی پایدار، بازگشت سریع پس از بحران‌ها و مهار تله‌های شناختی بیش‌تمرکزی",
            0,
        ),
        (
            "PURPOSE",
            "رسالت و جهت‌گیری (Purpose & Mission)",
            "اهداف بلندمدت، چشم‌انداز معنادار زندگی، ارزش‌های بنیادین و جهت‌گیری استراتژیک",
            2,
            "بازبینی فصلی هم‌راستایی پروژه‌های فعال با ارزش‌ها و رسالت فردی",
            "خلق محصولات اثرگذار، ماندگار و معنادار با نقش‌آفرینی سازنده در زندگی دیگران",
            0,
        ),
        (
            "SPIRITUALITY",
            "معنویت و ذهن‌آگاهی (Spirituality & Mindfulness)",
            "حضور در لحظه، آرامش درونی، ذهن‌آگاهی، خلوت ذهنی و شکرگزاری مستمر",
            2,
            "لحظات کوتاه روزانه برای تامل، تنفس آگاهانه و سکوت ذهنی بدون نمایشگر",
            "آرامش عمیق درونی، وضوح شهودی و رهایی از سر و صدای ذهنی روزمره",
            0,
        ),
        (
            "SANCTUARY",
            "محیط و ابزارها (Sanctuary & Tools)",
            "فضای فیزیکی کار و زندگی، ارگونومی، تجهیزات سخت‌افزاری و سازماندهی دیجیتال",
            2,
            "میز کار تمیز و خلوت، پشتیبان‌گیری منظم و نگهداری دوره‌ای ابزارهای کار",
            "محیط کار کاملاً ارگونومیک، مینیمال، بدون حواس‌پرتی و مجهز به روان‌ترین ابزارهای توسعه",
            0,
        ),
    ];

    for (code, title, desc, maturity, floor, desired, focus) in domains {
        conn.execute(
            "INSERT INTO life_domains (code, title, description, maturity_state, maintenance_floor, desired_state, is_growth_focus, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![code, title, desc, maturity, floor, desired, focus, now],
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_database_creates_all_m1_tables_and_indexes() {
        let temp_dir = std::env::temp_dir().join(format!("mab_schema_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test.sqlite");

        let conn = init_database(&db_path, &temp_dir).expect("Database initialization should succeed");

        let required_tables = [
            "life_domains",
            "outcomes",
            "work_lines",
            "calendar_days",
            "pending_proposals",
            "personal_vault_items",
        ];

        for table in required_tables {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    rusqlite::params![table],
                    |r| r.get::<_, i64>(0),
                )
                .map(|c| c > 0)
                .unwrap_or(false);
            assert!(exists, "Expected table {} to exist in database", table);
        }

        let required_indexes = [
            "idx_outcomes_domain",
            "idx_work_lines_project",
            "idx_work_lines_outcome",
            "idx_pending_proposals_status",
            "idx_personal_vault_items_domain",
            "idx_personal_vault_items_realm",
        ];

        for index in required_indexes {
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                    rusqlite::params![index],
                    |r| r.get::<_, i64>(0),
                )
                .map(|c| c > 0)
                .unwrap_or(false);
            assert!(exists, "Expected index {} to exist in database", index);
        }

        drop(conn);
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_projects_work_entities_and_knowledge_docs_have_all_m1_columns() {
        let temp_dir = std::env::temp_dir().join(format!("mab_col_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test_cols.sqlite");

        let conn = init_database(&db_path, &temp_dir).expect("Database initialization should succeed");

        let get_cols = |table: &str| -> Vec<String> {
            let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table)).unwrap();
            stmt.query_map([], |r| r.get::<_, String>(1))
                .unwrap()
                .filter_map(|r| r.ok())
                .collect()
        };

        // 7 columns on projects
        let project_cols = get_cols("projects");
        for expected in &[
            "domain_code",
            "primary_outcome_id",
            "lifecycle_status",
            "scope_mode",
            "realm",
            "priority_weight",
            "pause_resume_note",
        ] {
            assert!(
                project_cols.iter().any(|c| c == expected),
                "Missing expected column {} in projects: {:?}",
                expected,
                project_cols
            );
        }

        // 12 columns on work_entities
        let work_entity_cols = get_cols("work_entities");
        for expected in &[
            "work_line_id",
            "outcome_id",
            "sequence_order",
            "depends_on_task_id",
            "energy_tag",
            "duration_minutes",
            "device_tag",
            "location_tags_json",
            "scheduled_date",
            "scheduled_slot",
            "due_date",
            "priority_score",
        ] {
            assert!(
                work_entity_cols.iter().any(|c| c == expected),
                "Missing expected column {} in work_entities: {:?}",
                expected,
                work_entity_cols
            );
        }

        // 4 columns on knowledge_documents
        let doc_cols = get_cols("knowledge_documents");
        for expected in &[
            "binary_original_path",
            "tags_json",
            "realm",
            "domain_code",
        ] {
            assert!(
                doc_cols.iter().any(|c| c == expected),
                "Missing expected column {} in knowledge_documents: {:?}",
                expected,
                doc_cols
            );
        }

        drop(conn);
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_8_cadm_life_domains_seeded_properly() {
        let temp_dir = std::env::temp_dir().join(format!("mab_cadm_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test_cadm.sqlite");

        let conn = init_database(&db_path, &temp_dir).expect("Database initialization should succeed");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM life_domains", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 8, "Expected exactly 8 CADM life domains to be seeded");

        let expected_codes = [
            ("MIND_LEARNING", true),
            ("HEALTH_ENERGY", false),
            ("WEALTH_FINANCE", true),
            ("RELATIONS", false),
            ("RESILIENCE", false),
            ("PURPOSE", false),
            ("SPIRITUALITY", false),
            ("SANCTUARY", false),
        ];

        for (code, expected_growth) in expected_codes {
            let row: (String, String, i32, i32) = conn
                .query_row(
                    "SELECT title, description, maturity_state, is_growth_focus FROM life_domains WHERE code = ?1",
                    rusqlite::params![code],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .expect(&format!("Domain {} should exist", code));

            assert!(!row.0.is_empty(), "Title should not be empty for {}", code);
            assert!(!row.1.is_empty(), "Description should not be empty for {}", code);
            assert_eq!(row.2, 2, "Default maturity state should be 2 for {}", code);
            assert_eq!(
                row.3 == 1,
                expected_growth,
                "Growth focus mismatch for {}",
                code
            );
        }

        drop(conn);
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_schema_migration_idempotent() {
        let temp_dir = std::env::temp_dir().join(format!("mab_idempotent_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("test_idem.sqlite");

        // 1st run
        let conn1 = init_database(&db_path, &temp_dir).expect("First initialization");
        drop(conn1);

        // 2nd run on existing DB
        let conn2 = init_database(&db_path, &temp_dir).expect("Second initialization should be idempotent");

        let count: i64 = conn2
            .query_row("SELECT COUNT(*) FROM life_domains", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 8, "Idempotent run must not duplicate domain records");

        drop(conn2);
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
