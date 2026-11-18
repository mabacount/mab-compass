use crate::db::queries::{
    apply_copilot_envelope_with_artifacts, apply_copilot_envelope_with_event, create_new_project,
    get_dashboard_state_for_project, quick_complete_task, search_raw_events_fts,
    undo_last_mutation,
};
use crate::mab_focus_sync::synchronize_with_mab_focus;
use crate::models::{AtomicMutation, ChainStepInput, CopilotEnvelope};
use crate::projector::markdown_writer::sync_all_project_projections;
use crate::vault_manager::archive_raw_user_input;
use chrono::Local;
use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};
use uuid::Uuid;

fn legacy_tool_definitions() -> Value {
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

fn v3_tool_definitions() -> Value {
    json!([
        {
            "name": "upsert_domain_or_outcome",
            "description": "Update CADM life domain maturity/maintenance floors or insert/update strategic outcomes (OUT-xxxx).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["UPDATE_DOMAIN", "CREATE_OUTCOME", "UPDATE_OUTCOME", "UPSERT_OUTCOME"],
                        "description": "Action type"
                    },
                    "domain_code": {
                        "type": "string",
                        "description": "CADM domain code (MIND_LEARNING, HEALTH_ENERGY, WEALTH_FINANCE, RELATIONS, RESILIENCE, PURPOSE, SPIRITUALITY, SANCTUARY)"
                    },
                    "maturity_state": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 5,
                        "description": "Maturity level 0..5"
                    },
                    "maintenance_floor": { "type": "string", "description": "Daily maintenance floor statement" },
                    "desired_state": { "type": "string", "description": "Target aspirational state" },
                    "is_growth_focus": { "type": "boolean", "description": "Whether domain is active growth focus" },
                    "outcome_id": { "type": "string", "description": "Outcome ID (OUT-xxxx)" },
                    "title": { "type": "string", "description": "Outcome title" },
                    "desired_reality_change": { "type": "string", "description": "Desired reality change" },
                    "evidence_kr": { "type": "string", "description": "Key result evidence" },
                    "time_window": { "type": "string", "description": "Target quarter/time window (e.g. 1405-Q3)" },
                    "status": {
                        "type": "string",
                        "enum": ["ACTIVE", "ACHIEVED", "PAUSED", "DROPPED"],
                        "description": "Outcome status"
                    },
                    "progress_percent": { "type": "integer", "minimum": 0, "maximum": 100, "description": "Progress percentage" }
                },
                "required": ["action"]
            }
        },
        {
            "name": "create_work_line_chain",
            "description": "Atomically create a WorkLine and sequential chain of tasks with 4D context tagging. Step 1 is activated (IN_PROGRESS), steps 2..N are queued (QUEUED_IN_CHAIN) and locked until previous step completes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "Project ID (defaults to active project)" },
                    "line_title": { "type": "string", "description": "WorkLine title" },
                    "goal_statement": { "type": "string", "description": "Goal or deliverable statement" },
                    "line_mode": { "type": "string", "enum": ["SEQUENTIAL_CHAIN", "GOAL_CLUSTER"] },
                    "outcome_id": { "type": "string", "description": "Optional linked outcome ID (OUT-xxxx)" },
                    "steps": {
                        "type": "array",
                        "description": "List of sequential task steps to create atomically",
                        "items": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "definition_of_done": { "type": "string" },
                                "first_physical_action": { "type": "string" },
                                "category": { "type": "string" },
                                "energy_tag": { "type": "string", "enum": ["DEEP_HIGH", "MEDIUM_NORMAL", "LOW_ZOMBIE"] },
                                "duration_minutes": { "type": "integer" },
                                "device_tag": { "type": "string", "enum": ["LAPTOP_ONLY", "PHONE_OK", "AUDIO_HANDSFREE", "ANY_DEVICE"] },
                                "location_tags": { "type": "array", "items": { "type": "string" } },
                                "scheduled_date": { "type": "string" },
                                "scheduled_slot": { "type": "string" },
                                "due_date": { "type": "string" }
                            },
                            "required": ["title"]
                        }
                    }
                },
                "required": ["line_title", "steps"]
            }
        },
        {
            "name": "append_or_reorder_work_line",
            "description": "Append a new step to an existing WorkLine or modify the sequence order and predecessor dependency of existing tasks in the chain.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "work_line_id": { "type": "string", "description": "Target WorkLine ID (WLN-xxxx)" },
                    "task_id": { "type": "string", "description": "Task ID (TSK-xxxx)" },
                    "new_sequence_order": { "type": "integer", "minimum": 1, "description": "New 1-based sequence order" },
                    "depends_on_task_id": { "type": ["string", "null"], "description": "New predecessor task ID or null" },
                    "new_step": {
                        "type": "object",
                        "description": "New step to append to the work line",
                        "properties": {
                            "title": { "type": "string" },
                            "definition_of_done": { "type": "string" },
                            "first_physical_action": { "type": "string" },
                            "category": { "type": "string" },
                            "energy_tag": { "type": "string", "enum": ["DEEP_HIGH", "MEDIUM_NORMAL", "LOW_ZOMBIE"] },
                            "duration_minutes": { "type": "integer" },
                            "device_tag": { "type": "string", "enum": ["LAPTOP_ONLY", "PHONE_OK", "AUDIO_HANDSFREE", "ANY_DEVICE"] },
                            "location_tags": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["title"]
                    }
                },
                "required": ["work_line_id"]
            }
        },
        {
            "name": "query_tasks_by_context",
            "description": "Filter actionable tasks matching 4D execution context (Device, Location, Energy, Duration), strictly excluding chained locked tasks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "Optional project ID (defaults to active project)" },
                    "device_tag": { "type": "string", "enum": ["LAPTOP_ONLY", "PHONE_OK", "AUDIO_HANDSFREE", "ANY_DEVICE"] },
                    "location_tag": { "type": "string", "description": "Location context (DESK, METRO_COMMUTE, CAR_DRIVING, OUTSIDE_ERRAND)" },
                    "energy_tag": { "type": "string", "enum": ["DEEP_HIGH", "MEDIUM_NORMAL", "LOW_ZOMBIE"] },
                    "max_minutes": { "type": "integer", "description": "Maximum minutes available" },
                    "only_unlocked": { "type": "boolean", "default": true, "description": "Whether to exclude locked sequential chain steps" }
                }
            }
        },
        {
            "name": "schedule_task_on_calendar",
            "description": "Schedule a task on the daily calendar planner with date, slot, and duration, ensuring day capacity record exists.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "Task ID (TSK-xxxx)" },
                    "scheduled_date": { "type": "string", "description": "ISO date string (YYYY-MM-DD)" },
                    "scheduled_slot": {
                        "type": "string",
                        "enum": ["MORNING_DEEP", "MIDDAY_EXEC", "EVENING_WRAP", "NIGHT_REVIEW"]
                    },
                    "due_date": { "type": "string", "description": "Optional due date (YYYY-MM-DD)" },
                    "duration_minutes": { "type": "integer", "description": "Optional duration in minutes" }
                },
                "required": ["task_id", "scheduled_date"]
            }
        },
        {
            "name": "propose_project_lifecycle",
            "description": "Create a human-in-the-loop pending lifecycle proposal card (CREATE_PROJECT, PAUSE_PROJECT, ARCHIVE_PROJECT, PIVOT_PATH, RESUME_PROJECT) requiring user approval in chat.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "proposal_type": {
                        "type": "string",
                        "enum": ["CREATE_PROJECT", "PAUSE_PROJECT", "ARCHIVE_PROJECT", "PIVOT_PATH", "RESUME_PROJECT"],
                        "description": "Proposal action type"
                    },
                    "title": { "type": "string", "description": "Proposal card headline" },
                    "rationale": { "type": "string", "description": "Strategic rationale statement" },
                    "project_id": { "type": "string", "description": "Target project ID" },
                    "payload_json": { "type": "string", "description": "Optional JSON payload with parameters" }
                },
                "required": ["proposal_type", "title"]
            }
        },
        {
            "name": "save_personal_or_lesson_item",
            "description": "Store a memory card, lesson learned, instruction, personal interest, note, idea, or bookmark in the personal vault (PVI-xxxx).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "realm": { "type": "string", "enum": ["WORK", "LIFE", "LEARNING", "INTERESTS"] },
                    "domain_code": { "type": "string", "description": "CADM domain code (e.g. MIND_LEARNING)" },
                    "item_type": {
                        "type": "string",
                        "enum": ["MEMORY_CARD", "LESSON", "INSTRUCTION", "LESSON_LEARNED", "INTEREST", "IDEA", "BOOKMARK", "NOTE"]
                    },
                    "title": { "type": "string", "description": "Item title" },
                    "content_markdown": { "type": "string", "description": "Detailed Markdown content / lesson body" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "project_id": { "type": "string" }
                },
                "required": ["item_type", "title", "content_markdown"]
            }
        },
        {
            "name": "open_in_right_panel",
            "description": "Instruct the Mab-Compass UI to open or switch the right inspector drawer to a specific view and target entity.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "panel_type": {
                        "type": "string",
                        "enum": ["DOCUMENT", "ALL_TASKS_AND_LINES", "DOMAINS_OUTCOMES", "CALENDAR", "DECISIONS", "VAULT_DIR"]
                    },
                    "target_id": { "type": "string", "description": "Target entity ID, document ID, domain code, or date key" }
                },
                "required": ["panel_type"]
            }
        }
    ])
}

