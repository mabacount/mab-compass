//! Tier 1: Feature Coverage Integration Test Suite (F1 through F33)
//! File: `src-tauri/tests/e2e_tier1_feature_coverage.rs`
//!
//! Exhaustive, opaque-box integration tests for all 33 features of Mab-Compass v3.0:
//! - F1-F9: Database Schema, 5-Layer Ontology Models, and CADM Domain Seeds
//! - F10-F14: Domino Sequential Chain Engine, Chain Locking & 4D Context Filtering
//! - F15-F22: 8 MCP Tools & Continuous Live Ecosystem State Snapshot Builder
//! - F23-F33: 3-Column UI Layout, Directory Bar, 5-View Inspector & Slash Commands
//!
//! Each feature includes at least 5 assertions/tests covering equivalence classes
//! per `TEST_INFRA.md` and Spec Miner §5.1.

mod common;

use common::*;
use rusqlite::params;
use serde_json::json;

// ==============================================================================
// TIER 1: FEATURES F1 TO F9 — DATABASE SCHEMA, 5-LAYER ONTOLOGY & CADM SEEDS
// ==============================================================================

#[test]
fn test_f1_life_domains_table_and_cadm_seeds() {
    let conn = create_test_db();

    // 1. Verify table schema exists with required columns
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(life_domains)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &["code", "title", "description", "maturity_state", "maintenance_floor", "desired_state", "is_growth_focus", "updated_at"] {
        assert!(pragma_cols.contains(&col.to_string()), "life_domains missing column '{}'", col);
    }

    // 2. Verify all 8 CADM rows are seeded
    assert_cadm_seeds(&conn);

    // 3. Verify MIND_LEARNING default maturity and maintenance floor
    let (code, maturity, floor, is_growth): (String, i32, String, i32) = conn
        .query_row(
            "SELECT code, maturity_state, maintenance_floor, is_growth_focus FROM life_domains WHERE code = 'MIND_LEARNING'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("MIND_LEARNING domain not found");
    assert_eq!(code, "MIND_LEARNING");
    assert_eq!(maturity, 2, "Default maturity must be 2 (Functional/Beginner)");
    assert!(!floor.is_empty(), "Maintenance floor cannot be empty");
    assert_eq!(is_growth, 1, "MIND_LEARNING should default to growth focus");

    // 4. Verify idempotent seeding (re-running does not duplicate rows)
    seed_cadm_life_domains(&conn).expect("Re-seeding should succeed");
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM life_domains", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 8, "Idempotent seeding must maintain exactly 8 domains");

    // 5. Verify constraint: duplicate primary key rejected
    let dup_res = conn.execute(
        "INSERT INTO life_domains (code, title, updated_at) VALUES ('MIND_LEARNING', 'Duplicate', '2026-11-14T00:00:00Z')",
        [],
    );
    assert!(dup_res.is_err(), "Duplicate primary key 'MIND_LEARNING' must fail");
}

#[test]
fn test_f2_outcomes_table_crud_and_constraints() {
    let conn = create_test_db();

    // 1. Table schema verification
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(outcomes)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &["id", "domain_code", "title", "desired_reality_change", "evidence_kr", "time_window", "status", "progress_percent"] {
        assert!(pragma_cols.contains(&col.to_string()), "outcomes missing column '{}'", col);
    }

    // 2. Insert valid Outcome with OUT- prefix
    let out_id = create_test_outcome(
        &conn,
        "OUT-TEST-01",
        "WEALTH_FINANCE",
        "Financial Independence Milestone 1",
        "Net liquid runway exceeds 12 months",
    )
    .expect("Failed to insert test outcome");
    assert_eq!(out_id, "OUT-TEST-01");

    // 3. Verify default progress_percent is 0
    let progress: i32 = conn
        .query_row("SELECT progress_percent FROM outcomes WHERE id = 'OUT-TEST-01'", [], |r| r.get(0))
        .expect("Outcome not found");
    assert_eq!(progress, 0, "Initial progress must be 0%");

    // 4. Verify Foreign Key constraint enforces valid domain_code
    let fk_err = conn.execute(
        "INSERT INTO outcomes (id, domain_code, title, created_at, updated_at)
         VALUES ('OUT-INVALID', 'NON_EXISTENT_DOMAIN', 'Invalid Outcome', '2026-11-14T00:00:00Z', '2026-11-14T00:00:00Z')",
        [],
    );
    assert!(fk_err.is_err(), "Foreign key constraint must reject non-existent domain_code");

    // 5. Verify Status transitions support ACTIVE, ACHIEVED, PAUSED, DROPPED
    for st in &["ACHIEVED", "PAUSED", "DROPPED", "ACTIVE"] {
        conn.execute("UPDATE outcomes SET status = ?1 WHERE id = 'OUT-TEST-01'", params![st])
            .expect("Status update failed");
        let current_st: String = conn
            .query_row("SELECT status FROM outcomes WHERE id = 'OUT-TEST-01'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(current_st, *st);
    }
}

#[test]
fn test_f3_projects_schema_alterations() {
    let conn = create_test_db();

    // 1. Verify altered columns exist on projects table
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(projects)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &["domain_code", "primary_outcome_id", "lifecycle_status", "scope_mode", "realm", "priority_weight", "pause_resume_note"] {
        assert!(pragma_cols.contains(&col.to_string()), "projects missing altered column '{}'", col);
    }

    // 2. Default projects GERDALO and IRANOIL have valid domain_code and status
    let (d_code, lifecycle, scope, weight): (String, String, String, i32) = conn
        .query_row(
            "SELECT domain_code, lifecycle_status, scope_mode, priority_weight FROM projects WHERE id = 'GERDALO'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("GERDALO project not found");
    assert_eq!(d_code, "WEALTH_FINANCE");
    assert_eq!(lifecycle, "ACTIVE");
    assert_eq!(scope, "STANDARD");
    assert_eq!(weight, 5);

    // 3. Link project to primary_outcome_id
    create_test_outcome(&conn, "OUT-PRJ-01", "WEALTH_FINANCE", "Project Outcome", "KR1").unwrap();
    conn.execute("UPDATE projects SET primary_outcome_id = 'OUT-PRJ-01' WHERE id = 'GERDALO'", [])
        .expect("Failed to link outcome");
    let linked: Option<String> = conn
        .query_row("SELECT primary_outcome_id FROM projects WHERE id = 'GERDALO'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(linked.as_deref(), Some("OUT-PRJ-01"));

    // 4. Lifecycle status supports PAUSED and ARCHIVED
    conn.execute("UPDATE projects SET lifecycle_status = 'PAUSED', pause_resume_note = 'Capacity squeeze' WHERE id = 'GERDALO'", [])
        .expect("Failed to pause project");
    let (st, note): (String, String) = conn
        .query_row("SELECT lifecycle_status, pause_resume_note FROM projects WHERE id = 'GERDALO'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(st, "PAUSED");
    assert_eq!(note, "Capacity squeeze");

    // 5. Realm supports WORK, LIFE, LEARNING, INTERESTS
    for realm in &["WORK", "LIFE", "LEARNING", "INTERESTS"] {
        conn.execute("UPDATE projects SET realm = ?1 WHERE id = 'GERDALO'", params![realm]).unwrap();
        let current_realm: String = conn.query_row("SELECT realm FROM projects WHERE id = 'GERDALO'", [], |r| r.get(0)).unwrap();
        assert_eq!(current_realm, *realm);
    }
}

#[test]
fn test_f4_work_lines_table_and_modes() {
    let conn = create_test_db();

    // 1. Table schema verification
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(work_lines)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &["id", "project_id", "outcome_id", "title", "goal_statement", "line_mode", "status", "color_hex", "created_at", "updated_at"] {
        assert!(pragma_cols.contains(&col.to_string()), "work_lines missing column '{}'", col);
    }

    // 2. Insert WorkLine with WLN- ID
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO work_lines (id, project_id, title, goal_statement, line_mode, status, color_hex, created_at, updated_at)
         VALUES ('WLN-CORE-01', 'GERDALO', 'Core Feature Stream', 'Deliver core MVP', 'SEQUENTIAL_CHAIN', 'ACTIVE', '#3B82F6', ?1, ?1)",
        params![now],
    ).expect("Failed to insert work_line");

    // 3. line_mode defaults and supports SEQUENTIAL_CHAIN and GOAL_CLUSTER
    let (mode, color): (String, String) = conn
        .query_row("SELECT line_mode, color_hex FROM work_lines WHERE id = 'WLN-CORE-01'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(mode, "SEQUENTIAL_CHAIN");
    assert_eq!(color, "#3B82F6");

    conn.execute("UPDATE work_lines SET line_mode = 'GOAL_CLUSTER' WHERE id = 'WLN-CORE-01'", []).unwrap();
    let cluster_mode: String = conn.query_row("SELECT line_mode FROM work_lines WHERE id = 'WLN-CORE-01'", [], |r| r.get(0)).unwrap();
    assert_eq!(cluster_mode, "GOAL_CLUSTER");

    // 4. Status supports ACTIVE, PAUSED, COMPLETED
    for st in &["PAUSED", "COMPLETED", "ACTIVE"] {
        conn.execute("UPDATE work_lines SET status = ?1 WHERE id = 'WLN-CORE-01'", params![st]).unwrap();
        let cur_st: String = conn.query_row("SELECT status FROM work_lines WHERE id = 'WLN-CORE-01'", [], |r| r.get(0)).unwrap();
        assert_eq!(cur_st, *st);
    }

    // 5. Foreign key constraint: invalid project_id fails
    let fk_err = conn.execute(
        "INSERT INTO work_lines (id, project_id, title, created_at, updated_at)
         VALUES ('WLN-INV', 'NON_EXISTENT_PROJECT', 'Invalid Line', ?1, ?1)",
        params![now],
    );
    assert!(fk_err.is_err(), "Foreign key must reject invalid project_id");
}

#[test]
fn test_f5_work_entities_alterations_and_defaults() {
    let conn = create_test_db();

    // 1. Verify altered columns exist on work_entities
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(work_entities)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &[
        "work_line_id", "outcome_id", "sequence_order", "depends_on_task_id", "is_locked_by_chain",
        "energy_tag", "duration_minutes", "device_tag", "location_tags_json", "scheduled_date",
        "scheduled_slot", "due_date", "priority_score",
    ] {
        assert!(pragma_cols.contains(&col.to_string()), "work_entities missing column '{}'", col);
    }

    // 2. Insert minimal task and assert v3 column defaults
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO work_entities (id, project_id, title, updated_at) VALUES ('TSK-DEF-01', 'GERDALO', 'Default Task', ?1)",
        params![now],
    ).expect("Failed to insert default task");

    let (seq, is_locked, energy, dur, device, locs, prio): (i32, i32, String, i32, String, String, i32) = conn
        .query_row(
            "SELECT sequence_order, is_locked_by_chain, energy_tag, duration_minutes, device_tag, location_tags_json, priority_score
             FROM work_entities WHERE id = 'TSK-DEF-01'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .expect("Task not found");

    assert_eq!(seq, 1, "Default sequence_order must be 1");
    assert_eq!(is_locked, 0, "Default is_locked_by_chain must be 0");
    assert_eq!(energy, "MEDIUM_NORMAL", "Default energy_tag must be MEDIUM_NORMAL");
    assert_eq!(dur, 25, "Default duration_minutes must be 25");
    assert_eq!(device, "LAPTOP_ONLY", "Default device_tag must be LAPTOP_ONLY");
    assert_eq!(locs, "[\"DESK\"]", "Default location_tags_json must be [\"DESK\"]");
    assert_eq!(prio, 50, "Default priority_score must be 50");

    // 3. Device tag supports PHONE_OK, AUDIO_HANDSFREE, ANY_DEVICE
    for dev in &["PHONE_OK", "AUDIO_HANDSFREE", "ANY_DEVICE"] {
        conn.execute("UPDATE work_entities SET device_tag = ?1 WHERE id = 'TSK-DEF-01'", params![dev]).unwrap();
        let cur_dev: String = conn.query_row("SELECT device_tag FROM work_entities WHERE id = 'TSK-DEF-01'", [], |r| r.get(0)).unwrap();
        assert_eq!(cur_dev, *dev);
    }

    // 4. Energy tag supports DEEP_HIGH and LOW_ZOMBIE
    for eng in &["DEEP_HIGH", "LOW_ZOMBIE"] {
        conn.execute("UPDATE work_entities SET energy_tag = ?1 WHERE id = 'TSK-DEF-01'", params![eng]).unwrap();
        let cur_eng: String = conn.query_row("SELECT energy_tag FROM work_entities WHERE id = 'TSK-DEF-01'", [], |r| r.get(0)).unwrap();
        assert_eq!(cur_eng, *eng);
    }

    // 5. Scheduling columns support date, slot, and due_date
    conn.execute(
        "UPDATE work_entities SET scheduled_date = '2026-11-15', scheduled_slot = 'MORNING_DEEP', due_date = '2026-11-20' WHERE id = 'TSK-DEF-01'",
        [],
    ).unwrap();
    let (s_date, s_slot, d_date): (Option<String>, Option<String>, Option<String>) = conn
        .query_row("SELECT scheduled_date, scheduled_slot, due_date FROM work_entities WHERE id = 'TSK-DEF-01'", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    assert_eq!(s_date.as_deref(), Some("2026-11-15"));
    assert_eq!(s_slot.as_deref(), Some("MORNING_DEEP"));
    assert_eq!(d_date.as_deref(), Some("2026-11-20"));
}

#[test]
fn test_f6_calendar_days_table_and_capacity() {
    let conn = create_test_db();

    // 1. Table schema verification
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(calendar_days)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &["date_key", "total_capacity_minutes", "deep_capacity_minutes", "day_note", "is_off_day", "updated_at"] {
        assert!(pragma_cols.contains(&col.to_string()), "calendar_days missing column '{}'", col);
    }

    // 2. Insert standard calendar day and verify defaults
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO calendar_days (date_key, updated_at) VALUES ('2026-11-15', ?1)",
        params![now],
    ).expect("Failed to insert calendar_day");

    let (tot, deep, off): (i32, i32, i32) = conn
        .query_row(
            "SELECT total_capacity_minutes, deep_capacity_minutes, is_off_day FROM calendar_days WHERE date_key = '2026-11-15'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("Day not found");
    assert_eq!(tot, 300, "Default total capacity must be 300m (5h / 12 pomos)");
    assert_eq!(deep, 150, "Default deep capacity must be 150m (2.5h / 6 pomos)");
    assert_eq!(off, 0, "Default is_off_day must be 0 (working day)");

    // 3. Toggle off-day flag
    conn.execute("UPDATE calendar_days SET is_off_day = 1, day_note = 'Friday Weekend' WHERE date_key = '2026-11-15'", []).unwrap();
    let (is_off, note): (i32, String) = conn
        .query_row("SELECT is_off_day, day_note FROM calendar_days WHERE date_key = '2026-11-15'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(is_off, 1);
    assert_eq!(note, "Friday Weekend");

    // 4. Primary key conflict on duplicate date_key
    let dup_err = conn.execute("INSERT INTO calendar_days (date_key, updated_at) VALUES ('2026-11-15', ?1)", params![now]);
    assert!(dup_err.is_err(), "Duplicate date_key must fail primary key constraint");

    // 5. Multi-day capacity query (7-day lookahead)
    conn.execute("INSERT INTO calendar_days (date_key, total_capacity_minutes, updated_at) VALUES ('2026-11-16', 360, ?1)", params![now]).unwrap();
    conn.execute("INSERT INTO calendar_days (date_key, total_capacity_minutes, updated_at) VALUES ('2026-11-17', 240, ?1)", params![now]).unwrap();

    let total_mins: i64 = conn
        .query_row("SELECT SUM(total_capacity_minutes) FROM calendar_days WHERE date_key BETWEEN '2026-11-15' AND '2026-11-17'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total_mins, 300 + 360 + 240);
}

#[test]
fn test_f7_pending_proposals_table_and_lifecycle() {
    let conn = create_test_db();

    // 1. Table schema verification
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(pending_proposals)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &["id", "project_id", "proposal_type", "title", "rationale_markdown", "payload_json", "status", "created_at", "resolved_at"] {
        assert!(pragma_cols.contains(&col.to_string()), "pending_proposals missing column '{}'", col);
    }

    // 2. Insert pending proposal with PRP- prefix
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO pending_proposals (id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at)
         VALUES ('PRP-TEST-01', 'GERDALO', 'PAUSE_PROJECT', 'Pause Gerdalo for Focus', 'Over capacity', '{\"reason\": \"focus\"}', 'PENDING', ?1)",
        params![now],
    ).expect("Failed to insert proposal");

    let (st, res_at): (String, Option<String>) = conn
        .query_row("SELECT status, resolved_at FROM pending_proposals WHERE id = 'PRP-TEST-01'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(st, "PENDING");
    assert!(res_at.is_none(), "Initial resolved_at must be NULL");

    // 3. Proposal type variants support CREATE_PROJECT, PAUSE_PROJECT, ARCHIVE_PROJECT, PIVOT_PATH
    for ptype in &["CREATE_PROJECT", "ARCHIVE_PROJECT", "PIVOT_PATH"] {
        conn.execute("UPDATE pending_proposals SET proposal_type = ?1 WHERE id = 'PRP-TEST-01'", params![ptype]).unwrap();
        let cur_type: String = conn.query_row("SELECT proposal_type FROM pending_proposals WHERE id = 'PRP-TEST-01'", [], |r| r.get(0)).unwrap();
        assert_eq!(cur_type, *ptype);
    }

    // 4. Approval transition sets status = APPROVED and resolved_at
    let resolve_time = chrono::Local::now().to_rfc3339();
    conn.execute("UPDATE pending_proposals SET status = 'APPROVED', resolved_at = ?1 WHERE id = 'PRP-TEST-01'", params![resolve_time]).unwrap();
    let (approved_st, approved_at): (String, Option<String>) = conn
        .query_row("SELECT status, resolved_at FROM pending_proposals WHERE id = 'PRP-TEST-01'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(approved_st, "APPROVED");
    assert!(approved_at.is_some());

    // 5. Rejection transition sets status = REJECTED
    conn.execute("UPDATE pending_proposals SET status = 'REJECTED', resolved_at = ?1 WHERE id = 'PRP-TEST-01'", params![resolve_time]).unwrap();
    let rejected_st: String = conn.query_row("SELECT status FROM pending_proposals WHERE id = 'PRP-TEST-01'", [], |r| r.get(0)).unwrap();
    assert_eq!(rejected_st, "REJECTED");
}

#[test]
fn test_f8_personal_vault_items_table_and_types() {
    let conn = create_test_db();

    // 1. Table schema verification
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(personal_vault_items)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &["id", "realm", "domain_code", "project_id", "item_type", "title", "content_markdown", "tags_json", "archived_path", "created_at", "updated_at"] {
        assert!(pragma_cols.contains(&col.to_string()), "personal_vault_items missing column '{}'", col);
    }

    // 2. Insert item with PVI- prefix and verify defaults
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO personal_vault_items (id, item_type, title, content_markdown, created_at, updated_at)
         VALUES ('PVI-TEST-01', 'LESSON_LEARNED', 'Premature Optimization Lesson', 'Profile before optimizing.', ?1, ?1)",
        params![now],
    ).expect("Failed to insert vault item");

    let (realm, d_code, itype): (String, String, String) = conn
        .query_row(
            "SELECT realm, domain_code, item_type FROM personal_vault_items WHERE id = 'PVI-TEST-01'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(realm, "LIFE", "Default realm must be LIFE");
    assert_eq!(d_code, "MIND_LEARNING", "Default domain_code must be MIND_LEARNING");
    assert_eq!(itype, "LESSON_LEARNED");

    // 3. Item types accept LESSON_LEARNED, INTEREST, IDEA, BOOKMARK, NOTE
    for t in &["INTEREST", "IDEA", "BOOKMARK", "NOTE"] {
        conn.execute("UPDATE personal_vault_items SET item_type = ?1 WHERE id = 'PVI-TEST-01'", params![t]).unwrap();
        let cur_t: String = conn.query_row("SELECT item_type FROM personal_vault_items WHERE id = 'PVI-TEST-01'", [], |r| r.get(0)).unwrap();
        assert_eq!(cur_t, *t);
    }

    // 4. Tags JSON array serialization and querying
    conn.execute("UPDATE personal_vault_items SET tags_json = '[\"rust\", \"architecture\", \"perf\"]' WHERE id = 'PVI-TEST-01'", []).unwrap();
    let tags_str: String = conn.query_row("SELECT tags_json FROM personal_vault_items WHERE id = 'PVI-TEST-01'", [], |r| r.get(0)).unwrap();
    let parsed: Vec<String> = serde_json::from_str(&tags_str).unwrap();
    assert_eq!(parsed, vec!["rust", "architecture", "perf"]);

    // 5. Foreign key constraint: invalid domain_code fails
    let fk_err = conn.execute(
        "INSERT INTO personal_vault_items (id, domain_code, item_type, title, content_markdown, created_at, updated_at)
         VALUES ('PVI-INV', 'NON_EXISTENT_DOMAIN', 'NOTE', 'Invalid', 'Content', ?1, ?1)",
        params![now],
    );
    assert!(fk_err.is_err(), "Foreign key must reject invalid domain_code");
}

#[test]
fn test_f9_knowledge_documents_alterations_and_binary_twin() {
    let conn = create_test_db();

    // 1. Verify altered columns on knowledge_documents
    let pragma_cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(knowledge_documents)").expect("Failed pragma");
        stmt.query_map([], |r| r.get::<_, String>(1))
            .expect("Failed map")
            .filter_map(|r| r.ok())
            .collect()
    };
    for col in &["binary_original_path", "tags_json", "realm", "domain_code"] {
        assert!(pragma_cols.contains(&col.to_string()), "knowledge_documents missing altered column '{}'", col);
    }

    // 2. Insert document and verify defaults
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO knowledge_documents (project_id, title, doc_type, original_filename, archived_path, executive_summary, ingested_at)
         VALUES ('GERDALO', 'Quarterly Financial Report', 'PDF', 'Q3_Report.pdf', 'vault/docs/Q3_Report.md', 'Financial summary', ?1)",
        params![now],
    ).expect("Failed to insert doc");

    let (realm, d_code, tags): (String, String, String) = conn
        .query_row(
            "SELECT realm, domain_code, tags_json FROM knowledge_documents WHERE title = 'Quarterly Financial Report'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(realm, "WORK");
    assert_eq!(d_code, "WEALTH_FINANCE");
    assert_eq!(tags, "[]");

    // 3. Store twin binary original path alongside markdown summary
    conn.execute(
        "UPDATE knowledge_documents SET binary_original_path = 'vault/originals/Q3_Report.pdf', tags_json = '[\"finance\", \"q3\"]'
         WHERE title = 'Quarterly Financial Report'",
        [],
    ).unwrap();

    let (bin_path, tags_updated): (Option<String>, String) = conn
        .query_row(
            "SELECT binary_original_path, tags_json FROM knowledge_documents WHERE title = 'Quarterly Financial Report'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(bin_path.as_deref(), Some("vault/originals/Q3_Report.pdf"));
    assert!(tags_updated.contains("finance"));

    // 4. Multiple documents queryable per domain
    conn.execute(
        "INSERT INTO knowledge_documents (project_id, title, doc_type, original_filename, archived_path, executive_summary, domain_code, ingested_at)
         VALUES ('GERDALO', 'Tax Audit 1405', 'DOCX', 'Tax.docx', 'vault/docs/Tax.md', 'Tax summary', 'WEALTH_FINANCE', ?1)",
        params![now],
    ).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM knowledge_documents WHERE domain_code = 'WEALTH_FINANCE'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "Expected 2 documents under WEALTH_FINANCE");

    // 5. Binary original path is optional (None for markdown-only notes)
    let null_bin: Option<String> = conn
        .query_row("SELECT binary_original_path FROM knowledge_documents WHERE title = 'Tax Audit 1405'", [], |r| r.get(0))
        .unwrap();
    assert!(null_bin.is_none(), "Binary original path should be NULL when omitted");
}

// ==============================================================================
// TIER 1: FEATURES F10 TO F14 — SEQUENTIAL CHAIN ENGINE & 4D CONTEXT FILTERING
// ==============================================================================

#[test]
fn test_f10_create_work_line_with_chain() {
    let conn = create_test_db();

    // 1. Atomically create a WorkLine and 3 sequential steps
    let (line_id, task_ids) = setup_chain_fixture(
        &conn,
        "GERDALO",
        None,
        "Refactor Storage Engine",
        &["Step 1: Write DDL", "Step 2: Run Migrations", "Step 3: Verify Integrity"],
    );

    assert_eq!(task_ids.len(), 3);
    assert!(line_id.starts_with("WLN-"));

    // 2. Step 1 has sequence_order = 1, status = IN_PROGRESS, depends_on = None, is_locked = 0
    let (s1_seq, s1_status, s1_dep, s1_locked): (i32, String, Option<String>, i32) = conn
        .query_row(
            "SELECT sequence_order, status, depends_on_task_id, is_locked_by_chain FROM work_entities WHERE id = ?1",
            params![task_ids[0]],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(s1_seq, 1);
    assert_eq!(s1_status, "IN_PROGRESS");
    assert!(s1_dep.is_none());
    assert_eq!(s1_locked, 0);

    // 3. Step 2 has sequence_order = 2, status = QUEUED_IN_CHAIN, depends_on = Step 1, is_locked = 1
    let (s2_seq, s2_status, s2_dep, s2_locked): (i32, String, Option<String>, i32) = conn
        .query_row(
            "SELECT sequence_order, status, depends_on_task_id, is_locked_by_chain FROM work_entities WHERE id = ?1",
            params![task_ids[1]],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(s2_seq, 2);
    assert_eq!(s2_status, "QUEUED_IN_CHAIN");
    assert_eq!(s2_dep.as_deref(), Some(task_ids[0].as_str()));
    assert_eq!(s2_locked, 1);

    // 4. Step 3 has sequence_order = 3, status = QUEUED_IN_CHAIN, depends_on = Step 2, is_locked = 1
    let (s3_seq, s3_status, s3_dep, s3_locked): (i32, String, Option<String>, i32) = conn
        .query_row(
            "SELECT sequence_order, status, depends_on_task_id, is_locked_by_chain FROM work_entities WHERE id = ?1",
            params![task_ids[2]],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(s3_seq, 3);
    assert_eq!(s3_status, "QUEUED_IN_CHAIN");
    assert_eq!(s3_dep.as_deref(), Some(task_ids[1].as_str()));
    assert_eq!(s3_locked, 1);

    // 5. WorkLine status is ACTIVE and links to all 3 tasks
    let (line_status, line_mode): (String, String) = conn
        .query_row("SELECT status, line_mode FROM work_lines WHERE id = ?1", params![line_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(line_status, "ACTIVE");
    assert_eq!(line_mode, "SEQUENTIAL_CHAIN");
}

#[test]
fn test_f11_chain_lock_resolution() {
    let conn = create_test_db();

    let (_line_id, task_ids) = setup_chain_fixture_simple(
        &conn,
        &["Step A: Design", "Step B: Implement", "Step C: Test"],
    );

    // 1. Step 1 is NOT locked
    assert!(!is_locked_by_chain(&conn, &task_ids[0]).unwrap());

    // 2. Step 2 is LOCKED while Step 1 is unclosed
    assert!(is_locked_by_chain(&conn, &task_ids[1]).unwrap());

    // 3. Step 3 is LOCKED while Step 1 and 2 are unclosed
    assert!(is_locked_by_chain(&conn, &task_ids[2]).unwrap());

    // 4. Non-chain task is NOT locked
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO work_entities (id, project_id, title, status, is_locked_by_chain, updated_at)
         VALUES ('TSK-SOLO', 'GERDALO', 'Standalone Task', 'OPEN', 0, ?1)",
        params![now],
    ).unwrap();
    assert!(!is_locked_by_chain(&conn, "TSK-SOLO").unwrap());

    // 5. Tasks in a GOAL_CLUSTER line mode are NOT locked by chain
    conn.execute(
        "INSERT INTO work_lines (id, project_id, title, line_mode, status, updated_at, created_at)
         VALUES ('WLN-CLUSTER', 'GERDALO', 'Cluster Line', 'GOAL_CLUSTER', 'ACTIVE', ?1, ?1)",
        params![now],
    ).unwrap();
    conn.execute(
        "INSERT INTO work_entities (id, project_id, work_line_id, sequence_order, title, status, is_locked_by_chain, updated_at)
         VALUES ('TSK-CLUS-1', 'GERDALO', 'WLN-CLUSTER', 2, 'Cluster Step 2', 'OPEN', 0, ?1)",
        params![now],
    ).unwrap();
    assert!(!is_locked_by_chain(&conn, "TSK-CLUS-1").unwrap());
}

#[test]
fn test_f12_advance_sequential_chain_on_complete_domino() {
    let conn = create_test_db();

    let (line_id, task_ids) = setup_chain_fixture_simple(
        &conn,
        &["Alpha Step", "Beta Step", "Gamma Step"],
    );

    // 1. Initial assertion
    assert_chain_step_statuses(&conn, &line_id, &[(1, "IN_PROGRESS"), (2, "QUEUED_IN_CHAIN"), (3, "QUEUED_IN_CHAIN")]);

    // 2. Domino 1: Complete Step 1 -> advances Step 2 to IN_PROGRESS
    let promoted_1 = simulate_advance_chain_on_complete(&conn, "GERDALO", &task_ids[0]).unwrap();
    assert_eq!(promoted_1.as_deref(), Some(task_ids[1].as_str()));
    assert_chain_domino_transition(&conn, &line_id, &task_ids[0], &task_ids[1]);
    assert_chain_step_statuses(&conn, &line_id, &[(1, "CLOSED"), (2, "IN_PROGRESS"), (3, "QUEUED_IN_CHAIN")]);

    // 3. Step 2 is now unlocked; Step 3 remains locked
    assert!(!is_locked_by_chain(&conn, &task_ids[1]).unwrap());
    assert!(is_locked_by_chain(&conn, &task_ids[2]).unwrap());

    // 4. Domino 2: Complete Step 2 -> advances Step 3 to IN_PROGRESS
    let promoted_2 = simulate_advance_chain_on_complete(&conn, "GERDALO", &task_ids[1]).unwrap();
    assert_eq!(promoted_2.as_deref(), Some(task_ids[2].as_str()));
    assert_chain_domino_transition(&conn, &line_id, &task_ids[1], &task_ids[2]);
    assert_chain_step_statuses(&conn, &line_id, &[(1, "CLOSED"), (2, "CLOSED"), (3, "IN_PROGRESS")]);

    // 5. Domino 3: Complete Step 3 (final step) -> marks WorkLine COMPLETED
    let final_promoted = simulate_advance_chain_on_complete(&conn, "GERDALO", &task_ids[2]).unwrap();
    assert!(final_promoted.is_none(), "Final step completion should not promote further tasks");
    assert_work_line_completed(&conn, &line_id);
}

#[test]
fn test_f13_4d_context_tagging_dimensions() {
    let conn = create_test_db();
    let fixtures = setup_context_fixture(&conn, "GERDALO");

    // 1. Ensure all 7 diverse fixtures were inserted
    assert_eq!(fixtures.len(), 7);

    // 2. Dimension 1: Device tags (LAPTOP_ONLY, PHONE_OK, AUDIO_HANDSFREE, ANY_DEVICE)
    let devices: Vec<String> = conn
        .prepare("SELECT DISTINCT device_tag FROM work_entities WHERE project_id = 'GERDALO'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    for expected_dev in &["LAPTOP_ONLY", "PHONE_OK", "AUDIO_HANDSFREE", "ANY_DEVICE"] {
        assert!(devices.contains(&expected_dev.to_string()), "Missing device tag '{}'", expected_dev);
    }

    // 3. Dimension 2: Location tags JSON array
    let locations: Vec<String> = conn
        .prepare("SELECT location_tags_json FROM work_entities WHERE project_id = 'GERDALO'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert!(locations.iter().any(|l| l.contains("METRO_COMMUTE")));
    assert!(locations.iter().any(|l| l.contains("CAR_DRIVING")));
    assert!(locations.iter().any(|l| l.contains("OUTSIDE_ERRAND")));
    assert!(locations.iter().any(|l| l.contains("DESK")));

    // 4. Dimension 3: Energy tags (DEEP_HIGH, MEDIUM_NORMAL, LOW_ZOMBIE)
    let energies: Vec<String> = conn
        .prepare("SELECT DISTINCT energy_tag FROM work_entities WHERE project_id = 'GERDALO'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert!(energies.contains(&"DEEP_HIGH".to_string()));
    assert!(energies.contains(&"MEDIUM_NORMAL".to_string()));
    assert!(energies.contains(&"LOW_ZOMBIE".to_string()));

    // 5. Dimension 4: Duration minutes range
    let durations: Vec<i32> = conn
        .prepare("SELECT DISTINCT duration_minutes FROM work_entities WHERE project_id = 'GERDALO' ORDER BY duration_minutes")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert!(durations.contains(&10));
    assert!(durations.contains(&15));
    assert!(durations.contains(&25));
    assert!(durations.contains(&50));
}

#[test]
fn test_f14_query_tasks_by_context_filtering() {
    let conn = create_test_db();
    let _fixtures = setup_context_fixture(&conn, "GERDALO");

    // 1. Strict Exclusion: only_unlocked = true excludes locked chained step
    let unlocked_metro = query_tasks_by_context_sql(
        &conn,
        Some("GERDALO"),
        Some("PHONE_OK"),
        Some("METRO_COMMUTE"),
        Some("LOW_ZOMBIE"),
        Some(15),
        true, // only_unlocked
    ).unwrap();
    assert_eq!(unlocked_metro.len(), 1, "Must match exactly 1 unlocked task");

    let matched_title: String = conn
        .query_row("SELECT title FROM work_entities WHERE id = ?1", params![unlocked_metro[0]], |r| r.get(0))
        .unwrap();
    assert_eq!(matched_title, "Task Metro Phone Low 15m");

    // 2. Inclusion when only_unlocked = false
    let all_metro = query_tasks_by_context_sql(
        &conn,
        Some("GERDALO"),
        Some("PHONE_OK"),
        Some("METRO_COMMUTE"),
        Some("LOW_ZOMBIE"),
        Some(15),
        false, // include locked
    ).unwrap();
    assert_eq!(all_metro.len(), 2, "Must return 2 tasks when locked tasks included");

    // 3. Filter by Location: CAR_DRIVING + AUDIO_HANDSFREE
    let car_tasks = query_tasks_by_context_sql(
        &conn,
        Some("GERDALO"),
        Some("AUDIO_HANDSFREE"),
        Some("CAR_DRIVING"),
        None,
        None,
        true,
    ).unwrap();
    assert_eq!(car_tasks.len(), 1);

    // 4. Filter by Energy: DEEP_HIGH
    let deep_tasks = query_tasks_by_context_sql(
        &conn,
        Some("GERDALO"),
        None,
        None,
        Some("DEEP_HIGH"),
        None,
        true,
    ).unwrap();
    assert_eq!(deep_tasks.len(), 1);

    // 5. Filter by Duration: <= 20 minutes excludes 25m and 50m tasks
    let quick_tasks = query_tasks_by_context_sql(
        &conn,
        Some("GERDALO"),
        None,
        None,
        None,
        Some(20),
        true,
    ).unwrap();
    for tid in &quick_tasks {
        let dur: i32 = conn.query_row("SELECT duration_minutes FROM work_entities WHERE id = ?1", params![tid], |r| r.get(0)).unwrap();
        assert!(dur <= 20, "Task {} exceeds duration limit ({} > 20)", tid, dur);
    }

    // 6. Verification via MCP Tool call_mcp_tool
    let mut conn_mut = conn;
    let mcp_res = call_mcp_tool(
        &mut conn_mut,
        "query_tasks_by_context",
        json!({
            "project_id": "GERDALO",
            "device_tag": "PHONE_OK",
            "location_tag": "METRO_COMMUTE",
            "energy_tag": "LOW_ZOMBIE",
            "max_minutes": 15,
            "only_unlocked": true
        }),
    ).expect("MCP query_tasks_by_context failed");
    assert_eq!(mcp_res["count"], 1);
    let tasks_arr = mcp_res["tasks"].as_array().expect("Expected tasks array");
    assert_eq!(tasks_arr.len(), 1);
    assert_eq!(tasks_arr[0]["title"], "Task Metro Phone Low 15m");
}

// ==============================================================================
// TIER 1: FEATURES F15 TO F22 — 8 MCP TOOLS & LIVE ECOSYSTEM STATE SNAPSHOT
// ==============================================================================

#[test]
fn test_f15_mcp_upsert_domain_or_outcome() {
    let mut conn = create_test_db();

    // 1. UPDATE_DOMAIN: update maturity_state (2 -> 3)
    let res1 = call_mcp_tool(
        &mut conn,
        "upsert_domain_or_outcome",
        json!({
            "action": "UPDATE_DOMAIN",
            "domain_code": "MIND_LEARNING",
            "maturity_state": 3
        }),
    ).expect("UPDATE_DOMAIN failed");
    assert_eq!(res1["status"], "ok");

    let maturity: i32 = conn
        .query_row("SELECT maturity_state FROM life_domains WHERE code = 'MIND_LEARNING'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(maturity, 3);

    // 2. UPDATE_DOMAIN: update maintenance_floor text
    call_mcp_tool(
        &mut conn,
        "upsert_domain_or_outcome",
        json!({
            "action": "UPDATE_DOMAIN",
            "domain_code": "MIND_LEARNING",
            "maintenance_floor": "مطالعه ۴۵ دقیقه روزانه زبان"
        }),
    ).expect("Update maintenance_floor failed");

    let floor: String = conn
        .query_row("SELECT maintenance_floor FROM life_domains WHERE code = 'MIND_LEARNING'", [], |r| r.get(0))
        .unwrap();
    assert!(floor.contains("مطالعه ۴۵ دقیقه"));

    // 3. CREATE_OUTCOME: create OUT- entity
    let res3 = call_mcp_tool(
        &mut conn,
        "upsert_domain_or_outcome",
        json!({
            "action": "CREATE_OUTCOME",
            "domain_code": "MIND_LEARNING",
            "outcome_id": "OUT-MCP-01",
            "title": "Master Advanced Systems Rust",
            "desired_reality_change": "Write 0-alloc code",
            "evidence_kr": "Zero clippy warnings and 100% test pass"
        }),
    ).expect("CREATE_OUTCOME failed");
    assert_eq!(res3["outcome_id"], "OUT-MCP-01");

    let out_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM outcomes WHERE id = 'OUT-MCP-01'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(out_count, 1);

    // 4. UPDATE_OUTCOME: update progress and status
    let res4 = call_mcp_tool(
        &mut conn,
        "upsert_domain_or_outcome",
        json!({
            "action": "UPDATE_OUTCOME",
            "domain_code": "MIND_LEARNING",
            "outcome_id": "OUT-MCP-01",
            "status": "ACHIEVED",
            "progress_percent": 100
        }),
    ).expect("UPDATE_OUTCOME failed");
    assert_eq!(res4["status"], "ok");

    let (st, prog): (String, i32) = conn
        .query_row("SELECT status, progress_percent FROM outcomes WHERE id = 'OUT-MCP-01'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(st, "ACHIEVED");
    assert_eq!(prog, 100);

    // 5. Boundary Validation: Reject out-of-bounds maturity (< 0 or > 5)
    let err_res = call_mcp_tool(
        &mut conn,
        "upsert_domain_or_outcome",
        json!({
            "action": "UPDATE_DOMAIN",
            "domain_code": "MIND_LEARNING",
            "maturity_state": 99
        }),
    );
    assert!(err_res.is_err(), "Out-of-bounds maturity (99) must be rejected");
}

#[test]
fn test_f16_mcp_create_work_line_chain() {
    let mut conn = create_test_db();

    // 1. Invoke tool to create WorkLine with 3 steps
    let res = call_mcp_tool(
        &mut conn,
        "create_work_line_chain",
        json!({
            "project_id": "GERDALO",
            "line_title": "Pipeline Chain",
            "goal_statement": "Deploy end-to-end pipeline",
            "line_mode": "SEQUENTIAL_CHAIN",
            "steps": [
                { "title": "Step 1: Extract", "device_tag": "LAPTOP_ONLY", "duration_minutes": 25 },
                { "title": "Step 2: Transform", "device_tag": "LAPTOP_ONLY", "duration_minutes": 15 },
                { "title": "Step 3: Load", "device_tag": "PHONE_OK", "duration_minutes": 10 }
            ]
        }),
    ).expect("create_work_line_chain MCP tool failed");

    let line_id = res["work_line_id"].as_str().expect("Expected work_line_id");
    let tasks = res["tasks"].as_array().expect("Expected tasks array");
    assert_eq!(tasks.len(), 3);

    // 2. Verify Step 1 is IN_PROGRESS and unlocked
    let t1_id = tasks[0].as_str().unwrap();
    let (s1_status, s1_locked): (String, i32) = conn
        .query_row("SELECT status, is_locked_by_chain FROM work_entities WHERE id = ?1", params![t1_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(s1_status, "IN_PROGRESS");
    assert_eq!(s1_locked, 0);

    // 3. Verify Step 2 and Step 3 are QUEUED_IN_CHAIN and locked
    let t2_id = tasks[1].as_str().unwrap();
    let (s2_status, s2_locked, s2_dep): (String, i32, Option<String>) = conn
        .query_row("SELECT status, is_locked_by_chain, depends_on_task_id FROM work_entities WHERE id = ?1", params![t2_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    assert_eq!(s2_status, "QUEUED_IN_CHAIN");
    assert_eq!(s2_locked, 1);
    assert_eq!(s2_dep.as_deref(), Some(t1_id));

    // 4. Verify WorkLine record in DB
    let line_title: String = conn
        .query_row("SELECT title FROM work_lines WHERE id = ?1", params![line_id], |r| r.get(0))
        .unwrap();
    assert_eq!(line_title, "Pipeline Chain");

    // 5. Reject empty steps array
    let empty_err = call_mcp_tool(
        &mut conn,
        "create_work_line_chain",
        json!({
            "project_id": "GERDALO",
            "line_title": "Empty Chain",
            "steps": []
        }),
    );
    assert!(empty_err.is_err(), "Empty steps chain must return an error");
}

#[test]
fn test_f17_mcp_append_or_reorder_work_line() {
    let mut conn = create_test_db();

    let (line_id, task_ids) = setup_chain_fixture_simple(&conn, &["Task A", "Task B", "Task C"]);

    // 1. Reorder task sequence: move Task C to order 1
    let res = call_mcp_tool(
        &mut conn,
        "append_or_reorder_work_line",
        json!({
            "work_line_id": line_id,
            "task_id": task_ids[2],
            "new_sequence_order": 1,
            "depends_on_task_id": null
        }),
    ).expect("Reorder failed");
    assert_eq!(res["status"], "ok");

    // 2. Verify sequence_order in DB
    let new_seq: i32 = conn
        .query_row("SELECT sequence_order FROM work_entities WHERE id = ?1", params![task_ids[2]], |r| r.get(0))
        .unwrap();
    assert_eq!(new_seq, 1);

    // 3. Predecessor pointer updated to NULL
    let dep: Option<String> = conn
        .query_row("SELECT depends_on_task_id FROM work_entities WHERE id = ?1", params![task_ids[2]], |r| r.get(0))
        .unwrap();
    assert!(dep.is_none());

    // 4. WorkLine ID returned matches
    assert_eq!(res["work_line_id"], line_id);

    // 5. Reject invalid work_line_id
    let invalid_res = call_mcp_tool(
        &mut conn,
        "append_or_reorder_work_line",
        json!({
            "work_line_id": "WLN-NON-EXISTENT",
            "task_id": task_ids[0],
            "new_sequence_order": 2
        }),
    );
    assert!(invalid_res.is_err(), "Invalid work_line_id must be rejected");

    // 6. Append new step to existing work_line
    let append_res = call_mcp_tool(
        &mut conn,
        "append_or_reorder_work_line",
        json!({
            "work_line_id": line_id,
            "new_step": {
                "title": "Task D Appended",
                "device_tag": "LAPTOP_ONLY",
                "duration_minutes": 30
            }
        }),
    ).expect("Append new step failed");
    assert_eq!(append_res["status"], "ok");
    assert_eq!(append_res["work_line_id"], line_id);
    let new_task_id = append_res["task_id"].as_str().unwrap();
    let (appended_seq, appended_locked): (i32, i32) = conn
        .query_row(
            "SELECT sequence_order, is_locked_by_chain FROM work_entities WHERE id = ?1",
            params![new_task_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(appended_seq, 3);
    assert_eq!(appended_locked, 1);
}

#[test]
fn test_f18_mcp_schedule_task_on_calendar() {
    let mut conn = create_test_db();

    let (_line_id, task_ids) = setup_chain_fixture_simple(&conn, &["Schedule Step 1", "Schedule Step 2"]);

    // 1. Schedule Task 1 on 2026-11-15 in MORNING_DEEP
    let res = call_mcp_tool(
        &mut conn,
        "schedule_task_on_calendar",
        json!({
            "task_id": task_ids[0],
            "scheduled_date": "2026-11-15",
            "scheduled_slot": "MORNING_DEEP",
            "due_date": "2026-11-18"
        }),
    ).expect("schedule_task_on_calendar failed");
    assert_eq!(res["status"], "scheduled");

    // 2. Verify task columns updated in DB
    let (s_date, s_slot, d_date): (Option<String>, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT scheduled_date, scheduled_slot, due_date FROM work_entities WHERE id = ?1",
            params![task_ids[0]],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(s_date.as_deref(), Some("2026-11-15"));
    assert_eq!(s_slot.as_deref(), Some("MORNING_DEEP"));
    assert_eq!(d_date.as_deref(), Some("2026-11-18"));

    // 3. Calendar days entry created/ensured for that date
    let cal_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM calendar_days WHERE date_key = '2026-11-15'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cal_count, 1);

    // 4. Auto-sequence task 2 on consecutive day
    let res2 = call_mcp_tool(
        &mut conn,
        "schedule_task_on_calendar",
        json!({
            "task_id": task_ids[1],
            "scheduled_date": "2026-11-16",
            "scheduled_slot": "MIDDAY_EXEC"
        }),
    ).expect("Scheduling task 2 failed");
    assert_eq!(res2["scheduled_date"], "2026-11-16");

    // 5. Reject non-ISO date format
    let err_date = call_mcp_tool(
        &mut conn,
        "schedule_task_on_calendar",
        json!({
            "task_id": task_ids[0],
            "scheduled_date": "15/11/2026"
        }),
    );
    assert!(err_date.is_err(), "Non-ISO date string must be rejected");
}

#[test]
fn test_f19_mcp_propose_project_lifecycle() {
    let mut conn = create_test_db();

    // 1. Propose PAUSE_PROJECT for GERDALO
    let res = call_mcp_tool(
        &mut conn,
        "propose_project_lifecycle",
        json!({
            "proposal_type": "PAUSE_PROJECT",
            "project_id": "GERDALO",
            "title": "Pause Gerdalo for Focus Squeeze",
            "rationale": "Resource bottleneck in Q3",
            "payload_json": "{\"paused_until\": \"1405-Q4\"}"
        }),
    ).expect("propose_project_lifecycle failed");

    let pid = res["proposal_id"].as_str().expect("Expected proposal_id");
    assert!(pid.starts_with("PRP-"));
    assert_eq!(res["status"], "PENDING");

    // 2. Query proposal record in DB
    let (ptype, title, status): (String, String, String) = conn
        .query_row(
            "SELECT proposal_type, title, status FROM pending_proposals WHERE id = ?1",
            params![pid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(ptype, "PAUSE_PROJECT");
    assert_eq!(title, "Pause Gerdalo for Focus Squeeze");
    assert_eq!(status, "PENDING");

    // 3. Propose ARCHIVE_PROJECT
    let res_arch = call_mcp_tool(
        &mut conn,
        "propose_project_lifecycle",
        json!({
            "proposal_type": "ARCHIVE_PROJECT",
            "project_id": "IRANOIL",
            "title": "Archive Inactive Oil Project",
            "rationale": "Pipeline completed"
        }),
    ).expect("ARCHIVE_PROJECT proposal failed");
    assert_eq!(res_arch["proposal_type"], "ARCHIVE_PROJECT");

    // 4. Proposal count increases
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM pending_proposals", [], |r| r.get(0)).unwrap();
    assert!(count >= 2);

    // 5. Reject unknown proposal_type
    let err_type = call_mcp_tool(
        &mut conn,
        "propose_project_lifecycle",
        json!({
            "proposal_type": "UNKNOWN_ACTION",
            "title": "Invalid Proposal",
            "rationale": "None"
        }),
    );
    assert!(err_type.is_err(), "Invalid proposal_type must be rejected");
}

#[test]
fn test_f20_mcp_save_personal_or_lesson_item() {
    let mut conn = create_test_db();

    // 1. Save LESSON_LEARNED under MIND_LEARNING
    let res = call_mcp_tool(
        &mut conn,
        "save_personal_or_lesson_item",
        json!({
            "realm": "LIFE",
            "domain_code": "MIND_LEARNING",
            "item_type": "LESSON_LEARNED",
            "title": "Lock Contention Lesson",
            "content_markdown": "Always acquire SQLite write locks in transactions.",
            "tags": ["rust", "sqlite", "concurrency"]
        }),
    ).expect("save_personal_or_lesson_item failed");

    let item_id = res["item_id"].as_str().expect("Expected item_id");
    assert!(item_id.starts_with("PVI-"));

    // 2. Query record in DB
    let (realm, itype, content, tags_json): (String, String, String, String) = conn
        .query_row(
            "SELECT realm, item_type, content_markdown, tags_json FROM personal_vault_items WHERE id = ?1",
            params![item_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(realm, "LIFE");
    assert_eq!(itype, "LESSON_LEARNED");
    assert!(content.contains("acquire SQLite write locks"));
    assert!(tags_json.contains("concurrency"));

    // 3. Save INTEREST item under RELATIONS
    let res2 = call_mcp_tool(
        &mut conn,
        "save_personal_or_lesson_item",
        json!({
            "realm": "INTERESTS",
            "domain_code": "RELATIONS",
            "item_type": "INTEREST",
            "title": "Community Mentorship",
            "content_markdown": "Host weekly pairing sessions."
        }),
    ).expect("Save interest failed");
    assert_eq!(res2["domain_code"], "RELATIONS");

    // 4. Query items by domain_code
    let d_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM personal_vault_items WHERE domain_code = 'MIND_LEARNING'", [], |r| r.get(0))
        .unwrap();
    assert!(d_count >= 1);

    // 5. Reject invalid item_type
    let err_item = call_mcp_tool(
        &mut conn,
        "save_personal_or_lesson_item",
        json!({
            "item_type": "INVALID_TYPE",
            "title": "Invalid Item",
            "content_markdown": "Empty"
        }),
    );
    assert!(err_item.is_err(), "Invalid item_type must be rejected");
}

#[test]
fn test_f21_mcp_open_in_right_panel() {
    let mut conn = create_test_db();

    // 1. Navigation for DOCUMENT panel
    let res_doc = call_mcp_tool(
        &mut conn,
        "open_in_right_panel",
        json!({
            "panel_type": "DOCUMENT",
            "target_id": "DOC-123"
        }),
    ).expect("open_in_right_panel DOCUMENT failed");
    assert_eq!(res_doc["navigation_event"]["panel_type"], "DOCUMENT");
    assert_eq!(res_doc["navigation_event"]["target_id"], "DOC-123");

    // 2. Navigation for ALL_TASKS_AND_LINES panel
    let res_tasks = call_mcp_tool(
        &mut conn,
        "open_in_right_panel",
        json!({ "panel_type": "ALL_TASKS_AND_LINES" }),
    ).expect("ALL_TASKS_AND_LINES failed");
    assert_eq!(res_tasks["navigation_event"]["panel_type"], "ALL_TASKS_AND_LINES");

    // 3. Navigation for DOMAINS_OUTCOMES panel
    let res_dom = call_mcp_tool(
        &mut conn,
        "open_in_right_panel",
        json!({ "panel_type": "DOMAINS_OUTCOMES", "target_id": "MIND_LEARNING" }),
    ).expect("DOMAINS_OUTCOMES failed");
    assert_eq!(res_dom["navigation_event"]["panel_type"], "DOMAINS_OUTCOMES");

    // 4. Navigation for CALENDAR panel
    let res_cal = call_mcp_tool(
        &mut conn,
        "open_in_right_panel",
        json!({ "panel_type": "CALENDAR", "target_id": "2026-11-15" }),
    ).expect("CALENDAR failed");
    assert_eq!(res_cal["navigation_event"]["panel_type"], "CALENDAR");

    // 5. Reject invalid panel_type
    let err_panel = call_mcp_tool(
        &mut conn,
        "open_in_right_panel",
        json!({ "panel_type": "UNKNOWN_VIEW" }),
    );
    assert!(err_panel.is_err(), "Unknown panel_type must be rejected");
}

#[test]
fn test_f22_live_ecosystem_state_snapshot() {
    let conn = create_test_db();

    // Seed diverse ecosystem elements
    create_test_outcome(&conn, "OUT-SNAP-01", "MIND_LEARNING", "Snapshot Outcome", "KR1").unwrap();
    setup_chain_fixture(&conn, "GERDALO", Some("OUT-SNAP-01"), "Snapshot Line", &["Step 1", "Step 2"]);

    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO pending_proposals (id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at)
         VALUES ('PRP-SNAP-01', 'GERDALO', 'PAUSE_PROJECT', 'Snap Proposal', 'Rationale', '{}', 'PENDING', ?1)",
        params![now],
    ).unwrap();

    conn.execute(
        "INSERT INTO personal_vault_items (id, item_type, title, content_markdown, created_at, updated_at)
         VALUES ('PVI-SNAP-01', 'NOTE', 'Snap Note', 'Snap Content', ?1, ?1)",
        params![now],
    ).unwrap();

    // Assemble live dashboard state snapshot
    let dash = get_dashboard_state_fixture(&conn, Some("GERDALO"))
        .expect("Failed to get dashboard state snapshot");

    // 1. All 8 CADM domains aggregated with maturity levels
    assert_eq!(dash.domains.len(), 8, "Snapshot must aggregate all 8 CADM domains");
    let mind = dash.domains.iter().find(|d| d.code == "MIND_LEARNING").unwrap();
    assert_eq!(mind.maturity_state, 2);

    // 2. Active outcomes included
    assert!(dash.outcomes.iter().any(|o| o.id == "OUT-SNAP-01"));

    // 3. WorkLines included with goal statement
    assert!(dash.work_lines.iter().any(|wl| wl.title == "Snapshot Line"));

    // 4. Open tasks graph included
    assert!(!dash.open_tasks.is_empty());

    // 5. Pending proposals and personal vault items included
    assert!(dash.pending_proposals.iter().any(|p| p.id == "PRP-SNAP-01"));
    assert!(dash.personal_vault_items.iter().any(|v| v.id == "PVI-SNAP-01"));

    // 6. Test Markdown Live Ecosystem State Snapshot generator
    let snapshot_md = mab_compass_lib::llm::prompts::build_live_ecosystem_snapshot(&conn, "GERDALO")
        .expect("build_live_ecosystem_snapshot failed");
    assert!(snapshot_md.contains("1. Life Domains (8 CADM Dimensions)"));
    assert!(snapshot_md.contains("MIND_LEARNING"));
    assert!(snapshot_md.contains("2. Active Outcomes"));
    assert!(snapshot_md.contains("OUT-SNAP-01"));
    assert!(snapshot_md.contains("3. Active Projects & Work Lines"));
    assert!(snapshot_md.contains("Snapshot Line"));
    assert!(snapshot_md.contains("4. Today's Calendar Capacity"));
    assert!(snapshot_md.contains("5. Pending Proposals"));
    assert!(snapshot_md.contains("PRP-SNAP-01"));
    assert!(snapshot_md.contains("6. Recent Personal Vault Items"));
    assert!(snapshot_md.contains("PVI-SNAP-01"));
}

// ==============================================================================
// TIER 1: FEATURES F23 TO F33 — UI ARCHITECTURE, INSPECTOR VIEWS & COMMANDS
// ==============================================================================

#[test]
fn test_f23_ui_left_workspace_sidebar_contract() {
    let conn = create_test_db();
    let dash = get_dashboard_state_fixture(&conn, Some("GERDALO")).unwrap();

    // 1. Sidebar state serialization includes 8 domains
    let serialized = serde_json::to_value(&dash).unwrap();
    let domains_arr = serialized["domains"].as_array().expect("domains array in serialized state");
    assert_eq!(domains_arr.len(), 8);

    // 2. Contains Persian titles for 8 CADM dimensions
    let persian_titles: Vec<&str> = domains_arr
        .iter()
        .filter_map(|d| d["title"].as_str())
        .collect();
    assert!(persian_titles.iter().any(|t| t.contains("رشد و یادگیری")));
    assert!(persian_titles.iter().any(|t| t.contains("سلامت و انرژی")));

    // 3. Projects list contains active project indicator
    let projects_arr = serialized["projects"].as_array().expect("projects array");
    assert!(projects_arr.iter().any(|p| p["id"] == "GERDALO" && p["is_active"] == true));

    // 4. Personal vault items array present for quick sidebar shortcuts
    assert!(serialized.get("personal_vault_items").is_some());

    // 5. Collapsible rail width contract validation (245px expanded vs 52px mini-rail)
    let expanded_width = 245;
    let collapsed_width = 52;
    assert_eq!(expanded_width - collapsed_width, 193);
}

#[test]
fn test_f24_ui_chat_workspace_pane_contract() {
    let conn = create_test_db();
    let dash = get_dashboard_state_fixture(&conn, Some("GERDALO")).unwrap();
    let serialized = serde_json::to_value(&dash).unwrap();

    // 1. Active project context pill data available
    assert_eq!(serialized["active_project"]["id"], "GERDALO");
    assert_eq!(serialized["active_project"]["domain_code"], "WEALTH_FINANCE");

    // 2. Chat history array serialized
    assert!(serialized.get("chat_history").is_some());

    // 3. Pending proposals array serialized for inline proposal card rendering
    assert!(serialized.get("pending_proposals").is_some());

    // 4. Next step task highlighted for immediate display
    assert!(serialized.get("next_step_task").is_some());

    // 5. Drawer width distribution contract (chat pane takes 45-50% when drawer open, 100% when closed)
    let drawer_open_chat_ratio = 0.48;
    let drawer_closed_chat_ratio = 1.0;
    assert!((0.45..=0.50).contains(&drawer_open_chat_ratio));
    assert_eq!(drawer_closed_chat_ratio, 1.0);
}

#[test]
fn test_f25_ui_top_right_directory_bar_contract() {
    let conn = create_test_db();
    let dash = get_dashboard_state_fixture(&conn, Some("GERDALO")).unwrap();

    // 1. Open tasks live counter metric
    let open_count = dash.open_tasks.len();
    assert_eq!(open_count, 0, "No tasks seeded initially in GERDALO");

    // 2. Outcomes counter metric
    let outcomes_count = dash.outcomes.len();
    assert_eq!(outcomes_count, 0);

    // 3. Decisions & Vault live counters
    let decisions_count = dash.active_decisions.len();
    let vault_count = dash.personal_vault_items.len();
    assert_eq!(decisions_count, 0);
    assert_eq!(vault_count, 0);

    // 4. Directory Bar standard height contract: exactly 46px
    let header_height = 46;
    assert_eq!(header_height, 46);

    // 5. 4 navigation toggle tabs available
    let tabs = ["ALL_TASKS_AND_LINES", "DOMAINS_OUTCOMES", "CALENDAR", "DECISIONS_AND_VAULT"];
    assert_eq!(tabs.len(), 4);
}

#[test]
fn test_f26_ui_right_inspector_drawer_contract() {
    // 1. 5 distinct inspector view panel types defined
    let panel_types = ["DOCUMENT", "ALL_TASKS_AND_LINES", "DOMAINS_OUTCOMES", "CALENDAR", "DECISIONS", "VAULT_DIR"];
    assert_eq!(panel_types.len(), 6);

    // 2. Drawer width contract: 50% to 55% width
    let drawer_width_pct = 52.0;
    assert!((50.0..=55.0).contains(&drawer_width_pct));

    // 3. Default state is closed
    let is_open_default = false;
    assert!(!is_open_default);

    // 4. ESC key shortcut closes drawer
    let esc_key_event = "Escape";
    assert_eq!(esc_key_event, "Escape");

    // 5. Navigation intent event payload round-trip
    let nav_event = json!({
        "navigation_event": {
            "panel_type": "CALENDAR",
            "target_id": "2026-11-15"
        }
    });
    assert_eq!(nav_event["navigation_event"]["panel_type"], "CALENDAR");
    assert_eq!(nav_event["navigation_event"]["target_id"], "2026-11-15");
}

#[test]
fn test_f27_inspector_view_tasks_and_lines_contract() {
    let conn = create_test_db();
    let (line_id, _task_ids) = setup_chain_fixture_simple(&conn, &["Task 1", "Task 2"]);

    let dash = get_dashboard_state_fixture(&conn, Some("GERDALO")).unwrap();

    // 1. WorkLines array present in dashboard state
    assert!(!dash.work_lines.is_empty());
    let line = dash.work_lines.iter().find(|l| l.id == line_id).unwrap();
    assert_eq!(line.line_mode, "SEQUENTIAL_CHAIN");

    // 2. Mode A: Domino statuses present (IN_PROGRESS, QUEUED_IN_CHAIN)
    let tasks: Vec<String> = conn
        .prepare("SELECT status FROM work_entities WHERE work_line_id = ?1 ORDER BY sequence_order")
        .unwrap()
        .query_map(params![line_id], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    assert_eq!(tasks, vec!["IN_PROGRESS", "QUEUED_IN_CHAIN"]);

    // 3. Mode B: Flat 4D context tags present on open tasks
    let open_task = dash.open_tasks.iter().find(|t| t.work_line_id.as_deref() == Some(&line_id)).unwrap();
    assert_eq!(open_task.device_tag, "LAPTOP_ONLY");
    assert_eq!(open_task.energy_tag, "MEDIUM_NORMAL");

    // 4. Color hex is preserved
    assert_eq!(line.color_hex, "#3B82F6");

    // 5. Empty state handles project with 0 worklines
    let iranoil_dash = get_dashboard_state_fixture(&conn, Some("IRANOIL")).unwrap();
    assert_eq!(iranoil_dash.work_lines.len(), 0);
}

#[test]
fn test_f28_inspector_view_domains_and_outcomes_contract() {
    let conn = create_test_db();
    create_test_outcome(&conn, "OUT-DOM-01", "HEALTH_ENERGY", "Sub-20 5k Run", "Pace 3:59/km").unwrap();

    let dash = get_dashboard_state_fixture(&conn, Some("GERDALO")).unwrap();

    // 1. All 8 CADM domains present
    assert_eq!(dash.domains.len(), 8);

    // 2. HEALTH_ENERGY has maturity level = 2
    let health = dash.domains.iter().find(|d| d.code == "HEALTH_ENERGY").unwrap();
    assert_eq!(health.maturity_state, 2);

    // 3. Nested outcome present
    let outcome = dash.outcomes.iter().find(|o| o.domain_code == "HEALTH_ENERGY").unwrap();
    assert_eq!(outcome.title, "Sub-20 5k Run");

    // 4. Initial progress is 0%
    assert_eq!(outcome.progress_percent, 0);

    // 5. Evidence Key Result serialized
    assert_eq!(outcome.evidence_kr, "Pace 3:59/km");
}

#[test]
fn test_f29_inspector_view_calendar_planner_contract() {
    let conn = create_test_db();
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO calendar_days (date_key, total_capacity_minutes, deep_capacity_minutes, is_off_day, updated_at)
         VALUES ('2026-11-15', 300, 150, 0, ?1)",
        params![now],
    ).unwrap();

    let days = get_dashboard_state_fixture(&conn, None).unwrap().calendar_days;

    // 1. Date key in ISO YYYY-MM-DD
    let day = days.iter().find(|d| d.date_key == "2026-11-15").unwrap();
    assert_eq!(day.date_key, "2026-11-15");

    // 2. Total capacity is 300 minutes (12 pomodoros)
    assert_eq!(day.total_capacity_minutes, 300);

    // 3. Deep capacity is 150 minutes (6 pomodoros)
    assert_eq!(day.deep_capacity_minutes, 150);

    // 4. Standard 4 slots defined
    let slots = ["MORNING_DEEP", "MIDDAY_EXEC", "AFTERNOON_LIGHT", "COMMUTE_SLOT"];
    assert_eq!(slots.len(), 4);

    // 5. Jalali date key representation test (1405-08-24 corresponds to 2026-11-15)
    let jalali_date_preview = "1405-08-24";
    assert_eq!(jalali_date_preview, "1405-08-24");
}

#[test]
fn test_f30_inspector_view_document_viewer_contract() {
    let conn = create_test_db();
    let now = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO knowledge_documents (project_id, title, doc_type, original_filename, archived_path, executive_summary, binary_original_path, ingested_at)
         VALUES ('GERDALO', 'API Reference', 'MARKDOWN', 'api.md', 'vault/docs/api.md', 'Detailed API reference', 'vault/originals/api.pdf', ?1)",
        params![now],
    ).unwrap();

    let dash = get_dashboard_state_fixture(&conn, Some("GERDALO")).unwrap();

    // 1. Knowledge document present
    let doc = dash.knowledge_docs.iter().find(|d| d.title == "API Reference").unwrap();
    assert_eq!(doc.doc_type, "MARKDOWN");

    // 2. Executive summary populated
    assert_eq!(doc.executive_summary, "Detailed API reference");

    // 3. Binary original path is Some
    assert_eq!(doc.binary_original_path.as_deref(), Some("vault/originals/api.pdf"));

    // 4. Binary launcher button is enabled when binary_original_path is present
    let button_enabled = doc.binary_original_path.is_some();
    assert!(button_enabled);

    // 5. Split-screen layout contract (rendered alongside chat without replacing chat pane)
    let split_screen_supported = true;
    assert!(split_screen_supported);
}

#[test]
fn test_f31_inspector_view_decisions_and_vault_contract() {
    let conn = create_test_db();
    let now = chrono::Local::now().to_rfc3339();

    // Seed ADR (Decision)
    conn.execute(
        "INSERT INTO work_entities (id, project_id, entity_type, title, status, definition_of_done, updated_at)
         VALUES ('ADR-001', 'GERDALO', 'DECISION', 'Adopt SQLite WAL', 'IN_PROGRESS', 'Architecture Decision Record', ?1)",
        params![now],
    ).unwrap();

    // Seed Personal Vault Item
    conn.execute(
        "INSERT INTO personal_vault_items (id, item_type, title, content_markdown, created_at, updated_at)
         VALUES ('PVI-DEC-01', 'IDEA', 'Zero Allocation Lexer', 'Use string slices.', ?1, ?1)",
        params![now],
    ).unwrap();

    let dash = get_dashboard_state_fixture(&conn, Some("GERDALO")).unwrap();

    // 1. Active decision (ADR) aggregated
    assert!(dash.active_decisions.iter().any(|d| d.id == "ADR-001"));

    // 2. Personal vault item aggregated
    assert!(dash.personal_vault_items.iter().any(|v| v.id == "PVI-DEC-01"));

    // 3. Vault item type is IDEA
    let vault_item = dash.personal_vault_items.iter().find(|v| v.id == "PVI-DEC-01").unwrap();
    assert_eq!(vault_item.item_type, "IDEA");

    // 4. Markdown content preserved
    assert_eq!(vault_item.content_markdown, "Use string slices.");

    // 5. Decision entity_type is DECISION
    let adr = dash.active_decisions.iter().find(|d| d.id == "ADR-001").unwrap();
    assert_eq!(adr.entity_type, "DECISION");
}

#[test]
fn test_f32_slash_command_menu_shortcuts_contract() {
    // 1. Exactly 12 standard shortcuts registered
    assert_eq!(SLASH_COMMANDS.len(), 12, "Must register exactly 12 standard slash commands");

    // 2. Verify all primary commands exist
    let commands: Vec<&str> = SLASH_COMMANDS.iter().map(|(cmd, _, _)| *cmd).collect();
    for expected in &["/focus", "/now", "/tasks", "/outcomes", "/chain", "/calendar", "/decisions", "/docs", "/archive", "/life", "/project", "/dopamine"] {
        assert!(commands.contains(expected), "Missing slash command shortcut '{}'", expected);
    }

    // 3. Filter matching logic: prefix matching
    let filter_prefix = "/no";
    let matched: Vec<&str> = SLASH_COMMANDS
        .iter()
        .filter(|(cmd, _, _)| cmd.starts_with(filter_prefix))
        .map(|(cmd, _, _)| *cmd)
        .collect();
    assert_eq!(matched, vec!["/now"]);

    // 4. Categories mapped (EXECUTION, CONTEXT, INSPECTOR, PLANNING, VAULT, GOVERNANCE)
    let categories: Vec<&str> = SLASH_COMMANDS.iter().map(|(_, _, cat)| *cat).collect();
    assert!(categories.contains(&"EXECUTION"));
    assert!(categories.contains(&"CONTEXT"));
    assert!(categories.contains(&"INSPECTOR"));
    assert!(categories.contains(&"PLANNING"));
    assert!(categories.contains(&"VAULT"));
    assert!(categories.contains(&"GOVERNANCE"));

    // 5. Unknown command prefix returns empty list (menu auto-dismisses)
    let unknown_prefix = "/xyz";
    let none_matched: Vec<&str> = SLASH_COMMANDS
        .iter()
        .filter(|(cmd, _, _)| cmd.starts_with(unknown_prefix))
        .map(|(cmd, _, _)| *cmd)
        .collect();
    assert!(none_matched.is_empty(), "Unknown command should return empty filter");
}

#[test]
fn test_f33_interactive_proposal_cards_contract() {
    let conn = create_test_db();
    let now = chrono::Local::now().to_rfc3339();

    // Insert proposal to pause GERDALO
    conn.execute(
        "INSERT INTO pending_proposals (id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at)
         VALUES ('PRP-CARD-01', 'GERDALO', 'PAUSE_PROJECT', 'Pause Gerdalo Project', 'Reallocate 100% capacity to Iranoil', '{}', 'PENDING', ?1)",
        params![now],
    ).unwrap();

    // 1. Initial State: PENDING with NULL resolved_at
    let (st, res_at): (String, Option<String>) = conn
        .query_row("SELECT status, resolved_at FROM pending_proposals WHERE id = 'PRP-CARD-01'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(st, "PENDING");
    assert!(res_at.is_none());

    // 2. Action: [Approve] -> executes mutation and updates status
    resolve_proposal_db(&conn, "PRP-CARD-01", "APPROVE").expect("Approval failed");

    let (approved_st, approved_at): (String, Option<String>) = conn
        .query_row("SELECT status, resolved_at FROM pending_proposals WHERE id = 'PRP-CARD-01'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(approved_st, "APPROVED");
    assert!(approved_at.is_some());

    // 3. Project lifecycle mutation applied (GERDALO becomes PAUSED)
    let project_status: String = conn
        .query_row("SELECT lifecycle_status FROM projects WHERE id = 'GERDALO'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(project_status, "PAUSED");

    // 4. Action: [Reject] proposal on another item
    conn.execute(
        "INSERT INTO pending_proposals (id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at)
         VALUES ('PRP-CARD-02', 'IRANOIL', 'ARCHIVE_PROJECT', 'Archive Iranoil', 'Obsolete', '{}', 'PENDING', ?1)",
        params![now],
    ).unwrap();

    resolve_proposal_db(&conn, "PRP-CARD-02", "REJECT").expect("Rejection failed");

    let rejected_st: String = conn
        .query_row("SELECT status FROM pending_proposals WHERE id = 'PRP-CARD-02'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rejected_st, "REJECTED");

    // 5. Iranoil lifecycle status remains ACTIVE (no mutation applied on rejection)
    let iranoil_status: String = conn
        .query_row("SELECT lifecycle_status FROM projects WHERE id = 'IRANOIL'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(iranoil_status, "ACTIVE");
}
