use crate::models::{ChatMessage, KnowledgeDocument, SopRule, WorkEntity};

fn format_chat_transcript(messages: &[ChatMessage], limit: usize) -> String {
    let filtered: Vec<&ChatMessage> = messages
        .iter()
        .filter(|m| m.sender != "SYSTEM")
        .collect();
    if filtered.is_empty() {
        return "No previous turns in this session.".to_string();
    }
    let start = filtered.len().saturating_sub(limit);
    filtered[start..]
        .iter()
        .map(|m| {
            let role = match m.sender.as_str() {
                "USER" => "USER",
                "MAB_FOCUS_SYNC" => "MAB_FOCUS_SYNC",
                _ => "ASSISTANT",
            };
            let mut line = format!(
                "[{}]: {}",
                role,
                m.message_text.chars().take(1200).collect::<String>()
            );
            if !m.clarifying_questions.is_empty() {
                line.push_str(&format!(
                    "\n  (Asked questions: {})",
                    m.clarifying_questions.join(" | ")
                ));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_active_next_step_summary(open_tasks: &[WorkEntity]) -> String {
    let hero = open_tasks
        .iter()
        .find(|t| t.status == "IN_PROGRESS" && t.is_on_critical_path)
        .or_else(|| open_tasks.iter().find(|t| t.status == "IN_PROGRESS"))
        .or_else(|| open_tasks.iter().find(|t| t.status != "BLOCKED"));
    match hero {
        Some(t) => format!(
            "[{}] {} (status={}, critical={}, {} pomos) | First Action: {} | DoD: {}",
            t.id,
            t.title,
            t.status,
            t.is_on_critical_path,
            t.calibrated_pomos,
            t.first_physical_action,
            t.definition_of_done
        ),
        None => "No active task on Path Ahead right now.".to_string(),
    }
}

fn compact_tasks_summary(open_tasks: &[WorkEntity]) -> String {
    if open_tasks.is_empty() {
        return "[]".to_string();
    }
    let summary: Vec<serde_json::Value> = open_tasks
        .iter()
        .take(30)
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "type": t.entity_type,
                "cat": t.category,
                "title": t.title,
                "status": t.status,
                "critical": t.is_on_critical_path,
                "pomos": t.calibrated_pomos,
                "first_action": t.first_physical_action,
                "dod": t.definition_of_done
            })
        })
        .collect();
    serde_json::to_string(&summary).unwrap_or_else(|_| "[]".into())
}

