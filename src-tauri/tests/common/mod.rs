//! Common Test Harness and Fixtures for Mab-Compass v3.0 E2E Testing
//! Location: `src-tauri/tests/common/mod.rs`
//!
//! Provides:
//! - `create_test_db()`: In-memory SQLite connection with full v3.0 schema and seed data
//! - `TestContext`: RAII temporary SQLite database and sandbox on disk with auto-cleanup
//! - CADM 8-domain seed verification (`assert_cadm_seeds`)
//! - 5-layer ontology fixture helpers (`create_test_outcome`, `create_test_project_v3`, `setup_chain_fixture`)
//! - Sequential chain domino assertions (`assert_chain_domino_transition`, `assert_chain_step_statuses`, `assert_work_line_completed`)
//! - 4D execution context fixtures (`setup_context_fixture`) and context query helper
//! - MCP tool caller (`call_mcp_tool`) dispatching through `mab_compass_lib::mcp_tools::execute_tool`

#![allow(dead_code)]

use chrono::Local;
use rusqlite::{params, Connection, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

/// 8 CADM Life Domain Codes according to Turnkey Specification Blueprint §2.1 & §6.1
pub const CADM_DOMAIN_CODES: &[&str] = &[
    "MIND_LEARNING",
    "HEALTH_ENERGY",
    "WEALTH_FINANCE",
    "RELATIONS",
    "RESILIENCE",
    "PURPOSE",
    "SPIRITUALITY",
    "SANCTUARY",
];

// ==============================================================================
// 1. IN-MEMORY DB & RAII TEST CONTEXT
// ==============================================================================

/// Opens an in-memory SQLite database, enables foreign keys, applies the complete
/// Mab-Compass v3.0 schema (base + v3 tables + column alterations), seeds the 8
/// CADM life domains with default maturity levels and maintenance floors, and seeds
/// default projects (GERDALO and IRANOIL).
pub fn create_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory SQLite connection");

    // Configure PRAGMAs
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;",
    )
    .expect("Failed to configure database pragmas");

    // Apply base and v3 schema
    apply_base_schema(&conn).expect("Failed to apply base schema");
    ensure_v3_schema_and_seeds(&conn).expect("Failed to ensure v3 schema and seeds");
    let _ = mab_compass_lib::db::queries::ensure_calendar_events_schema(&conn);
    seed_default_projects(&conn).expect("Failed to seed default projects");

    conn
}

/// RAII Temporary Database and Vault Sandbox on Disk for E2E Tests.
/// Uses `tempfile::TempDir` to guarantee cleanup upon drop.
pub struct TestContext {
    pub conn: Connection,
    pub temp_dir: TempDir,
    pub vault_path: PathBuf,
    pub db_path: PathBuf,
}

impl TestContext {
    /// Creates a fresh, isolated SQLite database in a unique temporary directory,
    /// initializes base schema and directories via `schema::init_database`, ensures
    /// full v3 schema and seeds are active, and guarantees cleanup on drop.
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory for test");
        let dir_path = temp_dir.path().to_path_buf();
        let vault_path = dir_path.join("vault");
        fs::create_dir_all(&vault_path).expect("Failed to create temporary vault directory");

        let db_path = dir_path.join("test.sqlite");
        let conn = Connection::open(&db_path).expect("Failed to open test database");
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;",
        )
        .expect("Failed to configure database pragmas");

        // Apply base and v3 schema
        apply_base_schema(&conn).expect("Failed to apply base schema");
        ensure_v3_schema_and_seeds(&conn).expect("Failed to ensure v3 schema and seeds");
        let _ = mab_compass_lib::db::queries::ensure_calendar_events_schema(&conn);
        seed_default_projects(&conn).expect("Failed to seed default projects");

        Self {
            conn,
            temp_dir,
            vault_path,
            db_path,
        }
    }
}

// ==============================================================================
// 2. SCHEMA DEFINITIONS (BASE + V3.0)
// ==============================================================================

/// Applies the base v2.0 tables to an empty SQLite database
pub fn apply_base_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS app_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            vault_root_path TEXT NOT NULL DEFAULT '',
            mab_focus_data_path TEXT NOT NULL DEFAULT '',
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
            workspace_path TEXT NOT NULL DEFAULT '',
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
            entity_type TEXT NOT NULL DEFAULT 'TASK',
            category TEXT NOT NULL DEFAULT 'GENERAL',
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
        ",
    )?;

    conn.execute(
        "INSERT OR IGNORE INTO app_settings (id, vault_root_path, mab_focus_data_path)
         VALUES (1, 'vault', 'MabFocus')",
        [],
    )?;

    Ok(())
}

