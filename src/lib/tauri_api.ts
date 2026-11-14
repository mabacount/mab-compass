import { invoke } from "@tauri-apps/api/core";

export interface Project {
  id: string;
  name: string;
  workspace_path: string;
  external_focus_id?: string | null;
  is_active: boolean;
  updated_at: string;
}

export interface WorkEntity {
  id: string;
  project_id: string;
  entity_type: string;
  category: string;
  title: string;
  definition_of_done: string;
  first_physical_action: string;
  status: string;
  is_on_critical_path: boolean;
  optimistic_pomos: number;
  most_likely_pomos: number;
  pessimistic_pomos: number;
  calibrated_pomos: number;
  superseded_by_id?: string | null;
  stakeholder_tag?: string | null;
  updated_at: string;
}

export interface SopRule {
  id: number;
  project_id: string;
  category: string;
  rule_statement: string;
  updated_at: string;
}

export interface LearnedAntipattern {
  id: number;
  project_id: string;
  trigger_category: string;
  mistake_pattern: string;
  preventive_gate_question: string;
  severity_score: number;
  times_triggered: number;
}

export interface SessionWorkLog {
  id: number;
  project_id: string;
  task_id?: string | null;
  category: string;
  estimated_pomos: number;
  effective_pomos: number;
  deliverable_summary: string;
  internal_friction_note: string;
  client_sanitized_note: string;
  source_origin: string;
  logged_at: string;
}

export interface ChatMessage {
  id: number;
  project_id: string;
  session_id: string;
  sender: "USER" | "ASSISTANT" | "MAB_FOCUS_SYNC" | "SYSTEM";
  channel: "COPILOT" | "VAULT";
  message_text: string;
  archived_file_path?: string | null;
  applied_mutations_badge?: string | null;
  clarifying_questions: string[];
  created_at: string;
}

export interface ChatSession {
  id: string;
  project_id: string;
  channel: "COPILOT" | "VAULT";
  title: string;
  is_active: boolean;
  message_count: number;
  created_at: string;
  updated_at: string;
}

export interface RawEventPreview {
  id: number;
  source_type: string;
  snippet: string;
  archived_path?: string | null;
  created_at: string;
}

export interface KnowledgeDocument {
  id: number;
  project_id: string;
  title: string;
  doc_type: string;
  original_filename: string;
  archived_path: string;
  executive_summary: string;
  extracted_items_count: number;
  ingested_at: string;
}

export interface PendingQueueItem {
  id: number;
  project_id: string;
  project_name: string;
  session_id: string;
  channel: string;
  user_message_id?: number | null;
  system_message_id?: number | null;
  knowledge_doc_id?: number | null;
  raw_event_id?: number | null;
  title_preview: string;
  source_type: string;
  archived_path?: string | null;
  status: string;
  attempts: number;
  last_error?: string | null;
  created_at: string;
  updated_at: string;
}

export interface LlmConnectionStatus {
  connected: boolean;
  provider: string;
  model: string;
  message: string;
}

export interface PendingQueueProcessOutcome {
  dashboard: DashboardState;
  processed: boolean;
  error?: string | null;
}

export interface AppSettings {
  vault_root_path: string;
  mab_focus_data_path: string;
  auto_sync_mab_focus: boolean;
  active_llm_provider: string;
  agy_model: string;
  gemini_model: string;
  local_model: string;
  gemini_api_key: string;
  local_endpoint: string;
}

export interface LiveModelOption {
  id: string;
  label: string;
  details?: string | null;
}

export interface LiveModelsResponse {
  provider: string;
  models: LiveModelOption[];
  live_fetched: boolean;
  status_message: string;
}

export interface SyncReport {
  projects_imported_from_focus: number;
  projects_exported_to_focus: number;
  notes_ingested_from_focus: number;
  last_synced_at: string;
}

export interface DashboardState {
  projects: Project[];
  active_project: Project;
  next_step_task?: WorkEntity | null;
  horizon_tasks: WorkEntity[];
  active_guardrails: LearnedAntipattern[];
  active_sops: SopRule[];
  open_tasks: WorkEntity[];
  closed_tasks_count: number;
  active_decisions: WorkEntity[];
  recent_logs: SessionWorkLog[];
  chat_history: ChatMessage[];
  vault_chat_history: ChatMessage[];
  chat_sessions: ChatSession[];
  all_project_chat_sessions: ChatSession[];
  active_copilot_session_id: string;
  active_vault_session_id: string;
  raw_events_count: number;
  recent_raw_events: RawEventPreview[];
  knowledge_docs: KnowledgeDocument[];
  pending_queue: PendingQueueItem[];
  settings: AppSettings;
  can_undo: boolean;
  last_undo_summary?: string | null;
}

export interface CommittedChatTurn {
  dashboard: DashboardState;
  session_id: string;
  raw_event_id: number;
  user_message_id: number;
  archived_path: string;
}

export interface CommittedFileTurn {
  dashboard: DashboardState;
  session_id: string;
  raw_event_id: number;
  user_message_id: number;
  archived_path: string;
  extracted_text: string;
  combined_prompt_text: string;
  detected_format: string;
  effective_title: string;
  effective_filename: string;
  doc_type: string;
}

export interface AssistantTurnOutcome {
  dashboard: DashboardState;
  assistant_error?: string | null;
}

