import { useState, type ChangeEvent, type DragEvent } from "react";
import {
  ArrowLeft,
  ChevronsUpDown,
  FileCode2,
  FileText,
  RefreshCw,
  UploadCloud,
} from "lucide-react";
import { DashboardState } from "../lib/tauri_api";
import { CopilotChatDock } from "./chat/CopilotChatDock";
import { MabSelect } from "./ui/MabSelect";

interface KnowledgeVaultViewProps {
  dash: DashboardState;
  docQueueStatus: string | null;
  onBatchFilesSelected: (e: ChangeEvent<HTMLInputElement>) => void;
  onBatchFilesDropped: (files: FileList) => void;
  onManualDocIngest: (title: string, docType: string, content: string) => Promise<void>;
  vaultChatInput: string;
  onChangeVaultChatInput: (value: string) => void;
  isSendingVaultChat: boolean;
  onSendVaultChat: () => void;
  onNewVaultChat: () => void;
  onSwitchVaultSession: (sessionId: string) => void;
  onNavigateToPath: () => void;
  onOpenSettings: () => void;
  retryingMessageId: number | null;
  onRetrySystemMessage: (systemMessageId: number) => void;
}

const DOC_TYPE_OPTIONS = [
  { value: "FREE_TEXT", label: "متن آزاد / یادداشت سریع" },
  { value: "MEETING_MINUTES", label: "صورت‌جلسه (Meeting Minutes)" },
  { value: "PROJECT_CHARTER", label: "منشور پروژه (Charter / Spec)" },
  { value: "TECHNICAL_DOC", label: "مستند فنی و معماری" },
  { value: "SOURCE_CODE", label: "قطعه کد / لاگ فنی" },
];