fn v4_method_tool_definition() -> Value {
    let sub_step_schema = json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "title": { "type": "string" },
            "description": { "type": "string" },
            "element_type": { "type": "string" },
            "role": { "type": "string" },
            "notes": { "type": "string" }
        },
        "required": ["title"]
    });
    let step_schema = json!({
        "type": "object",
        "properties": {
            "id": { "type": "string" },
            "order": { "type": "integer" },
            "title": { "type": "string" },
            "description": { "type": "string" },
            "element_type": {
                "type": "string",
                "enum": ["task", "userTask", "serviceTask", "subProcess", "exclusiveGateway", "parallelGateway", "callActivity"]
            },
            "role": { "type": "string" },
            "inputs": { "type": "array", "items": { "type": "string" } },
            "outputs": { "type": "array", "items": { "type": "string" } },
            "checklist": { "type": "array", "items": { "type": "string" } },
            "notes": { "type": "string" },
            "condition_label": { "type": "string" },
            "sub_method_id": { "type": "string" },
            "sub_steps": {
                "type": "array",
                "items": sub_step_schema
            }
        },
        "required": ["title"]
    });
    json!({
        "name": "upsert_method_playbook",
        "description": "Create or automatically enrich/version-bump a repeatable step-by-step Method / Playbook (دستورالعمل انجام کار) with a visual BPMN 2.0 workflow (supporting nested subProcess composite steps for drill-down). Persists to SQLite and writes Markdown + BPMN 2.0 XML into 04_methods_playbooks/.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "project_id": { "type": "string", "description": "Optional project ID (defaults to active project)" },
                "method_id": { "type": "string", "description": "Optional existing MTH-xxxx ID to enrich/update" },
                "title": { "type": "string", "description": "Clear Persian title of the method/playbook" },
                "category": { "type": "string", "enum": ["BACKEND", "UI_UX", "DEPLOY", "VIDEO", "ARCHITECTURE", "GENERAL"] },
                "summary": { "type": "string", "description": "Concise 1-2 sentence Persian summary shown on chat artifact card" },
                "objective": { "type": "string", "description": "Goal/objective of this method (هدف)" },
                "prerequisites": { "type": "array", "items": { "type": "string" } },
                "steps": {
                    "type": "array",
                    "description": "Ordered workflow steps; composite steps can use element_type='subProcess' with sub_steps[] for internal drill-down BPMN workflows",
                    "items": step_schema
                },
                "checklist": { "type": "array", "items": { "type": "string" } },
                "notes_markdown": { "type": "string" },
                "bpmn_xml": { "type": "string", "description": "Optional complete valid BPMN 2.0 XML (<bpmn:definitions>...); auto-generated from steps if omitted" },
                "parent_method_id": { "type": "string" },
                "parent_element_id": { "type": "string" }
            },
            "required": ["title", "steps"]
        }
    })
}

