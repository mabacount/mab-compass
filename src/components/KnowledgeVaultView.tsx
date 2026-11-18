import { useMemo, useState, type ChangeEvent, type DragEvent } from "react";
import {
  AlertTriangle,
  ArrowLeft,
  CheckCircle2,
  ChevronsUpDown,
  Clock,
  FileCode2,
  FileText,
  RefreshCw,
  RotateCcw,
  UploadCloud,
} from "lucide-react";
import { DashboardState } from "../lib/tauri_api";
import { CopilotChatDock } from "./chat/CopilotChatDock";
import { MabSelect } from "./ui/MabSelect";
import { ResizableSplitPane } from "./ui/ResizableSplitPane";
import { formatShortTime } from "../lib/formatters";

interface KnowledgeVaultViewProps {
  dash: DashboardState;
  docQueueStatus: string | null;
  onBatchFilesSelected: (e: ChangeEvent<HTMLInputElement>) => void;
  onBatchFilesDropped: (files: FileList) => void;
  onManualDocIngest: (
    title: string,
    docType: string,
    content: string
  ) => Promise<void>;
  onRequeueDoc: (docId: number) => void;
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

type DocStatusFilter = "ALL" | "UNPROCESSED" | "PROCESSED";

export function KnowledgeVaultView({
  dash,
  docQueueStatus,
  onBatchFilesSelected,
  onBatchFilesDropped,
  onManualDocIngest,
  onRequeueDoc,
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
  const [docStatusFilter, setDocStatusFilter] =
    useState<DocStatusFilter>("ALL");

  const vaultSessions = dash.chat_sessions.filter((s) => s.channel === "VAULT");
  const activeVaultSession =
    vaultSessions.find((s) => s.id === dash.active_vault_session_id) ??
    vaultSessions[0];

  const unprocessedDocsCount = useMemo(
    () =>
      dash.knowledge_docs.filter((d) => d.processing_status !== "PROCESSED")
        .length,
    [dash.knowledge_docs]
  );

  const processedDocsCount = dash.knowledge_docs.length - unprocessedDocsCount;

  const filteredDocs = useMemo(() => {
    return dash.knowledge_docs.filter((doc) => {
      const isDone = doc.processing_status === "PROCESSED";
      if (docStatusFilter === "PROCESSED") return isDone;
      if (docStatusFilter === "UNPROCESSED") return !isDone;
      return true;
    });
  }, [dash.knowledge_docs, docStatusFilter]);

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
      label: "استخراج کارت حافظه",
      text: "از آخرین سند بارگذاری‌شده، کارت‌های حافظه اتمیک، درس‌آموخته‌ها و دستورالعمل‌ها را استخراج و ذخیره کن.",
    },
    {
      label: "ساخت لاین کاری",
      text: "از آخرین سند بارگذاری‌شده، تسک‌ها را به صورت لاین کاری پشت‌به‌پشت (WorkLine) متصل به Outcome بساز.",
      variant: "success" as const,
    },
    {
      label: "کشف تعارض‌ها",
      text: "آیا در اسناد آرشیو شده تناقض یا هم‌پوشانی نامشخصی در نیازمندی‌ها وجود دارد؟",
      variant: "warning" as const,
    },
  ];

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
          emptyStateTitle="تحلیل اسناد و استخراج خودکار تسک‌ها، لاین‌ها و کارت‌های دانش"
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

