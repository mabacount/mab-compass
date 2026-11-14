use crate::db::queries::{
    apply_copilot_envelope_with_event, create_new_project, get_dashboard_state_for_project,
    quick_complete_task, search_raw_events_fts, undo_last_mutation,
};
use crate::mab_focus_sync::synchronize_with_mab_focus;
use crate::models::{AtomicMutation, CopilotEnvelope};
use crate::projector::markdown_writer::sync_all_project_projections;
use crate::vault_manager::archive_raw_user_input;
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};

pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "list_projects",
            "description": "List all Mab-Compass projects, showing which project is active, workspace vault path, and Mab-Focus link ID.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "switch_project",
            "description": "Switch the active project in Mab-Compass and sync its 8-folder Markdown projections.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "Project ID or exact project name" }
                },
                "required": ["project_id"]
            }
        },
        {
            "name": "create_project",
            "description": "Create a new Mab-Compass project with its 8-folder Vault directories and make it active.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id_code": { "type": "string", "description": "Short project code (e.g. GERDALO, ZIGHOOSH)" },
                    "name": { "type": "string", "description": "Human-readable project name" }
                },
                "required": ["id_code", "name"]
            }
        },
        {
            "name": "get_path_ahead",
            "description": "Read the live Path Ahead (next_step_task, horizon_tasks), pre-flight guardrails, active SOP rules, open tasks, active decisions (ADRs), and recent work logs for a project (defaults to active project).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "Optional project ID (defaults to active project)" }
                }
            }
        },
        {
            "name": "search_memory",
            "description": "Full-text search (SQLite FTS5) over all raw_events (chat messages, ingested docs, Mab-Focus notes) for a project.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keywords" },
                    "project_id": { "type": "string", "description": "Optional project ID (defaults to active project)" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "create_work_entity",
            "description": "Create a new TASK, DECISION (ADR), or BLOCKER with PERT estimation, Undo snapshot, and Markdown projection sync.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string" },
                    "entity_type": { "type": "string", "enum": ["TASK", "DECISION", "BLOCKER"] },
                    "category": { "type": "string", "enum": ["BACKEND", "UI_UX", "DEPLOY", "VIDEO", "ARCHITECTURE", "GENERAL"] },
                    "title": { "type": "string" },
                    "definition_of_done": { "type": "string" },
                    "first_physical_action": { "type": "string" },
                    "status": { "type": "string", "enum": ["OPEN", "IN_PROGRESS", "BLOCKED", "CLOSED"] },
                    "is_on_critical_path": { "type": "boolean" },
                    "optimistic_pomos": { "type": "number" },
                    "most_likely_pomos": { "type": "number" },
                    "pessimistic_pomos": { "type": "number" },
                    "stakeholder_tag": { "type": "string" }
                },
                "required": ["title"]
            }
        },
        {
            "name": "update_work_entity",
            "description": "Update an existing TASK, DECISION, or BLOCKER by target_id (e.g. TSK-xxxx, ADR-xxxx, BLK-xxxx).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string" },
                    "target_id": { "type": "string" },
                    "entity_type": { "type": "string", "enum": ["TASK", "DECISION", "BLOCKER"] },
                    "category": { "type": "string", "enum": ["BACKEND", "UI_UX", "DEPLOY", "VIDEO", "ARCHITECTURE", "GENERAL"] },
                    "title": { "type": "string" },
                    "definition_of_done": { "type": "string" },
                    "first_physical_action": { "type": "string" },
                    "status": { "type": "string", "enum": ["OPEN", "IN_PROGRESS", "BLOCKED", "CLOSED", "SUPERSEDED"] },
                    "is_on_critical_path": { "type": "boolean" },
                    "optimistic_pomos": { "type": "number" },
                    "most_likely_pomos": { "type": "number" },
                    "pessimistic_pomos": { "type": "number" },
                    "stakeholder_tag": { "type": "string" }
                },
                "required": ["target_id"]
            }
        },
        {
            "name": "set_next_step",
            "description": "Promote an existing task (TSK-xxxx) to be the #1 active step on the Path Ahead (status=IN_PROGRESS, is_on_critical_path=true).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string" },
                    "target_id": { "type": "string" },
                    "title": { "type": "string" },
                    "definition_of_done": { "type": "string" },
                    "first_physical_action": { "type": "string" }
                },
                "required": ["target_id"]
            }
        },
        {
            "name": "complete_task",
            "description": "Mark a task as CLOSED, log effective pomodoros in session_work_logs, and advance the Path Ahead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string" },
                    "task_id": { "type": "string" },
                    "pomos_spent": { "type": "integer", "minimum": 1 }
                },
                "required": ["task_id"]
            }
        },
        {
            "name": "delete_work_entity",
            "description": "Delete a wrong or duplicate TASK, DECISION, or BLOCKER by target_id (reversible with undo_last_change).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string" },
                    "target_id": { "type": "string" }
                },
                "required": ["target_id"]
            }
        },
        {
            "name": "log_work_and_knowledge",
            "description": "Record a work log (LOG_WORK), SOP rule (UPSERT_SOP), antipattern guardrail (REGISTER_ANTIPATTERN), or raw memory note (INGEST_NOTE) into 00_raw_inputs + SQLite.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string" },
                    "action": { "type": "string", "enum": ["LOG_WORK", "UPSERT_SOP", "REGISTER_ANTIPATTERN", "INGEST_NOTE"] },
                    "task_id": { "type": "string" },
                    "category": { "type": "string", "enum": ["BACKEND", "UI_UX", "DEPLOY", "VIDEO", "ARCHITECTURE", "GENERAL"] },
                    "effective_pomos": { "type": "integer", "minimum": 1 },
                    "deliverable_summary": { "type": "string" },
                    "internal_friction_note": { "type": "string" },
                    "client_sanitized_note": { "type": "string" },
                    "sop_rule_statement": { "type": "string" },
                    "mistake_pattern": { "type": "string" },
                    "preventive_gate_question": { "type": "string" },
                    "severity_score": { "type": "integer", "minimum": 1, "maximum": 10 },
                    "note_content": { "type": "string" }
                },
                "required": ["action"]
            }
        },
        {
            "name": "sync_mab_focus",
            "description": "Run bidirectional project sync and deduplicated session/note ingestion with Mab-Focus (mab_focus_v3.db / mab_focus.db).",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }
        },
        {
            "name": "undo_last_change",
            "description": "Undo the last work-graph mutation on the specified or active project and re-sync Markdown projections.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string" }
                }
            }
        }
    ])
}