export function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const res = String(reader.result || "");
      const comma = res.indexOf(",");
      resolve(comma >= 0 ? res.slice(comma + 1) : res);
    };
    reader.onerror = () =>
      reject(reader.error || new Error(`خطا در خواندن فایل ${file.name}`));
    reader.readAsDataURL(file);
  });
}

export const CompassApi = {
  getDbDataVersion: () => invoke<number>("cmd_get_db_data_version"),
  getDashboard: () => invoke<DashboardState>("cmd_get_dashboard"),
  switchProject: (projectId: string) =>
    invoke<DashboardState>("cmd_switch_project", { projectId }),
  createProject: (idCode: string, name: string) =>
    invoke<DashboardState>("cmd_create_project", { idCode, name }),
  quickCompleteStep: (taskId: string, pomosSpent: number) =>
    invoke<DashboardState>("cmd_quick_complete_step", { taskId, pomosSpent }),
  createChatSession: (
    projectId: string | null,
    channel: "COPILOT" | "VAULT",
    title?: string
  ) =>
    invoke<DashboardState>("cmd_create_chat_session", {
      projectId,
      channel,
      title: title ?? null,
    }),
  switchChatSession: (sessionId: string) =>
    invoke<DashboardState>("cmd_switch_chat_session", { sessionId }),
  deleteChatSession: (sessionId: string) =>
    invoke<DashboardState>("cmd_delete_chat_session", { sessionId }),
  commitUserChatMessage: (
    channel: "COPILOT" | "VAULT",
    sessionId: string | null,
    messageText: string
  ) =>
    invoke<CommittedChatTurn>("cmd_commit_user_chat_message", {
      channel,
      sessionId,
      messageText,
    }),
  runAssistantTurn: (params: {
    channel: "COPILOT" | "VAULT";
    sessionId: string;
    messageText: string;
    archivedPath: string;
    rawEventId: number | null;
    userMessageId: number;
  }) => invoke<AssistantTurnOutcome>("cmd_run_assistant_turn", params),
  retrySystemMessage: (systemMessageId: number) =>
    invoke<AssistantTurnOutcome>("cmd_retry_system_message", {
      systemMessageId,
    }),
  commitUserFileMessage: (params: {
    channel: "COPILOT" | "VAULT";
    sessionId: string | null;
    originalFilename: string;
    contentBase64: string;
    userNote: string;
    title?: string;
    docType?: string;
  }) =>
    invoke<CommittedFileTurn>("cmd_commit_user_file_message", {
      channel: params.channel,
      sessionId: params.sessionId,
      originalFilename: params.originalFilename,
      contentBase64: params.contentBase64,
      userNote: params.userNote,
      title: params.title ?? null,
      docType: params.docType ?? null,
    }),
  commitVaultTextMessage: (params: {
    sessionId: string | null;
    title: string;
    docType: string;
    originalFilename: string;
    content: string;
  }) => invoke<CommittedFileTurn>("cmd_commit_vault_text_message", params),
  runFileAssistantTurn: (params: {
    channel: "COPILOT" | "VAULT";
    sessionId: string;
    rawEventId: number;
    userMessageId: number;
    effectiveTitle: string;
    effectiveFilename: string;
    docType: string;
    extractedText: string;
    combinedPromptText: string;
  }) => invoke<AssistantTurnOutcome>("cmd_run_file_assistant_turn", params),
  sendCopilotMessage: (messageText: string) =>
    invoke<DashboardState>("cmd_send_copilot_message", { messageText }),
  sendCopilotFileMessage: (
    originalFilename: string,
    contentBase64: string,
    userNote: string
  ) =>
    invoke<DashboardState>("cmd_send_copilot_file_message", {
      originalFilename,
      contentBase64,
      userNote,
    }),
  ingestOnboardingDocument: (
    title: string,
    docType: string,
    originalFilename: string,
    content: string
  ) =>
    invoke<DashboardState>("cmd_ingest_onboarding_document", {
      title,
      docType,
      originalFilename,
      content,
    }),
  ingestOnboardingFile: (
    title: string,
    docType: string,
    originalFilename: string,
    contentBase64: string
  ) =>
    invoke<DashboardState>("cmd_ingest_onboarding_file", {
      title,
      docType,
      originalFilename,
      contentBase64,
    }),
  sendVaultChatMessage: (messageText: string) =>
    invoke<DashboardState>("cmd_send_vault_chat_message", { messageText }),
  fetchLiveModels: (
    provider: string,
    localEndpoint: string,
    geminiApiKey: string
  ) =>
    invoke<LiveModelsResponse>("cmd_fetch_live_models", {
      provider,
      localEndpoint,
      geminiApiKey,
    }),
  syncMabFocusNow: () =>
    invoke<[SyncReport, DashboardState]>("cmd_sync_mab_focus_now"),
  saveSettings: (newSettings: AppSettings) =>
    invoke<DashboardState>("cmd_save_settings", { newSettings }),
  exportExcelReport: () => invoke<string>("cmd_export_excel_report"),
  undoLast: () => invoke<DashboardState>("cmd_undo_last"),
  checkLlmConnection: () => invoke<LlmConnectionStatus>("cmd_check_llm_connection"),
  scanUnprocessedQueue: () => invoke<DashboardState>("cmd_scan_unprocessed_queue"),
  processPendingQueueItem: (queueId: number) =>
    invoke<PendingQueueProcessOutcome>("cmd_process_pending_queue_item", { queueId }),
  dismissPendingQueueItem: (queueId: number) =>
    invoke<DashboardState>("cmd_dismiss_pending_queue_item", { queueId }),
};
