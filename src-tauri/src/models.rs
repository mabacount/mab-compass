use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_string_lenient<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match opt {
        Some(serde_json::Value::String(s)) => s,
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    })
}

fn deserialize_vec_string_lenient<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<Vec<serde_json::Value>>::deserialize(deserializer)?;
    Ok(match opt {
        Some(items) => items
            .into_iter()
            .filter_map(|v| match v {
                serde_json::Value::String(s) => Some(s),
                serde_json::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        None => Vec::new(),
    })
}

fn deserialize_vec_mutations_lenient<'de, D>(
    deserializer: D,
) -> Result<Vec<AtomicMutation>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<Vec<AtomicMutation>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn deserialize_vec_methods_lenient<'de, D>(
    deserializer: D,
) -> Result<Vec<MethodPlaybookInput>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<Vec<MethodPlaybookInput>>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn deserialize_bool_lenient<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match opt {
        Some(serde_json::Value::Bool(b)) => b,
        Some(serde_json::Value::Number(n)) => n.as_f64().map(|v| v != 0.0).unwrap_or(false),
        Some(serde_json::Value::String(s)) => {
            matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes")
        }
        _ => false,
    })
}

fn deserialize_opt_bool_lenient<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match opt {
        Some(serde_json::Value::Bool(b)) => Some(b),
        Some(serde_json::Value::Number(n)) => n.as_f64().map(|v| v != 0.0),
        Some(serde_json::Value::String(s)) => match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    })
}

fn deserialize_opt_f64_lenient<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match opt {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        Some(serde_json::Value::String(s)) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

fn deserialize_opt_i32_lenient<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match opt {
        Some(serde_json::Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                Some(i.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
            } else {
                n.as_f64()
                    .filter(|f| f.is_finite())
                    .map(|f| f.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32)
            }
        }
        Some(serde_json::Value::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else if let Ok(i) = trimmed.parse::<i32>() {
                Some(i)
            } else {
                trimmed
                    .parse::<f64>()
                    .ok()
                    .filter(|f| f.is_finite())
                    .map(|f| f.round().clamp(i32::MIN as f64, i32::MAX as f64) as i32)
            }
        }
        _ => None,
    })
}

fn deserialize_opt_string_lenient<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(match opt {
        Some(serde_json::Value::String(s)) => Some(s),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    })
}

fn default_domain_code() -> String {
    "WEALTH_FINANCE".to_string()
}

fn default_lifecycle_status() -> String {
    "ACTIVE".to_string()
}

fn default_scope_mode() -> String {
    "STANDARD".to_string()
}

fn default_realm_work() -> String {
    "WORK".to_string()
}

fn default_realm_life() -> String {
    "LIFE".to_string()
}

fn default_priority_weight() -> i32 {
    5
}

fn default_maturity_state() -> i32 {
    2
}

fn default_outcome_status() -> String {
    "ACTIVE".to_string()
}

fn default_line_mode() -> String {
    "SEQUENTIAL_CHAIN".to_string()
}

fn default_work_line_status() -> String {
    "ACTIVE".to_string()
}

fn default_color_hex() -> String {
    "#3B82F6".to_string()
}

fn default_total_capacity_minutes() -> i32 {
    300
}

fn default_deep_capacity_minutes() -> i32 {
    150
}

fn default_proposal_status() -> String {
    "PENDING".to_string()
}

fn default_energy_tag() -> String {
    "MEDIUM_NORMAL".to_string()
}

fn default_duration_minutes() -> i32 {
    25
}

fn default_device_tag() -> String {
    "LAPTOP_ONLY".to_string()
}

fn default_location_tags() -> Vec<String> {
    vec!["DESK".to_string()]
}

fn default_sequence_order() -> i32 {
    1
}

fn default_priority_score() -> i32 {
    50
}