pub fn copilot_envelope_json_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "conversational_reply": { "type": "string" },
            "clarifying_questions": {
                "type": "array",
                "items": { "type": "string" }
            },
            "needs_user_clarification": { "type": "boolean" },
            "meeting_detected": { "type": "boolean" },
            "meeting_title": { "type": ["string", "null"] },
            "meeting_summary_markdown": { "type": ["string", "null"] },
            "executive_doc_summary": { "type": ["string", "null"] },
            "mutations": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "op": {
                            "type": "string",
                            "enum": [
                                "RESOLVE_ENTITY",
                                "CREATE_ENTITY",
                                "UPDATE_ENTITY",
                                "SET_NEXT_STEP",
                                "DELETE_ENTITY",
                                "SUPERSEDE_DECISION",
                                "UPSERT_SOP",
                                "REGISTER_ANTIPATTERN",
                                "LOG_WORK"
                            ]
                        },
                        "target_id": { "type": ["string", "null"] },
                        "entity_type": { "type": ["string", "null"] },
                        "category": { "type": ["string", "null"] },
                        "title": { "type": ["string", "null"] },
                        "definition_of_done": { "type": ["string", "null"] },
                        "first_physical_action": { "type": ["string", "null"] },
                        "status": { "type": ["string", "null"] },
                        "is_on_critical_path": { "type": ["boolean", "null"] },
                        "optimistic_pomos": { "type": ["number", "null"] },
                        "most_likely_pomos": { "type": ["number", "null"] },
                        "pessimistic_pomos": { "type": ["number", "null"] },
                        "stakeholder_tag": { "type": ["string", "null"] },
                        "sop_rule_statement": { "type": ["string", "null"] },
                        "mistake_pattern": { "type": ["string", "null"] },
                        "preventive_gate_question": { "type": ["string", "null"] },
                        "severity_score": { "type": ["number", "null"] },
                        "effective_pomos": { "type": ["number", "null"] },
                        "deliverable_summary": { "type": ["string", "null"] },
                        "internal_friction_note": { "type": ["string", "null"] },
                        "client_sanitized_note": { "type": ["string", "null"] }
                    },
                    "required": ["op"]
                }
            }
        },
        "required": ["conversational_reply", "mutations"]
    })
}