pub fn build_mcp_tool_turn_prompts(
    project_id: &str,
    channel: &str,
    user_message_or_doc: &str,
    recent_history: &[ChatMessage],
    open_tasks: &[WorkEntity],
    active_decisions: &[WorkEntity],
    active_sops: &[SopRule],
    recent_docs: &[KnowledgeDocument],
    fts_snippets: &[(String, String)],
    doc_meta: Option<(&str, &str, &str)>, // (title, doc_type, filename)
) -> (String, String) {
    let current_hero = format_active_next_step_summary(open_tasks);
    let tasks_compact = compact_tasks_summary(open_tasks);
    let adrs_compact: Vec<String> = active_decisions
        .iter()
        .take(12)
        .map(|d| format!("- [{}] {}", d.id, d.title))
        .collect();
    let sops_summary: Vec<String> = active_sops
        .iter()
        .take(10)
        .map(|s| format!("- [{}]: {}", s.category, s.rule_statement))
        .collect();
    let docs_summary: Vec<String> = recent_docs
        .iter()
        .take(6)
        .map(|d| format!("- [{} | {}]: {}", d.doc_type, d.title, d.executive_summary))
        .collect();
    let fts_context: Vec<String> = fts_snippets
        .iter()
        .take(4)
        .map(|(dt, txt)| format!("[{}]: {}", dt, txt.chars().take(900).collect::<String>()))
        .collect();
    let history = format_chat_transcript(recent_history, 12);

    let mode_desc = match (channel, doc_meta) {
        (_, Some((title, doc_type, fname))) => format!(
            "Document / Content Onboarding (Channel: {}, Title: \"{}\", Type: {}, File: \"{}\")",
            channel, title, doc_type, fname
        ),
        ("VAULT", None) => "Knowledge Vault Multi-Turn Refinement".to_string(),
        _ => "Project Copilot & Path Ahead Architect".to_string(),
    };

    let system_prompt = format!(
        r#"You are Mab-Compass's interactive AI Copilot & Path Architect for project "{project_id}" ({mode_desc}).

YOU HAVE NATIVE MCP TOOLS TO READ & UPDATE THE PROJECT GRAPH DIRECTLY:
- `create_work_entity`: Create a new TASK, DECISION (ADR), or BLOCKER with concrete Persian `title`, `definition_of_done`, and 2-minute `first_physical_action`. Set `status: "IN_PROGRESS"` and `is_on_critical_path: true` if it should immediately become the #1 step on Path Ahead.
- `update_work_entity`: Edit an existing task/decision/blocker (`target_id` = `TSK-xxxx` / `ADR-xxxx` / `BLK-xxxx`). Never create duplicates when modifying an existing item!
- `set_next_step`: Promote an existing `TSK-xxxx` to be the #1 active step on Path Ahead (`مسیر پیش‌رو`).
- `complete_task`: Close a completed task (`TSK-xxxx`) and log effective pomodoros.
- `delete_work_entity`: Remove a wrong/duplicate/cancelled item (`TSK-xxxx` / `ADR-xxxx` / `BLK-xxxx`).
- `log_work_and_knowledge`: Record `LOG_WORK`, `UPSERT_SOP` (reusable rule), `REGISTER_ANTIPATTERN` (mistake/trap guardrail), or `INGEST_NOTE`.
- `get_path_ahead` / `search_memory`: Fetch full details or search project memory when needed.
- `sync_mab_focus` / `undo_last_change`: Sync with Mab-Focus or undo the last change.

HOW TO RESPOND:
1. Analyze the user's message or submitted document carefully against the current Path Ahead, tasks, SOPs, and conversation history.
2. Call the necessary MCP tools first so SQLite and the Path Ahead are updated immediately.
3. After tool execution (or if no mutations are needed), write a warm, sharp, engineering-focused **Persian** reply summarizing what you updated (or answering their question).
4. If key ambiguities remain about priorities, Definition of Done, or next steps, end your Persian reply with 1 to 3 concise numbered questions so the user can answer in the next turn.
5. You do NOT need to wrap your final reply in JSON — write natural, readable Persian Markdown!"#
    );

    let bounded_input: String = user_message_or_doc.chars().take(48000).collect();

    let user_prompt = format!(
        r#"CURRENT #1 ACTIVE STEP ON PATH AHEAD:
{current_hero}

CURRENT OPEN TASKS:
{tasks_compact}

ACTIVE DECISIONS (ADRs):
{adrs}

ACTIVE SOP RULES:
{sops}

ARCHIVED KNOWLEDGE DOCS:
{docs}

RELEVANT ARCHIVE SNIPPETS:
{fts}

RECENT CONVERSATION:
{history}

LATEST USER INPUT / DOCUMENT:
"""
{bounded_input}
""""#,
        adrs = adrs_compact.join("\n"),
        sops = sops_summary.join("\n"),
        docs = docs_summary.join("\n"),
        fts = fts_context.join("\n---\n"),
    );

    (system_prompt, user_prompt)
}