export function KnowledgeVaultView({
  dash,
  docQueueStatus,
  onBatchFilesSelected,
  onBatchFilesDropped,
  onManualDocIngest,
  vaultChatInput,
  onChangeVaultChatInput,
  isSendingVaultChat,
  onSendVaultChat,
  onNewVaultChat,
  onSwitchVaultSession,
  onNavigateToPath,
  onOpenSettings,
  retryingMessageId,
  onRetrySystemMessage,
}: KnowledgeVaultViewProps) {
  const [manualDocTitle, setManualDocTitle] = useState("");
  const [manualDocType, setManualDocType] = useState("FREE_TEXT");
  const [manualDocContent, setManualDocContent] = useState("");
  const [isManualExpanded, setIsManualExpanded] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [isFullVaultChat, setIsFullVaultChat] = useState(false);

  const vaultSessions = dash.chat_sessions.filter((s) => s.channel === "VAULT");
  const activeVaultSession =
    vaultSessions.find((s) => s.id === dash.active_vault_session_id) ??
    vaultSessions[0];

  const handleSubmitManual = async () => {
    if (!manualDocContent.trim() || docQueueStatus) return;
    await onManualDocIngest(manualDocTitle, manualDocType, manualDocContent);
    setManualDocTitle("");
    setManualDocContent("");
  };

  const handleDrop = (e: DragEvent<HTMLLabelElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    if (docQueueStatus) return;
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      onBatchFilesDropped(e.dataTransfer.files);
    }
  };

  const vaultPromptStarters = [
    {
      label: "خلاصه‌سازی کلیدی",
      text: "اسناد موجود را تحلیل کن و ۵ تصمیم کلیدی اتخاذ شده را لیست کن:",
    },
    {
      label: "استخراج گام‌ها",
      text: "از آخرین سند بارگذاری‌شده، وظایف و گام‌های عملیاتی را استخراج کرده و به WBS اضافه کن.",
      variant: "success" as const,
    },
    {
      label: "کشف تعارض‌ها",
      text: "آیا در اسناد آرشیو شده تناقض یا هم‌پوشانی نامشخصی در نیازمندی‌ها وجود دارد؟",
      variant: "warning" as const,
    },
  ];

  /* -------------------------------------------------------------------------- */
  /* Full Chat Mode Return                                                      */
  /* -------------------------------------------------------------------------- */
  if (isFullVaultChat) {
    return (
      <div className="flex flex-col gap-2.5 h-full min-h-0">
        <CopilotChatDock
          channel="VAULT"
          activeSession={activeVaultSession}
          sessions={vaultSessions}
          onSwitchSession={onSwitchVaultSession}
          onNewChat={onNewVaultChat}
          chatHistory={dash.vault_chat_history}
          chatInput={vaultChatInput}
          onChangeChatInput={onChangeVaultChatInput}
          isSendingChat={isSendingVaultChat}
          onSendChat={onSendVaultChat}
          isFullChatMode={isFullVaultChat}
          onToggleFullChatMode={setIsFullVaultChat}
          retryingMessageId={retryingMessageId}
          onRetrySystemMessage={onRetrySystemMessage}
          onOpenSettings={onOpenSettings}
          projectName={dash.active_project.name}
          promptStarters={vaultPromptStarters}
          emptyStateTitle="تحلیل اسناد و استخراج خودکار گراف تسک‌ها"
          extraHeaderControls={
            <button
              type="button"
              onClick={onNavigateToPath}
              className="px-2.5 py-1 rounded-full bg-emerald-500/10 hover:bg-emerald-500/20 border border-emerald-500/25 text-emerald-400 text-[11px] font-medium flex items-center gap-1 transition"
            >
              <span>مسیر پیش‌رو</span>
              <ArrowLeft className="w-3 h-3" />
            </button>
          }
        />
      </div>
    );
  }

  /* -------------------------------------------------------------------------- */
  /* Main Split View                                                            */
  /* -------------------------------------------------------------------------- */
  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-3.5 xl:gap-4 h-full min-h-0">
      {/* Right Column (RTL Start): Upload, Manual input, Document archive */}
      <div className="lg:col-span-6 xl:col-span-5 flex flex-col gap-3.5 overflow-y-auto pl-0.5 min-h-0">
        {/* Card 1: Anytype Dropzone */}
        <div className="mab-card p-4 space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-[13.5px] font-bold text-white">
              <UploadCloud className="w-4 h-4 text-blue-400" />
              <span>بارگذاری هر نوع سند یا فایل</span>
            </div>
            <span className="px-2 py-0.5 rounded-md bg-white/[0.05] border border-mab-border text-white/60 font-mono text-[10px]">
              ALL FORMATS
            </span>
          </div>

          <label
            onDragOver={(e) => {
              e.preventDefault();
              setIsDragging(true);
            }}
            onDragLeave={() => setIsDragging(false)}
            onDrop={handleDrop}
            className={`w-full py-6 px-4 border border-dashed rounded-xl bg-white/[0.01] hover:bg-white/[0.03] flex flex-col items-center justify-center gap-2 cursor-pointer transition ${
              isDragging
                ? "border-blue-500 bg-blue-500/[0.06]"
                : "border-white/[0.12] hover:border-white/[0.25]"
            }`}
          >
            <div className="w-10 h-10 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400">
              <UploadCloud className="w-5 h-5" />
            </div>
            <span className="text-[12.5px] font-semibold text-white text-center">
              انتخاب یا رها کردن فایل‌ها (PDF، DOCX، کد، متن، صورت‌جلسه)
            </span>
            <span className="text-[11px] text-mab-text-muted text-center">
              ذخیره بلادرنگ در حافظه و پوشه <code className="text-blue-400">00_raw_inputs</code>
            </span>
            <input
              type="file"
              multiple
              disabled={!!docQueueStatus}
              onChange={onBatchFilesSelected}
              className="hidden"
            />
          </label>

          {docQueueStatus && (
            <div className="p-3 rounded-xl bg-purple-500/10 border border-purple-500/30 text-purple-300 text-[12px] font-semibold flex items-center gap-2">
              <RefreshCw className="w-3.5 h-3.5 animate-spin shrink-0" />
              <span>{docQueueStatus}</span>
            </div>
          )}
        </div>

        {/* Card 2: Manual Ingest Form */}
        <div className="mab-card p-4 space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-[13.5px] font-bold text-white">
              <FileText className="w-4 h-4 text-emerald-400" />
              <span>ورود متن آزاد یا صورت‌جلسه</span>
            </div>
            <button
              type="button"
              onClick={() => setIsManualExpanded((v) => !v)}
              className="p-1 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-white/60 hover:text-white transition"
            >
              <ChevronsUpDown className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            <input
              value={manualDocTitle}
              onChange={(e) => setManualDocTitle(e.target.value)}
              placeholder="عنوان دلخواه..."
              className="w-full mab-input px-3 py-1.5 text-[12.5px] select-text"
            />
            {/* Custom MabSelect replacing raw select */}
            <MabSelect
              value={manualDocType}
              onChange={setManualDocType}
              options={DOC_TYPE_OPTIONS}
              className="w-full"
            />
          </div>

          <textarea
            value={manualDocContent}
            onChange={(e) => setManualDocContent(e.target.value)}
            placeholder="متن، تصمیمات، توضیحات پروژه یا صورت‌جلسه را اینجا وارد کنید..."
            rows={isManualExpanded ? 8 : 3}
            className={`w-full mab-input p-3 text-[13px] leading-relaxed select-text resize-none ${
              isManualExpanded ? "min-h-[180px]" : "min-h-[85px]"
            }`}
          />

          <button
            onClick={handleSubmitManual}
            disabled={!!docQueueStatus || !manualDocContent.trim()}
            className="w-full py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-[12.5px] transition disabled:opacity-40 shadow-sm"
          >
            ثبت در حافظه و شروع تحلیل گراف
          </button>
        </div>

        {/* Card 3: Ingested Knowledge Collection */}
        <div className="mab-card p-4 space-y-2.5">
          <div className="flex items-center justify-between">
            <span className="text-[13.5px] font-bold text-white flex items-center gap-1.5">
              <FileCode2 className="w-4 h-4 text-purple-400" />
              <span>اسناد ثبت‌شده ({dash.knowledge_docs.length})</span>
            </span>
            <span className="font-mono text-[11px] text-mab-text-muted">
              01_knowledge_archive/
            </span>
          </div>

          <div className="space-y-2 max-h-[260px] overflow-y-auto">
            {dash.knowledge_docs.map((doc) => (
              <div
                key={doc.id}
                className="p-3 rounded-xl bg-white/[0.02] hover:bg-white/[0.04] border border-mab-border space-y-1.5 transition"
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="px-1.5 py-0.5 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px]">
                      {doc.doc_type}
                    </span>
                    <span className="text-[13px] font-bold text-white truncate">
                      {doc.title}
                    </span>
                  </div>
                  <span className="px-1.5 py-0.5 rounded-full bg-emerald-500/15 text-emerald-300 font-mono text-[10px] font-bold shrink-0">
                    +{doc.extracted_items_count} آیتم
                  </span>
                </div>
                <p className="text-[12px] text-mab-text-secondary leading-relaxed line-clamp-2 select-text">
                  {doc.executive_summary}
                </p>
                <div className="text-[10px] text-mab-text-muted bidi-ltr truncate select-text opacity-70">
                  {doc.original_filename}
                </div>
              </div>
            ))}
            {dash.knowledge_docs.length === 0 && (
              <div className="py-6 text-center text-[12px] text-mab-text-muted">
                هنوز سندی در آرشیو دانش این پروژه ثبت نشده است.
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Left Column (RTL End): Copilot Chat Dock */}
      <div className="lg:col-span-6 xl:col-span-7 flex flex-col min-h-[500px] lg:min-h-0 lg:h-full">
        <CopilotChatDock
          channel="VAULT"
          activeSession={activeVaultSession}
          sessions={vaultSessions}
          onSwitchSession={onSwitchVaultSession}
          onNewChat={onNewVaultChat}
          chatHistory={dash.vault_chat_history}
          chatInput={vaultChatInput}
          onChangeChatInput={onChangeVaultChatInput}
          isSendingChat={isSendingVaultChat}
          onSendChat={onSendVaultChat}
          isFullChatMode={isFullVaultChat}
          onToggleFullChatMode={setIsFullVaultChat}
          retryingMessageId={retryingMessageId}
          onRetrySystemMessage={onRetrySystemMessage}
          onOpenSettings={onOpenSettings}
          projectName={dash.active_project.name}
          promptStarters={vaultPromptStarters}
          emptyStateTitle="تحلیل اسناد و استخراج خودکار گراف تسک‌ها"
          extraHeaderControls={
            <button
              type="button"
              onClick={onNavigateToPath}
              className="px-2.5 py-1 rounded-full bg-emerald-500/10 hover:bg-emerald-500/20 border border-emerald-500/25 text-emerald-400 text-[11px] font-medium flex items-center gap-1 transition"
            >
              <span>مسیر پیش‌رو</span>
              <ArrowLeft className="w-3 h-3" />
            </button>
          }
        />
      </div>
    </div>
  );
}

export default KnowledgeVaultView;
