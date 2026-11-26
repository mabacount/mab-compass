import { invoke } from "@tauri-apps/api/core";

export interface Project {
  id: string;
  name: string;
  workspace_path: string;
  external_focus_id?: string | null;
  is_active: boolean;
  updated_at: string;
}

export interface LifeDomain {
  code: string;
  title: string;
  description: string;
  maturity_state: number;
  maintenance_floor: string;
  desired_state: string;
  is_growth_focus: boolean;
  updated_at: string;
}

export interface Outcome {
  id: string;
  domain_code: string;
  title: string;
  desired_reality_change: string;
  evidence_kr: string;
  time_window: string;
  status: string;
  progress_percent: number;
  linked_projects_count: number;
  created_at: string;
  updated_at: string;
}

export interface WorkLine {
  id: string;
  project_id: string;
  project_name: string;
  outcome_id?: string | null;
  outcome_title?: string | null;
  title: string;
  goal_statement: string;
  line_mode: string; // "SEQUENTIAL_CHAIN" | "GOAL_CLUSTER"
  status: string;
  color_hex: string;
  total_tasks: number;
  completed_tasks: number;
  active_step_id?: string | null;
  created_at: string;
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
  project_name?: string;
  work_line_id?: string | null;
  work_line_title?: string | null;
  outcome_id?: string | null;
  sequence_order: number;
  depends_on_task_id?: string | null;
  is_locked_by_chain: boolean;
  energy_tag: string;
  duration_minutes: number;
  device_tag: string;
  location_tags: string[];
  scheduled_date?: string | null;
  scheduled_slot?: string | null;
  due_date?: string | null;
  priority_score: number;
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

export interface WrittenArtifactRef {
  artifact_type:
    | "METHOD_PLAYBOOK"
    | "KNOWLEDGE_DOC"
    | "MEETING_MINUTES"
    | "DECISION_ADR"
    | "SOP_RULE"
    | "VAULT_CARD"
    | "WORK_LINE"
    | "MARKDOWN_FILE"
    | string;
  entity_id: string;
  title: string;
  summary: string;
  file_path?: string | null;
  action: "CREATED" | "UPDATED" | string;
  version?: number | null;
  project_id?: string | null;
  project_name?: string | null;
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
  written_artifacts?: WrittenArtifactRef[];
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
  binary_original_path?: string | null;
  tags_json?: string;
  tags?: string[];
  realm?: string;
  domain_code?: string;
  processing_status: "UNPROCESSED" | "PROCESSING" | "PROCESSED" | "FAILED" | string;
  processed_at?: string | null;
}

export interface CalendarDay {
  date_key: string;
  total_capacity_minutes: number;
  deep_capacity_minutes: number;
  day_note: string;
  is_off_day: boolean;
  assigned_tasks_count: number;
  assigned_minutes: number;
  updated_at: string;
}

export interface PendingProposal {
  id: string;
  project_id?: string | null;
  proposal_type: string;
  title: string;
  rationale_markdown: string;
  payload_json: string;
  status: string;
  created_at: string;
  resolved_at?: string | null;
}

export interface PersonalVaultItem {
  id: string;
  realm: string;
  domain_code: string;
  project_id?: string | null;
  item_type: string; // "MEMORY_CARD" | "LESSON" | "INSTRUCTION" | "NOTE" | "INTEREST"
  title: string;
  content_markdown: string;
  tags_json?: string;
  tags: string[];
  archived_path?: string | null;
  created_at: string;
  updated_at: string;
}

export interface ChainStepInput {
  title: string;
  definition_of_done?: string;
  first_physical_action?: string;
  energy_tag?: string;
  duration_minutes?: number;
  device_tag?: string;
  location_tags?: string[];
  category?: string;
  scheduled_date?: string | null;
  scheduled_slot?: string | null;
  due_date?: string | null;
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
  priority_order?: number;
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

export interface MethodStep {
  id: string;
  order: number;
  title: string;
  description: string;
  element_type:
    | "task"
    | "userTask"
    | "serviceTask"
    | "subProcess"
    | "exclusiveGateway"
    | "parallelGateway"
    | "callActivity"
    | string;
  role: string;
  inputs: string[];
  outputs: string[];
  checklist: string[];
  notes: string;
  condition_label?: string | null;
  sub_method_id?: string | null;
  sub_steps: MethodStep[];
}

export interface MethodPlaybook {
  id: string;
  project_id: string;
  slug: string;
  title: string;
  category: string;
  summary: string;
  objective: string;
  prerequisites: string[];
  steps: MethodStep[];
  checklist: string[];
  notes_markdown: string;
  markdown_content: string;
  bpmn_xml: string;
  parent_method_id?: string | null;
  parent_element_id?: string | null;
  version: number;
  archived_path: string;
  bpmn_file_path: string;
  source_refs: string[];
  created_at: string;
  updated_at: string;
}

export interface MethodPlaybookInput {
  method_id?: string | null;
  title: string;
  category?: string | null;
  summary?: string | null;
  objective?: string | null;
  prerequisites?: string[];
  steps: Partial<MethodStep>[];
  checklist?: string[];
  notes_markdown?: string | null;
  bpmn_xml?: string | null;
  parent_method_id?: string | null;
  parent_element_id?: string | null;
}

export interface LegacyReprocessCandidate {
  source_kind: "KNOWLEDGE_DOC" | "RAW_EVENT" | string;
  source_id: number;
  project_id: string;
  title: string;
  doc_type: string;
  snippet: string;
  created_at: string;
  processing_status: string;
  already_upgraded: boolean;
  upgraded_at?: string | null;
  archived_path?: string | null;
}

export interface LegacyReprocessSelection {
  source_kind: string;
  source_id: number;
}

export interface DatabaseBackupResult {
  backup_path: string;
  backup_size_bytes: number;
  created_at: string;
}

export interface LegacyReprocessEnqueueResult {
  backup_path: string;
  backup_size_bytes: number;
  enqueued_count: number;
  dashboard: DashboardState;
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
  domains: LifeDomain[];
  outcomes: Outcome[];
  work_lines: WorkLine[];
  top_priority_steps?: WorkEntity[];
  calendar_days?: CalendarDay[];
  pending_proposals?: PendingProposal[];
  personal_vault_items?: PersonalVaultItem[];
  methods_playbooks?: MethodPlaybook[];
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
  knowledge_doc_id?: number | null;
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
  startTaskStep: (taskId: string) =>
    invoke<DashboardState>("cmd_start_task_step", { taskId }),
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
    knowledgeDocId?: number | null;
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
  reorderPendingQueueItem: (queueId: number, direction: "TOP" | "UP" | "DOWN") =>
    invoke<DashboardState>("cmd_reorder_pending_queue_item", { queueId, direction }),
  resetPendingQueueOrder: () =>
    invoke<DashboardState>("cmd_reset_pending_queue_order"),
  requeueKnowledgeDoc: (docId: number) =>
    invoke<DashboardState>("cmd_requeue_knowledge_doc", { docId }),
  upsertOutcome: (params: {
    outcomeId?: string | null;
    domainCode: string;
    title: string;
    desiredRealityChange: string;
    evidenceKr: string;
    timeWindow: string;
    status: string;
    progressPercent: number;
  }) =>
    invoke<DashboardState>("cmd_upsert_outcome", {
      outcomeId: params.outcomeId ?? null,
      domainCode: params.domainCode,
      title: params.title,
      desiredRealityChange: params.desiredRealityChange,
      evidenceKr: params.evidenceKr,
      timeWindow: params.timeWindow,
      status: params.status,
      progressPercent: params.progressPercent,
    }),
  deleteOutcome: (outcomeId: string) =>
    invoke<DashboardState>("cmd_delete_outcome", { outcomeId }),
  createWorkLineChain: (params: {
    project_id?: string;
    outcome_id?: string | null;
    title: string;
    goal_statement: string;
    line_mode: "SEQUENTIAL_CHAIN" | "GOAL_CLUSTER";
    color_hex?: string;
    steps: ChainStepInput[];
  }) =>
    invoke<{ status: string; work_line_id: string; dashboard?: DashboardState }>(
      "cmd_create_work_line_chain",
      { params }
    ),
  updateWorkLine: (params: {
    workLineId: string;
    title: string;
    goalStatement: string;
    outcomeId?: string | null;
    colorHex: string;
    status: string;
  }) =>
    invoke<DashboardState>("cmd_update_work_line", {
      workLineId: params.workLineId,
      title: params.title,
      goalStatement: params.goalStatement,
      outcomeId: params.outcomeId ?? null,
      colorHex: params.colorHex,
      status: params.status,
    }),
  assignTaskGrouping: (params: {
    taskId: string;
    workLineId?: string | null;
    outcomeId?: string | null;
    priorityScore?: number | null;
    energyTag?: string | null;
    deviceTag?: string | null;
    durationMinutes?: number | null;
    scheduledDate?: string | null;
  }) =>
    invoke<DashboardState>("cmd_assign_task_grouping", {
      taskId: params.taskId,
      workLineId: params.workLineId ?? null,
      outcomeId: params.outcomeId ?? null,
      priorityScore: params.priorityScore ?? null,
      energyTag: params.energyTag ?? null,
      deviceTag: params.deviceTag ?? null,
      durationMinutes: params.durationMinutes ?? null,
      scheduledDate: params.scheduledDate ?? null,
    }),
  createTaskManual: (params: {
    title: string;
    category: string;
    definitionOfDone: string;
    firstPhysicalAction: string;
    workLineId?: string | null;
    outcomeId?: string | null;
    energyTag: string;
    deviceTag: string;
    durationMinutes: number;
    scheduledDate?: string | null;
  }) =>
    invoke<DashboardState>("cmd_create_task_manual", {
      title: params.title,
      category: params.category,
      definitionOfDone: params.definitionOfDone,
      firstPhysicalAction: params.firstPhysicalAction,
      workLineId: params.workLineId ?? null,
      outcomeId: params.outcomeId ?? null,
      energyTag: params.energyTag,
      deviceTag: params.deviceTag,
      durationMinutes: params.durationMinutes,
      scheduledDate: params.scheduledDate ?? null,
    }),
  createPersonalVaultItem: (params: {
    realm: string;
    domainCode: string;
    itemType: string;
    title: string;
    contentMarkdown: string;
    tags: string[];
    attachToActiveProject: boolean;
  }) => invoke<DashboardState>("cmd_create_personal_vault_item", params),
  deletePersonalVaultItem: (itemId: string) =>
    invoke<DashboardState>("cmd_delete_personal_vault_item", { itemId }),
  resolvePendingProposal: (proposalId: string, approve: boolean) =>
    invoke<DashboardState>("cmd_resolve_pending_proposal", { proposalId, approve }),
  upsertCalendarDay: (params: {
    dateKey: string;
    totalCapacityMinutes: number;
    deepCapacityMinutes: number;
    dayNote: string;
    isOffDay: boolean;
  }) => invoke<DashboardState>("cmd_upsert_calendar_day", params),
  upsertMethodPlaybook: (input: MethodPlaybookInput) =>
    invoke<DashboardState>("cmd_upsert_method_playbook", { input }),
  deleteMethodPlaybook: (methodId: string) =>
    invoke<DashboardState>("cmd_delete_method_playbook", { methodId }),
  readWorkspaceFile: (filePath: string) =>
    invoke<string>("cmd_read_workspace_file", { filePath }),
  listLegacyReprocessCandidates: () =>
    invoke<LegacyReprocessCandidate[]>("cmd_list_legacy_reprocess_candidates"),
  backupDatabaseNow: () =>
    invoke<DatabaseBackupResult>("cmd_backup_database_now"),
  enqueueLegacyReprocess: (
    mode: "ALL_CHRONOLOGICAL" | "SELECTED",
    selectedItems: LegacyReprocessSelection[]
  ) =>
    invoke<LegacyReprocessEnqueueResult>("cmd_enqueue_legacy_reprocess", {
      mode,
      selectedItems,
    }),
};