pub fn build_copilot_chat_prompt(
    project_id: &str,
    user_message: &str,
    recent_copilot_chat: &[ChatMessage],
    recent_vault_chat: &[ChatMessage],
    open_tasks: &[WorkEntity],
    active_decisions: &[WorkEntity],
    active_sops: &[SopRule],
    recent_docs: &[KnowledgeDocument],
    fts_snippets: &[(String, String)],
) -> String {
    let tasks_json = serde_json::to_string(open_tasks).unwrap_or_else(|_| "[]".into());
    let adrs_json = serde_json::to_string(active_decisions).unwrap_or_else(|_| "[]".into());
    let sops_summary: Vec<String> = active_sops
        .iter()
        .take(12)
        .map(|s| format!("- [{}]: {}", s.category, s.rule_statement))
        .collect();
    let docs_summary: Vec<String> = recent_docs
        .iter()
        .take(8)
        .map(|d| format!("- [{} | {}]: {}", d.doc_type, d.title, d.executive_summary))
        .collect();
    let fts_context: Vec<String> = fts_snippets
        .iter()
        .map(|(dt, txt)| format!("[{}]: {}", dt, txt.chars().take(1500).collect::<String>()))
        .collect();
    let copilot_history = format_chat_transcript(recent_copilot_chat, 16);
    let vault_history = format_chat_transcript(recent_vault_chat, 8);
    let current_hero = format_active_next_step_summary(open_tasks);

    format!(
        r#"You are Mab-Compass's interactive multi-turn Project Copilot and Path Architect for project "{project_id}".
You work with the user across multiple turns to build, verify, modify, reorder, and sharpen the project's Task Graph and Path Ahead (مسیر پیش‌رو).

INTERACTIVE MULTI-TURN & PATH EDITING RULES:
1. Read the conversation history and current active tasks carefully — the user may be answering your previous clarifying questions, reporting completed work, requesting corrections/reordering to the Path Ahead (مسیر پیش‌رو), pasting free text/specs/code, or deleting wrong items.
2. Apply concrete graph mutations immediately to match the user's intent:
   - Use `SET_NEXT_STEP` (`target_id` = `TSK-xxxx`) whenever the user wants to make an existing task the immediate #1 active step on the Path Ahead (`مسیر پیش‌رو`); optionally also supply updated `title`, `definition_of_done`, `first_physical_action`, `category`, or PERT estimates in the same mutation!
   - Use `UPDATE_ENTITY` (`target_id` = existing `TSK-xxxx` / `ADR-xxxx` / `BLK-xxxx`) whenever the user asks to revise/edit a task's title, scope, Definition of Done, 2-minute physical action, PERT estimates, priority (`is_on_critical_path`), or `status` (`"IN_PROGRESS"`, `"OPEN"`, `"BLOCKED"`, `"CLOSED"`). Do NOT create duplicate tasks when editing an existing one!
   - Use `DELETE_ENTITY` (`target_id` = `TSK-xxxx` / `ADR-xxxx` / `BLK-xxxx`) whenever the user says a task/decision/blocker is wrong, duplicate, cancelled, or should be removed.
   - Use `CREATE_ENTITY` for newly requested/discovered tasks (set `status: "IN_PROGRESS"` + `is_on_critical_path: true` if it must immediately replace the current Path Ahead hero step), decisions (`DECISION`), or blockers (`BLOCKER`).
   - Use `RESOLVE_ENTITY` when a task is completed, `SUPERSEDE_DECISION` when an ADR changes, `UPSERT_SOP` for rules, `REGISTER_ANTIPATTERN` for traps/mistakes, and `LOG_WORK` for completed sessions.
3. PROACTIVE CLARIFYING QUESTIONS (`clarifying_questions`):
   - Whenever the user gives new tasks/ideas/files/free text OR ambiguities remain about priority/ordering on the Path Ahead, Definition of Done, dependencies, constraints, or estimates, ask 1 to 4 sharp, concrete Persian questions in `clarifying_questions` (and mention them naturally in `conversational_reply`) so the user can reply in the next turn to refine the graph!
   - If the user simply gave a clear direct edit/command and no ambiguity remains, `clarifying_questions` can be empty (`[]`) and `needs_user_clarification: false`.

CURRENT #1 ACTIVE STEP ON PATH AHEAD:
{current_hero}

CURRENT OPEN TASKS (with IDs for SET_NEXT_STEP / UPDATE_ENTITY / DELETE_ENTITY / RESOLVE_ENTITY):
{tasks_json}

CURRENT ACTIVE DECISIONS (ADRs):
{adrs_json}

ACTIVE SOP RULES:
{}

ARCHIVED KNOWLEDGE DOCUMENTS SUMMARY:
{}

RECENT COPILOT CONVERSATION HISTORY:
{copilot_history}

RECENT KNOWLEDGE VAULT Q&A CONTEXT:
{vault_history}

RELEVANT HISTORICAL SNIPPETS FROM VAULT:
{}

LATEST USER MESSAGE:
"""
{user_message}
"""

Respond with ONLY a single valid JSON object (no markdown fences) matching this exact schema:
{{
  "conversational_reply": "پاسخ صمیمی، مهندسی، دقیق و شفاف به فارسی (توضیح بده چه تغییراتی در گراف تسک‌ها و مسیر پیش‌رو دادی و اگر ابهامی هست سوالاتت را بپرس)",
  "clarifying_questions": ["سوال ۱ به فارسی برای تدقیق گراف یا مسیر پیش‌رو", "سوال ۲..."],
  "needs_user_clarification": boolean,
  "meeting_detected": boolean,
  "meeting_title": "string or null",
  "meeting_summary_markdown": "string or null",
  "executive_doc_summary": null,
  "mutations": [
    {{
      "op": "RESOLVE_ENTITY" | "CREATE_ENTITY" | "UPDATE_ENTITY" | "SET_NEXT_STEP" | "DELETE_ENTITY" | "SUPERSEDE_DECISION" | "UPSERT_SOP" | "REGISTER_ANTIPATTERN" | "LOG_WORK",
      "target_id": "string or null (existing TSK-xxx or ADR-xxx ID for UPDATE_ENTITY / RESOLVE_ENTITY / SUPERSEDE_DECISION)",
      "entity_type": "TASK" | "DECISION" | "BLOCKER" | null,
      "category": "BACKEND" | "UI_UX" | "DEPLOY" | "VIDEO" | "ARCHITECTURE" | "GENERAL",
      "title": "string or null (Persian title)",
      "definition_of_done": "string or null (concrete exit criteria in Persian)",
      "first_physical_action": "string or null (ultra-specific 2-minute physical starter action in Persian)",
      "status": "OPEN" | "IN_PROGRESS" | "BLOCKED" | "CLOSED" | null,
      "is_on_critical_path": boolean,
      "optimistic_pomos": number,
      "most_likely_pomos": number,
      "pessimistic_pomos": number,
      "stakeholder_tag": "string or null",
      "sop_rule_statement": "string or null",
      "mistake_pattern": "string or null",
      "preventive_gate_question": "string or null",
      "severity_score": number,
      "effective_pomos": number,
      "deliverable_summary": "string or null",
      "internal_friction_note": "string or null",
      "client_sanitized_note": "string or null"
    }}
  ]
}}"#,
        sops_summary.join("\n"),
        docs_summary.join("\n"),
        fts_context.join("\n---\n")
    )
}