/// Ensures all v3.0 tables, altered columns, and CADM domain seeds exist (idempotent DDL)
pub fn ensure_v3_schema_and_seeds(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        -- 1. life_domains table (8 CADM dimensions)
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

        -- 2. outcomes table (Desired reality changes)
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
            updated_at TEXT NOT NULL,
            FOREIGN KEY (domain_code) REFERENCES life_domains(code)
        );

        -- 3. work_lines table (Goal streams & sequential chains)
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
            updated_at TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id),
            FOREIGN KEY (outcome_id) REFERENCES outcomes(id)
        );

        -- 4. calendar_days table (Capacity planner)
        CREATE TABLE IF NOT EXISTS calendar_days (
            date_key TEXT PRIMARY KEY,
            total_capacity_minutes INTEGER NOT NULL DEFAULT 300,
            deep_capacity_minutes INTEGER NOT NULL DEFAULT 150,
            day_note TEXT NOT NULL DEFAULT '',
            is_off_day INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );

        -- 5. pending_proposals table (Human-in-the-loop proposals)
        CREATE TABLE IF NOT EXISTS pending_proposals (
            id TEXT PRIMARY KEY,
            project_id TEXT,
            proposal_type TEXT NOT NULL,
            title TEXT NOT NULL,
            rationale_markdown TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'PENDING',
            created_at TEXT NOT NULL,
            resolved_at TEXT,
            FOREIGN KEY (project_id) REFERENCES projects(id)
        );

        -- 6. personal_vault_items table (Vault lessons, ideas, interests)
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
            updated_at TEXT NOT NULL,
            FOREIGN KEY (domain_code) REFERENCES life_domains(code),
            FOREIGN KEY (project_id) REFERENCES projects(id)
        );
        ",
    )?;

    // Alter projects table
    ensure_col(conn, "projects", "domain_code", "ALTER TABLE projects ADD COLUMN domain_code TEXT NOT NULL DEFAULT 'WEALTH_FINANCE'")?;
    ensure_col(conn, "projects", "primary_outcome_id", "ALTER TABLE projects ADD COLUMN primary_outcome_id TEXT")?;
    ensure_col(conn, "projects", "lifecycle_status", "ALTER TABLE projects ADD COLUMN lifecycle_status TEXT NOT NULL DEFAULT 'ACTIVE'")?;
    ensure_col(conn, "projects", "scope_mode", "ALTER TABLE projects ADD COLUMN scope_mode TEXT NOT NULL DEFAULT 'STANDARD'")?;
    ensure_col(conn, "projects", "realm", "ALTER TABLE projects ADD COLUMN realm TEXT NOT NULL DEFAULT 'WORK'")?;
    ensure_col(conn, "projects", "priority_weight", "ALTER TABLE projects ADD COLUMN priority_weight INTEGER NOT NULL DEFAULT 5")?;
    ensure_col(conn, "projects", "pause_resume_note", "ALTER TABLE projects ADD COLUMN pause_resume_note TEXT NOT NULL DEFAULT ''")?;

    // Alter work_entities table
    ensure_col(conn, "work_entities", "work_line_id", "ALTER TABLE work_entities ADD COLUMN work_line_id TEXT")?;
    ensure_col(conn, "work_entities", "outcome_id", "ALTER TABLE work_entities ADD COLUMN outcome_id TEXT")?;
    ensure_col(conn, "work_entities", "sequence_order", "ALTER TABLE work_entities ADD COLUMN sequence_order INTEGER NOT NULL DEFAULT 1")?;
    ensure_col(conn, "work_entities", "depends_on_task_id", "ALTER TABLE work_entities ADD COLUMN depends_on_task_id TEXT")?;
    ensure_col(conn, "work_entities", "is_locked_by_chain", "ALTER TABLE work_entities ADD COLUMN is_locked_by_chain INTEGER NOT NULL DEFAULT 0")?;
    ensure_col(conn, "work_entities", "energy_tag", "ALTER TABLE work_entities ADD COLUMN energy_tag TEXT NOT NULL DEFAULT 'MEDIUM_NORMAL'")?;
    ensure_col(conn, "work_entities", "duration_minutes", "ALTER TABLE work_entities ADD COLUMN duration_minutes INTEGER NOT NULL DEFAULT 25")?;
    ensure_col(conn, "work_entities", "device_tag", "ALTER TABLE work_entities ADD COLUMN device_tag TEXT NOT NULL DEFAULT 'LAPTOP_ONLY'")?;
    ensure_col(conn, "work_entities", "location_tags_json", "ALTER TABLE work_entities ADD COLUMN location_tags_json TEXT NOT NULL DEFAULT '[\"DESK\"]'")?;
    ensure_col(conn, "work_entities", "scheduled_date", "ALTER TABLE work_entities ADD COLUMN scheduled_date TEXT")?;
    ensure_col(conn, "work_entities", "scheduled_slot", "ALTER TABLE work_entities ADD COLUMN scheduled_slot TEXT")?;
    ensure_col(conn, "work_entities", "due_date", "ALTER TABLE work_entities ADD COLUMN due_date TEXT")?;
    ensure_col(conn, "work_entities", "priority_score", "ALTER TABLE work_entities ADD COLUMN priority_score INTEGER NOT NULL DEFAULT 50")?;

    // Alter knowledge_documents table
    ensure_col(conn, "knowledge_documents", "binary_original_path", "ALTER TABLE knowledge_documents ADD COLUMN binary_original_path TEXT")?;
    ensure_col(conn, "knowledge_documents", "tags_json", "ALTER TABLE knowledge_documents ADD COLUMN tags_json TEXT NOT NULL DEFAULT '[]'")?;
    ensure_col(conn, "knowledge_documents", "realm", "ALTER TABLE knowledge_documents ADD COLUMN realm TEXT NOT NULL DEFAULT 'WORK'")?;
    ensure_col(conn, "knowledge_documents", "domain_code", "ALTER TABLE knowledge_documents ADD COLUMN domain_code TEXT NOT NULL DEFAULT 'WEALTH_FINANCE'")?;
    ensure_col(conn, "knowledge_documents", "processing_status", "ALTER TABLE knowledge_documents ADD COLUMN processing_status TEXT NOT NULL DEFAULT 'PROCESSED'")?;
    ensure_col(conn, "knowledge_documents", "processed_at", "ALTER TABLE knowledge_documents ADD COLUMN processed_at TEXT")?;
    ensure_col(conn, "pending_ai_queue", "priority_order", "ALTER TABLE pending_ai_queue ADD COLUMN priority_order INTEGER NOT NULL DEFAULT 0")?;

    // Compatibility column aliases for life_domains
    ensure_col(conn, "life_domains", "domain_code", "ALTER TABLE life_domains ADD COLUMN domain_code TEXT")?;
    ensure_col(conn, "life_domains", "name_fa", "ALTER TABLE life_domains ADD COLUMN name_fa TEXT NOT NULL DEFAULT ''")?;
    ensure_col(conn, "life_domains", "name_en", "ALTER TABLE life_domains ADD COLUMN name_en TEXT NOT NULL DEFAULT ''")?;
    ensure_col(conn, "life_domains", "maturity_level", "ALTER TABLE life_domains ADD COLUMN maturity_level INTEGER NOT NULL DEFAULT 2")?;
    ensure_col(conn, "life_domains", "maintenance_floor_minutes", "ALTER TABLE life_domains ADD COLUMN maintenance_floor_minutes INTEGER NOT NULL DEFAULT 60")?;
    ensure_col(conn, "life_domains", "status", "ALTER TABLE life_domains ADD COLUMN status TEXT NOT NULL DEFAULT 'ACTIVE'")?;

    // Compatibility column aliases for outcomes
    ensure_col(conn, "outcomes", "project_id", "ALTER TABLE outcomes ADD COLUMN project_id TEXT")?;
    ensure_col(conn, "outcomes", "progress_percentage", "ALTER TABLE outcomes ADD COLUMN progress_percentage INTEGER NOT NULL DEFAULT 0")?;
    ensure_col(conn, "outcomes", "desired_reality", "ALTER TABLE outcomes ADD COLUMN desired_reality TEXT NOT NULL DEFAULT ''")?;
    ensure_col(conn, "outcomes", "success_metric", "ALTER TABLE outcomes ADD COLUMN success_metric TEXT NOT NULL DEFAULT ''")?;
    ensure_col(conn, "outcomes", "target_date", "ALTER TABLE outcomes ADD COLUMN target_date TEXT NOT NULL DEFAULT ''")?;

    // Compatibility column aliases for work_lines
    ensure_col(conn, "work_lines", "mode", "ALTER TABLE work_lines ADD COLUMN mode TEXT NOT NULL DEFAULT 'SEQUENTIAL_CHAIN'")?;

    // Compatibility column aliases for calendar_days
    ensure_col(conn, "calendar_days", "day_date", "ALTER TABLE calendar_days ADD COLUMN day_date TEXT NOT NULL DEFAULT ''")?;
    ensure_col(conn, "calendar_days", "notes", "ALTER TABLE calendar_days ADD COLUMN notes TEXT NOT NULL DEFAULT ''")?;

    // Compatibility column aliases for pending_proposals
    ensure_col(conn, "pending_proposals", "rationale", "ALTER TABLE pending_proposals ADD COLUMN rationale TEXT NOT NULL DEFAULT ''")?;
    ensure_col(conn, "pending_proposals", "proposed_payload_json", "ALTER TABLE pending_proposals ADD COLUMN proposed_payload_json TEXT NOT NULL DEFAULT ''")?;

    // Compatibility column aliases for personal_vault_items
    ensure_col(conn, "personal_vault_items", "content", "ALTER TABLE personal_vault_items ADD COLUMN content TEXT NOT NULL DEFAULT ''")?;

    // Seed 8 CADM domains if empty
    seed_cadm_life_domains(conn)?;

    Ok(())
}

fn ensure_col(conn: &Connection, table: &str, col: &str, alter_sql: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma)?;
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(|r| r.ok())
        .collect();
    if !cols.iter().any(|c| c == col) {
        conn.execute(alter_sql, [])?;
    }
    Ok(())
}

/// Seeds the 8 CADM Life Domains according to Turnkey Blueprint §2.1 & §4.4
pub fn seed_cadm_life_domains(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM life_domains", [], |r| r.get(0))?;
    if count == 0 {
        let now = Local::now().to_rfc3339();
        let seeds = [
            ("MIND_LEARNING", "رشد و یادگیری (ذهن)", "Mind & Learning", "پرورش ذهن، یادگیری، تفکر و خلاقیت", 2, "مطالعه روزانه و ثبت یادداشت‌های کلیدی", 60, 1),
            ("HEALTH_ENERGY", "سلامت و انرژی (بدن)", "Health & Energy", "سلامت جسم، انرژی، ورزش، خواب و بازیابی", 2, "حداقل ۷ ساعت خواب و تحرک روزانه", 60, 1),
            ("WEALTH_FINANCE", "ثروت و استقلال مالی (امنیت)", "Wealth & Finance", "درآمد، مدیریت منابع، سرمایه‌گذاری و بهره‌وری مالی", 2, "پایش جریان نقدینگی هفتگی و کنترل هزینه‌ها", 60, 1),
            ("RELATIONS", "روابط و مشارکت اجتماعی (ارتباط)", "Relations & Family", "خانواده، دوستان، شبکه حرفه‌ای و مشارکت اجتماعی", 2, "ارتباط منظم با خانواده و شبکه کلیدی", 45, 0),
            ("RESILIENCE", "هوش هیجانی و تاب‌آوری (قلب)", "Resilience & Balance", "خودآگاهی، مدیریت هیجانات و بازیابی از شکست", 2, "توقف در فشار و بازبینی الگوهای فکری", 30, 0),
            ("PURPOSE", "هدف و ایکیگای (روح)", "Purpose & Contribution", "ارزش‌ها، معنا، هدفمندی و هماهنگی روزانه با هدف", 2, "همسوسازی هفتگی پروژه‌ها با قطب‌نما", 30, 0),
            ("SPIRITUALITY", "ارتباط معنوی و ایمان (ایمان)", "Spirituality", "عبادت، تفکر و ارتباط معنوی", 2, "خلوت و توجه روزانه", 30, 0),
            ("SANCTUARY", "محیط فیزیکی و پناهگاه (بنیان)", "Sanctuary & Environment", "خانه، محیط کار و محیط دیجیتال", 2, "ساماندهی فضای فیزیکی و دسکتاپ کاری", 30, 0),
        ];

        for (code, title_fa, name_en, desc, maturity, floor, floor_mins, focus) in seeds {
            conn.execute(
                "INSERT INTO life_domains (
                    code, title, description, maturity_state, maintenance_floor, desired_state,
                    is_growth_focus, updated_at, domain_code, name_fa, name_en, maturity_level,
                    maintenance_floor_minutes, status
                ) VALUES (?1, ?2, ?3, ?4, ?5, '', ?6, ?7, ?1, ?2, ?8, ?4, ?9, 'ACTIVE')",
                params![code, title_fa, desc, maturity, floor, focus, now, name_en, floor_mins],
            )?;
        }
    }
    Ok(())
}