fn default_step_category() -> String {
    "GENERAL".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub workspace_path: String,
    pub external_focus_id: Option<String>,
    pub is_active: bool,
    pub updated_at: String,

    // v3.0 5-Layer Ontology & Lifecycle Fields
    #[serde(default = "default_domain_code")]
    pub domain_code: String,
    #[serde(default)]
    pub primary_outcome_id: Option<String>,
    #[serde(default = "default_lifecycle_status")]
    pub lifecycle_status: String,
    #[serde(default = "default_scope_mode")]
    pub scope_mode: String,
    #[serde(default = "default_realm_work")]
    pub realm: String,
    #[serde(default = "default_priority_weight")]
    pub priority_weight: i32,
    #[serde(default)]
    pub pause_resume_note: String,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            workspace_path: String::new(),
            external_focus_id: None,
            is_active: false,
            updated_at: String::new(),
            domain_code: "WEALTH_FINANCE".to_string(),
            primary_outcome_id: None,
            lifecycle_status: "ACTIVE".to_string(),
            scope_mode: "STANDARD".to_string(),
            realm: "WORK".to_string(),
            priority_weight: 5,
            pause_resume_note: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkEntity {
    pub id: String,
    pub project_id: String,
    pub entity_type: String, // "TASK" | "DECISION" | "BLOCKER"
    pub category: String,    // "BACKEND" | "UI_UX" | "DEPLOY" | "VIDEO" | "ARCHITECTURE" | "GENERAL"
    pub title: String,
    pub definition_of_done: String,
    pub first_physical_action: String,
    pub status: String, // "OPEN" | "IN_PROGRESS" | "BLOCKED" | "CLOSED" | "SUPERSEDED" | "QUEUED_IN_CHAIN"
    pub is_on_critical_path: bool,
    pub optimistic_pomos: f64,
    pub most_likely_pomos: f64,
    pub pessimistic_pomos: f64,
    pub calibrated_pomos: f64,
    pub superseded_by_id: Option<String>,
    pub stakeholder_tag: Option<String>,
    pub updated_at: String,

    // v3.0 Sequential Chains, Context Tags & Calendar Fields
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub work_line_id: Option<String>,
    #[serde(default)]
    pub work_line_title: Option<String>,
    #[serde(default)]
    pub outcome_id: Option<String>,
    #[serde(default = "default_sequence_order")]
    pub sequence_order: i32,
    #[serde(default)]
    pub depends_on_task_id: Option<String>,
    #[serde(default)]
    pub is_locked_by_chain: bool,
    #[serde(default = "default_energy_tag")]
    pub energy_tag: String,
    #[serde(default = "default_duration_minutes")]
    pub duration_minutes: i32,
    #[serde(default = "default_device_tag")]
    pub device_tag: String,
    #[serde(default = "default_location_tags")]
    pub location_tags: Vec<String>,
    #[serde(default)]
    pub scheduled_date: Option<String>,
    #[serde(default)]
    pub scheduled_slot: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default = "default_priority_score")]
    pub priority_score: i32,
}