pub fn build_document_onboarding_prompt(
    project_id: &str,
    doc_title: &str,
    doc_type: &str,
    original_filename: &str,
    doc_content: &str,
    recent_vault_chat: &[ChatMessage],
    open_tasks: &[WorkEntity],
    active_decisions: &[WorkEntity],
) -> String {
    let tasks_json = serde_json::to_string(open_tasks).unwrap_or_else(|_| "[]".into());
    let adrs_json = serde_json::to_string(active_decisions).unwrap_or_else(|_| "[]".into());
    let vault_history = format_chat_transcript(recent_vault_chat, 10);

    // Cap overly long text safely on a UTF-8 char boundary for model context protection
    let bounded_content: String = doc_content.chars().take(48000).collect();

    format!(
        r#"You are ingesting content (document, source code, PDF/DOCX extract, meeting minutes, or free text) into the interactive Knowledge Vault of project "{project_id}".
Document Title: "{doc_title}" | Original File: "{original_filename}" | Type: {doc_type}

YOUR GOAL (INTERACTIVE KNOWLEDGE INGESTION):
1. Carefully read the submitted content and compare it with existing open tasks, decisions, and prior Knowledge Vault conversation:
   - Extract new remaining tasks with `CREATE_ENTITY` (break large items into concrete steps with 2-minute `first_physical_action`, concrete `definition_of_done`, and set `"status": "IN_PROGRESS"` + `is_on_critical_path: true` on the single best starting step if none is active yet).
   - Refine existing tasks/decisions with `UPDATE_ENTITY` (`target_id`) if this content adds detail or changes priority, rather than creating duplicates.
   - Resolve finished tasks with `RESOLVE_ENTITY`.
   - Extract decisions with `CREATE_ENTITY` (`entity_type="DECISION"`) or `SUPERSEDE_DECISION`.
   - Extract recurring guidelines/architecture conventions with `UPSERT_SOP` and risks/traps with `REGISTER_ANTIPATTERN`.
2. ASK CLARIFYING QUESTIONS (`clarifying_questions`):
   - Always evaluate what is STILL missing, ambiguous, underspecified, or open to prioritization in this content (e.g., which module/feature should go first on the Path Ahead, missing Definition of Done, target tech/environment, hidden dependencies, or timeline constraints).
   - Provide 1 to 4 concise, high-value clarifying questions in Persian inside `clarifying_questions` (with `needs_user_clarification: true`), and invite the user in `conversational_reply` to answer them right here so you can refine the task graph and Path Ahead!

EXISTING OPEN TASKS IN MEMORY:
{tasks_json}

EXISTING DECISIONS IN MEMORY:
{adrs_json}

RECENT KNOWLEDGE VAULT CONVERSATION:
{vault_history}

SUBMITTED CONTENT:
"""
{bounded_content}
"""

Output ONLY a single valid JSON object (no markdown fences) matching this exact schema:
{{
  "conversational_reply": "توضیح شفاف به فارسی از آنچه از این متن/فایل استخراج شد و دعوت از کاربر برای پاسخ به سوالات زیر جهت تدقیق مسیر پیش‌رو",
  "clarifying_questions": ["سوال ۱ به فارسی", "سوال ۲ به فارسی"],
  "needs_user_clarification": true,
  "meeting_detected": boolean,
  "meeting_title": "string or null",
  "meeting_summary_markdown": "string or null",
  "executive_doc_summary": "خلاصه مدیریتی ۳ تا ۵ خطی بسیار دقیق از محتوای کلیدی، تصمیمات و تعهدات این سند به فارسی",
  "mutations": [
    {{
      "op": "RESOLVE_ENTITY" | "CREATE_ENTITY" | "UPDATE_ENTITY" | "SUPERSEDE_DECISION" | "UPSERT_SOP" | "REGISTER_ANTIPATTERN",
      "target_id": "string or null",
      "entity_type": "TASK" | "DECISION" | "BLOCKER" | null,
      "category": "BACKEND" | "UI_UX" | "DEPLOY" | "VIDEO" | "ARCHITECTURE" | "GENERAL",
      "title": "string or null",
      "definition_of_done": "string or null",
      "first_physical_action": "string or null",
      "status": "OPEN" | "IN_PROGRESS" | "BLOCKED" | null,
      "is_on_critical_path": boolean,
      "optimistic_pomos": number,
      "most_likely_pomos": number,
      "pessimistic_pomos": number,
      "stakeholder_tag": "string or null",
      "sop_rule_statement": "string or null",
      "mistake_pattern": "string or null",
      "preventive_gate_question": "string or null",
      "severity_score": number,
      "effective_pomos": null,
      "deliverable_summary": null,
      "internal_friction_note": null,
      "client_sanitized_note": null
    }}
  ]
}}"#
    )
}

