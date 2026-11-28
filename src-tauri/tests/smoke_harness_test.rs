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

#[test]
fn test_cross_project_strategic_path_ahead_and_prioritization() {
    let mut conn = create_test_db();

    // 1. Create tasks in two different projects via MCP tools
    call_mcp_tool(
        &mut conn,
        "create_work_entity",
        json!({
            "project_id": "GERDALO",
            "title": "طراحی درگاه پرداخت گردالو",
            "status": "OPEN",
            "is_on_critical_path": false
        }),
    )
    .expect("create task in GERDALO");

    call_mcp_tool(
        &mut conn,
        "create_work_entity",
        json!({
            "project_id": "IRANOIL",
            "title": "خروجی گزارش مصرف ایران‌اویل",
            "status": "OPEN",
            "is_on_critical_path": false
        }),
    )
    .expect("create task in IRANOIL");

    // 2. Inspect Path Ahead across ALL projects in one higher-layer call
    let portfolio = call_mcp_tool(&mut conn, "get_all_projects_path_ahead", json!({}))
        .expect("get_all_projects_path_ahead");
    assert!(
        portfolio["projects_count"].as_i64().unwrap_or(0) >= 2,
        "Portfolio should cover at least 2 projects"
    );

    let queue = portfolio["global_prioritized_queue"]
        .as_array()
        .expect("global_prioritized_queue array");
    let g_tid = queue
        .iter()
        .find(|t| t["project_id"] == "GERDALO")
        .and_then(|t| t["id"].as_str())
        .expect("GERDALO task ID")
        .to_string();
    let i_tid = queue
        .iter()
        .find(|t| t["project_id"] == "IRANOIL")
        .and_then(|t| t["id"].as_str())
        .expect("IRANOIL task ID")
        .to_string();

    // 3. Batch-prioritize across BOTH projects without specifying project_id (auto-resolved!)
    let prio_res = call_mcp_tool(
        &mut conn,
        "prioritize_across_projects",
        json!({
            "items": [
                {
                    "task_id": i_tid,
                    "priority_score": 98,
                    "make_next_step": true,
                    "is_on_critical_path": true,
                    "scheduled_date": "2026-11-27",
                    "scheduled_slot": "MORNING_DEEP"
                },
                {
                    "task_id": g_tid,
                    "priority_score": 82,
                    "make_next_step": true,
                    "is_on_critical_path": true
                }
            ]
        }),
    )
    .expect("prioritize_across_projects");

    assert_eq!(prio_res["status"], "ok");
    let refreshed = prio_res["global_prioritized_queue"]
        .as_array()
        .expect("refreshed queue");
    assert_eq!(
        refreshed[0]["project_id"], "IRANOIL",
        "IRANOIL task prioritized at P98 must rank #1 globally"
    );
    assert_eq!(refreshed[0]["priority_score"], 98);
    assert_eq!(refreshed[0]["status"], "IN_PROGRESS");

    let snap = mab_compass_lib::llm::prompts::build_cross_project_portfolio_snapshot(&conn);
    assert!(snap.contains("IRANOIL"));
    assert!(snap.contains("GERDALO"));
    assert!(snap.contains("P=98"));
}

#[test]
fn test_calendar_event_creation_and_multi_task_linking() {
    let mut conn = create_test_db();

    let t1_res = call_mcp_tool(
        &mut conn,
        "create_work_entity",
        json!({
            "project_id": "GERDALO",
            "title": "تهیه اسلاید گزارش عملکرد گردالو",
            "status": "OPEN"
        }),
    )
    .expect("create t1");
    let t1_id = t1_res["open_tasks"][0]["id"]
        .as_str()
        .expect("t1 id")
        .to_string();

    let t2_res = call_mcp_tool(
        &mut conn,
        "create_work_entity",
        json!({
            "project_id": "IRANOIL",
            "title": "بررسی فاکتورهای باز ایران‌اویل",
            "status": "OPEN"
        }),
    )
    .expect("create t2");
    let t2_id = t2_res["open_tasks"][0]["id"]
        .as_str()
        .expect("t2 id")
        .to_string();

    let ev_res = call_mcp_tool(
        &mut conn,
        "upsert_calendar_event",
        json!({
            "title": "جلسه یکشنبه هماهنگی کلیدی",
            "event_type": "MEETING",
            "event_date": "2026-11-29",
            "start_time": "10:00",
            "end_time": "11:30",
            "project_id": "ALL",
            "task_ids": [t1_id, t2_id],
            "new_tasks": [
                {
                    "title": "چاپ صورت‌جلسه و چک‌لیست نهایی",
                    "project_id": "GERDALO",
                    "duration_minutes": 20
                },
                {
                    "title": "تایید زمان‌بندی فاز بعدی",
                    "project_id": "IRANOIL",
                    "duration_minutes": 30
                }
            ]
        }),
    )
    .expect("upsert_calendar_event");

    let ev_id = ev_res["event"]["id"]
        .as_str()
        .expect("event id")
        .to_string();
    assert_eq!(ev_res["linked_existing_count"], 2);
    assert_eq!(ev_res["created_and_linked_count"], 2);
    assert_eq!(ev_res["event"]["linked_tasks_count"], 4);

    // Complete one of the linked tasks and verify completion rollup
    call_mcp_tool(&mut conn, "complete_task", json!({ "task_id": t1_id }))
        .expect("complete linked task");

    let list_res = call_mcp_tool(
        &mut conn,
        "list_calendar_events",
        json!({ "status": "UPCOMING" }),
    )
    .expect("list_calendar_events");
    let ev = list_res["events"]
        .as_array()
        .expect("events array")
        .iter()
        .find(|e| e["id"].as_str() == Some(ev_id.as_str()))
        .expect("created event present");
    assert_eq!(ev["linked_tasks_count"], 4);
    assert_eq!(ev["linked_completed_count"], 1);

    let dash = mab_compass_lib::db::queries::get_dashboard_state_for_project(&conn, Some("GERDALO"))
        .expect("dashboard state");
    assert!(dash.calendar_events.iter().any(|e| e.id == ev_id && e.linked_tasks_count == 4));

    let snap = mab_compass_lib::llm::prompts::build_cross_project_portfolio_snapshot(&conn);
    assert!(snap.contains("UPCOMING CALENDAR EVENTS"));
    assert!(snap.contains("جلسه یکشنبه هماهنگی کلیدی"));
    assert!(snap.contains("linked=1/4 done"));
}