pub fn tool_definitions() -> Value {
    let mut all = legacy_tool_definitions()
        .as_array()
        .cloned()
        .unwrap_or_default();
    all.extend(v3_tool_definitions().as_array().cloned().unwrap_or_default());
    all.push(v4_method_tool_definition());
    Value::Array(all)
}

pub fn copilot_envelope_json_schema() -> Value {
    let method_step_schema = json!({
        "type": "object",
        "properties": {
            "id": { "type": ["string", "null"] },
            "order": { "type": ["integer", "null"] },
            "title": { "type": "string" },
            "description": { "type": ["string", "null"] },
            "element_type": { "type": ["string", "null"] },
            "role": { "type": ["string", "null"] },
            "checklist": { "type": "array", "items": { "type": "string" } },
            "notes": { "type": ["string", "null"] },
            "sub_steps": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "description": { "type": ["string", "null"] },
                        "element_type": { "type": ["string", "null"] }
                    },
                    "required": ["title"]
                }
            }
        },
        "required": ["title"]
    });
    let method_item_schema = json!({
        "type": "object",
        "properties": {
            "method_id": { "type": ["string", "null"] },
            "title": { "type": "string" },
            "category": { "type": ["string", "null"] },
            "summary": { "type": ["string", "null"] },
            "objective": { "type": ["string", "null"] },
            "prerequisites": { "type": "array", "items": { "type": "string" } },
            "steps": { "type": "array", "items": method_step_schema },
            "checklist": { "type": "array", "items": { "type": "string" } },
            "notes_markdown": { "type": ["string", "null"] },
            "bpmn_xml": { "type": ["string", "null"] }
        },
        "required": ["title"]
    });
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
            },
            "method_playbooks": {
                "type": "array",
                "items": method_item_schema
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
        .map(|mut t| {
            if let Some(props) = t.pointer_mut("/inputSchema/properties").and_then(|p| p.as_object_mut()) {
                for (_k, prop) in props.iter_mut() {
                    if let Some(prop_obj) = prop.as_object_mut() {
                        prop_obj.remove("description");
                    }
                }
            }
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
        ..Default::default()
    }
}

pub fn execute_tool(conn: &mut Connection, name: &str, args: &Value) -> Result<Value, String> {
    execute_tool_with_context(conn, name, args, None, None, "MCP_TOOL")
}

pub fn execute_mcp_tool(conn: &mut Connection, name: &str, args: &Value) -> Result<Value, String> {
    execute_tool(conn, name, args)
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
            let (badge, artifacts) = apply_copilot_envelope_with_artifacts(
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
            let mut out = json!({
                "badge": badge,
                "next_step_task": updated.next_step_task,
                "open_tasks": updated.open_tasks,
                "active_decisions": updated.active_decisions
            });
            if let Some(first_art) = artifacts.first() {
                if let Some(obj) = out.as_object_mut() {
                    obj.insert("artifact_ref".to_string(), json!(first_art));
                }
            }
            Ok(out)
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
                let note_preview: String = note.trim().chars().take(160).collect();
                let note_title: String = note
                    .lines()
                    .map(str::trim)
                    .find(|l| !l.is_empty())
                    .unwrap_or("یادداشت آرشیوشده")
                    .chars()
                    .take(64)
                    .collect();
                return Ok(json!({
                    "project_id": pid,
                    "archived_path": archived_path,
                    "status": "یادداشت خام در حافظه و 00_raw_inputs ثبت شد.",
                    "artifact_ref": {
                        "artifact_type": "MARKDOWN_FILE",
                        "entity_id": format!("NOTE-{}", &Uuid::new_v4().to_string()[..6].to_uppercase()),
                        "title": note_title,
                        "summary": note_preview,
                        "file_path": archived_path,
                        "action": "CREATED",
                        "version": null
                    }
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
            let mut out = json!({
                "badge": badge,
                "active_sops": updated.active_sops,
                "active_guardrails": updated.active_guardrails,
                "recent_logs": updated.recent_logs
            });
            if matches!(action.as_str(), "UPSERT_SOP" | "ADD_SOP") {
                if let Some(rule) = opt_str(args, "sop_rule_statement") {
                    let cat = opt_str(args, "category").unwrap_or_else(|| "GENERAL".into());
                    if let Some(obj) = out.as_object_mut() {
                        obj.insert(
                            "artifact_ref".to_string(),
                            json!({
                                "artifact_type": "SOP_GUARDRAIL",
                                "entity_id": format!("SOP-{}", &Uuid::new_v4().to_string()[..6].to_uppercase()),
                                "title": format!("قانون اجرایی ({})", cat),
                                "summary": rule.chars().take(180).collect::<String>(),
                                "file_path": null,
                                "action": "CREATED",
                                "version": null
                            }),
                        );
                    }
                }
            } else if action == "REGISTER_ANTIPATTERN" {
                if let Some(pattern) = opt_str(args, "mistake_pattern") {
                    let gate = opt_str(args, "preventive_gate_question").unwrap_or_default();
                    if let Some(obj) = out.as_object_mut() {
                        obj.insert(
                            "artifact_ref".to_string(),
                            json!({
                                "artifact_type": "SOP_GUARDRAIL",
                                "entity_id": format!("GRD-{}", &Uuid::new_v4().to_string()[..6].to_uppercase()),
                                "title": format!("سد ضدتکرار خطا: {}", pattern.chars().take(48).collect::<String>()),
                                "summary": if gate.is_empty() { pattern } else { gate },
                                "file_path": null,
                                "action": "CREATED",
                                "version": null
                            }),
                        );
                    }
                }
            }
            Ok(out)
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
        "upsert_domain_or_outcome" => {
            let action = opt_str(args, "action")
                .ok_or_else(|| "Missing required parameters: action".to_string())?;
            let domain_code = opt_str(args, "domain_code").unwrap_or_default();
            let now = Local::now().to_rfc3339();

            match action.as_str() {
                "UPDATE_DOMAIN" => {
                    if domain_code.is_empty() {
                        return Err("Missing required parameters: domain_code".to_string());
                    }
                    if let Some(maturity_val) = args.get("maturity_state").and_then(|v| v.as_i64()) {
                        if !(0..=5).contains(&maturity_val) {
                            return Err(format!("maturity_state {} is out of bounds (0..5)", maturity_val));
                        }
                        conn.execute(
                            "UPDATE life_domains SET maturity_state = ?1, updated_at = ?2 WHERE code = ?3",
                            params![maturity_val, now, domain_code],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    if let Some(floor) = opt_str(args, "maintenance_floor") {
                        conn.execute(
                            "UPDATE life_domains SET maintenance_floor = ?1, updated_at = ?2 WHERE code = ?3",
                            params![floor, now, domain_code],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    if let Some(desired) = opt_str(args, "desired_state") {
                        conn.execute(
                            "UPDATE life_domains SET desired_state = ?1, updated_at = ?2 WHERE code = ?3",
                            params![desired, now, domain_code],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    if let Some(is_focus) = args.get("is_growth_focus").and_then(|v| v.as_bool()) {
                        conn.execute(
                            "UPDATE life_domains SET is_growth_focus = ?1, updated_at = ?2 WHERE code = ?3",
                            params![if is_focus { 1 } else { 0 }, now, domain_code],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    Ok(json!({
                        "status": "ok",
                        "action": "UPDATE_DOMAIN",
                        "domain_code": domain_code
                    }))
                }
                "CREATE_OUTCOME" | "UPSERT_OUTCOME" => {
                    if domain_code.is_empty() {
                        return Err("Missing required parameters: domain_code".to_string());
                    }
                    let outcome_id = opt_str(args, "outcome_id")
                        .unwrap_or_else(|| format!("OUT-{}", &Uuid::new_v4().to_string()[..8].to_uppercase()));
                    let title = opt_str(args, "title").unwrap_or_else(|| "Untitled Outcome".to_string());
                    let desired_reality = opt_str(args, "desired_reality_change")
                        .or_else(|| opt_str(args, "desired_reality"))
                        .unwrap_or_default();
                    let evidence_kr = opt_str(args, "evidence_kr")
                        .or_else(|| opt_str(args, "success_metric"))
                        .unwrap_or_default();
                    let time_window = opt_str(args, "time_window").unwrap_or_else(|| "1405-Q3".to_string());
                    let status = opt_str(args, "status").unwrap_or_else(|| "ACTIVE".to_string());
                    let progress = opt_i32(args, "progress_percent")
                        .or_else(|| opt_i32(args, "progress_percentage"))
                        .unwrap_or(0);

                    let exists: i64 = conn
                        .query_row("SELECT COUNT(*) FROM outcomes WHERE id = ?1", params![outcome_id], |r| r.get(0))
                        .unwrap_or(0);

                    if exists > 0 {
                        conn.execute(
                            "UPDATE outcomes SET domain_code = ?1, title = ?2, desired_reality_change = ?3, desired_reality = ?3,
                                                evidence_kr = ?4, success_metric = ?4, time_window = ?5, status = ?6,
                                                progress_percent = ?7, progress_percentage = ?7, updated_at = ?8
                             WHERE id = ?9",
                            params![domain_code, title, desired_reality, evidence_kr, time_window, status, progress, now, outcome_id],
                        )
                        .map_err(|e| e.to_string())?;
                    } else {
                        conn.execute(
                            "INSERT INTO outcomes (
                                id, domain_code, title, desired_reality_change, evidence_kr, time_window,
                                status, progress_percent, created_at, updated_at, desired_reality, success_metric, progress_percentage
                            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9, ?4, ?5, ?8)",
                            params![outcome_id, domain_code, title, desired_reality, evidence_kr, time_window, status, progress, now],
                        )
                        .map_err(|e| e.to_string())?;
                    }

                    Ok(json!({
                        "status": "ok",
                        "action": action,
                        "outcome_id": outcome_id,
                        "domain_code": domain_code
                    }))
                }
                "UPDATE_OUTCOME" => {
                    let outcome_id = opt_str(args, "outcome_id")
                        .ok_or_else(|| "Missing outcome_id for UPDATE_OUTCOME".to_string())?;
                    if let Some(status) = opt_str(args, "status") {
                        conn.execute(
                            "UPDATE outcomes SET status = ?1, updated_at = ?2 WHERE id = ?3",
                            params![status, now, outcome_id],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    if let Some(progress) = opt_i32(args, "progress_percent").or_else(|| opt_i32(args, "progress_percentage")) {
                        conn.execute(
                            "UPDATE outcomes SET progress_percent = ?1, progress_percentage = ?1, updated_at = ?2 WHERE id = ?3",
                            params![progress, now, outcome_id],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    if let Some(evidence_kr) = opt_str(args, "evidence_kr").or_else(|| opt_str(args, "success_metric")) {
                        conn.execute(
                            "UPDATE outcomes SET evidence_kr = ?1, success_metric = ?1, updated_at = ?2 WHERE id = ?3",
                            params![evidence_kr, now, outcome_id],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    if let Some(title) = opt_str(args, "title") {
                        conn.execute(
                            "UPDATE outcomes SET title = ?1, updated_at = ?2 WHERE id = ?3",
                            params![title, now, outcome_id],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    if let Some(reality) = opt_str(args, "desired_reality_change").or_else(|| opt_str(args, "desired_reality")) {
                        conn.execute(
                            "UPDATE outcomes SET desired_reality_change = ?1, desired_reality = ?1, updated_at = ?2 WHERE id = ?3",
                            params![reality, now, outcome_id],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                    Ok(json!({
                        "status": "ok",
                        "action": "UPDATE_OUTCOME",
                        "outcome_id": outcome_id
                    }))
                }
                _ => Err(format!("Unknown action for upsert_domain_or_outcome: {}", action)),
            }
        }
        "create_work_line_chain" => {
            let pid = resolve_pid(args).unwrap_or_else(|| {
                conn.query_row("SELECT id FROM projects WHERE is_active = 1 LIMIT 1", [], |r| r.get(0))
                    .unwrap_or_else(|_| "GERDALO".to_string())
            });
            let line_title = opt_str(args, "line_title")
                .ok_or_else(|| "Missing required parameter: line_title".to_string())?;
            let goal_statement = opt_str(args, "goal_statement").unwrap_or_default();
            let line_mode = opt_str(args, "line_mode").unwrap_or_else(|| "SEQUENTIAL_CHAIN".to_string());
            let outcome_id = opt_str(args, "outcome_id");

            let steps_arr = args.get("steps").and_then(|v| v.as_array())
                .ok_or_else(|| "Missing required parameter: steps".to_string())?;
            if steps_arr.is_empty() {
                return Err("Chain requires at least 1 step".to_string());
            }

            let is_legacy_reprocess = source_origin.starts_with("LEGACY_REPROCESS");
            let mut steps_input = Vec::with_capacity(steps_arr.len());
            let mut skipped_duplicates = 0usize;
            for item in steps_arr {
                let step: ChainStepInput = serde_json::from_value(item.clone())
                    .map_err(|e| format!("Invalid step input: {}", e))?;
                if is_legacy_reprocess
                    && crate::method_engine::is_duplicate_task_title(conn, &pid, &step.title)
                {
                    skipped_duplicates += 1;
                    continue;
                }
                steps_input.push(step);
            }
            if steps_input.is_empty() {
                return Ok(json!({
                    "status": "skipped_duplicates",
                    "project_id": pid,
                    "skipped_duplicates": skipped_duplicates,
                    "tasks": []
                }));
            }

            let (work_line, created_tasks) = crate::db::queries::create_work_line_with_chain(
                conn,
                &pid,
                outcome_id.as_deref(),
                &line_title,
                &goal_statement,
                &line_mode,
                steps_input,
            )
            .map_err(|e| e.to_string())?;

            let task_ids: Vec<String> = created_tasks.into_iter().map(|t| t.id).collect();

            Ok(json!({
                "work_line_id": work_line.id,
                "project_id": pid,
                "tasks": task_ids,
                "artifact_ref": {
                    "artifact_type": "WORK_LINE",
                    "entity_id": work_line.id,
                    "title": work_line.title,
                    "summary": if work_line.goal_statement.trim().is_empty() {
                        format!("لاین کاری ({}) با {} گام اجرایی", work_line.line_mode, task_ids.len())
                    } else {
                        work_line.goal_statement
                    },
                    "file_path": null,
                    "action": "CREATED",
                    "version": null
                }
            }))
        }
        "append_or_reorder_work_line" => {
            let line_id = opt_str(args, "work_line_id")
                .ok_or_else(|| "Missing required parameter: work_line_id".to_string())?;
            let now = Local::now().to_rfc3339();

            // Verify work_line exists
            let line_exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM work_lines WHERE id = ?1",
                params![line_id],
                |r| r.get(0),
            ).map_err(|e| e.to_string())?;
            if line_exists == 0 {
                return Err(format!("work_line_id '{}' not found", line_id));
            }

            if let Some(new_step_val) = args.get("new_step") {
                let step: ChainStepInput = serde_json::from_value(new_step_val.clone())
                    .map_err(|e| format!("Invalid new_step input: {}", e))?;

                let (max_seq, last_id): (i32, Option<String>) = conn
                    .query_row(
                        "SELECT COALESCE(MAX(sequence_order), 0), (SELECT id FROM work_entities WHERE work_line_id = ?1 ORDER BY sequence_order DESC LIMIT 1) FROM work_entities WHERE work_line_id = ?1",
                        params![line_id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .unwrap_or((0, None));

                let new_seq = max_seq + 1;
                let new_task_id = format!("TSK-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());
                let is_step_one = max_seq == 0;
                let status = if is_step_one { "IN_PROGRESS" } else { "QUEUED_IN_CHAIN" };
                let is_locked = if is_step_one { 0 } else { 1 };
                let depends_on = if is_step_one { None } else { last_id };

                let project_id: String = conn.query_row(
                    "SELECT project_id FROM work_lines WHERE id = ?1",
                    params![line_id],
                    |r| r.get(0),
                ).map_err(|e| e.to_string())?;

                if source_origin.starts_with("LEGACY_REPROCESS")
                    && crate::method_engine::is_duplicate_task_title(conn, &project_id, &step.title)
                {
                    return Ok(json!({
                        "status": "skipped_duplicate",
                        "work_line_id": line_id,
                        "title": step.title
                    }));
                }

                let locs_json = serde_json::to_string(&step.location_tags)
                    .unwrap_or_else(|_| "[\"DESK\"]".to_string());

                conn.execute(
                    "INSERT INTO work_entities (
                        id, project_id, entity_type, category, title, definition_of_done, first_physical_action,
                        status, is_on_critical_path, optimistic_pomos, most_likely_pomos, pessimistic_pomos, calibrated_pomos,
                        work_line_id, sequence_order, depends_on_task_id, is_locked_by_chain,
                        energy_tag, duration_minutes, device_tag, location_tags_json, priority_score, updated_at
                    ) VALUES (
                        ?1, ?2, 'TASK', ?3, ?4, ?5, ?6,
                        ?7, ?8, 1.0, 2.0, 3.0, 2.0,
                        ?9, ?10, ?11, ?12,
                        ?13, ?14, ?15, ?16, 50, ?17
                    )",
                    params![
                        new_task_id, project_id, step.category, step.title, step.definition_of_done, step.first_physical_action,
                        status, if is_step_one { 1 } else { 0 },
                        line_id, new_seq, depends_on, is_locked,
                        step.energy_tag, step.duration_minutes, step.device_tag, locs_json, now
                    ],
                ).map_err(|e| e.to_string())?;

                Ok(json!({
                    "status": "ok",
                    "work_line_id": line_id,
                    "task_id": new_task_id
                }))
            } else if let Some(task_id) = opt_str(args, "task_id") {
                if let Some(new_seq) = opt_i32(args, "new_sequence_order") {
                    let dep = args.get("depends_on_task_id").and_then(|v| {
                        if v.is_null() {
                            None
                        } else {
                            v.as_str()
                        }
                    });
                    conn.execute(
                        "UPDATE work_entities SET sequence_order = ?1, depends_on_task_id = ?2, updated_at = ?3 WHERE id = ?4",
                        params![new_seq, dep, now, task_id],
                    ).map_err(|e| e.to_string())?;
                }
                Ok(json!({
                    "status": "ok",
                    "work_line_id": line_id,
                    "task_id": task_id
                }))
            } else {
                Err("Missing required parameter: task_id or new_step".to_string())
            }
        }
        "query_tasks_by_context" => {
            let pid = resolve_pid(args).unwrap_or_else(|| {
                conn.query_row("SELECT id FROM projects WHERE is_active = 1 LIMIT 1", [], |r| r.get(0))
                    .unwrap_or_else(|_| "GERDALO".to_string())
            });
            let dev = opt_str(args, "device_tag");
            let loc = opt_str(args, "location_tag");
            let energy = opt_str(args, "energy_tag");
            let max_min = opt_i32(args, "max_minutes");
            let only_unlocked = args.get("only_unlocked").and_then(|v| v.as_bool()).unwrap_or(true);

            let tasks = crate::db::queries::query_tasks_by_context(
                conn,
                &pid,
                dev.as_deref(),
                loc.as_deref(),
                energy.as_deref(),
                max_min,
                only_unlocked,
            )
            .map_err(|e| e.to_string())?;

            let count = tasks.len();
            Ok(json!({
                "tasks": tasks,
                "count": count
            }))
        }
        "schedule_task_on_calendar" => {
            let task_id = opt_str(args, "task_id")
                .ok_or_else(|| "Missing task_id".to_string())?;
            let scheduled_date = opt_str(args, "scheduled_date")
                .ok_or_else(|| "Missing scheduled_date".to_string())?;
            let now = Local::now().to_rfc3339();

            // Validate ISO date format YYYY-MM-DD
            if scheduled_date.len() != 10
                || scheduled_date.chars().nth(4) != Some('-')
                || scheduled_date.chars().nth(7) != Some('-')
            {
                return Err(format!(
                    "Invalid date format '{}', expected ISO YYYY-MM-DD",
                    scheduled_date
                ));
            }

            let slot = opt_str(args, "scheduled_slot").unwrap_or_else(|| "MORNING_DEEP".to_string());
            let due_date = opt_str(args, "due_date");

            if let Some(dur) = opt_i32(args, "duration_minutes") {
                conn.execute(
                    "UPDATE work_entities SET scheduled_date = ?1, scheduled_slot = ?2, due_date = ?3, duration_minutes = ?4, updated_at = ?5 WHERE id = ?6",
                    params![scheduled_date, slot, due_date, dur, now, task_id],
                ).map_err(|e| e.to_string())?;
            } else {
                conn.execute(
                    "UPDATE work_entities SET scheduled_date = ?1, scheduled_slot = ?2, due_date = ?3, updated_at = ?4 WHERE id = ?5",
                    params![scheduled_date, slot, due_date, now, task_id],
                ).map_err(|e| e.to_string())?;
            }

            // Ensure calendar_days record exists for scheduled_date
            conn.execute(
                "INSERT OR IGNORE INTO calendar_days (date_key, total_capacity_minutes, deep_capacity_minutes, day_note, is_off_day, updated_at)
                 VALUES (?1, 300, 150, '', 0, ?2)",
                params![scheduled_date, now],
            ).map_err(|e| e.to_string())?;

            Ok(json!({
                "task_id": task_id,
                "scheduled_date": scheduled_date,
                "scheduled_slot": slot,
                "status": "scheduled"
            }))
        }
        "propose_project_lifecycle" => {
            let proposal_type = opt_str(args, "proposal_type")
                .ok_or_else(|| "Missing proposal_type".to_string())?;
            if !matches!(proposal_type.as_str(), "CREATE_PROJECT" | "PAUSE_PROJECT" | "ARCHIVE_PROJECT" | "PIVOT_PATH" | "RESUME_PROJECT") {
                return Err(format!("Invalid proposal_type: {}", proposal_type));
            }

            let title = opt_str(args, "title")
                .ok_or_else(|| "Missing title".to_string())?;
            let rationale = opt_str(args, "rationale")
                .or_else(|| opt_str(args, "rationale_markdown"))
                .unwrap_or_default();
            let project_id = opt_str(args, "project_id");
            let payload_json = opt_str(args, "payload_json").unwrap_or_else(|| "{}".to_string());
            let now = Local::now().to_rfc3339();

            let proposal_id = format!("PRP-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());

            conn.execute(
                "INSERT INTO pending_proposals (
                    id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at, rationale, proposed_payload_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'PENDING', ?7, ?5, ?6)",
                params![proposal_id, project_id, proposal_type, title, rationale, payload_json, now],
            ).map_err(|e| e.to_string())?;

            Ok(json!({
                "proposal_id": proposal_id,
                "proposal_type": proposal_type,
                "status": "PENDING",
                "title": title
            }))
        }
        "save_personal_or_lesson_item" => {
            let item_type = opt_str(args, "item_type")
                .ok_or_else(|| "Missing item_type".to_string())?;
            if !matches!(
                item_type.as_str(),
                "MEMORY_CARD" | "LESSON" | "INSTRUCTION" | "LESSON_LEARNED" | "INTEREST" | "IDEA" | "BOOKMARK" | "NOTE"
            ) {
                return Err(format!("Invalid item_type: {}", item_type));
            }

            let title = opt_str(args, "title")
                .ok_or_else(|| "Missing title".to_string())?;
            let content = opt_str(args, "content_markdown")
                .ok_or_else(|| "Missing content_markdown".to_string())?;
            let realm = opt_str(args, "realm").unwrap_or_else(|| "LIFE".to_string());
            let domain_code = opt_str(args, "domain_code").unwrap_or_else(|| "MIND_LEARNING".to_string());
            let project_id = resolve_pid(args);
            let tags_val = args.get("tags").cloned().unwrap_or_else(|| json!([]));
            let tags_json = tags_val.to_string();
            let now = Local::now().to_rfc3339();

            let item_id = format!("PVI-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());

            conn.execute(
                "INSERT INTO personal_vault_items (
                    id, realm, domain_code, project_id, item_type, title, content_markdown, tags_json, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                params![item_id, realm, domain_code, project_id, item_type, title, content, tags_json, now],
            ).map_err(|e| e.to_string())?;

            let summary_preview: String = content.trim().chars().take(160).collect();
            Ok(json!({
                "item_id": item_id,
                "item_type": item_type,
                "domain_code": domain_code,
                "title": title,
                "artifact_ref": {
                    "artifact_type": "VAULT_CARD",
                    "entity_id": item_id,
                    "title": title,
                    "summary": summary_preview,
                    "file_path": null,
                    "action": "CREATED",
                    "version": null
                }
            }))
        }
        "upsert_method_playbook" => {
            let req_pid = resolve_pid(args);
            let dash = get_dashboard_state_for_project(conn, req_pid.as_deref())
                .map_err(|e| e.to_string())?;
            let pid = dash.active_project.id.clone();
            let workspace_path = dash.active_project.workspace_path.clone();

            let input: crate::models::MethodPlaybookInput = serde_json::from_value(args.clone())
                .map_err(|e| format!("ورودی نامعتبر برای upsert_method_playbook: {}", e))?;
            let src_ref = existing_event_id
                .map(|id| format!("event:{}", id))
                .unwrap_or_else(|| format!("mcp:{}", source_origin));

            let (method, is_created, artifact_ref) = crate::method_engine::upsert_method_playbook(
                conn,
                &pid,
                &workspace_path,
                input,
                Some(&src_ref),
            )?;

            Ok(json!({
                "status": "ok",
                "action": if is_created { "CREATED" } else { "UPDATED" },
                "method_id": method.id,
                "title": method.title,
                "version": method.version,
                "archived_path": method.archived_path,
                "bpmn_file_path": method.bpmn_file_path,
                "steps_count": method.steps.len(),
                "artifact_ref": artifact_ref
            }))
        }
        "open_in_right_panel" => {
            let panel_type = opt_str(args, "panel_type")
                .ok_or_else(|| "Missing required parameter: panel_type".to_string())?;
            if !matches!(
                panel_type.as_str(),
                "DOCUMENT" | "ALL_TASKS_AND_LINES" | "DOMAINS_OUTCOMES" | "CALENDAR" | "DECISIONS" | "VAULT_DIR"
            ) {
                return Err(format!("Invalid panel_type: {}", panel_type));
            }
            let target_id = opt_str(args, "target_id");

            Ok(json!({
                "navigation_event": {
                    "panel_type": panel_type,
                    "target_id": target_id
                }
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