pub fn build_vault_chat_prompt(
    project_id: &str,
    user_message: &str,
    vault_chat_history: &[ChatMessage],
    open_tasks: &[WorkEntity],
    active_decisions: &[WorkEntity],
    active_sops: &[SopRule],
    recent_docs: &[KnowledgeDocument],
    fts_snippets: &[(String, String)],
) -> String {
    let tasks_json = serde_json::to_string(open_tasks).unwrap_or_else(|_| "[]".into());
    let adrs_json = serde_json::to_string(active_decisions).unwrap_or_else(|_| "[]".into());
    let sops_summary: Vec<String> = active_sops
        .iter()
        .take(12)
        .map(|s| format!("- [{}]: {}", s.category, s.rule_statement))
        .collect();
    let docs_summary: Vec<String> = recent_docs
        .iter()
        .take(10)
        .map(|d| format!("- [{} | {}]: {}", d.doc_type, d.title, d.executive_summary))
        .collect();
    let fts_context: Vec<String> = fts_snippets
        .iter()
        .map(|(dt, txt)| format!("[{}]: {}", dt, txt.chars().take(1500).collect::<String>()))
        .collect();
    let history = format_chat_transcript(vault_chat_history, 20);
    let current_hero = format_active_next_step_summary(open_tasks);

    format!(
        r#"You are the Knowledge Vault Interactive Architect for project "{project_id}".
You are in a multi-turn refinement conversation with the user after ingesting documents, source code, or free text.
The user is replying to your questions or providing extra context so you can build, restructure, modify, and sharpen the Task Graph and Path Ahead (مسیر پیش‌رو).

INSTRUCTIONS:
1. Carefully read `RECENT VAULT CONVERSATION` (including the questions you previously asked) and `LATEST USER ANSWER / MESSAGE`.
2. Update the task graph decisively:
   - Use `SET_NEXT_STEP` (`target_id`) if the user indicates which task must become the #1 immediate step on the Path Ahead (`مسیر پیش‌رو`).
   - Prefer `UPDATE_ENTITY` (`target_id`) to refine existing tasks' titles, `definition_of_done`, `first_physical_action`, `is_on_critical_path`, `status` (`"IN_PROGRESS"` for the immediate #1 next step), and PERT pomos based on the user's answers!
   - Use `DELETE_ENTITY` (`target_id`) to remove any wrong, duplicate, or cancelled task/decision.
   - Use `CREATE_ENTITY` to split vague tasks into concrete steps or add newly clarified tasks/decisions/blockers.
   - Use `RESOLVE_ENTITY` to close tasks the user confirmed are already done or no longer needed.
   - Use `UPSERT_SOP` / `REGISTER_ANTIPATTERN` for rules or traps mentioned by the user.
3. Follow-up `clarifying_questions`:
   - If key ambiguities or missing priorities still block a crisp Path Ahead, ask 1 to 3 targeted follow-up questions in `clarifying_questions` (`needs_user_clarification: true`).
   - Once the Path Ahead and tasks are clear and complete, return `clarifying_questions: []` and `needs_user_clarification: false`, confirming the ready Path Ahead!

CURRENT #1 ACTIVE STEP ON PATH AHEAD:
{current_hero}

CURRENT OPEN TASKS:
{tasks_json}

CURRENT ACTIVE DECISIONS:
{adrs_json}

ACTIVE SOP RULES:
{}

INGESTED VAULT DOCUMENTS:
{}

RELEVANT SNIPPETS FROM ARCHIVE:
{}

RECENT VAULT CONVERSATION:
{history}

LATEST USER ANSWER / MESSAGE:
"""
{user_message}
"""

Output ONLY a single valid JSON object (no markdown fences) matching this exact schema:
{{
  "conversational_reply": "پاسخ دقیق به فارسی (توضیح بده بر اساس پاسخ کاربر چه تسک‌هایی ساخته/تدقیق شدند و الان کدام گام در صدر مسیر پیش‌رو قرار دارد)",
  "clarifying_questions": ["سوال تکمیلی در صورت وجود ابهام"],
  "needs_user_clarification": boolean,
  "meeting_detected": boolean,
  "meeting_title": "string or null",
  "meeting_summary_markdown": "string or null",
  "executive_doc_summary": null,
  "mutations": [
    {{
      "op": "RESOLVE_ENTITY" | "CREATE_ENTITY" | "UPDATE_ENTITY" | "SET_NEXT_STEP" | "DELETE_ENTITY" | "SUPERSEDE_DECISION" | "UPSERT_SOP" | "REGISTER_ANTIPATTERN" | "LOG_WORK",
      "target_id": "string or null",
      "entity_type": "TASK" | "DECISION" | "BLOCKER" | null,
      "category": "BACKEND" | "UI_UX" | "DEPLOY" | "VIDEO" | "ARCHITECTURE" | "GENERAL",
      "title": "string or null",
      "definition_of_done": "string or null",
      "first_physical_action": "string or null",
      "status": "OPEN" | "IN_PROGRESS" | "BLOCKED" | "CLOSED" | null,
      "is_on_critical_path": boolean,
      "optimistic_pomos": number,
      "most_likely_pomos": number,
      "pessimistic_pomos": number,
      "stakeholder_tag": "string or null",
      "sop_rule_statement": "string or null",
      "mistake_pattern": "string or null",
      "preventive_gate_question": "string or null",
      "severity_score": number,
      "effective_pomos": number,
      "deliverable_summary": "string or null",
      "internal_friction_note": "string or null",
      "client_sanitized_note": "string or null"
    }}
  ]
}}"#,
        sops_summary.join("\n"),
        docs_summary.join("\n"),
        fts_context.join("\n---\n")
    )
}