#[test]
fn test_queue_track_separation_and_packed_background_batch_prompt() {
    use mab_compass_lib::db::queries::classify_queue_kind;
    use mab_compass_lib::llm::prompts::{
        build_packed_background_batch_prompts, PackedBackgroundItemInput,
    };

    assert_eq!(classify_queue_kind("USER_CHAT", Some(42)), "USER_COMMAND");
    assert_eq!(classify_queue_kind("COPILOT_FILE", Some(43)), "USER_COMMAND");
    assert_eq!(classify_queue_kind("MAB_FOCUS_SYNC", None), "BACKGROUND");
    assert_eq!(classify_queue_kind("LEGACY_REPROCESS", None), "BACKGROUND");
    assert_eq!(classify_queue_kind("LEGACY_REPROCESS_DOC", None), "BACKGROUND");

    let conn = create_test_db();
    let dash = mab_compass_lib::db::queries::get_dashboard_state_for_project(&conn, Some("GERDALO"))
        .expect("dash");

    let packed: Vec<PackedBackgroundItemInput> = (1..=20)
        .map(|i| PackedBackgroundItemInput {
            queue_id: i,
            project_id: if i % 2 == 0 {
                "IRANOIL".to_string()
            } else {
                "GERDALO".to_string()
            },
            project_name: if i % 2 == 0 {
                "ایران‌اویل".to_string()
            } else {
                "گردالو".to_string()
            },
            source_type: "MAB_FOCUS_SYNC".to_string(),
            created_at: format!("2026-11-28T10:{:02}:00+03:30", i),
            knowledge_doc_id: Some(100 + i),
            title_preview: format!("گزارش خط {}", i),
            content: format!("یادداشت یک‌خطی شماره {} برای همگام‌سازی زمینه", i),
        })
        .collect();

    let (sys_prompt, user_prompt) = build_packed_background_batch_prompts(
        &dash.active_project.id,
        &packed,
        &dash.open_tasks,
        &dash.active_decisions,
        &dash.active_sops,
        &dash.methods_playbooks,
        &dash.projects,
        "",
    );

    assert!(sys_prompt.contains("PACKED BATCH of 20"));
    assert!(user_prompt.contains("PACKED_ITEM #1 | queue_id=1"));
    assert!(user_prompt.contains("PACKED_ITEM #20 | queue_id=20"));
    assert!(user_prompt.contains("source_type=MAB_FOCUS_SYNC"));
    assert!(user_prompt.contains("knowledge_doc_id=120"));
}

#[test]
fn test_draw_instruction_diagram_or_bpmn_tool() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let vault = tmp.path().join("vault");
    std::fs::create_dir_all(&vault).expect("vault dir");
    let mut conn = mab_compass_lib::db::schema::init_database(
        &tmp.path().join("smoke_bpmn.sqlite"),
        &vault,
    )
    .expect("init_database");

    let res = call_mcp_tool(
        &mut conn,
        "draw_instruction_diagram_or_bpmn",
        json!({
            "project_id": "GERDALO",
            "title": "دستورالعمل بررسی و انتشار نسخه",
            "category": "DEPLOY",
            "objective": "تضمین انتشار امن همراه با تست",
            "steps": [
                { "title": "اجرای تست واحد", "element_type": "serviceTask", "role": "CI" },
                { "title": "تایید تست‌ها؟", "element_type": "exclusiveGateway", "condition_label": "بله" },
                { "title": "ساخت و انتشار", "element_type": "userTask", "role": "DevOps" }
            ]
        }),
    )
    .expect("draw_instruction_diagram_or_bpmn");

    assert_eq!(res["status"], "ok");
    assert_eq!(res["steps_count"], 3);
    let mid = res["method_id"].as_str().expect("method_id");
    assert!(mid.starts_with("MTH-"));

    let dash = mab_compass_lib::db::queries::get_dashboard_state_for_project(&conn, Some("GERDALO"))
        .expect("dash");
    let m = dash
        .methods_playbooks
        .iter()
        .find(|x| x.id == mid)
        .expect("method persisted");
    assert!(m.bpmn_xml.contains("bpmn:exclusiveGateway"));
    assert!(std::path::Path::new(&m.archived_path).exists());
    assert!(std::path::Path::new(&m.bpmn_file_path).exists());
}