  return (
    <ResizableSplitPane
      storageKey="mab_compass_vault_split_pct"
      defaultPrimaryPercent={45}
      minPrimaryPercent={28}
      maxPrimaryPercent={72}
      primary={
        <div className="flex flex-col gap-3.5 overflow-y-auto pl-1 min-h-0 h-full">
        {/* Card 1: Dropzone */}
        <div className="mab-card p-4 space-y-3">
          <div className="flex items-center justify-between gap-2 flex-wrap">
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
            className={`w-full py-5 px-4 border border-dashed rounded-xl bg-white/[0.01] hover:bg-white/[0.03] flex flex-col items-center justify-center gap-2 cursor-pointer transition ${
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
              ثبت فوری با تگ <strong className="text-amber-300">پردازش‌نشده</strong> در صف زمانی و استخراج خودکار پس از اتصال مدل
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
              <span className="min-w-0 truncate">{docQueueStatus}</span>
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
            type="button"
            onClick={() => void handleSubmitManual()}
            disabled={!!docQueueStatus || !manualDocContent.trim()}
            className="w-full py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-[12.5px] transition disabled:opacity-40 shadow-sm"
          >
            ثبت در حافظه و ارسال به صف پردازش هوشمند
          </button>
        </div>

        {/* Card 3: Ingested Knowledge Collection with Processing Status Tags */}
        <div className="mab-card p-4 space-y-3">
          <div className="flex items-center justify-between gap-2 flex-wrap">
            <span className="text-[13.5px] font-bold text-white flex items-center gap-1.5">
              <FileCode2 className="w-4 h-4 text-purple-400" />
              <span>اسناد ثبت‌شده ({dash.knowledge_docs.length})</span>
            </span>

            {/* Filter Pills */}
            <div className="flex items-center gap-1 bg-mab-base/80 p-0.5 rounded-lg border border-mab-border text-[10.5px]">
              <button
                type="button"
                onClick={() => setDocStatusFilter("ALL")}
                className={`px-2 py-0.5 rounded-md transition ${
                  docStatusFilter === "ALL"
                    ? "bg-white/[0.1] text-white font-semibold"
                    : "text-mab-text-muted hover:text-white"
                }`}
              >
                همه ({dash.knowledge_docs.length})
              </button>
              <button
                type="button"
                onClick={() => setDocStatusFilter("UNPROCESSED")}
                className={`px-2 py-0.5 rounded-md transition ${
                  docStatusFilter === "UNPROCESSED"
                    ? "bg-amber-500/20 text-amber-300 font-semibold"
                    : "text-mab-text-muted hover:text-white"
                }`}
              >
                پردازش‌نشده ({unprocessedDocsCount})
              </button>
              <button
                type="button"
                onClick={() => setDocStatusFilter("PROCESSED")}
                className={`px-2 py-0.5 rounded-md transition ${
                  docStatusFilter === "PROCESSED"
                    ? "bg-emerald-500/20 text-emerald-300 font-semibold"
                    : "text-mab-text-muted hover:text-white"
                }`}
              >
                پردازش‌شده ({processedDocsCount})
              </button>
            </div>
          </div>

          <div className="space-y-2 max-h-[340px] overflow-y-auto pl-0.5">
            {filteredDocs.map((doc) => {
              const st = (doc.processing_status || "UNPROCESSED").toUpperCase();
              const isProcessed = st === "PROCESSED";
              const isProcessing = st === "PROCESSING";
              const isFailed = st === "FAILED";

              return (
                <div
                  key={doc.id}
                  className={`p-3 rounded-xl border space-y-1.5 transition ${
                    isProcessing
                      ? "bg-blue-500/[0.06] border-blue-500/35"
                      : isFailed
                      ? "bg-rose-500/[0.04] border-rose-500/30"
                      : !isProcessed
                      ? "bg-amber-500/[0.04] border-amber-500/25"
                      : "bg-white/[0.02] hover:bg-white/[0.04] border-mab-border"
                  }`}
                >
                  <div className="flex items-center justify-between gap-2 flex-wrap">
                    <div className="flex items-center gap-1.5 min-w-0 flex-1">
                      <span className="px-1.5 py-0.5 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px] shrink-0">
                        {doc.doc_type}
                      </span>
                      <span className="text-[12.5px] font-bold text-white truncate">
                        {doc.title}
                      </span>
                    </div>

                    {/* Processing Status Badge */}
                    <div className="flex items-center gap-1.5 shrink-0">
                      {isProcessed ? (
                        <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 border border-emerald-500/30 text-emerald-300 text-[10px] font-bold inline-flex items-center gap-1">
                          <CheckCircle2 className="w-3 h-3" />
                          <span>
                            پردازش‌شده (+{doc.extracted_items_count} آیتم)
                          </span>
                        </span>
                      ) : isProcessing ? (
                        <span className="px-2 py-0.5 rounded-full bg-blue-500/15 border border-blue-500/30 text-blue-300 text-[10px] font-bold inline-flex items-center gap-1">
                          <RefreshCw className="w-3 h-3 animate-spin" />
                          <span>در حال پردازش...</span>
                        </span>
                      ) : isFailed ? (
                        <span className="px-2 py-0.5 rounded-full bg-rose-500/15 border border-rose-500/30 text-rose-300 text-[10px] font-bold inline-flex items-center gap-1">
                          <AlertTriangle className="w-3 h-3" />
                          <span>خطا (پردازش‌نشده)</span>
                        </span>
                      ) : (
                        <span className="px-2 py-0.5 rounded-full bg-amber-500/15 border border-amber-500/30 text-amber-300 text-[10px] font-bold inline-flex items-center gap-1">
                          <Clock className="w-3 h-3" />
                          <span>پردازش‌نشده</span>
                        </span>
                      )}
                    </div>
                  </div>

                  <p className="text-[12px] text-mab-text-secondary leading-relaxed line-clamp-2 select-text">
                    {doc.executive_summary}
                  </p>

                  <div className="flex items-center justify-between gap-2 pt-0.5 flex-wrap">
                    <div className="flex items-center gap-2 min-w-0 text-[10px] text-mab-text-muted">
                      <span className="bidi-ltr truncate max-w-[180px] opacity-80">
                        {doc.original_filename}
                      </span>
                      <span className="font-timer shrink-0">
                        {formatShortTime(doc.ingested_at)}
                      </span>
                    </div>

                    {!isProcessing && (
                      <button
                        type="button"
                        onClick={() => onRequeueDoc(doc.id)}
                        className="px-2 py-0.5 rounded-md bg-white/[0.04] hover:bg-blue-500/20 border border-mab-border hover:border-blue-500/35 text-[10.5px] text-white/75 hover:text-blue-300 inline-flex items-center gap-1 transition"
                        title={
                          isProcessed
                            ? "بازگرداندن به وضعیت پردازش‌نشده و ارسال مجدد به صف"
                            : "ارسال مجدد به صف پردازش"
                        }
                      >
                        <RotateCcw className="w-2.5 h-2.5" />
                        <span>
                          {isProcessed ? "تحلیل مجدد" : "ارسال به صف"}
                        </span>
                      </button>
                    )}
                  </div>
                </div>
              );
            })}

            {filteredDocs.length === 0 && (
              <div className="py-6 text-center text-[12px] text-mab-text-muted">
                سندی مطابق این فیلتر در آرشیو دانش این پروژه یافت نشد.
              </div>
            )}
          </div>
        </div>
      </div>

      }
      secondary={
        <div className="flex flex-col min-h-[460px] lg:min-h-0 lg:h-full">
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
            emptyStateTitle="تحلیل اسناد و استخراج خودکار تسک‌ها، لاین‌ها و کارت‌های دانش"
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
      }
    />
  );
}

export default KnowledgeVaultView;