/// Seeds default test projects GERDALO and IRANOIL
pub fn seed_default_projects(conn: &Connection) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO projects (id, name, workspace_path, external_focus_id, is_active, domain_code, lifecycle_status, scope_mode, realm, priority_weight, pause_resume_note, updated_at)
         VALUES ('GERDALO', 'پروژه گردالو (Gerdalo)', 'vault/GERDALO', 'GERDALO', 1, 'WEALTH_FINANCE', 'ACTIVE', 'STANDARD', 'WORK', 5, '', ?1)",
        params![now],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO projects (id, name, workspace_path, external_focus_id, is_active, domain_code, lifecycle_status, scope_mode, realm, priority_weight, pause_resume_note, updated_at)
         VALUES ('IRANOIL', 'پروژه ایران‌اویل‌جاب / زیگهوش', 'vault/IRANOIL', 'IRANOIL', 0, 'WEALTH_FINANCE', 'ACTIVE', 'STANDARD', 'WORK', 5, '', ?1)",
        params![now],
    )?;
    Ok(())
}

// ==============================================================================
// 3. DOMAIN & CADM SEED ASSERTIONS
// ==============================================================================

/// Asserts that all 8 CADM life domains are seeded with valid maturity (0..5) and maintenance floors
pub fn assert_cadm_seeds(conn: &Connection) {
    let mut stmt = conn
        .prepare("SELECT code, title, maturity_state, maintenance_floor FROM life_domains ORDER BY code")
        .expect("Failed to query life_domains");

    let rows: Vec<(String, String, i32, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .expect("Failed to map life_domains")
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(
        rows.len(),
        8,
        "Expected exactly 8 CADM life domains seeded, found {}",
        rows.len()
    );

    for expected_code in CADM_DOMAIN_CODES {
        let domain = rows.iter().find(|(code, _, _, _)| code == expected_code);
        assert!(
            domain.is_some(),
            "CADM domain code '{}' missing from life_domains",
            expected_code
        );
        let (_, title, maturity, floor) = domain.unwrap();
        assert!(!title.is_empty(), "Domain '{}' title cannot be empty", expected_code);
        assert!(
            *maturity >= 0 && *maturity <= 5,
            "Domain '{}' maturity {} outside [0..5]",
            expected_code,
            maturity
        );
        assert!(
            !floor.is_empty(),
            "Domain '{}' maintenance_floor cannot be empty",
            expected_code
        );
    }
}

// ==============================================================================
// 4. 5-LAYER ONTOLOGY FIXTURE BUILDERS
// ==============================================================================

/// Creates an Outcome fixture record linked to a CADM domain
pub fn create_test_outcome(
    conn: &Connection,
    id: &str,
    domain_code: &str,
    title: &str,
    evidence_kr: &str,
) -> Result<String> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO outcomes (
            id, domain_code, title, desired_reality_change, evidence_kr, time_window,
            status, progress_percent, created_at, updated_at, desired_reality, success_metric, progress_percentage
        ) VALUES (?1, ?2, ?3, 'Test reality change', ?4, '1405-Q3', 'ACTIVE', 0, ?5, ?5, 'Test reality change', ?4, 0)",
        params![id, domain_code, title, evidence_kr, now],
    )?;
    Ok(id.to_string())
}

/// Creates a Project fixture record linked to a domain and optional primary outcome
pub fn create_test_project_v3(
    conn: &Connection,
    id: &str,
    name: &str,
    domain_code: &str,
    primary_outcome_id: Option<&str>,
) -> Result<String> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO projects (
            id, name, workspace_path, external_focus_id, is_active, updated_at,
            domain_code, primary_outcome_id, lifecycle_status, scope_mode, realm, priority_weight, pause_resume_note
        ) VALUES (?1, ?2, ?1, ?1, 1, ?3, ?4, ?5, 'ACTIVE', 'STANDARD', 'WORK', 5, '')",
        params![id, name, now, domain_code, primary_outcome_id],
    )?;
    Ok(id.to_string())
}

/// Creates a WorkLine and sequential chain of tasks.
/// Step 1 is unlocked (`IN_PROGRESS` / `OPEN`, sequence_order=1, `is_locked_by_chain = 0`).
/// Steps 2..N are locked (`QUEUED_IN_CHAIN`, sequence_order=2..N, `depends_on_task_id = previous`, `is_locked_by_chain = 1`).
///
/// Returns `(work_line_id, Vec<task_id>)`.
pub fn setup_chain_fixture(
    conn: &Connection,
    project_id: &str,
    outcome_id: Option<&str>,
    line_title: &str,
    steps: &[&str],
) -> (String, Vec<String>) {
    assert!(!steps.is_empty(), "Cannot create a sequential chain with 0 steps");
    let line_id = format!("WLN-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
    let now = Local::now().to_rfc3339();

    conn.execute(
        "INSERT INTO work_lines (
            id, project_id, outcome_id, title, goal_statement, line_mode, status, color_hex,
            created_at, updated_at, mode
        ) VALUES (?1, ?2, ?3, ?4, 'Deliverable goal', 'SEQUENTIAL_CHAIN', 'ACTIVE', '#3B82F6', ?5, ?5, 'SEQUENTIAL_CHAIN')",
        params![line_id, project_id, outcome_id, line_title, now],
    ).expect("Failed to insert work_line");

    let mut task_ids = Vec::new();
    let mut prev_task_id: Option<String> = None;

    for (idx, step_title) in steps.iter().enumerate() {
        let task_id = format!("TSK-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
        let seq_order = (idx + 1) as i32;
        let is_step_one = idx == 0;
        let status = if is_step_one { "IN_PROGRESS" } else { "QUEUED_IN_CHAIN" };
        let is_locked = if is_step_one { 0 } else { 1 };

        conn.execute(
            "INSERT INTO work_entities (
                id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos, calibrated_pomos,
                work_line_id, outcome_id, sequence_order, depends_on_task_id, is_locked_by_chain,
                energy_tag, duration_minutes, device_tag, location_tags_json, priority_score, updated_at
            ) VALUES (
                ?1, ?2, 'TASK', 'BACKEND', ?3, 'Done criteria', 'First physical action',
                ?4, ?5, 1.0, 2.0, 3.0, 2.0,
                ?6, ?7, ?8, ?9, ?10,
                'MEDIUM_NORMAL', 25, 'LAPTOP_ONLY', '[\"DESK\"]', 50, ?11
            )",
            params![
                task_id, project_id, step_title,
                status, if is_step_one { 1 } else { 0 },
                line_id, outcome_id, seq_order, prev_task_id, is_locked, now
            ],
        ).expect("Failed to insert work_entity for chain step");

        prev_task_id = Some(task_id.clone());
        task_ids.push(task_id);
    }

    (line_id, task_ids)
}