fn sanitize_schema_for_gemini(val: &Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                if matches!(
                    k.as_str(),
                    "additionalProperties" | "minimum" | "maximum"
                ) {
                    continue;
                }
                if k == "type" {
                    if let Some(arr) = v.as_array() {
                        let non_null = arr
                            .iter()
                            .filter_map(|x| x.as_str())
                            .find(|s| *s != "null")
                            .unwrap_or("string");
                        out.insert("type".to_string(), json!(non_null));
                        out.insert("nullable".to_string(), json!(true));
                        continue;
                    }
                }
                out.insert(k.clone(), sanitize_schema_for_gemini(v));
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sanitize_schema_for_gemini).collect()),
        other => other.clone(),
    }
}

pub fn gemini_copilot_response_schema() -> Value {
    sanitize_schema_for_gemini(&copilot_envelope_json_schema())
}

pub fn openai_tools_payload() -> Value {
    let defs = tool_definitions();
    let arr = defs.as_array().cloned().unwrap_or_default();
    let tools: Vec<Value> = arr
        .into_iter()
        .filter(|t| {
            let n = t["name"].as_str().unwrap_or("");
            !matches!(n, "switch_project" | "create_project")
        })
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t["name"],
                    "description": t["description"],
                    "parameters": t["inputSchema"]
                }
            })
        })
        .collect();
    Value::Array(tools)
}

pub fn gemini_tools_payload() -> Value {
    let defs = tool_definitions();
    let arr = defs.as_array().cloned().unwrap_or_default();
    let decls: Vec<Value> = arr
        .into_iter()
        .filter(|t| {
            let n = t["name"].as_str().unwrap_or("");
            !matches!(n, "switch_project" | "create_project")
        })
        .map(|t| {
            json!({
                "name": t["name"],
                "description": t["description"],
                "parameters": sanitize_schema_for_gemini(&t["inputSchema"])
            })
        })
        .collect();
    json!([{ "functionDeclarations": decls }])
}

pub fn resource_definitions() -> Value {
    json!([
        {
            "uri": "compass://projects",
            "name": "Mab-Compass Projects",
            "description": "List of all projects and active project selection",
            "mimeType": "application/json"
        },
        {
            "uri": "compass://active/path-ahead",
            "name": "Active Project Path Ahead",
            "description": "Live Path Ahead step, horizon tasks, pre-flight guardrails, SOPs, and active decisions",
            "mimeType": "application/json"
        },
        {
            "uri": "compass://active/wbs",
            "name": "Active Project WBS",
            "description": "All open tasks and work breakdown structure for the active project",
            "mimeType": "application/json"
        }
    ])
}

