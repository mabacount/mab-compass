import { useEffect, useRef, useState, type DragEvent } from "react";
import {
  ArrowUp,
  BookOpen,
  ChevronDown,
  Database,
  Expand,
  FileText,
  GitBranch,
  HelpCircle,
  Paperclip,
  Plus,
  RefreshCw,
  Settings,
  Sparkles,
  Workflow,
  X,
} from "lucide-react";
import {
  ChatMessage,
  ChatSession,
  DashboardState,
  LlmConnectionStatus,
  WrittenArtifactRef,
} from "../lib/tauri_api";
import { formatShortTime } from "../lib/formatters";
import { MarkdownRenderer } from "./ui/MarkdownRenderer";
import { PendingQueueBanner } from "./PendingQueueBanner";
import { QuickModelSwitcher } from "./QuickModelSwitcher";

interface AntigravityChatPaneProps {
  dash: DashboardState;
  activeChannel: "COPILOT" | "VAULT";
  onChangeChannel: (ch: "COPILOT" | "VAULT") => void;
  activeSession?: ChatSession;
  chatHistory: ChatMessage[];
  chatInput: string;
  onChangeChatInput: (val: string) => void;
  isSendingChat: boolean;
  onSendChat: () => void;
  onSendFiles: (files: FileList) => void;
  retryingMessageId: number | null;
  onRetrySystemMessage: (systemMessageId: number) => void;
  onOpenArtifactTab: (artifact: WrittenArtifactRef) => void;
  onOpenSettingsTab: () => void;
  onQuickChangeModel?: (provider: string, modelId: string, modelLabel: string) => void;
  onOpenReviewTab: () => void;
  // Queue banner props
  connectionStatus: LlmConnectionStatus | null;
  isCheckingConnection: boolean;
  isProcessingQueue: boolean;
  processingQueueId: number | null;
  autoQueueEnabled: boolean;
  cooldownSecondsLeft: number;
  onToggleAutoQueue: (enabled: boolean) => void;
  onRecheckConnection: () => void;
  onProcessNowAll: () => void;
  onProcessSingleQueueItem: (queueId: number) => void;
  onReorderQueueItem: (queueId: number, direction: "TOP" | "UP" | "DOWN") => void;
  onResetQueueOrder: () => void;
  onDismissQueueItem: (queueId: number) => void;
}

function getArtifactIcon(artifactType: string) {
  switch (artifactType) {
    case "METHOD_PLAYBOOK":
      return <Workflow className="w-4 h-4 text-purple-300 shrink-0" />;
    case "WORK_LINE":
      return <GitBranch className="w-4 h-4 text-blue-300 shrink-0" />;
    case "VAULT_CARD":
      return <BookOpen className="w-4 h-4 text-emerald-300 shrink-0" />;
    default:
      return <FileText className="w-4 h-4 text-white/85 shrink-0" />;
  }
}

function getArtifactBadge(art: WrittenArtifactRef): string {
  if (art.artifact_type === "METHOD_PLAYBOOK") {
    return art.action === "UPDATED"
      ? `به‌روزرسانی v${art.version ?? 2} • BPMN`
      : `دستورالعمل جدید v${art.version ?? 1} • BPMN`;
  }
  if (art.artifact_type === "KNOWLEDGE_DOC") return "سند دانش";
  if (art.artifact_type === "MEETING_MINUTES") return "صورت‌جلسه";
  if (art.artifact_type === "DECISION_ADR") return "تصمیم ADR";
  if (art.artifact_type === "SOP_GUARDRAIL") return "قانون / سد پیشگیرانه";
  if (art.artifact_type === "VAULT_CARD") return "کارت دانش";
  if (art.artifact_type === "WORK_LINE") return "لاین کاری • BPMN";
  return "سند";
}

