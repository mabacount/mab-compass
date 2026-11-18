//! Smoke Test for Mab-Compass v3.0 E2E Test Harness
//! File: `src-tauri/tests/smoke_harness_test.rs`
//!
//! Validates:
//! 1. `common/mod.rs` module compiles and links properly with `mab_compass_lib`.
//! 2. `create_test_db()` creates in-memory SQLite with foreign keys and complete v3 schema.
//! 3. 8 CADM life domains are seeded with maturity floors and valid levels (0..5).
//! 4. Default projects (`GERDALO`, `IRANOIL`) are seeded.
//! 5. 5-layer ontology fixtures (`create_test_outcome`, `create_test_project_v3`) function correctly.
//! 6. Sequential chain domino auto-advancement and chain assertions execute accurately.
//! 7. 4D context tagging and SQL filtering accurately isolate unlocked matching tasks.
//! 8. RAII `TestContext` operates cleanly with tempdir sandbox.
//! 9. `call_mcp_tool` helper successfully dispatches calls into `mab_compass_lib::mcp_tools`.

mod common;

use common::*;
use serde_json::json;

#[test]
fn test_in_memory_db_schema_and_cadm_seeds() {
    let conn = create_test_db();

    // Verify foreign keys pragma is active
    let fk_enabled: i32 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .expect("Failed to query foreign_keys pragma");
    assert_eq!(fk_enabled, 1, "Foreign keys must be enabled on test database");

    // Verify CADM 8-domain seed validation helper
    assert_cadm_seeds(&conn);

    // Verify specific domain details (e.g. MIND_LEARNING)
    let (code, title, maturity, floor): (String, String, i32, String) = conn
        .query_row(
            "SELECT code, title, maturity_state, maintenance_floor FROM life_domains WHERE code = 'MIND_LEARNING'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("MIND_LEARNING domain not found");

    assert_eq!(code, "MIND_LEARNING");
    assert!(!title.is_empty());
    assert_eq!(maturity, 2);
    assert!(!floor.is_empty());

    // Verify default projects exist
    let p1_name: String = conn
        .query_row("SELECT name FROM projects WHERE id = 'GERDALO'", [], |r| r.get(0))
        .expect("GERDALO project not found");
    assert!(p1_name.contains("گردالو"));

    let p2_name: String = conn
        .query_row("SELECT name FROM projects WHERE id = 'IRANOIL'", [], |r| r.get(0))
        .expect("IRANOIL project not found");
    assert!(p2_name.contains("ایران‌اویل‌جاب"));
}

#[test]
fn test_5layer_ontology_fixtures() {
    let conn = create_test_db();

    // 1. Create an Outcome
    let outcome_id = create_test_outcome(
        &conn,
        "OUT-MIND-01",
        "MIND_LEARNING",
        "Master Rust Systems Programming",
        "100% E2E test coverage and zero memory leaks",
    )
    .expect("Failed to create test outcome");
    assert_eq!(outcome_id, "OUT-MIND-01");

    // 2. Create a Project linked to domain and outcome
    let project_id = create_test_project_v3(
        &conn,
        "PRJ-RUST-EXP",
        "Rust Expansion Project",
        "MIND_LEARNING",
        Some(&outcome_id),
    )
    .expect("Failed to create test project");
    assert_eq!(project_id, "PRJ-RUST-EXP");

    // Verify project table data
    let (domain_code, primary_outcome, lifecycle): (String, Option<String>, String) = conn
        .query_row(
            "SELECT domain_code, primary_outcome_id, lifecycle_status FROM projects WHERE id = ?1",
            rusqlite::params![project_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("Project not found in DB");

    assert_eq!(domain_code, "MIND_LEARNING");
    assert_eq!(primary_outcome.as_deref(), Some("OUT-MIND-01"));
    assert_eq!(lifecycle, "ACTIVE");
}

#[test]
fn test_sequential_chain_domino_engine_and_assertions() {
    let conn = create_test_db();

    // Setup a 3-step sequential chain
    let (work_line_id, task_ids) = setup_chain_fixture(
        &conn,
        "GERDALO",
        None,
        "Smoke Test Chain Line",
        &["Step 1: Architect", "Step 2: Implement", "Step 3: Verify"],
    );

    assert_eq!(task_ids.len(), 3);

    // Initial state: Step 1 is IN_PROGRESS (unlocked); Steps 2 and 3 are QUEUED_IN_CHAIN (locked)
    assert_chain_step_statuses(
        &conn,
        &work_line_id,
        &[
            (1, "IN_PROGRESS"),
            (2, "QUEUED_IN_CHAIN"),
            (3, "QUEUED_IN_CHAIN"),
        ],
    );

    assert!(!is_locked_by_chain(&conn, &task_ids[0]).unwrap());
    assert!(is_locked_by_chain(&conn, &task_ids[1]).unwrap());
    assert!(is_locked_by_chain(&conn, &task_ids[2]).unwrap());

    // Domino 1: Complete Step 1 -> advances Step 2 to IN_PROGRESS
    let next_task = simulate_advance_chain_on_complete(&conn, "GERDALO", &task_ids[0])
        .expect("Failed to advance chain on step 1 complete");
    assert_eq!(next_task.as_deref(), Some(task_ids[1].as_str()));

    assert_chain_domino_transition(&conn, &work_line_id, &task_ids[0], &task_ids[1]);
    assert_chain_step_statuses(
        &conn,
        &work_line_id,
        &[
            (1, "CLOSED"),
            (2, "IN_PROGRESS"),
            (3, "QUEUED_IN_CHAIN"),
        ],
    );

    assert!(!is_locked_by_chain(&conn, &task_ids[0]).unwrap());
    assert!(!is_locked_by_chain(&conn, &task_ids[1]).unwrap());
    assert!(is_locked_by_chain(&conn, &task_ids[2]).unwrap());

    // Domino 2: Complete Step 2 -> advances Step 3 to IN_PROGRESS
    let next_task_2 = simulate_advance_chain_on_complete(&conn, "GERDALO", &task_ids[1])
        .expect("Failed to advance chain on step 2 complete");
    assert_eq!(next_task_2.as_deref(), Some(task_ids[2].as_str()));

    assert_chain_domino_transition(&conn, &work_line_id, &task_ids[1], &task_ids[2]);

    // Domino 3: Complete Step 3 -> all steps completed, WorkLine marked COMPLETED
    let final_next = simulate_advance_chain_on_complete(&conn, "GERDALO", &task_ids[2])
        .expect("Failed to advance chain on step 3 complete");
    assert_eq!(final_next, None);

    assert_work_line_completed(&conn, &work_line_id);
}

#[test]
fn test_4d_context_fixtures_and_filter() {
    let conn = create_test_db();
    let fixtures = setup_context_fixture(&conn, "GERDALO");

    assert_eq!(fixtures.len(), 7);

    // Filter for Metro commute with phone, low energy, <= 15 minutes, only unlocked
    let matched_ids = query_tasks_by_context_sql(
        &conn,
        Some("GERDALO"),
        Some("PHONE_OK"),
        Some("METRO_COMMUTE"),
        Some("LOW_ZOMBIE"),
        Some(15),
        true, // only_unlocked = true
    )
    .expect("Failed to query tasks by context");

    // Must match "Task Metro Phone Low 15m" but strictly EXCLUDE "Task Metro Phone Low 15m (LOCKED IN CHAIN)"
    assert_eq!(matched_ids.len(), 1, "Expected exactly 1 matching unlocked task");

    let matched_title: String = conn
        .query_row(
            "SELECT title FROM work_entities WHERE id = ?1",
            rusqlite::params![matched_ids[0]],
            |r| r.get(0),
        )
        .expect("Failed to query task title");

    assert_eq!(matched_title, "Task Metro Phone Low 15m");

    // If only_unlocked = false, then both the open and the locked task should be returned
    let all_matched = query_tasks_by_context_sql(
        &conn,
        Some("GERDALO"),
        Some("PHONE_OK"),
        Some("METRO_COMMUTE"),
        Some("LOW_ZOMBIE"),
        Some(15),
        false, // only_unlocked = false
    )
    .expect("Failed to query tasks with locked included");

    assert_eq!(all_matched.len(), 2, "Expected 2 tasks when locked tasks are included");
}

#[test]
fn test_raii_test_context_sandbox() {
    let ctx = TestContext::new();

    // Verify temp database file and vault directory exist on disk
    assert!(ctx.db_path.exists(), "Temporary SQLite DB file must exist");
    assert!(ctx.vault_path.exists(), "Temporary vault folder must exist");

    // Verify CADM seeds are present in the disk-backed sandbox
    assert_cadm_seeds(&ctx.conn);

    let temp_dir_path = ctx.temp_dir.path().to_path_buf();
    assert!(temp_dir_path.exists());

    drop(ctx);

    // After drop, temp_dir is cleaned up
    assert!(
        !temp_dir_path.exists() || !temp_dir_path.join("test.sqlite").exists(),
        "Temporary database should be dropped and cleaned"
    );
}

#[test]
fn test_call_mcp_tool_helper() {
    let mut conn = create_test_db();

    // Invoke 'list_projects' MCP tool through the helper
    let result = call_mcp_tool(&mut conn, "list_projects", json!({}))
        .expect("Failed to invoke list_projects via call_mcp_tool");

    let projects = result
        .get("projects")
        .and_then(|p| p.as_array())
        .expect("Expected projects array in result");

    assert!(
        projects.len() >= 2,
        "Expected at least 2 seeded projects, found {}",
        projects.len()
    );

    let gerdalo = projects.iter().find(|p| p.get("id").and_then(|v| v.as_str()) == Some("GERDALO"));
    assert!(gerdalo.is_some(), "GERDALO project should be returned by list_projects");

    let iranoil = projects.iter().find(|p| p.get("id").and_then(|v| v.as_str()) == Some("IRANOIL"));
    assert!(iranoil.is_some(), "IRANOIL project should be returned by list_projects");
}
