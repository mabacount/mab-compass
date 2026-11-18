use mab_compass_lib::db::queries::{
    append_assistant_chat_message_with_artifacts, apply_copilot_envelope_with_artifacts,
    get_dashboard_state, query_pending_ai_queue, sync_unprocessed_items_into_queue,
};
use mab_compass_lib::db::schema::init_database;
use mab_compass_lib::mcp_tools::execute_tool_with_context;
use mab_compass_lib::method_engine::{
    backup_sqlite_database, enqueue_legacy_reprocess, is_valid_bpmn_xml,
    list_legacy_reprocess_candidates, mark_legacy_reprocessed, upsert_method_playbook,
};
use mab_compass_lib::models::{
    AtomicMutation, CopilotEnvelope, LegacyReprocessSelection, MethodPlaybookInput, MethodStep,
};
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[test]
fn test_v4_mcp_upsert_method_playbook_and_vault_card_artifacts() {
    let dir = std::env::temp_dir().join(format!("mab_v4_mcp_{}", Uuid::new_v4()));
    let vault = dir.join("vault");
    fs::create_dir_all(&vault).unwrap();
    let mut conn = init_database(&dir.join("v4.sqlite"), &vault).unwrap();

    // 1. Create a Method Playbook via MCP tool with composite subProcess
    let res = execute_tool_with_context(
        &mut conn,
        "upsert_method_playbook",
        &json!({
            "project_id": "GERDALO",
            "title": "متد انتشار نسخه تولید و بررسی کیفیت",
            "category": "DEPLOY",
            "summary": "روش استاندارد انتشار نسخه بدون قطعی به همراه چک‌لیست تست",
            "objective": "تضمین کیفیت و پایداری در هر انتشار",
            "prerequisites": ["تست‌های واحد سبز باشند", "بکاپ دیتابیس آماده باشد"],
            "steps": [
                {
                    "id": "S1",
                    "order": 1,
                    "title": "بررسی کد و بیلد",
                    "description": "اجرای tsc و cargo test",
                    "element_type": "serviceTask",
                    "role": "توسعه‌دهنده"
                },
                {
                    "id": "S2",
                    "order": 2,
                    "title": "مجموعه کار استقرار روی سرور",
                    "description": "آپلود باینری و ری‌استارت سرویس",
                    "element_type": "subProcess",
                    "sub_steps": [
                        { "title": "آپلود فایل باینری", "element_type": "serviceTask" },
                        { "title": "تایید Health Check", "element_type": "userTask" }
                    ]
                }
            ],
            "checklist": ["تایید لاگ بدون خطا"],
            "notes_markdown": "همیشه قبل از انتشار بکاپ بگیرید."
        }),
        Some("GERDALO"),
        None,
        "USER_CHAT",
    )
    .expect("upsert_method_playbook MCP tool should succeed");

    assert_eq!(res["status"], "ok");
    assert_eq!(res["action"], "CREATED");
    assert_eq!(res["version"], 1);
    assert_eq!(res["artifact_ref"]["artifact_type"], "METHOD_PLAYBOOK");

    let md_path = res["archived_path"].as_str().unwrap();
    let bpmn_path = res["bpmn_file_path"].as_str().unwrap();
    assert!(Path::new(md_path).exists(), "Markdown playbook should exist on disk");
    assert!(Path::new(bpmn_path).exists(), "BPMN 2.0 XML should exist on disk");
    assert!(md_path.contains("04_methods_playbooks"));

    let bpmn_xml = fs::read_to_string(bpmn_path).unwrap();
    assert!(is_valid_bpmn_xml(&bpmn_xml));
    assert!(bpmn_xml.contains("<bpmn:subProcess"));

    // 2. Test save_personal_or_lesson_item with MEMORY_CARD on production schema (no legacy `content` column!)
    let card_res = execute_tool_with_context(
        &mut conn,
        "save_personal_or_lesson_item",
        &json!({
            "project_id": "GERDALO",
            "realm": "WORK",
            "domain_code": "MIND_LEARNING",
            "item_type": "MEMORY_CARD",
            "title": "الگوی VACUUM INTO در SQLite",
            "content_markdown": "برای پشتیبان‌گیری اتمیک در حالت WAL از `VACUUM INTO` استفاده کنید.",
            "tags": ["SQLite", "Backup"]
        }),
        Some("GERDALO"),
        None,
        "USER_CHAT",
    )
    .expect("save_personal_or_lesson_item with MEMORY_CARD should succeed on production schema");

    assert_eq!(card_res["item_type"], "MEMORY_CARD");
    assert_eq!(card_res["artifact_ref"]["artifact_type"], "VAULT_CARD");

    let dash = get_dashboard_state(&conn).unwrap();
    assert_eq!(dash.methods_playbooks.len(), 1);
    assert_eq!(dash.methods_playbooks[0].steps.len(), 2);
    assert_eq!(dash.methods_playbooks[0].steps[1].sub_steps.len(), 2);
    assert!(dash
        .personal_vault_items
        .iter()
        .any(|c| c.title == "الگوی VACUUM INTO در SQLite"));

    drop(conn);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_v4_written_artifacts_persisted_on_chat_messages_and_invalid_xml_fallback() {
    let dir = std::env::temp_dir().join(format!("mab_v4_chat_art_{}", Uuid::new_v4()));
    let vault = dir.join("vault");
    fs::create_dir_all(&vault).unwrap();
    let mut conn = init_database(&dir.join("v4_chat.sqlite"), &vault).unwrap();

    // Test invalid BPMN XML gracefully falls back to valid auto-generated BPMN 2.0 XML
    let env = CopilotEnvelope {
        conversational_reply: "متد جدید و تصمیم معماری ثبت شد.".into(),
        mutations: vec![AtomicMutation {
            op: "CREATE_ENTITY".into(),
            entity_type: Some("DECISION".into()),
            category: Some("ARCHITECTURE".into()),
            title: Some("استفاده از BPMN 2.0 برای مستندسازی ورک‌فلوها".into()),
            definition_of_done: Some("رندر نمودار و قابلیت Drill-Down".into()),
            ..Default::default()
        }],
        method_playbooks: vec![MethodPlaybookInput {
            title: "متد تحلیل گزارش‌های روزانه".into(),
            objective: Some("استخراج منظم درس‌آموخته‌ها".into()),
            steps: vec![MethodStep {
                title: "مرور گزارش کار".into(),
                ..Default::default()
            }],
            bpmn_xml: Some("<broken><xml".into()), // deliberately malformed XML
            ..Default::default()
        }],
        ..Default::default()
    };

    let (badge, artifacts) = apply_copilot_envelope_with_artifacts(
        &mut conn,
        "GERDALO",
        "گزارش کار",
        None,
        "USER_CHAT",
        &env,
        None,
    )
    .unwrap();

    assert!(badge.contains("متد/BPMN: 1"));
    assert_eq!(artifacts.len(), 2); // 1 DECISION_ADR + 1 METHOD_PLAYBOOK

    let dash_before = get_dashboard_state(&conn).unwrap();
    assert!(is_valid_bpmn_xml(&dash_before.methods_playbooks[0].bpmn_xml));

    append_assistant_chat_message_with_artifacts(
        &conn,
        "GERDALO",
        &dash_before.active_copilot_session_id,
        "COPILOT",
        "ASSISTANT",
        &env.conversational_reply,
        None,
        &badge,
        &[],
        &artifacts,
    )
    .unwrap();

    let dash_after = get_dashboard_state(&conn).unwrap();
    let last_msg = dash_after.chat_history.last().unwrap();
    assert_eq!(last_msg.written_artifacts.len(), 2);
    assert!(last_msg
        .written_artifacts
        .iter()
        .any(|a| a.artifact_type == "METHOD_PLAYBOOK" && a.title == "متد تحلیل گزارش‌های روزانه"));
    assert!(last_msg
        .written_artifacts
        .iter()
        .any(|a| a.artifact_type == "DECISION_ADR"));

    // Empty title validation edge case
    let empty_err = upsert_method_playbook(
        &conn,
        "GERDALO",
        &dash_after.active_project.workspace_path,
        MethodPlaybookInput {
            title: "   ".into(),
            ..Default::default()
        },
        None,
    );
    assert!(empty_err.is_err());

    drop(conn);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_v4_legacy_reprocess_anti_duplicate_backup_and_queue_persistence() {
    let dir = std::env::temp_dir().join(format!("mab_v4_legacy_{}", Uuid::new_v4()));
    let vault = dir.join("vault");
    fs::create_dir_all(&vault).unwrap();
    let mut conn = init_database(&dir.join("v4_legacy.sqlite"), &vault).unwrap();

    // Seed an already-processed legacy doc and an existing task
    conn.execute(
        "INSERT INTO work_entities (id, project_id, entity_type, category, title, status, updated_at)
         VALUES ('TSK-OLD1', 'GERDALO', 'TASK', 'BACKEND', 'ساخت سرویس احراز هویت JWT', 'OPEN', '2026-10-01T10:00:00Z')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO knowledge_documents (project_id, title, doc_type, original_filename, archived_path, executive_summary, extracted_items_count, ingested_at, processing_status)
         VALUES ('GERDALO', 'معماری احراز هویت قدیمی', 'TECHNICAL_DOC', 'auth_old.md', '/tmp/auth_old.md', 'سند قدیمی پردازش‌شده در نسخه ۲', 3, '2026-10-02T10:00:00Z', 'PROCESSED')",
        [],
    )
    .unwrap();

    // 1. Atomic backup produces a real readable SQLite database with the existing task
    let backup = backup_sqlite_database(&conn, &dir.join("backups")).unwrap();
    let backup_conn = Connection::open(&backup.backup_path).unwrap();
    let backed_up_tasks: i64 = backup_conn
        .query_row("SELECT COUNT(*) FROM work_entities WHERE id = 'TSK-OLD1'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(backed_up_tasks, 1);
    drop(backup_conn);

    // 2. Enqueue legacy doc and ensure sync_unprocessed_items_into_queue does NOT auto-close it!
    let cands = list_legacy_reprocess_candidates(&conn, "GERDALO").unwrap();
    assert_eq!(cands.len(), 1);
    let enq = enqueue_legacy_reprocess(
        &conn,
        "GERDALO",
        "SELECTED",
        &[LegacyReprocessSelection {
            source_kind: "KNOWLEDGE_DOC".into(),
            source_id: cands[0].source_id,
        }],
    )
    .unwrap();
    assert_eq!(enq, 1);

    let _ = sync_unprocessed_items_into_queue(&conn).unwrap();
    let queue = query_pending_ai_queue(&conn).unwrap();
    assert_eq!(
        queue.len(),
        1,
        "LEGACY_REPROCESS_DOC item must stay PENDING even though knowledge_doc is PROCESSED"
    );
    assert_eq!(queue[0].source_type, "LEGACY_REPROCESS_DOC");

    // 3. Anti-duplicate guard during LEGACY_REPROCESS_DOC skips duplicate task but extracts new Method
    let legacy_env = CopilotEnvelope {
        conversational_reply: "متد احراز هویت استخراج شد.".into(),
        mutations: vec![
            AtomicMutation {
                op: "CREATE_ENTITY".into(),
                entity_type: Some("TASK".into()),
                category: Some("BACKEND".into()),
                title: Some("ساخت سرویس احراز هویت JWT".into()), // duplicate!
                ..Default::default()
            },
            AtomicMutation {
                op: "CREATE_ENTITY".into(),
                entity_type: Some("TASK".into()),
                category: Some("BACKEND".into()),
                title: Some("افزودن تست انقضای توکن Refresh".into()), // genuinely new
                ..Default::default()
            },
        ],
        method_playbooks: vec![MethodPlaybookInput {
            title: "متد پیاده‌سازی احراز هویت JWT".into(),
            steps: vec![MethodStep {
                title: "تولید کلید امضای توکن".into(),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let (_badge, arts) = apply_copilot_envelope_with_artifacts(
        &mut conn,
        "GERDALO",
        "legacy doc text",
        None,
        "LEGACY_REPROCESS_DOC",
        &legacy_env,
        None,
    )
    .unwrap();

    let dash = get_dashboard_state(&conn).unwrap();
    // Should be 2 total open tasks (1 original + 1 genuinely new, duplicate skipped!)
    assert_eq!(dash.open_tasks.len(), 2);
    assert_eq!(dash.methods_playbooks.len(), 1);
    assert_eq!(arts.len(), 1);

    mark_legacy_reprocessed(
        &conn,
        "KNOWLEDGE_DOC",
        cands[0].source_id,
        "GERDALO",
        "done",
    )
    .unwrap();
    let cands_after = list_legacy_reprocess_candidates(&conn, "GERDALO").unwrap();
    assert!(cands_after[0].already_upgraded);

    drop(conn);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_v4_regression_chronological_queue_mcp_antidup_and_bpmn_edge_cases() {
    let dir = std::env::temp_dir().join(format!("mab_v4_reg_{}", Uuid::new_v4()));
    let vault = dir.join("vault");
    let proj_ws = vault.join("GERDALO");
    fs::create_dir_all(&proj_ws).unwrap();
    let mut conn = init_database(&dir.join("v4_reg.sqlite"), &vault).unwrap();

    // 1. Empty steps BPMN 2.0 XML has consistent start/end sequenceFlow IDs (no dangling _Flow_End!)
    let (empty_m, _, _) = upsert_method_playbook(
        &conn,
        "GERDALO",
        &proj_ws.to_string_lossy(),
        MethodPlaybookInput {
            title: "متد بدون گام اولیه".into(),
            steps: vec![],
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert!(is_valid_bpmn_xml(&empty_m.bpmn_xml));
    assert!(empty_m.bpmn_xml.contains("<bpmn:incoming>Elem_Flow_Start</bpmn:incoming>"));
    assert!(!empty_m.bpmn_xml.contains("Elem_Flow_End"));

    // 2. Duplicate / digit-starting / Persian step IDs become unique valid XML NCNames
    let (dup_id_m, _, _) = upsert_method_playbook(
        &conn,
        "GERDALO",
        &proj_ws.to_string_lossy(),
        MethodPlaybookInput {
            title: "متد با شناسه‌های تکراری و فارسی".into(),
            steps: vec![
                MethodStep {
                    id: "1-گام".into(),
                    title: "اول".into(),
                    ..Default::default()
                },
                MethodStep {
                    id: "1-گام".into(),
                    title: "دوم".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        None,
    )
    .unwrap();
    assert!(is_valid_bpmn_xml(&dup_id_m.bpmn_xml));
    assert!(dup_id_m.bpmn_xml.contains("Elem_N_1"));
    assert!(dup_id_m.bpmn_xml.contains("_2\""));

    // 3. Strict chronological order in pending_ai_queue across KNOWLEDGE_DOC (Oct 1) and RAW_EVENT (Oct 5)
    conn.execute(
        "INSERT INTO knowledge_documents (project_id, title, doc_type, original_filename, archived_path, executive_summary, extracted_items_count, ingested_at, processing_status)
         VALUES ('GERDALO', 'سند قدیمی اکتبر ۱', 'TECHNICAL_DOC', 'oct1.md', '/tmp/oct1.md', 'سند قدیمی‌تر', 1, '2026-10-01T08:00:00Z', 'PROCESSED')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO raw_events (project_id, raw_content, archived_path, source_type, created_at)
         VALUES ('GERDALO', 'گزارش کار مفصل پنجم اکتبر برای بررسی ترتیب زمانی صف بازپردازش حافظه قدیمی', '/tmp/oct5.md', 'USER_CHAT', '2026-10-05T09:00:00Z')",
        [],
    )
    .unwrap();

    let enq = enqueue_legacy_reprocess(&conn, "GERDALO", "ALL_CHRONOLOGICAL", &[]).unwrap();
    assert_eq!(enq, 2);
    let queue = query_pending_ai_queue(&conn).unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(
        queue[0].source_type, "LEGACY_REPROCESS_DOC",
        "Older KNOWLEDGE_DOC (Oct 1) must be queued BEFORE newer RAW_EVENT (Oct 5)!"
    );
    assert_eq!(queue[1].source_type, "LEGACY_REPROCESS_EVENT");

    // 4. MCP create_work_entity(DECISION) and log_work_and_knowledge(UPSERT_SOP) emit artifact_ref
    let adr_res = execute_tool_with_context(
        &mut conn,
        "create_work_entity",
        &json!({
            "project_id": "GERDALO",
            "entity_type": "DECISION",
            "category": "ARCHITECTURE",
            "title": "تصمیم استفاده از DAG برای چیدمان BPMN",
            "definition_of_done": "چرخه‌ها بدون بهم‌ریختگی رندر شوند"
        }),
        Some("GERDALO"),
        None,
        "USER_CHAT",
    )
    .unwrap();
    assert_eq!(adr_res["artifact_ref"]["artifact_type"], "DECISION_ADR");

    let sop_res = execute_tool_with_context(
        &mut conn,
        "log_work_and_knowledge",
        &json!({
            "project_id": "GERDALO",
            "action": "UPSERT_SOP",
            "category": "DEPLOY",
            "sop_rule_statement": "قبل از هر بازپردازش حافظه حتماً بکاپ اتمیک SQLite گرفته شود"
        }),
        Some("GERDALO"),
        None,
        "USER_CHAT",
    )
    .unwrap();
    assert_eq!(sop_res["artifact_ref"]["artifact_type"], "SOP_GUARDRAIL");

    // 5. MCP create_work_line_chain skips duplicate tasks during LEGACY_REPROCESS_DOC
    conn.execute(
        "INSERT INTO work_entities (id, project_id, entity_type, category, title, status, updated_at)
         VALUES ('TSK-EX1', 'GERDALO', 'TASK', 'BACKEND', 'پیاده‌سازی سیستم بکاپ خودکار', 'OPEN', '2026-10-01T10:00:00Z')",
        [],
    )
    .unwrap();
    let wl_dup = execute_tool_with_context(
        &mut conn,
        "create_work_line_chain",
        &json!({
            "project_id": "GERDALO",
            "line_title": "لاین بکاپ",
            "steps": [
                { "title": "پیاده سازی سیستم بکاپ خودکار", "category": "BACKEND" }
            ]
        }),
        Some("GERDALO"),
        None,
        "LEGACY_REPROCESS_DOC",
    )
    .unwrap();
    assert_eq!(wl_dup["status"], "skipped_duplicates");
    assert_eq!(wl_dup["skipped_duplicates"], 1);

    drop(conn);
    let _ = fs::remove_dir_all(dir);
}
