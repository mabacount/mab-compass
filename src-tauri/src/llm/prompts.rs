use crate::models::{ChatMessage, KnowledgeDocument, MethodPlaybook, SopRule, WorkEntity};
use chrono::Local;
use rusqlite::{params, Connection, Result};

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

pub fn format_existing_methods_summary(methods: &[MethodPlaybook]) -> String {
    if methods.is_empty() {
        return "*(No methods/playbooks saved yet)*".to_string();
    }
    methods
        .iter()
        .take(20)
        .map(|m| {
            format!(
                "- [{} | v{} | {}] {} ({} steps): {}",
                m.id,
                m.version,
                m.category,
                m.title,
                m.steps.len(),
                m.summary.chars().take(120).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[allow(clippy::too_many_arguments)]
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
    build_mcp_tool_turn_prompts_with_methods(
        project_id,
        channel,
        user_message_or_doc,
        recent_history,
        open_tasks,
        active_decisions,
        active_sops,
        recent_docs,
        &[],
        fts_snippets,
        doc_meta,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_mcp_tool_turn_prompts_with_methods(
    project_id: &str,
    channel: &str,
    user_message_or_doc: &str,
    recent_history: &[ChatMessage],
    open_tasks: &[WorkEntity],
    active_decisions: &[WorkEntity],
    active_sops: &[SopRule],
    recent_docs: &[KnowledgeDocument],
    existing_methods: &[MethodPlaybook],
    fts_snippets: &[(String, String)],
    doc_meta: Option<(&str, &str, &str)>, // (title, doc_type, filename)
    is_legacy_reprocess: bool,
) -> (String, String) {
    let current_hero = format_active_next_step_summary(open_tasks);
    let tasks_compact = compact_tasks_summary(open_tasks);
    let methods_summary = format_existing_methods_summary(existing_methods);
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

    let mode_desc = if is_legacy_reprocess {
        "LEGACY MEMORY UPGRADE & ANTI-DUPLICATE EXTRACTION".to_string()
    } else {
        match (channel, doc_meta) {
            (_, Some((title, doc_type, fname))) => format!(
                "Document / Content Onboarding (Channel: {}, Title: \"{}\", Type: {}, File: \"{}\")",
                channel, title, doc_type, fname
            ),
            ("VAULT", None) => "Knowledge Vault Multi-Turn Refinement".to_string(),
            _ => "Project Copilot & Path Ahead Architect".to_string(),
        }
    };

    let legacy_rules = if is_legacy_reprocess {
        r#"
CRITICAL ANTI-DUPLICATE LEGACY REPROCESSING RULES:
- You are re-analyzing a HISTORICAL document or past work report to extract capabilities that did not exist before.
- DO NOT create duplicate tasks that already exist in CURRENT OPEN TASKS or were already completed.
- FOCUS heavily on extracting MISSING:
  1. Repeatable Work Methods / Playbooks + BPMN 2.0 Workflows (`upsert_method_playbook`) — enrich existing methods if similar!
  2. Atomic Memory Cards (`MEMORY_CARD`), Lessons Learned (`LESSON`), Instructions (`INSTRUCTION`) via `save_personal_or_lesson_item`.
  3. Missing WorkLines (`create_work_line_chain`) or Outcomes (`upsert_domain_or_outcome`) ONLY if genuinely absent.
"#
    } else {
        ""
    };

    let system_prompt = format!(
        r#"You are Mab-Compass v4.0's interactive AI Copilot, Knowledge & Method Architect for project "{project_id}" ({mode_desc}).
{legacy_rules}
YOU HAVE NATIVE MCP TOOLS TO READ & UPDATE THE FULL ECOSYSTEM DIRECTLY:
1. Methods, Playbooks & BPMN 2.0 Workflows (دستورالعمل و متد انجام کار):
   - `upsert_method_playbook`: WHENEVER you detect a repeatable work pattern, procedure, workflow, or method in a work report, incremental messages, or document — OR whenever the user explicitly asks for a method/playbook/workflow — call `upsert_method_playbook`!
   - If a similar method ALREADY exists in `EXISTING METHODS & PLAYBOOKS`, pass its `method_id` (or matching title) so it automatically enriches & bumps version (`v1 -> v2`) instead of duplicating!
   - Include Persian `title`, `summary`, `objective` (هدف), `prerequisites` (پیش‌نیازها), ordered `steps` (with `element_type`: `task`/`userTask`/`serviceTask`/`subProcess`/`exclusiveGateway`/`parallelGateway`, `role`, `inputs`, `outputs`, `checklist`, `notes`), overall `checklist`, and `notes_markdown`.
   - For composite stages/groups of tasks, set `element_type: "subProcess"` and populate `sub_steps` so the user can visually click and drill down into the internal BPMN 2.0 sub-workflow!
   - Files are automatically written to `04_methods_playbooks/` (.md + .bpmn) and displayed as clickable Artifact Cards beneath your chat reply.
2. Strategic Domains & Outcomes:
   - `upsert_domain_or_outcome`: Create or update Outcomes (`OUT-xxxx`) linked to CADM life domains (`MIND_LEARNING`, `HEALTH_ENERGY`, `WEALTH_FINANCE`, `RELATIONS`, `RESILIENCE`, `PURPOSE`, `SPIRITUALITY`, `SANCTUARY`).
3. WorkLines (Sequential Chains & Goal Clusters):
   - `create_work_line_chain` & `append_or_reorder_work_line`: Group multi-step work into `SEQUENTIAL_CHAIN` or `GOAL_CLUSTER` WorkLines (`WLN-xxxx`).
4. Tasks, Decisions & Blockers:
   - `create_work_entity`, `update_work_entity`, `set_next_step`, `complete_task`, `delete_work_entity`, `schedule_task_on_calendar`.
5. Atomic Knowledge Cards, Lessons Learned, Instructions & Guardrails:
   - `save_personal_or_lesson_item`: Use `item_type`: `"MEMORY_CARD"`, `"LESSON"`, `"INSTRUCTION"`, `"NOTE"`.
   - `log_work_and_knowledge`: Record `LOG_WORK`, `UPSERT_SOP`, `REGISTER_ANTIPATTERN`, or `INGEST_NOTE`.
6. Project Lifecycle & Memory Search:
   - `propose_project_lifecycle`, `get_path_ahead`, `search_memory`, `sync_mab_focus`, `undo_last_change`.

DOCUMENT & INPUT PROCESSING RULES:
1. Analyze the user's message, work report, or document carefully against existing tasks, Methods/Playbooks, WorkLines, Outcomes, SOPs, and conversation history.
2. Extract EVERY valuable atomic piece using MCP tools BEFORE replying (especially repeatable Methods/Playbooks + BPMN workflows, WorkLines, Tasks, Memory Cards, Lessons, Guardrails).
3. After tool execution, write a warm, sharp, engineering-focused **Persian Markdown** reply explicitly noting every document/method/playbook/workflow you wrote or updated (their clickable cards will appear right below your message).
4. If key ambiguities remain, end your Persian reply with 1 to 3 concise numbered questions."#
    );

    let bounded_input: String = user_message_or_doc.chars().take(48000).collect();

    let user_prompt = format!(
        r#"CURRENT #1 ACTIVE STEP ON PATH AHEAD:
{current_hero}

EXISTING METHODS & PLAYBOOKS (UPDATE/ENRICH BY method_id IF SIMILAR):
{methods_summary}

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
  ],
  "method_playbooks": [
    {{
      "method_id": "string or null (existing MTH-xxxx ID if enriching an existing method)",
      "title": "عنوان دستورالعمل و متد انجام کار به فارسی (اگر الگوی کاری تکرارپذیر در گزارش کار یا درخواست کاربر وجود دارد)",
      "category": "BACKEND" | "UI_UX" | "DEPLOY" | "VIDEO" | "ARCHITECTURE" | "GENERAL",
      "summary": "خلاصه ۱ تا ۲ خطی برای کارت قابل کلیک در چت",
      "objective": "هدف دقیق این روش و دستورالعمل",
      "prerequisites": ["پیش‌نیاز ۱", "پیش‌نیاز ۲"],
      "steps": [
        {{
          "id": "S1",
          "order": 1,
          "title": "عنوان گام",
          "description": "توضیح کامل گام",
          "element_type": "task" | "userTask" | "serviceTask" | "subProcess" | "exclusiveGateway" | "parallelGateway",
          "role": "مسئول یا نقش",
          "checklist": ["مورد کنترل ۱"],
          "notes": "نکته کلیدی",
          "sub_steps": [{{ "title": "زیرگام داخلی در صورت subProcess بودن", "description": "شرح", "element_type": "task" }}]
        }}
      ],
      "checklist": ["چک‌لیست نهایی"],
      "notes_markdown": "نکات و هشدارها",
      "bpmn_xml": "optional valid BPMN 2.0 XML or null"
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
  ],
  "method_playbooks": [
    {{
      "method_id": "string or null (existing MTH-xxxx ID if enriching an existing method)",
      "title": "عنوان دستورالعمل و متد انجام کار به فارسی (اگر الگوی کاری تکرارپذیر در گزارش کار یا درخواست کاربر وجود دارد)",
      "category": "BACKEND" | "UI_UX" | "DEPLOY" | "VIDEO" | "ARCHITECTURE" | "GENERAL",
      "summary": "خلاصه ۱ تا ۲ خطی برای کارت قابل کلیک در چت",
      "objective": "هدف دقیق این روش و دستورالعمل",
      "prerequisites": ["پیش‌نیاز ۱", "پیش‌نیاز ۲"],
      "steps": [
        {{
          "id": "S1",
          "order": 1,
          "title": "عنوان گام",
          "description": "توضیح کامل گام",
          "element_type": "task" | "userTask" | "serviceTask" | "subProcess" | "exclusiveGateway" | "parallelGateway",
          "role": "مسئول یا نقش",
          "checklist": ["مورد کنترل ۱"],
          "notes": "نکته کلیدی",
          "sub_steps": [{{ "title": "زیرگام داخلی در صورت subProcess بودن", "description": "شرح", "element_type": "task" }}]
        }}
      ],
      "checklist": ["چک‌لیست نهایی"],
      "notes_markdown": "نکات و هشدارها",
      "bpmn_xml": "optional valid BPMN 2.0 XML or null"
    }}
  ]
}}"#,
        sops_summary.join("\n"),
        docs_summary.join("\n"),
        fts_context.join("\n---\n")
    )
}

/// Specialized Anti-Duplicate Legacy Reprocessing Prompt for structured JSON fallback models.
#[allow(clippy::too_many_arguments)]
pub fn build_legacy_reprocess_prompt(
    project_id: &str,
    source_label: &str,
    historical_content: &str,
    open_tasks: &[WorkEntity],
    active_decisions: &[WorkEntity],
    active_sops: &[SopRule],
    existing_methods: &[MethodPlaybook],
) -> String {
    let tasks_compact = compact_tasks_summary(open_tasks);
    let methods_summary = format_existing_methods_summary(existing_methods);
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
    let bounded_content: String = historical_content.chars().take(48000).collect();

    format!(
        r#"You are performing an IDEMPOTENT / ANTI-DUPLICATE LEGACY MEMORY UPGRADE on a historical document or work report for project "{project_id}" ({source_label}).

STRICT RULES:
1. DO NOT duplicate any task that already exists in CURRENT OPEN TASKS (or is already completed). Leave `mutations` empty for tasks unless a truly missing critical item is found.
2. YOUR PRIMARY MISSION IS TO EXTRACT MISSING:
   - Repeatable Methods / Playbooks + BPMN 2.0 Workflows inside `method_playbooks[]` (enrich an existing method by passing its `method_id` if a similar one exists in EXISTING METHODS & PLAYBOOKS!). Use `element_type: "subProcess"` + `sub_steps` for composite work groups.
   - Reusable SOP rules (`UPSERT_SOP`) and Mistake Guardrails (`REGISTER_ANTIPATTERN`) if missing.
   - Missing Architecture Decisions (`CREATE_ENTITY` with `entity_type: "DECISION"`) if missing.
3. Summarize concisely in Persian what new methods, playbooks, BPMN workflows, or guardrails were extracted from this legacy memory item (or state cleanly if everything was already covered).

EXISTING METHODS & PLAYBOOKS:
{methods_summary}

CURRENT OPEN TASKS (DO NOT DUPLICATE):
{tasks_compact}

ACTIVE DECISIONS:
{adrs}

ACTIVE SOPS:
{sops}

HISTORICAL CONTENT TO UPGRADE:
"""
{bounded_content}
"""

Output ONLY a single valid JSON object (no markdown fences) matching the CopilotEnvelope schema (`conversational_reply`, `clarifying_questions`, `needs_user_clarification`, `meeting_detected`, `meeting_title`, `meeting_summary_markdown`, `executive_doc_summary`, `mutations`, `method_playbooks`)."#,
        adrs = adrs_compact.join("\n"),
        sops = sops_summary.join("\n"),
    )
}

/// Builds a structured, continuous 6-layer Live Ecosystem State Snapshot
/// formatted in Markdown for injection into LLM system prompts and chat turns.
pub fn build_live_ecosystem_snapshot(conn: &Connection, project_id: &str) -> Result<String> {
    let mut out = String::with_capacity(4096);
    out.push_str("# LIVE ECOSYSTEM STATE SNAPSHOT\n\n");

    // --------------------------------------------------------------------------
    // 1. Life Domains (CADM v2.0 - 8 Dimensions)
    // --------------------------------------------------------------------------
    out.push_str("## 1. Life Domains (8 CADM Dimensions)\n");
    let mut dom_stmt = conn.prepare(
        "SELECT code, title, maturity_state, maintenance_floor, is_growth_focus
         FROM life_domains ORDER BY code ASC",
    )?;
    let domains: Vec<(String, String, i32, String, bool)> = dom_stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, i32>(2).unwrap_or(2),
                r.get::<_, String>(3).unwrap_or_default(),
                r.get::<_, i32>(4).unwrap_or(0) == 1,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if domains.is_empty() {
        out.push_str("*(No life domains configured)*\n\n");
    } else {
        out.push_str("| Domain Code | Title | Maturity (0..5) | Focus | Maintenance Floor |\n");
        out.push_str("|---|---|:---:|:---:|---|\n");
        for (code, title, maturity, floor, is_focus) in &domains {
            let focus_str = if *is_focus { "🎯 YES" } else { "NO" };
            let floor_str = if floor.trim().is_empty() {
                "—"
            } else {
                floor.trim()
            };
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                code, title, maturity, focus_str, floor_str
            ));
        }
        out.push('\n');
    }

    // --------------------------------------------------------------------------
    // 2. Active Outcomes
    // --------------------------------------------------------------------------
    out.push_str("## 2. Active Outcomes\n");
    let mut out_stmt = conn.prepare(
        "SELECT id, domain_code, title, desired_reality_change, evidence_kr, status, progress_percent
         FROM outcomes
         WHERE status != 'DROPPED'
         ORDER BY domain_code, id ASC",
    )?;
    let outcomes: Vec<(String, String, String, String, String, String, i32)> = out_stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get::<_, String>(3).unwrap_or_default(),
                r.get::<_, String>(4).unwrap_or_default(),
                r.get::<_, String>(5).unwrap_or_else(|_| "ACTIVE".into()),
                r.get::<_, i32>(6).unwrap_or(0),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if outcomes.is_empty() {
        out.push_str("*(No active outcomes defined)*\n\n");
    } else {
        for (id, dom, title, reality, kr, status, prog) in &outcomes {
            out.push_str(&format!(
                "- [{}] ({}) \"{}\" (Status: {}, Progress: {}%)\n",
                id, dom, title, status, prog
            ));
            if !reality.is_empty() {
                out.push_str(&format!("  - Desired Reality: {}\n", reality));
            }
            if !kr.is_empty() {
                out.push_str(&format!("  - Key Result (Evidence): {}\n", kr));
            }
        }
        out.push('\n');
    }

    // --------------------------------------------------------------------------
    // 3. Active Projects & Work Lines (Sequential Chains)
    // --------------------------------------------------------------------------
    out.push_str("## 3. Active Projects & Work Lines\n");
    let mut p_stmt = conn.prepare(
        "SELECT id, name, domain_code, lifecycle_status, priority_weight, is_active
         FROM projects
         WHERE lifecycle_status != 'ARCHIVED'
         ORDER BY is_active DESC, priority_weight DESC, id ASC",
    )?;
    let projects: Vec<(String, String, String, String, i32, bool)> = p_stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, String>(2).unwrap_or_else(|_| "WEALTH_FINANCE".into()),
                r.get::<_, String>(3).unwrap_or_else(|_| "ACTIVE".into()),
                r.get::<_, i32>(4).unwrap_or(5),
                r.get::<_, i32>(5).unwrap_or(0) == 1,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let target_pid = if projects.iter().any(|p| p.0 == project_id) {
        project_id
    } else {
        projects
            .iter()
            .find(|p| p.5)
            .map(|p| p.0.as_str())
            .unwrap_or(project_id)
    };

    if let Some(target_p) = projects.iter().find(|p| p.0 == target_pid) {
        out.push_str(&format!(
            "### Project: {} [{}] (Status: {}, Domain: {}, Priority: {})\n",
            target_p.1, target_p.0, target_p.3, target_p.2, target_p.4
        ));
    } else {
        out.push_str(&format!("### Project: [{}]\n", target_pid));
    }

    let mut wl_stmt = conn.prepare(
        "SELECT id, title, goal_statement, line_mode, status
         FROM work_lines
         WHERE project_id = ?1
         ORDER BY status ASC, id ASC",
    )?;
    let work_lines: Vec<(String, String, String, String, String)> = wl_stmt
        .query_map(params![target_pid], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, String>(2).unwrap_or_default(),
                r.get::<_, String>(3).unwrap_or_else(|_| "SEQUENTIAL_CHAIN".into()),
                r.get::<_, String>(4).unwrap_or_else(|_| "ACTIVE".into()),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if work_lines.is_empty() {
        out.push_str("*(No work lines defined for this project)*\n\n");
    } else {
        for (w_id, title, goal, mode, status) in &work_lines {
            // Fetch steps for this line
            let mut steps_stmt = conn.prepare(
                "SELECT id, sequence_order, title, status, is_locked_by_chain, duration_minutes, device_tag
                 FROM work_entities
                 WHERE work_line_id = ?1
                 ORDER BY sequence_order ASC",
            )?;
            let steps: Vec<(String, i32, String, String, bool, i32, String)> = steps_stmt
                .query_map(params![w_id], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get::<_, String>(3).unwrap_or_else(|_| "OPEN".into()),
                        r.get::<_, i32>(4).unwrap_or(0) == 1,
                        r.get::<_, i32>(5).unwrap_or(25),
                        r.get::<_, String>(6).unwrap_or_else(|_| "LAPTOP_ONLY".into()),
                    ))
                })?
                .filter_map(|r| r.ok())
                .collect();

            let total_steps = steps.len();
            let completed_steps = steps.iter().filter(|s| s.3 == "CLOSED").count();
            let active_step = steps.iter().find(|s| s.3 == "IN_PROGRESS");
            let queued_count = steps
                .iter()
                .filter(|s| s.3 == "QUEUED_IN_CHAIN" || s.4)
                .count();

            out.push_str(&format!(
                "- WorkLine [{}] \"{}\" [{}] (Status: {}, Progress: {}/{} completed)\n",
                w_id, title, mode, status, completed_steps, total_steps
            ));
            if !goal.is_empty() {
                out.push_str(&format!("  - Goal: {}\n", goal));
            }
            if let Some(active) = active_step {
                out.push_str(&format!(
                    "  - Active Step (#{}: [{}] \"{}\" ({}, {}m, {}))\n",
                    active.1, active.0, active.2, active.3, active.5, active.6
                ));
            }
            if queued_count > 0 {
                out.push_str(&format!(
                    "  - Queued: {} steps waiting in chain\n",
                    queued_count
                ));
            }
        }
        out.push('\n');
    }

    // --------------------------------------------------------------------------
    // 4. Today's Calendar Capacity & Scheduled Tasks
    // --------------------------------------------------------------------------
    let today_key = Local::now().format("%Y-%m-%d").to_string();
    out.push_str(&format!(
        "## 4. Today's Calendar Capacity & Scheduled Tasks ({})\n",
        today_key
    ));

    let cal_row = conn
        .query_row(
            "SELECT total_capacity_minutes, deep_capacity_minutes, is_off_day, day_note
         FROM calendar_days WHERE date_key = ?1",
            params![today_key],
            |r| {
                Ok((
                    r.get::<_, i32>(0).unwrap_or(300),
                    r.get::<_, i32>(1).unwrap_or(150),
                    r.get::<_, i32>(2).unwrap_or(0) == 1,
                    r.get::<_, String>(3).unwrap_or_default(),
                ))
            },
        )
        .unwrap_or((300, 150, false, String::new()));

    let (total_cap, deep_cap, is_off, day_note) = cal_row;

    let mut sched_stmt = conn.prepare(
        "SELECT id, title, scheduled_slot, duration_minutes, status
         FROM work_entities
         WHERE scheduled_date = ?1
         ORDER BY
             CASE scheduled_slot
                 WHEN 'MORNING_DEEP' THEN 1
                 WHEN 'MIDDAY_EXEC' THEN 2
                 WHEN 'AFTERNOON_LIGHT' THEN 3
                 WHEN 'COMMUTE_SLOT' THEN 4
                 ELSE 5 END ASC,
             sequence_order ASC",
    )?;
    let sched_tasks: Vec<(String, String, String, i32, String)> = sched_stmt
        .query_map(params![today_key], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, String>(2).unwrap_or_else(|_| "MIDDAY_EXEC".into()),
                r.get::<_, i32>(3).unwrap_or(25),
                r.get::<_, String>(4).unwrap_or_else(|_| "OPEN".into()),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let booked_minutes: i32 = sched_tasks.iter().map(|t| t.3).sum();
    let available_minutes = total_cap.saturating_sub(booked_minutes);
    let off_str = if is_off { "YES (Off Day)" } else { "NO" };

    out.push_str(&format!(
        "- Capacity: {}m total / {}m deep | Booked: {}m | Available: {}m | Off Day: {}\n",
        total_cap, deep_cap, booked_minutes, available_minutes, off_str
    ));
    if !day_note.is_empty() {
        out.push_str(&format!("- Day Note: {}\n", day_note));
    }

    if sched_tasks.is_empty() {
        out.push_str("- Scheduled Tasks: *(No tasks scheduled for today)*\n\n");
    } else {
        out.push_str("- Scheduled Tasks:\n");
        for (tid, title, slot, dur, status) in &sched_tasks {
            out.push_str(&format!(
                "  - [{}] \"{}\" (Slot: {}, {}m, Status: {})\n",
                tid, title, slot, dur, status
            ));
        }
        out.push('\n');
    }

    // --------------------------------------------------------------------------
    // 5. Pending Proposals (Human-in-the-Loop)
    // --------------------------------------------------------------------------
    out.push_str("## 5. Pending Proposals\n");
    let mut prop_stmt = conn.prepare(
        "SELECT id, project_id, proposal_type, title, rationale_markdown
         FROM pending_proposals
         WHERE status = 'PENDING'
         ORDER BY created_at DESC
         LIMIT 5",
    )?;
    let proposals: Vec<(String, Option<String>, String, String, String)> = prop_stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1).ok(),
                r.get(2)?,
                r.get(3)?,
                r.get::<_, String>(4).unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if proposals.is_empty() {
        out.push_str("*(No pending proposals awaiting decision)*\n\n");
    } else {
        for (prp_id, pid_opt, ptype, title, rationale) in &proposals {
            let p_tag = pid_opt.as_deref().unwrap_or("GLOBAL");
            out.push_str(&format!(
                "- [{}] [{}] \"{}\" (Project: {})\n  - Rationale: {}\n",
                prp_id, ptype, title, p_tag, rationale
            ));
        }
        out.push('\n');
    }

    // --------------------------------------------------------------------------
    // 6. Recent Personal Vault Items (Lessons & Notes)
    // --------------------------------------------------------------------------
    out.push_str("## 6. Recent Personal Vault Items\n");
    let mut v_stmt = conn.prepare(
        "SELECT id, realm, domain_code, item_type, title, content_markdown
         FROM personal_vault_items
         ORDER BY updated_at DESC, id DESC
         LIMIT 6",
    )?;
    let vault_items: Vec<(String, String, String, String, String, String)> = v_stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get::<_, String>(1).unwrap_or_else(|_| "LIFE".into()),
                r.get::<_, String>(2).unwrap_or_else(|_| "MIND_LEARNING".into()),
                r.get(3)?,
                r.get(4)?,
                r.get::<_, String>(5).unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    if vault_items.is_empty() {
        out.push_str("*(Personal vault is empty)*\n\n");
    } else {
        for (v_id, realm, dom, itype, title, content) in &vault_items {
            let snippet: String = content.chars().take(120).collect();
            out.push_str(&format!(
                "- [{}] [{}] \"{}\" (Domain: {}, Realm: {}): {}\n",
                v_id,
                itype,
                title,
                dom,
                realm,
                snippet.trim()
            ));
        }
        out.push('\n');
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_live_ecosystem_snapshot_coverage() {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::db::schema::init_database(&dir.path().join("test.sqlite"), dir.path()).unwrap();
        // 1. Snapshot on freshly seeded DB (empty tables except life_domains)
        let snap = build_live_ecosystem_snapshot(&conn, "GERDALO").unwrap();
        assert!(snap.contains("1. Life Domains (8 CADM Dimensions)"));
        assert!(snap.contains("MIND_LEARNING"));
        assert!(snap.contains("HEALTH_ENERGY"));
        assert!(snap.contains("2. Active Outcomes"));
        assert!(snap.contains("3. Active Projects & Work Lines"));
        assert!(snap.contains("4. Today's Calendar Capacity"));
        assert!(snap.contains("5. Pending Proposals"));
        assert!(snap.contains("6. Recent Personal Vault Items"));

        // 2. Snapshot on populated DB
        let now = chrono::Local::now().to_rfc3339();
        conn.execute(
            "INSERT INTO outcomes (id, domain_code, title, desired_reality_change, evidence_kr, time_window, status, progress_percent, created_at, updated_at)
             VALUES ('OUT-UNIT-01', 'MIND_LEARNING', 'Learn Systems Rust', 'Deep mastery', '100% test pass', '1405-Q3', 'ACTIVE', 50, ?1, ?1)",
            params![now],
        ).unwrap();

        conn.execute(
            "INSERT INTO pending_proposals (id, project_id, proposal_type, title, rationale_markdown, payload_json, status, created_at)
             VALUES ('PRP-UNIT-01', 'GERDALO', 'PAUSE_PROJECT', 'Pause Unit', 'Focus rationale', '{}', 'PENDING', ?1)",
            params![now],
        ).unwrap();

        conn.execute(
            "INSERT INTO personal_vault_items (id, realm, domain_code, item_type, title, content_markdown, created_at, updated_at)
             VALUES ('PVI-UNIT-01', 'LIFE', 'MIND_LEARNING', 'LESSON_LEARNED', 'Test Lesson', 'Content of lesson', ?1, ?1)",
            params![now],
        ).unwrap();

        let populated_snap = build_live_ecosystem_snapshot(&conn, "GERDALO").unwrap();
        assert!(populated_snap.contains("OUT-UNIT-01"));
        assert!(populated_snap.contains("Learn Systems Rust"));
        assert!(populated_snap.contains("PRP-UNIT-01"));
        assert!(populated_snap.contains("Pause Unit"));
        assert!(populated_snap.contains("PVI-UNIT-01"));
        assert!(populated_snap.contains("Test Lesson"));
    }
}