fn opt_str(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn opt_i32(args: &Value, key: &str) -> Option<i32> {
    args.get(key).and_then(|v| {
        v.as_i64()
            .map(|n| n as i32)
            .or_else(|| v.as_f64().map(|f| f.round() as i32))
    })
}

fn opt_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn empty_mutation(op: &str) -> AtomicMutation {
    AtomicMutation {
        op: op.to_string(),
        target_id: None,
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
    }
}

pub fn execute_tool(conn: &mut Connection, name: &str, args: &Value) -> Result<Value, String> {
    execute_tool_with_context(conn, name, args, None, None, "MCP_TOOL")
}

pub fn execute_tool_with_context(
    conn: &mut Connection,
    name: &str,
    args: &Value,
    default_project_id: Option<&str>,
    existing_event_id: Option<i64>,
    source_origin: &str,
) -> Result<Value, String> {
    let resolve_pid = |args: &Value| -> Option<String> {
        opt_str(args, "project_id").or_else(|| default_project_id.map(str::to_string))
    };

    match name {
        "list_projects" => {
            let dash = get_dashboard_state_for_project(conn, None).map_err(|e| e.to_string())?;
            Ok(json!({
                "active_project_id": dash.active_project.id,
                "projects": dash.projects
            }))
        }
        "switch_project" => {
            let req = opt_str(args, "project_id")
                .ok_or_else(|| "فیلد project_id الزامی است.".to_string())?;
            let dash = get_dashboard_state_for_project(conn, Some(&req))
                .map_err(|_| format!("پروژه «{}» یافت نشد.", req))?;
            conn.execute(
                "UPDATE projects SET is_active = CASE WHEN id = ?1 THEN 1 ELSE 0 END",
                params![dash.active_project.id],
            )
            .map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, None).map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({
                "switched_to": updated.active_project,
                "next_step_task": updated.next_step_task,
                "open_tasks_count": updated.open_tasks.len()
            }))
        }
        "create_project" => {
            let id_code = opt_str(args, "id_code")
                .ok_or_else(|| "فیلد id_code الزامی است.".to_string())?;
            let name_str = opt_str(args, "name")
                .ok_or_else(|| "فیلد name الزامی است.".to_string())?;
            let created = create_new_project(conn, &id_code, &name_str).map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, None).map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({ "created_project": created }))
        }
        "get_path_ahead" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| format!("پروژه یافت نشد: {}", e))?;
            Ok(json!({
                "project": dash.active_project,
                "next_step_task": dash.next_step_task,
                "horizon_tasks": dash.horizon_tasks,
                "active_guardrails": dash.active_guardrails,
                "active_sops": dash.active_sops,
                "open_tasks": dash.open_tasks,
                "closed_tasks_count": dash.closed_tasks_count,
                "active_decisions": dash.active_decisions,
                "recent_logs": dash.recent_logs,
                "knowledge_docs": dash.knowledge_docs
            }))
        }
        "search_memory" => {
            let query = opt_str(args, "query")
                .ok_or_else(|| "فیلد query الزامی است.".to_string())?;
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let hits = search_raw_events_fts(conn, &dash.active_project.id, &query)
                .map_err(|e| e.to_string())?;
            let formatted: Vec<Value> = hits
                .into_iter()
                .map(|(created_at, content)| json!({ "created_at": created_at, "content": content }))
                .collect();
            Ok(json!({
                "project_id": dash.active_project.id,
                "query": query,
                "hits_count": formatted.len(),
                "hits": formatted
            }))
        }
        "create_work_entity" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let pid = dash.active_project.id.clone();
            let title = opt_str(args, "title")
                .ok_or_else(|| "فیلد title الزامی است.".to_string())?;

            let mut m = empty_mutation("CREATE_ENTITY");
            m.entity_type = Some(opt_str(args, "entity_type").unwrap_or_else(|| "TASK".into()));
            m.category = Some(opt_str(args, "category").unwrap_or_else(|| "GENERAL".into()));
            m.title = Some(title.clone());
            m.definition_of_done = opt_str(args, "definition_of_done");
            m.first_physical_action = opt_str(args, "first_physical_action");
            m.status = opt_str(args, "status");
            m.is_on_critical_path = args.get("is_on_critical_path").and_then(|v| v.as_bool());
            m.optimistic_pomos = opt_f64(args, "optimistic_pomos");
            m.most_likely_pomos = opt_f64(args, "most_likely_pomos");
            m.pessimistic_pomos = opt_f64(args, "pessimistic_pomos");
            m.stakeholder_tag = opt_str(args, "stakeholder_tag");

            let env = CopilotEnvelope {
                conversational_reply: format!("MCP CREATE_ENTITY: {}", title),
                mutations: vec![m],
                ..Default::default()
            };
            let badge = apply_copilot_envelope_with_event(
                conn,
                &pid,
                &format!("[MCP] create_work_entity: {}", title),
                None,
                source_origin,
                &env,
                existing_event_id,
            )
            .map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, Some(&pid))
                .map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({
                "badge": badge,
                "next_step_task": updated.next_step_task,
                "open_tasks": updated.open_tasks,
                "active_decisions": updated.active_decisions
            }))
        }
        "update_work_entity" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let pid = dash.active_project.id.clone();
            let target_id = opt_str(args, "target_id")
                .ok_or_else(|| "فیلد target_id الزامی است.".to_string())?;

            let mut m = empty_mutation("UPDATE_ENTITY");
            m.target_id = Some(target_id.clone());
            m.entity_type = opt_str(args, "entity_type");
            m.category = opt_str(args, "category");
            m.title = opt_str(args, "title");
            m.definition_of_done = opt_str(args, "definition_of_done");
            m.first_physical_action = opt_str(args, "first_physical_action");
            m.status = opt_str(args, "status");
            m.is_on_critical_path = args.get("is_on_critical_path").and_then(|v| v.as_bool());
            m.optimistic_pomos = opt_f64(args, "optimistic_pomos");
            m.most_likely_pomos = opt_f64(args, "most_likely_pomos");
            m.pessimistic_pomos = opt_f64(args, "pessimistic_pomos");
            m.stakeholder_tag = opt_str(args, "stakeholder_tag");

            let env = CopilotEnvelope {
                conversational_reply: format!("MCP UPDATE_ENTITY: {}", target_id),
                mutations: vec![m],
                ..Default::default()
            };
            let badge = apply_copilot_envelope_with_event(
                conn,
                &pid,
                &format!("[MCP] update_work_entity: {}", target_id),
                None,
                source_origin,
                &env,
                existing_event_id,
            )
            .map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, Some(&pid))
                .map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({
                "badge": badge,
                "next_step_task": updated.next_step_task,
                "open_tasks": updated.open_tasks
            }))
        }
        "set_next_step" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let pid = dash.active_project.id.clone();
            let target_id = opt_str(args, "target_id")
                .ok_or_else(|| "فیلد target_id الزامی است.".to_string())?;

            let mut m = empty_mutation("SET_NEXT_STEP");
            m.target_id = Some(target_id.clone());
            m.title = opt_str(args, "title");
            m.definition_of_done = opt_str(args, "definition_of_done");
            m.first_physical_action = opt_str(args, "first_physical_action");

            let env = CopilotEnvelope {
                conversational_reply: format!("MCP SET_NEXT_STEP: {}", target_id),
                mutations: vec![m],
                ..Default::default()
            };
            let badge = apply_copilot_envelope_with_event(
                conn,
                &pid,
                &format!("[MCP] set_next_step: {}", target_id),
                None,
                source_origin,
                &env,
                existing_event_id,
            )
            .map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, Some(&pid))
                .map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({
                "badge": badge,
                "next_step_task": updated.next_step_task,
                "horizon_tasks": updated.horizon_tasks
            }))
        }
        "complete_task" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let pid = dash.active_project.id.clone();
            let task_id = opt_str(args, "task_id")
                .ok_or_else(|| "فیلد task_id الزامی است.".to_string())?;
            let pomos = opt_i32(args, "pomos_spent").unwrap_or(1).max(1);
            let msg = quick_complete_task(conn, &pid, &task_id, pomos)
                .map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, Some(&pid))
                .map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({
                "message": msg,
                "next_step_task": updated.next_step_task,
                "closed_tasks_count": updated.closed_tasks_count
            }))
        }
        "delete_work_entity" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let pid = dash.active_project.id.clone();
            let target_id = opt_str(args, "target_id")
                .ok_or_else(|| "فیلد target_id الزامی است.".to_string())?;

            let mut m = empty_mutation("DELETE_ENTITY");
            m.target_id = Some(target_id.clone());

            let env = CopilotEnvelope {
                conversational_reply: format!("MCP DELETE_ENTITY: {}", target_id),
                mutations: vec![m],
                ..Default::default()
            };
            let badge = apply_copilot_envelope_with_event(
                conn,
                &pid,
                &format!("[MCP] delete_work_entity: {}", target_id),
                None,
                source_origin,
                &env,
                existing_event_id,
            )
            .map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, Some(&pid))
                .map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({
                "badge": badge,
                "open_tasks_count": updated.open_tasks.len()
            }))
        }
        "log_work_and_knowledge" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let pid = dash.active_project.id.clone();
            let action = opt_str(args, "action")
                .ok_or_else(|| "فیلد action الزامی است.".to_string())?;

            if action == "INGEST_NOTE" {
                let note = opt_str(args, "note_content")
                    .ok_or_else(|| "برای INGEST_NOTE فیلد note_content الزامی است.".to_string())?;
                let archived_path = archive_raw_user_input(
                    &dash.active_project.workspace_path,
                    "mcp_note",
                    &note,
                )?;
                let now = chrono::Local::now().to_rfc3339();
                conn.execute(
                    "INSERT INTO raw_events (project_id, raw_content, archived_path, source_type, created_at)
                     VALUES (?1, ?2, ?3, 'MCP_NOTE', ?4)",
                    params![pid, note, archived_path, now],
                )
                .map_err(|e| e.to_string())?;
                return Ok(json!({
                    "project_id": pid,
                    "archived_path": archived_path,
                    "status": "یادداشت خام در حافظه و 00_raw_inputs ثبت شد."
                }));
            }

            let mut m = empty_mutation(&action);
            m.target_id = opt_str(args, "task_id");
            m.category = Some(opt_str(args, "category").unwrap_or_else(|| "GENERAL".into()));
            m.effective_pomos = opt_i32(args, "effective_pomos");
            m.deliverable_summary = opt_str(args, "deliverable_summary");
            m.internal_friction_note = opt_str(args, "internal_friction_note");
            m.client_sanitized_note = opt_str(args, "client_sanitized_note");
            m.sop_rule_statement = opt_str(args, "sop_rule_statement");
            m.mistake_pattern = opt_str(args, "mistake_pattern");
            m.preventive_gate_question = opt_str(args, "preventive_gate_question");
            m.severity_score = opt_i32(args, "severity_score");

            let env = CopilotEnvelope {
                conversational_reply: format!("MCP {}", action),
                mutations: vec![m],
                ..Default::default()
            };
            let badge = apply_copilot_envelope_with_event(
                conn,
                &pid,
                &format!("[MCP] {}", action),
                None,
                source_origin,
                &env,
                existing_event_id,
            )
            .map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, Some(&pid))
                .map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({
                "badge": badge,
                "active_sops": updated.active_sops,
                "active_guardrails": updated.active_guardrails,
                "recent_logs": updated.recent_logs
            }))
        }
        "sync_mab_focus" => {
            let report = synchronize_with_mab_focus(conn)?;
            let updated = get_dashboard_state_for_project(conn, None).map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({ "sync_report": report }))
        }
        "undo_last_change" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let pid = dash.active_project.id.clone();
            let msg = undo_last_mutation(conn, &pid).map_err(|e| e.to_string())?;
            let updated = get_dashboard_state_for_project(conn, Some(&pid))
                .map_err(|e| e.to_string())?;
            let _ = sync_all_project_projections(&updated, None, None);
            Ok(json!({
                "message": msg,
                "next_step_task": updated.next_step_task
            }))
        }
        _ => Err(format!("ابزار ناشناخته: {}", name)),
    }
}

