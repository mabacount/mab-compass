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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub workspace_path: String,
    pub external_focus_id: Option<String>,
    pub is_active: bool,
    pub updated_at: String,
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
    pub status: String, // "OPEN" | "IN_PROGRESS" | "BLOCKED" | "CLOSED" | "SUPERSEDED"
    pub is_on_critical_path: bool,
    pub optimistic_pomos: f64,
    pub most_likely_pomos: f64,
    pub pessimistic_pomos: f64,
    pub calibrated_pomos: f64,
    pub superseded_by_id: Option<String>,
    pub stakeholder_tag: Option<String>,
    pub updated_at: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub projects_imported_from_focus: i32,
    pub projects_exported_to_focus: i32,
    pub notes_ingested_from_focus: i32,
    pub last_synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoSnapshot {
    pub entities: Vec<WorkEntity>,
    pub max_sop_id: i64,
    pub max_anti_id: i64,
    pub max_log_id: i64,
}