export function AntigravityChatPane({
  dash,
  activeChannel,
  onChangeChannel,
  activeSession,
  chatHistory,
  chatInput,
  onChangeChatInput,
  isSendingChat,
  onSendChat,
  onSendFiles,
  retryingMessageId,
  onRetrySystemMessage,
  onOpenArtifactTab,
  onOpenSettingsTab,
  onQuickChangeModel,
  onOpenReviewTab,
  connectionStatus,
  isCheckingConnection,
  isProcessingQueue,
  processingQueueId,
  autoQueueEnabled,
  cooldownSecondsLeft,
  onToggleAutoQueue,
  onRecheckConnection,
  onProcessNowAll,
  onProcessSingleQueueItem,
  onReorderQueueItem,
  onResetQueueOrder,
  onDismissQueueItem,
}: AntigravityChatPaneProps) {
  const chatEndRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const modalTextareaRef = useRef<HTMLTextAreaElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const [isDraggingFile, setIsDraggingFile] = useState(false);
  const [showMemoryDrawer, setShowMemoryDrawer] = useState(false);
  const [showFullComposer, setShowFullComposer] = useState(false);
  const [isModelSwitcherOpen, setIsModelSwitcherOpen] = useState(false);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatHistory.length, isSendingChat, activeSession?.id]);

  useEffect(() => {
    if (showFullComposer) {
      window.setTimeout(() => modalTextareaRef.current?.focus(), 0);
    }
  }, [showFullComposer]);

  const activeModelLabel = (() => {
    const s = dash.settings;
    if (s.active_llm_provider === "GEMINI_REST") {
      return s.gemini_model || "gemini-2.5-flash";
    }
    if (s.active_llm_provider === "LOCAL_OPENAI") {
      return s.local_model || "LM Studio Local";
    }
    return s.agy_model || "Antigravity CLI";
  })();

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDraggingFile(false);
    if (isSendingChat) return;
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      onSendFiles(e.dataTransfer.files);
    }
  };

  const insertPrompt = (text: string) => {
    const next = chatInput.trim() ? `${chatInput.trimEnd()}\n${text}` : text;
    onChangeChatInput(next);
    window.setTimeout(() => textareaRef.current?.focus(), 0);
  };

  return (
    <section
      onDragOver={(e) => {
        e.preventDefault();
        setIsDraggingFile(true);
      }}
      onDragLeave={() => setIsDraggingFile(false)}
      onDrop={handleDrop}
      className={`flex-1 min-w-[300px] bg-[#0D0E11] flex flex-col min-h-0 relative overflow-hidden ${
        isDraggingFile ? "ring-2 ring-inset ring-blue-500/60" : ""
      }`}
    >
      {/* Drag & drop overlay */}
      {isDraggingFile && (
        <div
          dir="rtl"
          className="absolute inset-0 bg-blue-500/10 backdrop-blur-[2px] z-30 flex items-center justify-center pointer-events-none"
        >
          <div className="bg-[#16171C] border border-blue-500/40 rounded-xl px-5 py-3 shadow-2xl flex items-center gap-2 text-white text-[13px] font-medium">
            <Paperclip className="w-4 h-4 text-blue-400" />
            <span>فایل‌ها را رها کنید تا در حافظه ثبت و توسط مدل تحلیل شوند</span>
          </div>
        </div>
      )}

      {/* Slim Conversation Header (Antigravity Style) */}
      <div
        dir="ltr"
        className="h-[38px] px-3.5 border-b border-white/[0.06] bg-[#0E0F12] flex items-center justify-between gap-2 shrink-0"
      >
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-[12.5px] font-medium text-white/85 truncate" dir="auto">
            {activeSession?.title || "گفتگوی همیار"}
          </span>
          {activeSession?.project_id === "GLOBAL" ? (
            <span
              dir="rtl"
              className="px-2 py-0.5 rounded-md bg-cyan-500/15 border border-cyan-500/25 text-[10px] font-medium text-cyan-300 shrink-0"
            >
              آزاد • همه پروژه‌ها
            </span>
          ) : (
            <span
              dir="auto"
              className="px-2 py-0.5 rounded-md bg-white/[0.05] border border-white/[0.08] text-[10px] text-white/60 shrink-0 truncate max-w-[130px]"
            >
              {dash.active_project.name}
            </span>
          )}
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          {/* Copilot / Vault Channel Switcher */}
          <div
            dir="rtl"
            className="flex items-center bg-[#15161B] border border-white/[0.07] rounded-lg p-0.5 text-[11px]"
          >
            <button
              type="button"
              onClick={() => onChangeChannel("COPILOT")}
              className={`px-2 py-0.5 rounded-md transition ${
                activeChannel === "COPILOT"
                  ? "bg-white/[0.1] text-white font-semibold"
                  : "text-white/50 hover:text-white"
              }`}
            >
              همیار
            </button>
            <button
              type="button"
              onClick={() => onChangeChannel("VAULT")}
              className={`px-2 py-0.5 rounded-md transition ${
                activeChannel === "VAULT"
                  ? "bg-purple-500/25 text-purple-200 font-semibold"
                  : "text-white/50 hover:text-white"
              }`}
            >
              اسناد
            </button>
          </div>

          <button
            type="button"
            onClick={() => setShowMemoryDrawer((v) => !v)}
            title={`حافظه خام پروژه (${dash.raw_events_count} رویداد)`}
            className={`px-2 py-1 rounded-md text-[11px] inline-flex items-center gap-1 transition ${
              showMemoryDrawer
                ? "bg-emerald-500/15 text-emerald-300 border border-emerald-500/30"
                : "text-white/55 hover:text-white hover:bg-white/[0.06]"
            }`}
          >
            <Database className="w-3.5 h-3.5" />
            <span className="font-mono text-[10.5px]">{dash.raw_events_count}</span>
          </button>
        </div>
      </div>

      {/* Collapsible Pending Queue Strip */}
      {dash.pending_queue.length > 0 && (
        <div dir="rtl" className="px-3 pt-2.5 shrink-0">
          <PendingQueueBanner
            queue={dash.pending_queue}
            connectionStatus={connectionStatus}
            isCheckingConnection={isCheckingConnection}
            isProcessingQueue={isProcessingQueue}
            processingQueueId={processingQueueId}
            autoQueueEnabled={autoQueueEnabled}
            cooldownSecondsLeft={cooldownSecondsLeft}
            onToggleAutoQueue={onToggleAutoQueue}
            onRecheckConnection={onRecheckConnection}
            onProcessNowAll={onProcessNowAll}
            onProcessSingleItem={onProcessSingleQueueItem}
            onReorderItem={onReorderQueueItem}
            onResetQueueOrder={onResetQueueOrder}
            onDismissItem={onDismissQueueItem}
            onOpenSettings={onOpenSettingsTab}
          />
        </div>
      )}

      {/* Optional Live Memory Drawer */}
      {showMemoryDrawer && (
        <div
          dir="rtl"
          className="px-4 py-3 border-b border-white/[0.07] bg-[#111216] space-y-2 max-h-52 overflow-y-auto shrink-0"
        >
          <div className="flex items-center justify-between text-[11.5px]">
            <span className="font-semibold text-emerald-400 flex items-center gap-1.5">
              <Database className="w-3.5 h-3.5" />
              <span>حافظه بلادرنگ SQLite ({dash.raw_events_count} رویداد)</span>
            </span>
            <button
              type="button"
              onClick={() => setShowMemoryDrawer(false)}
              className="text-white/45 hover:text-white"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
          <div className="space-y-1.5">
            {dash.recent_raw_events.slice(0, 6).map((ev) => (
              <div
                key={ev.id}
                className="p-2 rounded-lg bg-[#16171C] border border-white/[0.06] text-[11.5px] select-text"
              >
                <div className="flex items-center justify-between text-[10px] text-white/45">
                  <span className="font-mono text-blue-400">
                    #{ev.id} • {ev.source_type}
                  </span>
                  <span className="font-mono">{formatShortTime(ev.created_at)}</span>
                </div>
                <p className="text-white/80 line-clamp-2 mt-0.5">{ev.snippet}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Main Obsidian Chat Feed */}
      <div
        dir="rtl"
        className="flex-1 min-h-0 overflow-y-auto px-4 sm:px-5 py-4 space-y-5 select-text"
      >
        {chatHistory.length === 0 && (
          <div className="max-w-xl mx-auto mt-8 p-5 rounded-2xl bg-[#131418] border border-white/[0.07] space-y-3 text-[13px] text-mab-text-secondary">
            <div className="flex items-center gap-2 text-white font-bold text-[14.5px]">
              <Sparkles className="w-4 h-4 text-blue-400" />
              <span>همیار هوشمند Mab-Compass آماده است</span>
            </div>
            <p className="leading-[1.85]">
              گزارش کار، یادداشت‌های خرد، ایده‌ها یا فایل‌های خود را اینجا بنویسید یا رها کنید. مدل
              به‌طور خودکار تسک‌ها، لاین‌های کاری، کارت‌های دانش و{" "}
              <strong className="text-purple-300">متد و دستورالعمل انجام کار + نمودار BPMN 2.0</strong>{" "}
              را استخراج و ذخیره می‌کند و کارت هر سند نوشته‌شده را زیر پیام خود نشان می‌دهد.
            </p>
            <div className="flex flex-wrap gap-1.5 pt-1">
              {[
                {
                  label: "استخراج متد و BPMN",
                  text: "از این گزارش کار، دستورالعمل گام‌به‌گام (متد انجام کار) و نمودار BPMN 2.0 را استخراج و ذخیره کن:\n",
                },
                {
                  label: "ساخت لاین کاری",
                  text: "برای این هدف یک لاین کاری پشت‌به‌پشت (WorkLine) با گام‌های دقیق بساز:\n",
                },
                {
                  label: "ثبت گزارش کار",
                  text: "گزارش کار امروز من:\n",
                },
              ].map((chip, idx) => (
                <button
                  key={idx}
                  type="button"
                  onClick={() => insertPrompt(chip.text)}
                  className="px-2.5 py-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] text-[11.5px] text-white/80 hover:text-white transition"
                >
                  {chip.label}
                </button>
              ))}
            </div>
          </div>
        )}

        {chatHistory.map((m) => {
          const artifacts = m.written_artifacts ?? [];

          // USER MESSAGE: sleek dark Antigravity rounded box
          if (m.sender === "USER") {
            return (
              <div key={m.id} className="space-y-1">
                <div className="rounded-xl bg-[#16171C] border border-white/[0.08] px-4 py-3 text-[13.5px] text-[#F3F4F6] leading-[1.85] whitespace-pre-wrap break-words">
                  {m.message_text}
                </div>
                <div className="flex items-center justify-end px-1 text-[10px] text-white/35 font-mono">
                  {formatShortTime(m.created_at)}
                </div>
              </div>
            );
          }

          // SYSTEM ERROR MESSAGE
          if (m.sender === "SYSTEM") {
            return (
              <div
                key={m.id}
                className="rounded-xl bg-amber-500/[0.07] border border-amber-500/25 p-3.5 space-y-2.5 text-[13px]"
              >
                <div className="flex items-center justify-between text-[11px] text-amber-300">
                  <span className="font-semibold">اعلان سیستم • ذخیره در حافظه</span>
                  <span className="font-mono opacity-75">{formatShortTime(m.created_at)}</span>
                </div>
                <p className="text-white/90 whitespace-pre-wrap leading-relaxed">
                  {m.message_text}
                </p>
                <div className="flex items-center gap-2 flex-wrap">
                  <button
                    type="button"
                    disabled={isSendingChat || retryingMessageId !== null}
                    onClick={() => onRetrySystemMessage(m.id)}
                    className="px-3 py-1 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[11.5px] font-semibold inline-flex items-center gap-1.5 transition disabled:opacity-40"
                  >
                    <RefreshCw
                      className={`w-3 h-3 ${retryingMessageId === m.id ? "animate-spin" : ""}`}
                    />
                    <span>
                      {retryingMessageId === m.id ? "در حال ارسال مجدد..." : "ارسال مجدد به مدل"}
                    </span>
                  </button>
                  <button
                    type="button"
                    onClick={onOpenSettingsTab}
                    className="px-3 py-1 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] border border-white/[0.09] text-amber-300 text-[11.5px] font-medium inline-flex items-center gap-1.5 transition"
                  >
                    <Settings className="w-3 h-3" />
                    <span>تنظیمات مدل</span>
                  </button>
                </div>
              </div>
            );
          }

          // MAB_FOCUS_SYNC MESSAGE
          if (m.sender === "MAB_FOCUS_SYNC") {
            return (
              <div
                key={m.id}
                className="rounded-xl bg-emerald-500/[0.06] border border-emerald-500/20 px-4 py-3 space-y-1.5 text-[13px]"
              >
                <div className="flex items-center justify-between text-[11px] text-emerald-400">
                  <span className="font-semibold">سینک خودکار از Mab-Focus</span>
                  <span className="font-mono opacity-75">{formatShortTime(m.created_at)}</span>
                </div>
                <p className="text-white/90 whitespace-pre-wrap leading-relaxed">
                  {m.message_text}
                </p>
              </div>
            );
          }

          // ASSISTANT MESSAGE: Exact Antigravity borderless prose + Inline Artifact Cards + Review Strip!
          return (
            <div key={m.id} className="space-y-3">
              {/* Clean borderless Markdown response */}
              <MarkdownRenderer content={m.message_text} compact />

              {/* Exact Antigravity Inline Document / Method / Artifact Cards (matching screenshot 2!) */}
              {artifacts.length > 0 && (
                <div className="space-y-2 pt-0.5">
                  {artifacts.map((art, idx) => (
                    <button
                      key={`${m.id}-art-${idx}-${art.entity_id}`}
                      type="button"
                      onClick={() => onOpenArtifactTab(art)}
                      className="w-full text-right rounded-xl bg-[#131419] hover:bg-[#191B22] border border-white/[0.10] hover:border-white/[0.22] px-4 py-3 transition group block shadow-sm"
                    >
                      <div className="flex items-center justify-between gap-2 mb-1">
                        <div className="flex items-center gap-2 min-w-0">
                          {getArtifactIcon(art.artifact_type)}
                          <span className="text-[13.5px] font-bold text-white group-hover:text-blue-300 transition truncate">
                            {art.title}
                          </span>
                        </div>
                        <div className="flex items-center gap-1.5 shrink-0">
                          {(art.project_name || art.project_id) && (
                            <span className="px-2 py-0.5 rounded-md bg-cyan-500/10 border border-cyan-500/20 text-[10px] font-medium text-cyan-300">
                              {art.project_name ?? art.project_id}
                            </span>
                          )}
                          <span className="px-2 py-0.5 rounded-md bg-white/[0.05] border border-white/[0.08] text-[10px] font-medium text-white/70">
                            {getArtifactBadge(art)}
                          </span>
                        </div>
                      </div>
                      {art.summary && (
                        <p className="text-[12.5px] text-[#9CA3AF] leading-relaxed line-clamp-2">
                          {art.summary}
                        </p>
                      )}
                    </button>
                  ))}
                </div>
              )}

              {/* Clarifying Questions Chips */}
              {m.clarifying_questions && m.clarifying_questions.length > 0 && (
                <div className="p-3 rounded-xl bg-amber-500/[0.06] border border-amber-500/20 space-y-2">
                  <div className="flex items-center gap-1.5 text-[11.5px] font-bold text-amber-300">
                    <HelpCircle className="w-3.5 h-3.5 shrink-0" />
                    <span>سوالات همیار برای تدقیق (کلیک کنید):</span>
                  </div>
                  <div className="space-y-1.5">
                    {m.clarifying_questions.map((q, idx) => (
                      <button
                        key={`${m.id}-cq-${idx}`}
                        type="button"
                        onClick={() => insertPrompt(`پاسخ سوال ${idx + 1} («${q}»):\n`)}
                        className="w-full text-right p-2 rounded-lg bg-[#14151A] border border-white/[0.07] hover:border-amber-500/40 text-[12px] text-white/90 transition flex items-start gap-2"
                      >
                        <span className="font-mono text-amber-400 font-bold text-[11px] mt-0.5 shrink-0">
                          {idx + 1}.
                        </span>
                        <span>{q}</span>
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Antigravity-style bottom Mutation / Review strip */}
              {m.applied_mutations_badge &&
                m.applied_mutations_badge !== "پرسش/پاسخ (بدون تغییر گراف)" && (
                  <div className="rounded-lg bg-[#121318] border border-white/[0.07] px-3 py-1.5 flex items-center justify-between gap-2 flex-wrap text-[11.5px]">
                    <span className="text-emerald-300/90 font-medium truncate">
                      ✓ {m.applied_mutations_badge}
                    </span>
                    <button
                      type="button"
                      onClick={onOpenReviewTab}
                      className="px-2.5 py-0.5 rounded-md bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.09] text-white/85 hover:text-white text-[11px] font-medium transition shrink-0"
                    >
                      بررسی
                    </button>
                  </div>
                )}

              <div className="flex items-center justify-between text-[10px] text-white/30 px-0.5">
                <span>همیار هوشمند</span>
                <span className="font-mono">{formatShortTime(m.created_at)}</span>
              </div>
            </div>
          );
        })}

        {isSendingChat && (
          <div className="flex items-center gap-2.5 py-2 text-[12.5px] text-white/60">
            <RefreshCw className="w-3.5 h-3.5 text-blue-400 animate-spin shrink-0" />
            <span>پیام در حافظه ذخیره شد • در حال تحلیل و استخراج...</span>
          </div>
        )}

        <div ref={chatEndRef} />
      </div>

      {/* Antigravity Elevated Bottom Composer Box */}
      <div className="px-3.5 pb-3 pt-1.5 shrink-0 bg-[#0D0E11] relative z-20">
        <div className="rounded-2xl bg-[#16171C] border border-white/[0.10] focus-within:border-white/[0.22] transition shadow-lg relative">
          <textarea
            ref={textareaRef}
            dir="rtl"
            value={chatInput}
            onChange={(e) => onChangeChatInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                onSendChat();
              }
            }}
            placeholder={
              activeChannel === "VAULT"
                ? "سوال یا دستور برای اسناد و استخراج متد بنویسید... (Enter ارسال)"
                : "پیام، گزارش کار یا دستور خود را بنویسید... (Enter ارسال • Shift+Enter خط جدید)"
            }
            rows={3}
            className="w-full bg-transparent px-4 pt-3 pb-2 text-[13.5px] leading-[1.8] text-white placeholder-white/35 resize-none outline-none min-h-[76px] max-h-[190px] select-text rounded-t-2xl"
          />

          {/* Bottom Composer Toolbar (matching Antigravity screenshot!) */}
          <div
            dir="ltr"
            className="px-3 pb-2.5 pt-1 flex items-center justify-between gap-2 rounded-b-2xl"
          >
            {/* Left side: + attach, expand, model selector pill */}
            <div className="flex items-center gap-1.5 min-w-0">
              <input
                ref={fileInputRef}
                type="file"
                multiple
                className="hidden"
                disabled={isSendingChat}
                onChange={(e) => {
                  if (e.target.files && e.target.files.length > 0) {
                    onSendFiles(e.target.files);
                    e.target.value = "";
                  }
                }}
              />
              <button
                type="button"
                disabled={isSendingChat}
                onClick={() => fileInputRef.current?.click()}
                title="ضمیمه فایل (PDF، DOCX، کد، متن)"
                className="w-7 h-7 rounded-lg hover:bg-white/[0.08] text-white/60 hover:text-white flex items-center justify-center transition disabled:opacity-40 shrink-0"
              >
                <Plus className="w-4 h-4" />
              </button>

              <button
                type="button"
                onClick={() => setShowFullComposer(true)}
                title="ویرایشگر تمام‌صفحه"
                className="w-7 h-7 rounded-lg hover:bg-white/[0.08] text-white/55 hover:text-white flex items-center justify-center transition shrink-0"
              >
                <Expand className="w-3.5 h-3.5" />
              </button>

              <div className="relative">
                <button
                  type="button"
                  onClick={() => setIsModelSwitcherOpen((prev) => !prev)}
                  title="تغییر سریع مدل یا سرویس‌دهنده هوش مصنوعی"
                  className="px-2 py-1 rounded-lg hover:bg-white/[0.07] text-[11.5px] text-white/65 hover:text-white inline-flex items-center gap-1 transition truncate max-w-[200px]"
                >
                  <span className="truncate font-mono text-[11px]">{activeModelLabel}</span>
                  <ChevronDown className="w-3 h-3 opacity-60 shrink-0" />
                </button>

                <QuickModelSwitcher
                  isOpen={isModelSwitcherOpen}
                  onClose={() => setIsModelSwitcherOpen(false)}
                  settings={dash.settings}
                  onSelectModel={(provider, modelId, label) => {
                    if (onQuickChangeModel) {
                      onQuickChangeModel(provider, modelId, label);
                    }
                  }}
                  onOpenSettingsTab={onOpenSettingsTab}
                />
              </div>
            </div>

            {/* Right side: Circular Antigravity Blue Send Button */}
            <button
              type="button"
              onClick={onSendChat}
              disabled={isSendingChat || !chatInput.trim()}
              title="ارسال (Enter)"
              className="w-8 h-8 rounded-full bg-[#3B82F6] hover:bg-blue-500 text-white flex items-center justify-center shadow-sm transition disabled:opacity-35 disabled:cursor-not-allowed shrink-0"
            >
              {isSendingChat ? (
                <RefreshCw className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <ArrowUp className="w-4 h-4 stroke-[2.5]" />
              )}
            </button>
          </div>
        </div>
      </div>

      {/* Full-Screen Composer Modal */}
      {showFullComposer && (
        <div
          dir="rtl"
          className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4"
          onClick={() => setShowFullComposer(false)}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            className="bg-[#16171C] border border-white/[0.1] w-full max-w-3xl p-5 space-y-3 rounded-2xl shadow-2xl flex flex-col max-h-[88vh]"
          >
            <div className="flex items-center justify-between border-b border-white/[0.08] pb-3">
              <span className="text-[14px] font-bold text-white flex items-center gap-2">
                <Sparkles className="w-4 h-4 text-blue-400" />
                <span>ویرایشگر گسترده ({dash.active_project.name})</span>
              </span>
              <button
                type="button"
                onClick={() => setShowFullComposer(false)}
                className="p-1.5 rounded-lg hover:bg-white/[0.08] text-white/60 hover:text-white"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <textarea
              ref={modalTextareaRef}
              value={chatInput}
              onChange={(e) => onChangeChatInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") setShowFullComposer(false);
                else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  setShowFullComposer(false);
                  onSendChat();
                }
              }}
              placeholder="متن کامل گزارش کار، دستورالعمل یا پیام خود را اینجا بنویسید..."
              className="w-full flex-1 min-h-[300px] bg-transparent p-2 text-[14px] leading-relaxed text-white placeholder-white/30 resize-none outline-none select-text"
            />

            <div className="flex items-center justify-between pt-3 border-t border-white/[0.08]">
              <span className="text-[11px] text-white/45">
                ارسال سریع: <code className="text-blue-400">Ctrl+Enter</code>
              </span>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setShowFullComposer(false)}
                  className="px-4 py-1.5 rounded-lg bg-white/[0.05] text-white/75 text-[12px]"
                >
                  بستن
                </button>
                <button
                  type="button"
                  disabled={isSendingChat || !chatInput.trim()}
                  onClick={() => {
                    setShowFullComposer(false);
                    onSendChat();
                  }}
                  className="px-4 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[12px] font-semibold disabled:opacity-40"
                >
                  ارسال
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

export default AntigravityChatPane;