pub fn read_resource(conn: &Connection, uri: &str) -> Result<Value, String> {
    let dash = get_dashboard_state_for_project(conn, None).map_err(|e| e.to_string())?;
    let payload = match uri {
        "compass://projects" => json!({
            "active_project_id": dash.active_project.id,
            "projects": dash.projects
        }),
        "compass://active/path-ahead" => json!({
            "project": dash.active_project,
            "next_step_task": dash.next_step_task,
            "horizon_tasks": dash.horizon_tasks,
            "active_guardrails": dash.active_guardrails,
            "active_sops": dash.active_sops,
            "active_decisions": dash.active_decisions
        }),
        "compass://active/wbs" => json!({
            "project": dash.active_project,
            "open_tasks": dash.open_tasks,
            "closed_tasks_count": dash.closed_tasks_count
        }),
        _ => return Err(format!("منبع یافت نشد: {}", uri)),
    };
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        }]
    }))
}

pub fn handle_jsonrpc_request(conn: &mut Connection, req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = req.get("id").cloned();

    let req_id = match id {
        Some(v) if !v.is_null() => v,
        _ => return None,
    };

    let result: Result<Value, (i32, String)> = match method {
        "initialize" => {
            let client_ver = req
                .pointer("/params/protocolVersion")
                .and_then(|v| v.as_str())
                .unwrap_or("2024-11-05");
            let server_ver = if client_ver == "2025-03-26" {
                "2025-03-26"
            } else {
                "2024-11-05"
            };
            Ok(json!({
                "protocolVersion": server_ver,
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "subscribe": false, "listChanged": false }
                },
                "serverInfo": {
                    "name": "mab-compass-mcp",
                    "version": "2.0.0"
                }
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let tool_name = req
                .pointer("/params/name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = req
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            match execute_tool(conn, tool_name, &args) {
                Ok(val) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string())
                    }],
                    "isError": false
                })),
                Err(err) => Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": err
                    }],
                    "isError": true
                })),
            }
        }
        "resources/list" => Ok(json!({ "resources": resource_definitions() })),
        "resources/read" => {
            let uri = req
                .pointer("/params/uri")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            read_resource(conn, uri).map_err(|e| (-32602, e))
        }
        _ => Err((-32601, format!("Method not found: {}", method))),
    };

    Some(match result {
        Ok(res) => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": res
        }),
        Err((code, msg)) => json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": { "code": code, "message": msg }
        }),
    })
}