/// Convenience overload for `setup_chain_fixture` using default project 'GERDALO'
pub fn setup_chain_fixture_simple(conn: &Connection, steps: &[&str]) -> (String, Vec<String>) {
    setup_chain_fixture(conn, "GERDALO", None, "Default Test Sequential Chain", steps)
}

// ==============================================================================
// 5. SEQUENTIAL CHAIN DOMINO ENGINE HELPERS & ASSERTIONS
// ==============================================================================

/// Simulates completing a task in a chain and executes the domino auto-advancement:
/// 1. Sets completed task status = 'CLOSED', is_locked_by_chain = 0
/// 2. Promotes next sequential step in the chain from 'QUEUED_IN_CHAIN' to 'IN_PROGRESS', is_locked_by_chain = 0
/// 3. If all steps are 'CLOSED', sets parent work_lines status = 'COMPLETED'
pub fn simulate_advance_chain_on_complete(
    conn: &Connection,
    project_id: &str,
    completed_task_id: &str,
) -> Result<Option<String>> {
    let now = Local::now().to_rfc3339();

    // 1. Mark target task as CLOSED
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

    let mut promoted_task_id = None;

    if let Some((Some(line_id), seq_order)) = meta {
        // Find next step in chain (status = 'QUEUED_IN_CHAIN' and sequence_order > current)
        let next_task: Option<String> = conn
            .query_row(
                "SELECT id FROM work_entities
                 WHERE work_line_id = ?1 AND status = 'QUEUED_IN_CHAIN' AND sequence_order > ?2
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
            promoted_task_id = Some(next_id);
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

    Ok(promoted_task_id)
}

/// Evaluates whether a task is locked by its sequential chain (F11 contract)
pub fn is_locked_by_chain(conn: &Connection, task_id: &str) -> Result<bool> {
    let row: Option<(Option<String>, i32, String, i32)> = conn
        .query_row(
            "SELECT work_line_id, sequence_order, status, is_locked_by_chain FROM work_entities WHERE id = ?1",
            params![task_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();

    match row {
        None => Ok(false),
        Some((None, _, status, is_locked)) => {
            Ok(is_locked == 1 || status == "QUEUED_IN_CHAIN")
        }
        Some((Some(line_id), seq_order, status, is_locked)) => {
            if is_locked == 1 || status == "QUEUED_IN_CHAIN" {
                return Ok(true);
            }
            if seq_order <= 1 {
                return Ok(false);
            }
            // Check if any prior step is unclosed
            let unclosed_prior: i64 = conn.query_row(
                "SELECT COUNT(*) FROM work_entities
                 WHERE work_line_id = ?1 AND sequence_order < ?2 AND status != 'CLOSED'",
                params![line_id, seq_order],
                |r| r.get(0),
            )?;
            Ok(unclosed_prior > 0)
        }
    }
}

/// Asserts exact chain statuses across all sequential steps of a WorkLine.
/// Expected: slice of (step_index_1_based, expected_status)
pub fn assert_chain_step_statuses(
    conn: &Connection,
    work_line_id: &str,
    expected_statuses: &[(usize, &str)],
) {
    let mut stmt = conn
        .prepare(
            "SELECT id, sequence_order, status FROM work_entities
             WHERE work_line_id = ?1 ORDER BY sequence_order ASC",
        )
        .expect("Failed to prepare chain status query");

    let steps: Vec<(String, i32, String)> = stmt
        .query_map(params![work_line_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("Failed to query chain tasks")
        .filter_map(|r| r.ok())
        .collect();

    for &(step_idx, expected_status) in expected_statuses {
        assert!(
            step_idx > 0 && step_idx <= steps.len(),
            "Step index {} out of range for chain with {} steps",
            step_idx,
            steps.len()
        );
        let (task_id, seq_order, status) = &steps[step_idx - 1];
        assert_eq!(
            *seq_order, step_idx as i32,
            "Step index mismatch: expected sequence_order {}, found {}",
            step_idx, seq_order
        );
        assert_eq!(
            status.as_str(),
            expected_status,
            "Task {} (Step {}) expected status '{}', but got '{}'",
            task_id,
            step_idx,
            expected_status,
            status
        );
    }
}

/// Asserts that a sequential chain domino transition succeeded:
/// - `completed_task_id` is CLOSED
/// - `promoted_task_id` is IN_PROGRESS
/// - Subsequent tasks are QUEUED_IN_CHAIN
pub fn assert_chain_domino_transition(
    conn: &Connection,
    work_line_id: &str,
    completed_task_id: &str,
    promoted_task_id: &str,
) {
    let status_completed: String = conn
        .query_row(
            "SELECT status FROM work_entities WHERE id = ?1",
            params![completed_task_id],
            |r| r.get(0),
        )
        .expect("Completed task not found");
    assert_eq!(
        status_completed, "CLOSED",
        "Task {} was not CLOSED (found: {})",
        completed_task_id, status_completed
    );

    let status_promoted: String = conn
        .query_row(
            "SELECT status FROM work_entities WHERE id = ?1",
            params![promoted_task_id],
            |r| r.get(0),
        )
        .expect("Promoted task not found");
    assert_eq!(
        status_promoted, "IN_PROGRESS",
        "Next task {} was not promoted to IN_PROGRESS (found: {})",
        promoted_task_id, status_promoted
    );

    // Verify all tasks after promoted_task_id in the chain remain QUEUED_IN_CHAIN
    let promoted_order: i32 = conn
        .query_row(
            "SELECT sequence_order FROM work_entities WHERE id = ?1",
            params![promoted_task_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let mut stmt = conn
        .prepare(
            "SELECT id, status FROM work_entities
             WHERE work_line_id = ?1 AND sequence_order > ?2",
        )
        .expect("Failed to prepare query");

    let later_tasks: Vec<(String, String)> = stmt
        .query_map(params![work_line_id, promoted_order], |r| Ok((r.get(0)?, r.get(1)?)))
        .expect("Failed to query later tasks")
        .filter_map(|r| r.ok())
        .collect();

    for (tid, st) in later_tasks {
        assert_eq!(
            st, "QUEUED_IN_CHAIN",
            "Later step {} in chain should remain QUEUED_IN_CHAIN, found '{}'",
            tid, st
        );
    }
}

/// Asserts that the entire WorkLine is completed and all its tasks are closed
pub fn assert_work_line_completed(conn: &Connection, work_line_id: &str) {
    let status: String = conn
        .query_row(
            "SELECT status FROM work_lines WHERE id = ?1",
            params![work_line_id],
            |r| r.get(0),
        )
        .expect("WorkLine not found");
    assert_eq!(
        status, "COMPLETED",
        "WorkLine {} is not COMPLETED (found: {})",
        work_line_id, status
    );

    let open_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM work_entities WHERE work_line_id = ?1 AND status != 'CLOSED'",
            params![work_line_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    assert_eq!(
        open_count, 0,
        "WorkLine {} still has {} unclosed tasks",
        work_line_id, open_count
    );
}

// ==============================================================================
// 6. 4D EXECUTION CONTEXT FIXTURES & ASSERTIONS
// ==============================================================================

#[derive(Debug, Clone)]
pub struct ContextTaskFixture {
    pub id: String,
    pub title: String,
    pub device_tag: String,
    pub location_tag: String,
    pub energy_tag: String,
    pub duration_minutes: i32,
    pub is_locked_chain_step: bool,
}

/// Populates a diverse set of tasks exercising all 4 dimensions of context tagging:
/// - Device: LAPTOP_ONLY, PHONE_OK, AUDIO_HANDSFREE, ANY_DEVICE
/// - Location: DESK, METRO_COMMUTE, CAR_DRIVING, OUTSIDE_ERRAND
/// - Energy: DEEP_HIGH, MEDIUM_NORMAL, LOW_ZOMBIE
/// - Duration: 10, 15, 20, 25, 30, 50, 90 minutes
/// - Chain Locking: unchained open tasks vs locked chained step (QUEUED_IN_CHAIN)
pub fn setup_context_fixture(conn: &Connection, project_id: &str) -> Vec<ContextTaskFixture> {
    let now = Local::now().to_rfc3339();
    let configs = [
        ("Task Laptop Deep Desk 50m", "LAPTOP_ONLY", "DESK", "DEEP_HIGH", 50, false),
        ("Task Metro Phone Low 15m", "PHONE_OK", "METRO_COMMUTE", "LOW_ZOMBIE", 15, false),
        ("Task Car Audio Low 10m", "AUDIO_HANDSFREE", "CAR_DRIVING", "LOW_ZOMBIE", 10, false),
        ("Task Any Desk Medium 25m", "ANY_DEVICE", "DESK", "MEDIUM_NORMAL", 25, false),
        ("Task Metro Phone Low 15m (LOCKED IN CHAIN)", "PHONE_OK", "METRO_COMMUTE", "LOW_ZOMBIE", 15, true),
        ("Task Metro Phone Low 20m", "PHONE_OK", "METRO_COMMUTE", "LOW_ZOMBIE", 20, false),
        ("Task Outside Errand Low 30m", "PHONE_OK", "OUTSIDE_ERRAND", "LOW_ZOMBIE", 30, false),
    ];

    let mut result = Vec::new();

    for (title, device, loc, energy, dur, locked) in configs {
        let task_id = format!("CTX-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
        let status = if locked { "QUEUED_IN_CHAIN" } else { "OPEN" };
        let is_locked = if locked { 1 } else { 0 };
        let loc_json = format!("[\"{}\"]", loc);

        conn.execute(
            "INSERT INTO work_entities (
                id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos, calibrated_pomos,
                sequence_order, energy_tag, duration_minutes, device_tag, location_tags_json,
                is_locked_by_chain, priority_score, updated_at
            ) VALUES (
                ?1, ?2, 'TASK', 'GENERAL', ?3, 'Done', 'Action',
                ?4, 0, 1.0, 2.0, 3.0, 2.0,
                1, ?5, ?6, ?7, ?8, ?9, 50, ?10
            )",
            params![task_id, project_id, title, status, energy, dur, device, loc_json, is_locked, now],
        ).expect("Failed to insert context fixture task");

        result.push(ContextTaskFixture {
            id: task_id,
            title: title.to_string(),
            device_tag: device.to_string(),
            location_tag: loc.to_string(),
            energy_tag: energy.to_string(),
            duration_minutes: dur,
            is_locked_chain_step: locked,
        });
    }

    result
}

/// Opaque SQL query helper for context task filtering
pub fn query_tasks_by_context_sql(
    conn: &Connection,
    project_id: Option<&str>,
    device_tag: Option<&str>,
    location_tag: Option<&str>,
    energy_tag: Option<&str>,
    max_minutes: Option<i32>,
    only_unlocked: bool,
) -> Result<Vec<String>> {
    let mut sql = String::from("SELECT id FROM work_entities WHERE entity_type = 'TASK'");

    if only_unlocked {
        sql.push_str(" AND status NOT IN ('CLOSED', 'SUPERSEDED', 'QUEUED_IN_CHAIN') AND is_locked_by_chain = 0");
    } else {
        sql.push_str(" AND status NOT IN ('CLOSED', 'SUPERSEDED')");
    }

    if let Some(pid) = project_id {
        sql.push_str(&format!(" AND project_id = '{}'", pid));
    }

    if let Some(dev) = device_tag {
        sql.push_str(&format!(
            " AND (device_tag = '{}' OR device_tag = 'ANY_DEVICE')",
            dev
        ));
    }

    if let Some(loc) = location_tag {
        sql.push_str(&format!(" AND location_tags_json LIKE '%\"{}\"%'", loc));
    }

    if let Some(eng) = energy_tag {
        sql.push_str(&format!(" AND energy_tag = '{}'", eng));
    }

    if let Some(max_m) = max_minutes {
        sql.push_str(&format!(" AND duration_minutes <= {}", max_m));
    }

    sql.push_str(" ORDER BY priority_score DESC, duration_minutes ASC");

    let mut stmt = conn.prepare(&sql)?;
    let ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(ids)
}

// ==============================================================================
// 7. MCP TOOL CALLER HELPER & PROPOSAL RESOLUTION
// ==============================================================================

/// Dispatches an MCP tool call through `mab_compass_lib::mcp_tools::execute_tool`,
/// executing the production dispatcher directly.
pub fn call_mcp_tool(conn: &mut Connection, name: &str, args: Value) -> Result<Value, String> {
    mab_compass_lib::mcp_tools::execute_tool(conn, name, &args)
}

fn dispatch_v3_mcp_tool_fallback(conn: &mut Connection, name: &str, args: &Value) -> Result<Value, String> {
    let now = Local::now().to_rfc3339();
    match name {
        "list_projects" => {
            let mut stmt = conn
                .prepare("SELECT id, name, is_active FROM projects ORDER BY is_active DESC, id")
                .map_err(|e| e.to_string())?;
            let projects: Vec<serde_json::Value> = stmt
                .query_map([], |r| {
                    Ok(serde_json::json!({
                        "id": r.get::<_, String>(0)?,
                        "name": r.get::<_, String>(1)?,
                        "is_active": r.get::<_, i32>(2)? == 1
                    }))
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(serde_json::json!({
                "projects": projects
            }))
        }
        "upsert_domain_or_outcome" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let domain_code = args.get("domain_code").and_then(|v| v.as_str()).unwrap_or("");
            if action.is_empty() || domain_code.is_empty() {
                return Err("Missing required parameters: action, domain_code".to_string());
            }

            match action {
                "UPDATE_DOMAIN" => {
                    if let Some(maturity) = args.get("maturity_state").and_then(|v| v.as_i64()) {
                        if !(0..=5).contains(&maturity) {
                            return Err(format!("maturity_state {} is out of bounds (0..5)", maturity));
                        }
                        conn.execute(
                            "UPDATE life_domains SET maturity_state = ?1, updated_at = ?2 WHERE code = ?3",
                            params![maturity, now, domain_code],
                        ).map_err(|e| e.to_string())?;
                    }
                    if let Some(floor) = args.get("maintenance_floor").and_then(|v| v.as_str()) {
                        conn.execute(
                            "UPDATE life_domains SET maintenance_floor = ?1, updated_at = ?2 WHERE code = ?3",
                            params![floor, now, domain_code],
                        ).map_err(|e| e.to_string())?;
                    }
                    Ok(serde_json::json!({
                        "status": "ok",
                        "action": "UPDATE_DOMAIN",
                        "domain_code": domain_code
                    }))
                }
                "CREATE_OUTCOME" => {
                    let outcome_id = args.get("outcome_id").and_then(|v| v.as_str())
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("OUT-{}", Uuid::new_v4().to_string()[..8].to_uppercase()));
                    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Outcome");
                    let desired_reality = args.get("desired_reality_change").and_then(|v| v.as_str()).unwrap_or("");
                    let evidence_kr = args.get("evidence_kr").and_then(|v| v.as_str()).unwrap_or("");
                    let time_window = args.get("time_window").and_then(|v| v.as_str()).unwrap_or("1405-Q3");

                    conn.execute(
                        "INSERT INTO outcomes (
                            id, domain_code, title, desired_reality_change, evidence_kr, time_window,
                            status, progress_percent, created_at, updated_at, desired_reality, success_metric, progress_percentage
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'ACTIVE', 0, ?7, ?7, ?4, ?5, 0)",
                        params![outcome_id, domain_code, title, desired_reality, evidence_kr, time_window, now],
                    ).map_err(|e| e.to_string())?;

                    Ok(serde_json::json!({
                        "status": "ok",
                        "action": "CREATE_OUTCOME",
                        "outcome_id": outcome_id,
                        "domain_code": domain_code
                    }))
                }
                "UPDATE_OUTCOME" => {
                    let outcome_id = args.get("outcome_id").and_then(|v| v.as_str())
                        .ok_or_else(|| "Missing outcome_id for UPDATE_OUTCOME".to_string())?;
                    if let Some(status) = args.get("status").and_then(|v| v.as_str()) {
                        conn.execute(
                            "UPDATE outcomes SET status = ?1, updated_at = ?2 WHERE id = ?3",
                            params![status, now, outcome_id],
                        ).map_err(|e| e.to_string())?;
                    }
                    if let Some(progress) = args.get("progress_percent").and_then(|v| v.as_i64()) {
                        conn.execute(
                            "UPDATE outcomes SET progress_percent = ?1, progress_percentage = ?1, updated_at = ?2 WHERE id = ?3",
                            params![progress, now, outcome_id],
                        ).map_err(|e| e.to_string())?;
                    }
                    if let Some(evidence_kr) = args.get("evidence_kr").and_then(|v| v.as_str()) {
                        conn.execute(
                            "UPDATE outcomes SET evidence_kr = ?1, success_metric = ?1, updated_at = ?2 WHERE id = ?3",
                            params![evidence_kr, now, outcome_id],
                        ).map_err(|e| e.to_string())?;
                    }
                    Ok(serde_json::json!({
                        "status": "ok",
                        "action": "UPDATE_OUTCOME",
                        "outcome_id": outcome_id
                    }))
                }
                _ => Err(format!("Unknown action for upsert_domain_or_outcome: {}", action)),
            }
        }
        "create_work_line_chain" => {
            let project_id = args.get("project_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: project_id".to_string())?;
            let line_title = args.get("line_title").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: line_title".to_string())?;
            let outcome_id = args.get("outcome_id").and_then(|v| v.as_str());
            let goal_statement = args.get("goal_statement").and_then(|v| v.as_str()).unwrap_or("");
            let line_mode = args.get("line_mode").and_then(|v| v.as_str()).unwrap_or("SEQUENTIAL_CHAIN");

            let steps_arr = args.get("steps").and_then(|v| v.as_array())
                .ok_or_else(|| "Missing required parameter: steps".to_string())?;
            if steps_arr.is_empty() {
                return Err("Chain requires at least 1 step".to_string());
            }

            let line_id = format!("WLN-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
            conn.execute(
                "INSERT INTO work_lines (
                    id, project_id, outcome_id, title, goal_statement, line_mode, status, color_hex,
                    created_at, updated_at, mode
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'ACTIVE', '#3B82F6', ?7, ?7, ?6)",
                params![line_id, project_id, outcome_id, line_title, goal_statement, line_mode, now],
            ).map_err(|e| e.to_string())?;

            let mut task_ids = Vec::new();
            let mut prev_task_id: Option<String> = None;

            for (idx, step_val) in steps_arr.iter().enumerate() {
                let task_id = format!("TSK-{}", Uuid::new_v4().to_string()[..8].to_uppercase());
                let title = step_val.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Step");
                let dod = step_val.get("definition_of_done").and_then(|v| v.as_str()).unwrap_or("");
                let fpa = step_val.get("first_physical_action").and_then(|v| v.as_str()).unwrap_or("");
                let energy = step_val.get("energy_tag").and_then(|v| v.as_str()).unwrap_or("MEDIUM_NORMAL");
                let dur = step_val.get("duration_minutes").and_then(|v| v.as_i64()).unwrap_or(25);
                let device = step_val.get("device_tag").and_then(|v| v.as_str()).unwrap_or("LAPTOP_ONLY");
                let locs = step_val.get("location_tags").cloned().unwrap_or_else(|| serde_json::json!(["DESK"])).to_string();

                let is_step_one = idx == 0;
                let status = if is_step_one { "IN_PROGRESS" } else { "QUEUED_IN_CHAIN" };
                let is_locked = if is_step_one { 0 } else { 1 };
                let seq = (idx + 1) as i32;

                conn.execute(
                    "INSERT INTO work_entities (
                        id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                        status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos, calibrated_pomos,
                        work_line_id, outcome_id, sequence_order, depends_on_task_id, is_locked_by_chain,
                        energy_tag, duration_minutes, device_tag, location_tags_json, priority_score, updated_at
                    ) VALUES (
                        ?1, ?2, 'TASK', 'BACKEND', ?3, ?4, ?5,
                        ?6, ?7, 1.0, 2.0, 3.0, 2.0,
                        ?8, ?9, ?10, ?11, ?12,
                        ?13, ?14, ?15, ?16, 50, ?17
                    )",
                    params![
                        task_id, project_id, title, dod, fpa,
                        status, if is_step_one { 1 } else { 0 },
                        line_id, outcome_id, seq, prev_task_id, is_locked,
                        energy, dur, device, locs, now
                    ],
                ).map_err(|e| e.to_string())?;

                prev_task_id = Some(task_id.clone());
                task_ids.push(task_id);
            }

            Ok(serde_json::json!({
                "work_line_id": line_id,
                "project_id": project_id,
                "tasks": task_ids
            }))
        }
        "append_or_reorder_work_line" => {
            let line_id = args.get("work_line_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: work_line_id".to_string())?;
            let task_id = args.get("task_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: task_id".to_string())?;

            // Verify work_line exists
            let line_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM work_lines WHERE id = ?1",
                params![line_id],
                |r| r.get(0),
            ).map_err(|e| e.to_string())?;
            if line_exists == 0 {
                return Err(format!("work_line_id '{}' not found", line_id));
            }

            if let Some(new_seq) = args.get("new_sequence_order").and_then(|v| v.as_i64()) {
                let dep = args.get("depends_on_task_id").and_then(|v| v.as_str());
                conn.execute(
                    "UPDATE work_entities SET sequence_order = ?1, depends_on_task_id = ?2, updated_at = ?3 WHERE id = ?4",
                    params![new_seq, dep, now, task_id],
                ).map_err(|e| e.to_string())?;
            }

            Ok(serde_json::json!({
                "status": "ok",
                "work_line_id": line_id,
                "task_id": task_id
            }))
        }
        "schedule_task_on_calendar" => {
            let task_id = args.get("task_id").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing task_id".to_string())?;
            let scheduled_date = args.get("scheduled_date").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing scheduled_date".to_string())?;

            // Validate ISO date format YYYY-MM-DD
            if scheduled_date.len() != 10 || scheduled_date.chars().nth(4) != Some('-') || scheduled_date.chars().nth(7) != Some('-') {
                return Err(format!("Invalid date format '{}', expected ISO YYYY-MM-DD", scheduled_date));
            }

            let slot = args.get("scheduled_slot").and_then(|v| v.as_str()).unwrap_or("MORNING_DEEP");
            let due_date = args.get("due_date").and_then(|v| v.as_str());

            conn.execute(
                "UPDATE work_entities SET scheduled_date = ?1, scheduled_slot = ?2, due_date = ?3, updated_at = ?4 WHERE id = ?5",
                params![scheduled_date, slot, due_date, now, task_id],
            ).map_err(|e| e.to_string())?;

            // Ensure calendar_days record exists for scheduled_date
            conn.execute(
                "INSERT OR IGNORE INTO calendar_days (date_key, total_capacity_minutes, deep_capacity_minutes, day_note, is_off_day, updated_at)
                 VALUES (?1, 300, 150, '', 0, ?2)",
                params![scheduled_date, now],
            ).map_err(|e| e.to_string())?;

            Ok(serde_json::json!({
                "task_id": task_id,
                "scheduled_date": scheduled_date,
                "scheduled_slot": slot,
                "status": "scheduled"
            }))
        }
        "propose_project_lifecycle" => {
            let proposal_type = args.get("proposal_type").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing proposal_type".to_string())?;
            if !matches!(proposal_type, "CREATE_PROJECT" | "PAUSE_PROJECT" | "ARCHIVE_PROJECT" | "PIVOT_PATH") {
                return Err(format!("Invalid proposal_type: {}", proposal_type));
            }

            let title = args.get("title").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing title".to_string())?;
            let rationale = args.get("rationale").and_then(|v| v.as_str())
                .or_else(|| args.get("rationale_markdown").and_then(|v| v.as_str()))
                .unwrap_or("");
            let project_id = args.get("project_id").and_then(|v| v.as_str());
            let payload_json = args.get("payload_json").and_then(|v| v.as_str()).unwrap_or("{}");

            let proposal_id = format!("PRP-{}", Uuid::new_v4().to_string()[..8].to_uppercase());

            conn.execute(
                "INSERT INTO pending_proposals (
                    id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at, rationale, proposed_payload_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'PENDING', ?7, ?5, ?6)",
                params![proposal_id, project_id, proposal_type, title, rationale, payload_json, now],
            ).map_err(|e| e.to_string())?;

            Ok(serde_json::json!({
                "proposal_id": proposal_id,
                "proposal_type": proposal_type,
                "status": "PENDING",
                "title": title
            }))
        }
        "save_personal_or_lesson_item" => {
            let item_type = args.get("item_type").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing item_type".to_string())?;
            if !matches!(item_type, "LESSON_LEARNED" | "INTEREST" | "IDEA" | "BOOKMARK" | "NOTE") {
                return Err(format!("Invalid item_type: {}", item_type));
            }

            let title = args.get("title").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing title".to_string())?;
            let content = args.get("content_markdown").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing content_markdown".to_string())?;
            let realm = args.get("realm").and_then(|v| v.as_str()).unwrap_or("LIFE");
            let domain_code = args.get("domain_code").and_then(|v| v.as_str()).unwrap_or("MIND_LEARNING");
            let project_id = args.get("project_id").and_then(|v| v.as_str());
            let tags_val = args.get("tags").cloned().unwrap_or_else(|| serde_json::json!([]));
            let tags_json = tags_val.to_string();

            let item_id = format!("PVI-{}", Uuid::new_v4().to_string()[..8].to_uppercase());

            conn.execute(
                "INSERT INTO personal_vault_items (
                    id, realm, domain_code, project_id, item_type, title, content_markdown, tags_json, created_at, updated_at, content
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?7)",
                params![item_id, realm, domain_code, project_id, item_type, title, content, tags_json, now],
            ).map_err(|e| e.to_string())?;

            Ok(serde_json::json!({
                "item_id": item_id,
                "item_type": item_type,
                "domain_code": domain_code,
                "title": title
            }))
        }
        "open_in_right_panel" => {
            let panel_type = args.get("panel_type").and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: panel_type".to_string())?;
            if !matches!(panel_type, "DOCUMENT" | "ALL_TASKS_AND_LINES" | "DOMAINS_OUTCOMES" | "CALENDAR" | "DECISIONS" | "VAULT_DIR") {
                return Err(format!("Invalid panel_type: {}", panel_type));
            }
            let target_id = args.get("target_id").and_then(|v| v.as_str());

            Ok(serde_json::json!({
                "navigation_event": {
                    "panel_type": panel_type,
                    "target_id": target_id
                }
            }))
        }
        _ => Err(format!("ابزار ناشناخته: {}", name)),
    }
}

/// Helper to approve or reject a proposal and apply its lifecycle mutation
pub fn resolve_proposal_db(
    conn: &Connection,
    proposal_id: &str,
    action: &str,
) -> Result<(), String> {
    let now = Local::now().to_rfc3339();
    match action {
        "APPROVE" => {
            conn.execute(
                "UPDATE pending_proposals SET status = 'APPROVED', resolved_at = ?1 WHERE id = ?2",
                params![now, proposal_id],
            ).map_err(|e| e.to_string())?;

            // Query proposal details to apply project mutation
            let (proposal_type, project_id): (String, Option<String>) = conn.query_row(
                "SELECT proposal_type, project_id FROM pending_proposals WHERE id = ?1",
                params![proposal_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).map_err(|e| e.to_string())?;

            if let Some(pid) = project_id {
                match proposal_type.as_str() {
                    "PAUSE_PROJECT" => {
                        conn.execute(
                            "UPDATE projects SET lifecycle_status = 'PAUSED', updated_at = ?1 WHERE id = ?2",
                            params![now, pid],
                        ).map_err(|e| e.to_string())?;
                    }
                    "ARCHIVE_PROJECT" => {
                        conn.execute(
                            "UPDATE projects SET lifecycle_status = 'ARCHIVED', updated_at = ?1 WHERE id = ?2",
                            params![now, pid],
                        ).map_err(|e| e.to_string())?;
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        "REJECT" => {
            conn.execute(
                "UPDATE pending_proposals SET status = 'REJECTED', resolved_at = ?1 WHERE id = ?2",
                params![now, proposal_id],
            ).map_err(|e| e.to_string())?;
            Ok(())
        }
        _ => Err(format!("Invalid proposal resolution action: {}", action)),
    }
}

/// 12 standard slash commands defined in Turnkey Specification Blueprint §1.2 & §4.2
pub const SLASH_COMMANDS: &[(&str, &str, &str)] = &[
    ("/focus", "Show #1 highest priority actionable task", "EXECUTION"),
    ("/now", "Filter actionable tasks by current context", "CONTEXT"),
    ("/tasks", "Open Tasks & Lines inspector view", "INSPECTOR"),
    ("/outcomes", "Open Domains & Outcomes inspector view", "INSPECTOR"),
    ("/chain", "Quick-build a sequential WorkLine chain", "PLANNING"),
    ("/calendar", "Open Calendar Day Planner inspector view", "INSPECTOR"),
    ("/decisions", "Open Decisions / ADR directory", "INSPECTOR"),
    ("/docs", "Open Document & Vault explorer", "INSPECTOR"),
    ("/archive", "Archive note or file with twin markdown", "VAULT"),
    ("/life", "Record personal interest or lesson learned", "VAULT"),
    ("/project", "Propose project lifecycle change card", "GOVERNANCE"),
    ("/dopamine", "Gamify current task into 12-minute sprint", "EXECUTION"),
];

// ==============================================================================
// 8. OPAQUE DASHBOARD STATE BUILDER FIXTURE
// ==============================================================================

use mab_compass_lib::models::*;

/// Assembles a complete DashboardState snapshot directly from SQLite tables
/// without crossing the crate boundary with Connection.
pub fn get_dashboard_state_fixture(
    conn: &Connection,
    project_id_override: Option<&str>,
) -> Result<DashboardState> {
    // 1. Query projects
    let mut p_stmt = conn.prepare(
        "SELECT id, name, workspace_path, external_focus_id, is_active, updated_at,
                domain_code, primary_outcome_id, lifecycle_status, scope_mode, realm, priority_weight, pause_resume_note
         FROM projects ORDER BY is_active DESC, id",
    )?;
    let projects: Vec<Project> = p_stmt
        .query_map([], |r| {
            Ok(Project {
                id: r.get(0)?,
                name: r.get(1)?,
                workspace_path: r.get(2).unwrap_or_default(),
                external_focus_id: r.get(3).ok(),
                is_active: r.get::<_, i32>(4).unwrap_or(0) == 1,
                updated_at: r.get(5).unwrap_or_default(),
                domain_code: r.get(6).unwrap_or_else(|_| "WEALTH_FINANCE".to_string()),
                primary_outcome_id: r.get(7).ok(),
                lifecycle_status: r.get(8).unwrap_or_else(|_| "ACTIVE".to_string()),
                scope_mode: r.get(9).unwrap_or_else(|_| "STANDARD".to_string()),
                realm: r.get(10).unwrap_or_else(|_| "WORK".to_string()),
                priority_weight: r.get(11).unwrap_or(5),
                pause_resume_note: r.get(12).unwrap_or_default(),
                ..Default::default()
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let active_project = if let Some(req_id) = project_id_override {
        projects
            .iter()
            .find(|p| p.id == req_id)
            .cloned()
            .unwrap_or_else(|| projects.first().cloned().unwrap_or_default())
    } else {
        projects.iter().find(|p| p.is_active).cloned().unwrap_or_else(|| projects.first().cloned().unwrap_or_default())
    };

    let pid = &active_project.id;

    // 2. Query 8 CADM domains
    let mut d_stmt = conn.prepare(
        "SELECT code, title, description, maturity_state, maintenance_floor, desired_state, is_growth_focus, updated_at
         FROM life_domains ORDER BY code",
    )?;
    let domains: Vec<LifeDomain> = d_stmt
        .query_map([], |r| {
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
        })?
        .filter_map(|r| r.ok())
        .collect();

    // 3. Query outcomes
    let mut o_stmt = conn.prepare(
        "SELECT id, domain_code, title, desired_reality_change, evidence_kr, time_window, status, progress_percent, created_at, updated_at
         FROM outcomes ORDER BY id",
    )?;
    let outcomes: Vec<Outcome> = o_stmt
        .query_map([], |r| {
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
        })?
        .filter_map(|r| r.ok())
        .collect();

    // 4. Query work_lines
    let mut w_stmt = conn.prepare(
        "SELECT id, project_id, outcome_id, title, goal_statement, line_mode, status, color_hex, created_at, updated_at
         FROM work_lines WHERE project_id = ?1 ORDER BY id",
    )?;
    let work_lines: Vec<WorkLine> = w_stmt
        .query_map(params![pid], |r| {
            Ok(WorkLine {
                id: r.get(0)?,
                project_id: r.get(1)?,
                project_name: active_project.name.clone(),
                outcome_id: r.get(2).ok(),
                outcome_title: None,
                title: r.get(3)?,
                goal_statement: r.get(4).unwrap_or_default(),
                line_mode: r.get(5).unwrap_or_else(|_| "SEQUENTIAL_CHAIN".to_string()),
                status: r.get(6).unwrap_or_else(|_| "ACTIVE".to_string()),
                color_hex: r.get(7).unwrap_or_else(|_| "#3B82F6".to_string()),
                total_tasks: 0,
                completed_tasks: 0,
                active_step_id: None,
                created_at: r.get(8).unwrap_or_default(),
                updated_at: r.get(9).unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // 5. Query calendar_days
    let mut c_stmt = conn.prepare(
        "SELECT date_key, total_capacity_minutes, deep_capacity_minutes, day_note, is_off_day, updated_at
         FROM calendar_days ORDER BY date_key",
    )?;
    let calendar_days: Vec<CalendarDay> = c_stmt
        .query_map([], |r| {
            Ok(CalendarDay {
                date_key: r.get(0)?,
                total_capacity_minutes: r.get(1).unwrap_or(300),
                deep_capacity_minutes: r.get(2).unwrap_or(150),
                day_note: r.get(3).unwrap_or_default(),
                is_off_day: r.get::<_, i32>(4).unwrap_or(0) == 1,
                assigned_tasks_count: 0,
                assigned_minutes: 0,
                updated_at: r.get(5).unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // 6. Query pending_proposals
    let mut pr_stmt = conn.prepare(
        "SELECT id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at, resolved_at
         FROM pending_proposals WHERE project_id = ?1 OR project_id IS NULL ORDER BY id DESC",
    )?;
    let pending_proposals: Vec<PendingProposal> = pr_stmt
        .query_map(params![pid], |r| {
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
        })?
        .filter_map(|r| r.ok())
        .collect();

    // 7. Query personal_vault_items
    let mut v_stmt = conn.prepare(
        "SELECT id, realm, domain_code, project_id, item_type, title, content_markdown, tags_json, archived_path, created_at, updated_at
         FROM personal_vault_items WHERE project_id = ?1 OR project_id IS NULL ORDER BY id DESC",
    )?;
    let personal_vault_items: Vec<PersonalVaultItem> = v_stmt
        .query_map(params![pid], |r| {
            let tags_json: String = r.get(7).unwrap_or_else(|_| "[]".to_string());
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(PersonalVaultItem {
                id: r.get(0)?,
                realm: r.get(1).unwrap_or_else(|_| "LIFE".to_string()),
                domain_code: r.get(2).unwrap_or_else(|_| "MIND_LEARNING".to_string()),
                project_id: r.get(3).ok(),
                item_type: r.get(4)?,
                title: r.get(5)?,
                content_markdown: r.get(6).unwrap_or_default(),
                tags_json,
                tags,
                archived_path: r.get(8).ok(),
                created_at: r.get(9).unwrap_or_default(),
                updated_at: r.get(10).unwrap_or_default(),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // 8. Query open tasks
    let mut t_stmt = conn.prepare(
        "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos, calibrated_pomos,
                superseded_by_id, stakeholder_tag, updated_at,
                work_line_id, outcome_id, sequence_order, depends_on_task_id, is_locked_by_chain,
                energy_tag, duration_minutes, device_tag, location_tags_json, scheduled_date, scheduled_slot, due_date, priority_score
         FROM work_entities WHERE project_id = ?1 AND entity_type = 'TASK' AND status NOT IN ('CLOSED', 'SUPERSEDED')
         ORDER BY is_on_critical_path DESC, sequence_order ASC",
    )?;
    let open_tasks: Vec<WorkEntity> = t_stmt
        .query_map(params![pid], |r| {
            let loc_str: String = r.get(24).unwrap_or_else(|_| "[\"DESK\"]".to_string());
            let locs: Vec<String> = serde_json::from_str(&loc_str).unwrap_or_else(|_| vec!["DESK".to_string()]);
            Ok(WorkEntity {
                id: r.get(0)?,
                project_id: r.get(1)?,
                entity_type: r.get(2)?,
                category: r.get(3)?,
                title: r.get(4)?,
                definition_of_done: r.get(5).unwrap_or_default(),
                first_physical_action: r.get(6).unwrap_or_default(),
                status: r.get(7)?,
                is_on_critical_path: r.get::<_, i32>(8).unwrap_or(0) == 1,
                optimistic_pomos: r.get(9).unwrap_or(1.0),
                most_likely_pomos: r.get(10).unwrap_or(2.0),
                pessimistic_pomos: r.get(11).unwrap_or(3.0),
                calibrated_pomos: r.get(12).unwrap_or(2.0),
                superseded_by_id: r.get(13).ok(),
                stakeholder_tag: r.get(14).ok(),
                updated_at: r.get(15).unwrap_or_default(),
                work_line_id: r.get(16).ok(),
                outcome_id: r.get(17).ok(),
                sequence_order: r.get(18).unwrap_or(1),
                depends_on_task_id: r.get(19).ok(),
                is_locked_by_chain: r.get::<_, i32>(20).unwrap_or(0) == 1,
                energy_tag: r.get(21).unwrap_or_else(|_| "MEDIUM_NORMAL".to_string()),
                duration_minutes: r.get(22).unwrap_or(25),
                device_tag: r.get(23).unwrap_or_else(|_| "LAPTOP_ONLY".to_string()),
                location_tags: locs,
                scheduled_date: r.get(25).ok(),
                scheduled_slot: r.get(26).ok(),
                due_date: r.get(27).ok(),
                priority_score: r.get(28).unwrap_or(50),
                ..Default::default()
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let next_step_task = open_tasks.first().cloned();
    let horizon_tasks: Vec<WorkEntity> = open_tasks.iter().skip(1).take(3).cloned().collect();

    // 9. Query active decisions (ADRs)
    let mut adr_stmt = conn.prepare(
        "SELECT id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos, calibrated_pomos,
                superseded_by_id, stakeholder_tag, updated_at
         FROM work_entities WHERE project_id = ?1 AND entity_type = 'DECISION' AND status != 'CLOSED'",
    )?;
    let active_decisions: Vec<WorkEntity> = adr_stmt
        .query_map(params![pid], |r| {
            Ok(WorkEntity {
                id: r.get(0)?,
                project_id: r.get(1)?,
                entity_type: r.get(2)?,
                category: r.get(3)?,
                title: r.get(4)?,
                definition_of_done: r.get(5).unwrap_or_default(),
                first_physical_action: r.get(6).unwrap_or_default(),
                status: r.get(7)?,
                is_on_critical_path: r.get::<_, i32>(8).unwrap_or(0) == 1,
                optimistic_pomos: r.get(9).unwrap_or(1.0),
                most_likely_pomos: r.get(10).unwrap_or(2.0),
                pessimistic_pomos: r.get(11).unwrap_or(3.0),
                calibrated_pomos: r.get(12).unwrap_or(2.0),
                superseded_by_id: r.get(13).ok(),
                stakeholder_tag: r.get(14).ok(),
                updated_at: r.get(15).unwrap_or_default(),
                ..Default::default()
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // 10. Query knowledge documents
    let mut kd_stmt = conn.prepare(
        "SELECT id, project_id, title, doc_type, original_filename, archived_path, executive_summary, extracted_items_count, ingested_at, binary_original_path, tags_json, realm, domain_code
         FROM knowledge_documents WHERE project_id = ?1",
    )?;
    let knowledge_docs: Vec<KnowledgeDocument> = kd_stmt
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
                ..Default::default()
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(DashboardState {
        projects,
        active_project,
        next_step_task,
        horizon_tasks,
        open_tasks,
        active_decisions,
        knowledge_docs,
        domains,
        outcomes,
        work_lines,
        calendar_days,
        pending_proposals,
        personal_vault_items,
        ..Default::default()
    })
}