impl Default for WorkEntity {
    fn default() -> Self {
        Self {
            id: String::new(),
            project_id: String::new(),
            project_name: String::new(),
            entity_type: "TASK".to_string(),
            category: "GENERAL".to_string(),
            title: String::new(),
            definition_of_done: String::new(),
            first_physical_action: String::new(),
            status: "OPEN".to_string(),
            is_on_critical_path: false,
            optimistic_pomos: 1.0,
            most_likely_pomos: 2.0,
            pessimistic_pomos: 3.0,
            calibrated_pomos: 2.0,
            superseded_by_id: None,
            stakeholder_tag: None,
            updated_at: String::new(),
            work_line_id: None,
            work_line_title: None,
            outcome_id: None,
            sequence_order: 1,
            depends_on_task_id: None,
            is_locked_by_chain: false,
            energy_tag: "MEDIUM_NORMAL".to_string(),
            duration_minutes: 25,
            device_tag: "LAPTOP_ONLY".to_string(),
            location_tags: vec!["DESK".to_string()],
            scheduled_date: None,
            scheduled_slot: None,
            due_date: None,
            priority_score: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopRule {
    pub id: i64,
    pub project_id: String,
    pub category: String,
    pub rule_statement: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedAntipattern {
    pub id: i64,
    pub project_id: String,
    pub trigger_category: String,
    pub mistake_pattern: String,
    pub preventive_gate_question: String,
    pub severity_score: i32,
    pub times_triggered: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWorkLog {
    pub id: i64,
    pub project_id: String,
    pub task_id: Option<String>,
    pub category: String,
    pub estimated_pomos: f64,
    pub effective_pomos: i32,
    pub deliverable_summary: String,
    pub internal_friction_note: String,
    pub client_sanitized_note: String,
    pub source_origin: String, // "USER_CHAT" | "MAB_FOCUS_SYNC" | "DOC_ONBOARDING" | "QUICK_COMPLETE"
    pub logged_at: String,
}

fn default_channel() -> String {
    "COPILOT".to_string()
}

fn default_gemini_model() -> String {
    "gemini-2.5-flash".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WrittenArtifactRef {
    pub artifact_type: String, // "METHOD_PLAYBOOK" | "KNOWLEDGE_DOC" | "MEETING_MINUTES" | "DECISION_ADR" | "SOP_RULE" | "VAULT_CARD" | "WORK_LINE" | "MARKDOWN_FILE"
    pub entity_id: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub action: String, // "CREATED" | "UPDATED"
    #[serde(default)]
    pub version: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub project_id: String,
    #[serde(default)]
    pub session_id: String,
    pub sender: String, // "USER" | "ASSISTANT" | "MAB_FOCUS_SYNC" | "SYSTEM"
    #[serde(default = "default_channel")]
    pub channel: String, // "COPILOT" | "VAULT"
    pub message_text: String,
    pub archived_file_path: Option<String>,
    pub applied_mutations_badge: Option<String>,
    #[serde(default)]
    pub clarifying_questions: Vec<String>,
    #[serde(default)]
    pub written_artifacts: Vec<WrittenArtifactRef>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub project_id: String,
    pub channel: String, // "COPILOT" | "VAULT"
    pub title: String,
    pub is_active: bool,
    pub message_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEventPreview {
    pub id: i64,
    pub source_type: String,
    pub snippet: String,
    pub archived_path: Option<String>,
    pub created_at: String,
}

fn default_processing_status() -> String {
    "UNPROCESSED".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDocument {
    pub id: i64,
    pub project_id: String,
    pub title: String,
    pub doc_type: String, // "MEETING_MINUTES" | "PROJECT_CHARTER" | "TECHNICAL_DOC" | "ONBOARDING_FILE" | "FREE_TEXT" | "SOURCE_CODE"
    pub original_filename: String,
    pub archived_path: String,
    pub executive_summary: String,
    pub extracted_items_count: i32,
    pub ingested_at: String,

    // v3.0 Twin Archive, Classification & Processing Lifecycle Fields
    #[serde(default)]
    pub binary_original_path: Option<String>,
    #[serde(default)]
    pub tags_json: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_realm_work")]
    pub realm: String,
    #[serde(default = "default_domain_code")]
    pub domain_code: String,
    #[serde(default = "default_processing_status")]
    pub processing_status: String, // "UNPROCESSED" | "PROCESSING" | "PROCESSED" | "FAILED"
    #[serde(default)]
    pub processed_at: Option<String>,
}

impl Default for KnowledgeDocument {
    fn default() -> Self {
        Self {
            id: 0,
            project_id: String::new(),
            title: String::new(),
            doc_type: "TECHNICAL_DOC".to_string(),
            original_filename: String::new(),
            archived_path: String::new(),
            executive_summary: String::new(),
            extracted_items_count: 0,
            ingested_at: String::new(),
            binary_original_path: None,
            tags_json: "[]".to_string(),
            tags: Vec::new(),
            realm: "WORK".to_string(),
            domain_code: "WEALTH_FINANCE".to_string(),
            processing_status: "UNPROCESSED".to_string(),
            processed_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub vault_root_path: String,
    pub mab_focus_data_path: String,
    pub auto_sync_mab_focus: bool,
    pub active_llm_provider: String, // "AGY_CLI" | "GEMINI_REST" | "LOCAL_OPENAI"
    pub agy_model: String,
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
    #[serde(default)]
    pub local_model: String,
    pub gemini_api_key: String,
    pub local_endpoint: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            vault_root_path: String::new(),
            mab_focus_data_path: String::new(),
            auto_sync_mab_focus: true,
            active_llm_provider: "AGY_CLI".to_string(),
            agy_model: "Gemini 3.1 Pro (High)".to_string(),
            gemini_model: "gemini-2.5-flash".to_string(),
            local_model: String::new(),
            gemini_api_key: String::new(),
            local_endpoint: "http://127.0.0.1:1234/v1/chat/completions".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub projects_imported_from_focus: i32,
    pub projects_exported_to_focus: i32,
    pub notes_ingested_from_focus: i32,
    pub last_synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardState {
    pub projects: Vec<Project>,
    pub active_project: Project,
    pub next_step_task: Option<WorkEntity>,
    pub horizon_tasks: Vec<WorkEntity>,
    pub active_guardrails: Vec<LearnedAntipattern>,
    pub active_sops: Vec<SopRule>,
    pub open_tasks: Vec<WorkEntity>,
    pub closed_tasks_count: i32,
    pub active_decisions: Vec<WorkEntity>,
    pub recent_logs: Vec<SessionWorkLog>,
    pub chat_history: Vec<ChatMessage>,
    #[serde(default)]
    pub vault_chat_history: Vec<ChatMessage>,
    #[serde(default)]
    pub chat_sessions: Vec<ChatSession>,
    #[serde(default)]
    pub all_project_chat_sessions: Vec<ChatSession>,
    #[serde(default)]
    pub active_copilot_session_id: String,
    #[serde(default)]
    pub active_vault_session_id: String,
    #[serde(default)]
    pub raw_events_count: i32,
    #[serde(default)]
    pub recent_raw_events: Vec<RawEventPreview>,
    pub knowledge_docs: Vec<KnowledgeDocument>,
    #[serde(default)]
    pub pending_queue: Vec<PendingQueueItem>,
    pub settings: AppSettings,
    pub can_undo: bool,
    pub last_undo_summary: Option<String>,

    // v3.0 5-Layer Ecosystem Live State Additions
    #[serde(default)]
    pub domains: Vec<LifeDomain>,
    #[serde(default)]
    pub outcomes: Vec<Outcome>,
    #[serde(default)]
    pub work_lines: Vec<WorkLine>,
    #[serde(default)]
    pub top_priority_steps: Vec<WorkEntity>,
    #[serde(default)]
    pub calendar_days: Vec<CalendarDay>,
    #[serde(default)]
    pub pending_proposals: Vec<PendingProposal>,
    #[serde(default)]
    pub personal_vault_items: Vec<PersonalVaultItem>,
    #[serde(default)]
    pub methods_playbooks: Vec<MethodPlaybook>,
}

fn default_bpmn_element_type() -> String {
    "task".to_string()
}

fn default_method_version() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MethodStep {
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub id: String,
    #[serde(default = "default_sequence_order")]
    pub order: i32,
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub description: String,
    #[serde(default = "default_bpmn_element_type", deserialize_with = "deserialize_string_lenient")]
    pub element_type: String, // "task" | "userTask" | "serviceTask" | "subProcess" | "exclusiveGateway" | "parallelGateway" | "callActivity"
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub role: String,
    #[serde(default, deserialize_with = "deserialize_vec_string_lenient")]
    pub inputs: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_vec_string_lenient")]
    pub outputs: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_vec_string_lenient")]
    pub checklist: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub notes: String,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub condition_label: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub sub_method_id: Option<String>,
    #[serde(default)]
    pub sub_steps: Vec<MethodStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MethodPlaybook {
    pub id: String, // MTH-XXXX
    pub project_id: String,
    pub slug: String,
    pub title: String,
    #[serde(default = "default_step_category")]
    pub category: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub objective: String,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub steps: Vec<MethodStep>,
    #[serde(default)]
    pub checklist: Vec<String>,
    #[serde(default)]
    pub notes_markdown: String,
    #[serde(default)]
    pub markdown_content: String,
    #[serde(default)]
    pub bpmn_xml: String,
    #[serde(default)]
    pub parent_method_id: Option<String>,
    #[serde(default)]
    pub parent_element_id: Option<String>,
    #[serde(default = "default_method_version")]
    pub version: i32,
    #[serde(default)]
    pub archived_path: String,
    #[serde(default)]
    pub bpmn_file_path: String,
    #[serde(default)]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MethodPlaybookInput {
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub method_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub category: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub objective: Option<String>,
    #[serde(default, deserialize_with = "deserialize_vec_string_lenient")]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub steps: Vec<MethodStep>,
    #[serde(default, deserialize_with = "deserialize_vec_string_lenient")]
    pub checklist: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub notes_markdown: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub bpmn_xml: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub parent_method_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub parent_element_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LegacyReprocessCandidate {
    pub source_kind: String, // "KNOWLEDGE_DOC" | "RAW_EVENT"
    pub source_id: i64,
    pub project_id: String,
    pub title: String,
    pub doc_type: String,
    pub snippet: String,
    pub created_at: String,
    pub processing_status: String,
    pub already_upgraded: bool,
    pub upgraded_at: Option<String>,
    pub archived_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyReprocessSelection {
    pub source_kind: String,
    pub source_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseBackupResult {
    pub backup_path: String,
    pub backup_size_bytes: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyReprocessEnqueueResult {
    pub backup_path: String,
    pub backup_size_bytes: u64,
    pub enqueued_count: usize,
    pub dashboard: DashboardState,
}

// 1. LifeDomain
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LifeDomain {
    pub code: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_maturity_state")]
    pub maturity_state: i32,
    #[serde(default)]
    pub maintenance_floor: String,
    #[serde(default)]
    pub desired_state: String,
    #[serde(default)]
    pub is_growth_focus: bool,
    #[serde(default)]
    pub updated_at: String,
}

// 2. Outcome
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Outcome {
    pub id: String, // OUT-XXXX
    pub domain_code: String,
    pub title: String,
    #[serde(default)]
    pub desired_reality_change: String,
    #[serde(default)]
    pub evidence_kr: String,
    #[serde(default)]
    pub time_window: String,
    #[serde(default = "default_outcome_status")]
    pub status: String,
    #[serde(default)]
    pub progress_percent: i32,
    #[serde(default)]
    pub linked_projects_count: i32,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

// 3. WorkLine
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkLine {
    pub id: String, // WLN-XXXX
    pub project_id: String,
    #[serde(default)]
    pub project_name: String,
    pub outcome_id: Option<String>,
    #[serde(default)]
    pub outcome_title: Option<String>,
    pub title: String,
    #[serde(default)]
    pub goal_statement: String,
    #[serde(default = "default_line_mode")]
    pub line_mode: String,
    #[serde(default = "default_work_line_status")]
    pub status: String,
    #[serde(default = "default_color_hex")]
    pub color_hex: String,
    #[serde(default)]
    pub total_tasks: i32,
    #[serde(default)]
    pub completed_tasks: i32,
    #[serde(default)]
    pub active_step_id: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

// 4. CalendarDay
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarDay {
    pub date_key: String, // "YYYY-MM-DD"
    #[serde(default = "default_total_capacity_minutes")]
    pub total_capacity_minutes: i32,
    #[serde(default = "default_deep_capacity_minutes")]
    pub deep_capacity_minutes: i32,
    #[serde(default)]
    pub day_note: String,
    #[serde(default)]
    pub is_off_day: bool,
    #[serde(default, alias = "tasks_count")]
    pub assigned_tasks_count: i32,
    #[serde(default, alias = "allocated_minutes")]
    pub assigned_minutes: i32,
    #[serde(default)]
    pub updated_at: String,
}

// 5. PendingProposal
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PendingProposal {
    pub id: String, // PRP-XXXX
    pub project_id: Option<String>,
    pub proposal_type: String,
    pub title: String,
    #[serde(default)]
    pub rationale_markdown: String,
    #[serde(default)]
    pub payload_json: String,
    #[serde(default = "default_proposal_status")]
    pub status: String,
    #[serde(default)]
    pub created_at: String,
    pub resolved_at: Option<String>,
}

// 6. PersonalVaultItem
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonalVaultItem {
    pub id: String, // PVI-XXXX
    #[serde(default = "default_realm_life")]
    pub realm: String,
    #[serde(default = "default_domain_code")]
    pub domain_code: String,
    pub project_id: Option<String>,
    pub item_type: String,
    pub title: String,
    #[serde(default)]
    pub content_markdown: String,
    #[serde(default)]
    pub tags_json: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub archived_path: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

// 7. ChainStepInput
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChainStepInput {
    pub title: String,
    #[serde(default)]
    pub definition_of_done: String,
    #[serde(default)]
    pub first_physical_action: String,
    #[serde(default = "default_energy_tag")]
    pub energy_tag: String,
    #[serde(default = "default_duration_minutes")]
    pub duration_minutes: i32,
    #[serde(default = "default_device_tag")]
    pub device_tag: String,
    #[serde(default = "default_location_tags")]
    pub location_tags: Vec<String>,
    #[serde(default = "default_step_category")]
    pub category: String,
    #[serde(default)]
    pub scheduled_date: Option<String>,
    #[serde(default)]
    pub scheduled_slot: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQueueItem {
    pub id: i64,
    pub project_id: String,
    pub project_name: String,
    pub session_id: String,
    pub channel: String,
    pub user_message_id: Option<i64>,
    pub system_message_id: Option<i64>,
    pub knowledge_doc_id: Option<i64>,
    pub raw_event_id: Option<i64>,
    pub title_preview: String,
    pub source_type: String,
    pub archived_path: Option<String>,
    pub status: String, // "PENDING" | "PROCESSING" | "DONE" | "DISMISSED"
    pub attempts: i32,
    pub last_error: Option<String>,
    #[serde(default)]
    pub priority_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConnectionStatus {
    pub connected: bool,
    pub provider: String,
    pub model: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQueueProcessOutcome {
    pub dashboard: DashboardState,
    pub processed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommittedChatTurn {
    pub dashboard: DashboardState,
    pub session_id: String,
    pub raw_event_id: i64,
    pub user_message_id: i64,
    pub archived_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommittedFileTurn {
    pub dashboard: DashboardState,
    pub session_id: String,
    pub raw_event_id: i64,
    pub user_message_id: i64,
    pub archived_path: String,
    pub extracted_text: String,
    pub combined_prompt_text: String,
    pub detected_format: String,
    pub effective_title: String,
    pub effective_filename: String,
    pub doc_type: String,
    #[serde(default)]
    pub knowledge_doc_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantTurnOutcome {
    pub dashboard: DashboardState,
    pub assistant_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub active_provider: String,
    pub agy_model: String,
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
    #[serde(default)]
    pub local_model: String,
    pub gemini_api_key: String,
    pub local_endpoint: String,
}

impl From<&AppSettings> for LlmConfig {
    fn from(s: &AppSettings) -> Self {
        Self {
            active_provider: s.active_llm_provider.clone(),
            agy_model: s.agy_model.clone(),
            gemini_model: s.gemini_model.clone(),
            local_model: s.local_model.clone(),
            gemini_api_key: s.gemini_api_key.clone(),
            local_endpoint: s.local_endpoint.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveModelOption {
    pub id: String,
    pub label: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveModelsResponse {
    pub provider: String,
    pub models: Vec<LiveModelOption>,
    pub live_fetched: bool,
    pub status_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CopilotEnvelope {
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub conversational_reply: String,
    #[serde(default, deserialize_with = "deserialize_vec_string_lenient")]
    pub clarifying_questions: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_bool_lenient")]
    pub needs_user_clarification: bool,
    #[serde(default, deserialize_with = "deserialize_bool_lenient")]
    pub meeting_detected: bool,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub meeting_title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub meeting_summary_markdown: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub executive_doc_summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_vec_mutations_lenient")]
    pub mutations: Vec<AtomicMutation>,
    #[serde(default, deserialize_with = "deserialize_vec_methods_lenient")]
    pub method_playbooks: Vec<MethodPlaybookInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AtomicMutation {
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub op: String, // "RESOLVE_ENTITY" | "CREATE_ENTITY" | "UPDATE_ENTITY" | "SET_NEXT_STEP" | "DELETE_ENTITY" | "SUPERSEDE_DECISION" | "UPSERT_SOP" | "REGISTER_ANTIPATTERN" | "LOG_WORK"
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub target_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub entity_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub category: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub definition_of_done: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub first_physical_action: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_bool_lenient")]
    pub is_on_critical_path: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_opt_f64_lenient")]
    pub optimistic_pomos: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_opt_f64_lenient")]
    pub most_likely_pomos: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_opt_f64_lenient")]
    pub pessimistic_pomos: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub stakeholder_tag: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub sop_rule_statement: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub mistake_pattern: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub preventive_gate_question: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_i32_lenient")]
    pub severity_score: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_opt_i32_lenient")]
    pub effective_pomos: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub deliverable_summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub internal_friction_note: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub client_sanitized_note: Option<String>,

    // v3.0 Mutation extensions
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub work_line_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub outcome_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_i32_lenient")]
    pub sequence_order: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub depends_on_task_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub energy_tag: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_i32_lenient")]
    pub duration_minutes: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub device_tag: Option<String>,
    #[serde(default)]
    pub location_tags: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub scheduled_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub scheduled_slot: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_string_lenient")]
    pub due_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_i32_lenient")]
    pub priority_score: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoSnapshot {
    pub entities: Vec<WorkEntity>,
    pub max_sop_id: i64,
    pub max_anti_id: i64,
    pub max_log_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_v2_serde_backward_compatibility() {
        let legacy_project_json = r#"{
            "id": "P1",
            "name": "Test Project",
            "workspace_path": "/path/to/project",
            "external_focus_id": null,
            "is_active": true,
            "updated_at": "2026-11-14T18:00:00Z"
        }"#;

        let p: Project = serde_json::from_str(legacy_project_json).expect("Project legacy deserialize");
        assert_eq!(p.id, "P1");
        assert_eq!(p.name, "Test Project");
        assert_eq!(p.domain_code, "WEALTH_FINANCE");
        assert_eq!(p.lifecycle_status, "ACTIVE");
        assert_eq!(p.scope_mode, "STANDARD");
        assert_eq!(p.realm, "WORK");
        assert_eq!(p.priority_weight, 5);
        assert_eq!(p.pause_resume_note, "");

        let legacy_task_json = r#"{
            "id": "TSK-01",
            "project_id": "P1",
            "entity_type": "TASK",
            "category": "BACKEND",
            "title": "Legacy Task Title",
            "definition_of_done": "Pass test",
            "first_physical_action": "Open editor",
            "status": "OPEN",
            "is_on_critical_path": true,
            "optimistic_pomos": 1.0,
            "most_likely_pomos": 2.0,
            "pessimistic_pomos": 3.0,
            "calibrated_pomos": 2.0,
            "superseded_by_id": null,
            "stakeholder_tag": null,
            "updated_at": "2026-11-14T18:00:00Z"
        }"#;

        let t: WorkEntity = serde_json::from_str(legacy_task_json).expect("WorkEntity legacy deserialize");
        assert_eq!(t.id, "TSK-01");
        assert_eq!(t.energy_tag, "MEDIUM_NORMAL");
        assert_eq!(t.duration_minutes, 25);
        assert_eq!(t.device_tag, "LAPTOP_ONLY");
        assert_eq!(t.location_tags, vec!["DESK".to_string()]);
        assert_eq!(t.sequence_order, 1);
        assert_eq!(t.priority_score, 50);
        assert!(!t.is_locked_by_chain);

        let legacy_doc_json = r#"{
            "id": 10,
            "project_id": "P1",
            "title": "Legacy Spec",
            "doc_type": "TECHNICAL_DOC",
            "original_filename": "spec.md",
            "archived_path": "/vault/spec.md",
            "executive_summary": "Summary",
            "extracted_items_count": 5,
            "ingested_at": "2026-11-14T18:00:00Z"
        }"#;

        let d: KnowledgeDocument = serde_json::from_str(legacy_doc_json).expect("KnowledgeDoc legacy deserialize");
        assert_eq!(d.realm, "WORK");
        assert_eq!(d.domain_code, "WEALTH_FINANCE");
        assert!(d.tags.is_empty());

        let legacy_mutation_json = r#"{
            "op": "CREATE_ENTITY",
            "title": "New entity"
        }"#;

        let m: AtomicMutation = serde_json::from_str(legacy_mutation_json).expect("AtomicMutation deserialize");
        assert_eq!(m.op, "CREATE_ENTITY");
        assert_eq!(m.title.as_deref(), Some("New entity"));
        assert!(m.work_line_id.is_none());
    }

    #[test]
    fn test_all_new_m1_domain_models_default_and_roundtrip() {
        let domain = LifeDomain {
            code: "MIND_LEARNING".to_string(),
            title: "ذهن و یادگیری".to_string(),
            description: "توسعه فردی".to_string(),
            maturity_state: 3,
            maintenance_floor: "مطالعه روزانه".to_string(),
            desired_state: "تسلط کامل".to_string(),
            is_growth_focus: true,
            updated_at: "2026-11-14T18:00:00Z".to_string(),
        };
        let domain_json = serde_json::to_string(&domain).unwrap();
        let domain_roundtrip: LifeDomain = serde_json::from_str(&domain_json).unwrap();
        assert_eq!(domain_roundtrip.code, "MIND_LEARNING");
        assert!(domain_roundtrip.is_growth_focus);

        let outcome = Outcome {
            id: "OUT-1001".to_string(),
            domain_code: "WEALTH_FINANCE".to_string(),
            title: "جریان درآمدی ارزی".to_string(),
            desired_reality_change: "درآمد ارزی پایدار".to_string(),
            evidence_kr: "$2000/mo MRR".to_string(),
            time_window: "Q4 2026".to_string(),
            status: "ACTIVE".to_string(),
            progress_percent: 45,
            linked_projects_count: 2,
            created_at: "2026-11-14T18:00:00Z".to_string(),
            updated_at: "2026-11-14T18:00:00Z".to_string(),
        };
        let outcome_json = serde_json::to_string(&outcome).unwrap();
        let outcome_roundtrip: Outcome = serde_json::from_str(&outcome_json).unwrap();
        assert_eq!(outcome_roundtrip.id, "OUT-1001");
        assert_eq!(outcome_roundtrip.progress_percent, 45);

        let work_line = WorkLine {
            id: "WLN-01".to_string(),
            project_id: "GERDALO".to_string(),
            project_name: "پروژه گردالو".to_string(),
            outcome_id: Some("OUT-1001".to_string()),
            outcome_title: Some("جریان درآمدی".to_string()),
            title: "خط کار احراز هویت".to_string(),
            goal_statement: "پیاده‌سازی امن".to_string(),
            line_mode: "SEQUENTIAL_CHAIN".to_string(),
            status: "ACTIVE".to_string(),
            color_hex: "#10B981".to_string(),
            total_tasks: 4,
            completed_tasks: 1,
            active_step_id: Some("TSK-02".to_string()),
            created_at: "2026-11-14T18:00:00Z".to_string(),
            updated_at: "2026-11-14T18:00:00Z".to_string(),
        };
        let wln_json = serde_json::to_string(&work_line).unwrap();
        let wln_roundtrip: WorkLine = serde_json::from_str(&wln_json).unwrap();
        assert_eq!(wln_roundtrip.line_mode, "SEQUENTIAL_CHAIN");
        assert_eq!(wln_roundtrip.total_tasks, 4);

        let day = CalendarDay {
            date_key: "2026-11-14".to_string(),
            total_capacity_minutes: 360,
            deep_capacity_minutes: 180,
            day_note: "تمرکز روی مایلسنگ ۱".to_string(),
            is_off_day: false,
            assigned_tasks_count: 5,
            assigned_minutes: 250,
            updated_at: "2026-11-14T18:00:00Z".to_string(),
        };
        let day_json = serde_json::to_string(&day).unwrap();
        let day_roundtrip: CalendarDay = serde_json::from_str(&day_json).unwrap();
        assert_eq!(day_roundtrip.date_key, "2026-11-14");
        assert_eq!(day_roundtrip.deep_capacity_minutes, 180);

        let prop = PendingProposal {
            id: "PRP-01".to_string(),
            project_id: Some("GERDALO".to_string()),
            proposal_type: "PAUSE_PROJECT".to_string(),
            title: "توقف موقت پروژه".to_string(),
            rationale_markdown: "نیاز به بازنگری".to_string(),
            payload_json: "{}".to_string(),
            status: "PENDING".to_string(),
            created_at: "2026-11-14T18:00:00Z".to_string(),
            resolved_at: None,
        };
        let prop_json = serde_json::to_string(&prop).unwrap();
        let prop_roundtrip: PendingProposal = serde_json::from_str(&prop_json).unwrap();
        assert_eq!(prop_roundtrip.proposal_type, "PAUSE_PROJECT");

        let vault_item = PersonalVaultItem {
            id: "PVI-01".to_string(),
            realm: "LIFE".to_string(),
            domain_code: "HEALTH_ENERGY".to_string(),
            project_id: None,
            item_type: "LESSON_LEARNED".to_string(),
            title: "اهمیت خواب شبانه".to_string(),
            content_markdown: "حداقل ۷ ساعت خواب".to_string(),
            tags_json: "[\"SLEEP\",\"ENERGY\"]".to_string(),
            tags: vec!["SLEEP".to_string(), "ENERGY".to_string()],
            archived_path: None,
            created_at: "2026-11-14T18:00:00Z".to_string(),
            updated_at: "2026-11-14T18:00:00Z".to_string(),
        };
        let pvi_json = serde_json::to_string(&vault_item).unwrap();
        let pvi_roundtrip: PersonalVaultItem = serde_json::from_str(&pvi_json).unwrap();
        assert_eq!(pvi_roundtrip.tags.len(), 2);

        let step_input = ChainStepInput {
            title: "طراحی DDL جدول".to_string(),
            definition_of_done: "کوئری بدون خطا اجرا شود".to_string(),
            first_physical_action: "باز کردن schema.rs".to_string(),
            energy_tag: "DEEP_HIGH".to_string(),
            duration_minutes: 45,
            device_tag: "LAPTOP_ONLY".to_string(),
            location_tags: vec!["DESK".to_string()],
            category: "BACKEND".to_string(),
            scheduled_date: Some("2026-11-14".to_string()),
            scheduled_slot: Some("MORNING_DEEP".to_string()),
            due_date: None,
        };
        let step_json = serde_json::to_string(&step_input).unwrap();
        let step_roundtrip: ChainStepInput = serde_json::from_str(&step_json).unwrap();
        assert_eq!(step_roundtrip.energy_tag, "DEEP_HIGH");
        assert_eq!(step_roundtrip.duration_minutes, 45);
    }
}
