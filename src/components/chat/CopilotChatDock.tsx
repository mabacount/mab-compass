import { useEffect, useRef, useState, type ReactNode, type FC, type DragEvent } from "react";
import {
  Check,
  ChevronsUpDown,
  Database,
  Expand,
  HelpCircle,
  Maximize2,
  MessageSquarePlus,
  Minimize2,
  Paperclip,
  Plus,
  RefreshCw,
  Send,
  Settings,
  Sparkles,
  X,
} from "lucide-react";
import { ChatMessage, ChatSession } from "../../lib/tauri_api";
import { formatShortTime } from "../../lib/formatters";

export interface RawEventItem {
  id: number;
  source_type: string;
  created_at: string;
  snippet: string;
}

export interface CopilotChatDockProps {
  channel?: "COPILOT" | "VAULT";
  activeSession?: ChatSession;
  sessions: ChatSession[];
  onSwitchSession: (sessionId: string) => void;
  onNewChat: () => void;
  chatHistory: ChatMessage[];
  chatInput: string;
  onChangeChatInput: (value: string) => void;
  isSendingChat: boolean;
  onSendChat: () => void;
  onSendFiles?: (files: FileList) => void;
  isFullChatMode?: boolean;
  onToggleFullChatMode?: (full: boolean) => void;
  rawEventsCount?: number;
  recentRawEvents?: RawEventItem[];
  retryingMessageId?: number | null;
  onRetrySystemMessage?: (systemMessageId: number) => void;
  onOpenSettings?: () => void;
  activeStepId?: string;
  firstHorizonId?: string;
  projectName?: string;
  extraHeaderControls?: ReactNode;
  emptyStateTitle?: string;
  emptyStateDescription?: ReactNode;
  promptStarters?: { label: string; text: string; variant?: "default" | "warning" | "success" }[];
  className?: string;
}

export const CopilotChatDock: FC<CopilotChatDockProps> = ({
  channel = "COPILOT",
  activeSession,
  sessions,
  onSwitchSession,
  onNewChat,
  chatHistory,
  chatInput,
  onChangeChatInput,
  isSendingChat,
  onSendChat,
  onSendFiles,
  isFullChatMode = false,
  onToggleFullChatMode,
  rawEventsCount,
  recentRawEvents,
  retryingMessageId,
  onRetrySystemMessage,
  onOpenSettings,
  activeStepId,
  firstHorizonId,
  projectName = "پروژه ماب",
  extraHeaderControls,
  emptyStateTitle,
  emptyStateDescription,
  promptStarters,
  className = "",
}) => {
  const chatEndRef = useRef<HTMLDivElement | null>(null);
  const chatTextareaRef = useRef<HTMLTextAreaElement | null>(null);
  const modalTextareaRef = useRef<HTMLTextAreaElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const [isDraggingFile, setIsDraggingFile] = useState(false);
  const [showMemoryDrawer, setShowMemoryDrawer] = useState(false);
  const [isComposerExpanded, setIsComposerExpanded] = useState(false);
  const [showComposerModal, setShowComposerModal] = useState(false);
  const [isSessionDropdownOpen, setIsSessionDropdownOpen] = useState(false);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatHistory.length, isSendingChat, isFullChatMode]);

  useEffect(() => {
    if (showComposerModal) {
      window.setTimeout(() => modalTextareaRef.current?.focus(), 0);
    }
  }, [showComposerModal]);

  const insertPromptStarter = (starter: string) => {
    const nextVal = chatInput.trim() ? `${chatInput.trimEnd()}\n${starter}` : starter;
    onChangeChatInput(nextVal);
    window.setTimeout(() => chatTextareaRef.current?.focus(), 0);
  };

  const handleAnswerQuestionClick = (question: string, index: number) => {
    insertPromptStarter(`پاسخ سوال ${index + 1} («${question}»):\n`);
  };

  const handleDrop = (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDraggingFile(false);
    if (isSendingChat || !onSendFiles) return;
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      onSendFiles(e.dataTransfer.files);
    }
  };

  // Default prompt starter chips if none provided
  const defaultStarters = [
    { label: "تغییر مسیر", text: "مسیر پیش‌رو را طوری اصلاح کن که گام اول این باشد: " },
    ...(activeStepId
      ? [
          {
            label: `ویرایش [${activeStepId}]`,
            text: `گام فعلی [${activeStepId}] را این‌طور اصلاح کن: `,
          },
        ]
      : []),
    ...(firstHorizonId
      ? [
          {
            label: `اولویت به [${firstHorizonId}]`,
            text: `گام [${firstHorizonId}] را همین الان بگذار گام اول مسیر پیش‌رو`,
            variant: "success" as const,
          },
        ]
      : []),
    ...(activeStepId
      ? [
          {
            label: "حذف تسک",
            text: `تسک [${activeStepId}] اشتباه یا تکراری است، حذفش کن.`,
            variant: "warning" as const,
          },
        ]
      : []),
  ];

  const activePromptStarters = promptStarters || defaultStarters;
  const isVault = channel === "VAULT";

  return (
    <div
      onDragOver={(e) => {
        if (!onSendFiles) return;
        e.preventDefault();
        setIsDraggingFile(true);
      }}
      onDragLeave={() => setIsDraggingFile(false)}
      onDrop={handleDrop}
      className={`mab-card flex flex-col flex-1 overflow-hidden transition relative ${
        isDraggingFile ? "ring-2 ring-blue-500/50 border-blue-500" : ""
      } ${className}`}
    >
      {/* Drag & drop overlay hint */}
      {isDraggingFile && (
        <div className="absolute inset-0 bg-blue-500/10 backdrop-blur-sm z-30 flex items-center justify-center pointer-events-none">
          <div className="bg-mab-surface border border-blue-500/40 rounded-xl px-5 py-3 shadow-xl flex items-center gap-2 text-white font-medium text-sm">
            <Paperclip className="w-4 h-4 text-blue-400" />
            <span>فایل‌ها را رها کنید تا بلافاصله به حافظه و همیار منتقل شوند</span>
          </div>
        </div>
      )}

      {/* Header of Copilot Panel */}
      <div className="h-[46px] px-3.5 border-b border-mab-border flex items-center justify-between gap-2 bg-mab-sidebar shrink-0">
        {/* Custom Session Selector Dropdown */}
        <div className="relative flex items-center gap-2 min-w-0">
          <div
            className={`w-6 h-6 rounded-lg flex items-center justify-center shrink-0 border ${
              isVault
                ? "bg-purple-500/15 border-purple-500/30 text-purple-400"
                : "bg-blue-500/15 border-blue-500/30 text-blue-400"
            }`}
          >
            <Sparkles className="w-3.5 h-3.5" />
          </div>

          <div className="relative">
            <button
              type="button"
              onClick={() => setIsSessionDropdownOpen((v) => !v)}
              className="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-white/[0.03] hover:bg-white/[0.07] border border-mab-border text-[12px] font-medium text-mab-text-primary transition max-w-[190px] sm:max-w-[240px]"
            >
              <span className="truncate">
                {activeSession?.title || (isVault ? "گفتگوی دانش" : "گفتگوی همیار")}
              </span>
              {activeSession && (
                <span className="font-timer text-[10.5px] text-mab-text-muted shrink-0">
                  ({activeSession.message_count})
                </span>
              )}
              <ChevronsUpDown className="w-3 h-3 text-mab-text-secondary shrink-0 mr-0.5" />
            </button>

            {isSessionDropdownOpen && (
              <>
                <div
                  className="fixed inset-0 z-40"
                  onClick={() => setIsSessionDropdownOpen(false)}
                />
                <div className="absolute top-full right-0 mt-1.5 w-64 bg-mab-elevated border border-mab-border rounded-xl shadow-2xl z-50 p-1 space-y-0.5 backdrop-blur-md">
                  <div className="px-2.5 py-1.5 text-[11px] font-semibold text-mab-text-muted border-b border-mab-border flex items-center justify-between">
                    <span>{isVault ? "گفتگوهای آرشیو دانش" : "تاریخچه گفتگوهای این پروژه"}</span>
                    <button
                      type="button"
                      onClick={() => {
                        setIsSessionDropdownOpen(false);
                        onNewChat();
                      }}
                      className="text-blue-400 hover:text-blue-300 text-[10.5px] flex items-center gap-1 font-medium"
                    >
                      <Plus className="w-3 h-3" />
                      <span>چت جدید</span>
                    </button>
                  </div>
                  <div className="max-h-52 overflow-y-auto space-y-0.5 py-1">
                    {sessions.map((s) => {
                      const isSelected = s.id === activeSession?.id;
                      return (
                        <button
                          key={s.id}
                          type="button"
                          onClick={() => {
                            onSwitchSession(s.id);
                            setIsSessionDropdownOpen(false);
                          }}
                          className={`w-full text-right px-2.5 py-1.5 rounded-lg text-[12px] flex items-center justify-between gap-1.5 transition ${
                            isSelected
                              ? isVault
                                ? "bg-purple-500/15 text-purple-300 font-medium"
                                : "bg-blue-500/15 text-blue-300 font-medium"
                              : "text-mab-text-secondary hover:bg-mab-hover hover:text-mab-text-primary"
                          }`}
                        >
                          <span className="truncate">{s.title}</span>
                          <div className="flex items-center gap-1.5 shrink-0">
                            <span className="font-timer text-[10px] opacity-60">
                              {s.message_count}
                            </span>
                            {isSelected && (
                              <Check
                                className={`w-3 h-3 ${
                                  isVault ? "text-purple-400" : "text-blue-400"
                                } stroke-[2.5]`}
                              />
                            )}
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </div>
              </>
            )}
          </div>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-1.5 shrink-0">
          <button
            type="button"
            onClick={onNewChat}
            className={`px-2.5 py-1 rounded-full border text-[11px] font-medium flex items-center gap-1 transition ${
              isVault
                ? "bg-purple-500/10 hover:bg-purple-500/20 border-purple-500/25 text-purple-400"
                : "bg-blue-500/10 hover:bg-blue-500/20 border-blue-500/25 text-blue-400"
            }`}
            title="شروع گفتگوی جدید"
          >
            <MessageSquarePlus className="w-3 h-3" />
            <span className="hidden sm:inline">چت جدید</span>
          </button>

          {rawEventsCount !== undefined && (
            <button
              type="button"
              onClick={() => setShowMemoryDrawer((v) => !v)}
              className={`px-2.5 py-1 rounded-full border text-[11px] font-medium flex items-center gap-1 transition ${
                showMemoryDrawer
                  ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-400"
                  : "bg-white/[0.03] border-mab-border text-mab-text-secondary hover:text-white"
              }`}
              title="مشاهده رویدادها و اسناد ثبت‌شده در پایگاه SQLite"
            >
              <Database className="w-3 h-3" />
              <span className="font-timer">حافظه ({rawEventsCount})</span>
            </button>
          )}

          {onToggleFullChatMode && (
            <button
              type="button"
              onClick={() => onToggleFullChatMode(!isFullChatMode)}
              className={`px-2 py-1 rounded-full border text-[11px] font-medium flex items-center gap-1 transition ${
                isFullChatMode
                  ? "bg-white/[0.1] border-white/[0.2] text-white"
                  : "bg-white/[0.03] border-mab-border text-mab-text-secondary hover:text-white"
              }`}
              title={isFullChatMode ? "بازگشت به نمای استاندارد" : "حالت چت کامل"}
            >
              {isFullChatMode ? (
                <Minimize2 className="w-3 h-3" />
              ) : (
                <Maximize2 className="w-3 h-3" />
              )}
            </button>
          )}

          {extraHeaderControls}
        </div>
      </div>

      {/* Live Memory Drawer */}
      {showMemoryDrawer && rawEventsCount !== undefined && (
        <div className="p-3 border-b border-mab-border bg-mab-base/80 space-y-2 max-h-52 overflow-y-auto shrink-0">
          <div className="flex items-center justify-between text-[11.5px]">
            <span className="font-semibold text-emerald-400 flex items-center gap-1.5">
              <Database className="w-3.5 h-3.5" />
              <span>حافظه بلادرنگ پروژه ({rawEventsCount} رویداد SQLite)</span>
            </span>
            <span className="text-mab-text-muted text-[10.5px]">
              ثبت فوری بدون نیاز به پردازش مدل
            </span>
          </div>
          {(!recentRawEvents || recentRawEvents.length === 0) ? (
            <p className="text-[12px] text-mab-text-muted">
              هنوز پیامی در حافظه این پروژه ثبت نشده است.
            </p>
          ) : (
            <div className="space-y-1.5">
              {recentRawEvents.slice(0, 6).map((ev) => (
                <div
                  key={ev.id}
                  className="p-2.5 rounded-lg bg-mab-surface border border-mab-border text-[12px] space-y-0.5 select-text hover:bg-mab-hover transition"
                >
                  <div className="flex items-center justify-between text-[10.5px] text-mab-text-muted">
                    <span className="font-mono text-blue-400 font-semibold">
                      #{ev.id} • {ev.source_type}
                    </span>
                    <span className="font-timer">{formatShortTime(ev.created_at)}</span>
                  </div>
                  <p className="text-mab-text-primary line-clamp-2 text-[12px]">
                    {ev.snippet}
                  </p>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Chat Messages Feed */}
      <div className="flex-1 p-3.5 sm:p-4 overflow-y-auto space-y-3.5 select-text min-h-0">
        {chatHistory.length === 0 && (
          <div className="p-5 rounded-2xl bg-white/[0.02] border border-mab-border text-[13px] text-mab-text-secondary leading-[1.8] space-y-3">
            <div className="flex items-center gap-2 text-white font-bold text-[14px]">
              <Sparkles
                className={`w-4 h-4 ${isVault ? "text-purple-400" : "text-blue-400"}`}
              />
              <span>
                {emptyStateTitle ||
                  (isVault
                    ? "تحلیل اسناد و استخراج خودکار گراف تسک‌ها"
                    : "همیار هوشمند سنتز مسیر پیش‌رو")}
              </span>
            </div>
            {emptyStateDescription || (
              <>
                <p>
                  به محض ارسال هر پیام، دستور یا فایل (PDF، کد، متون و گزارش‌ها)، اطلاعات شما{" "}
                  <strong className="text-emerald-400">بلافاصله در پایگاه داده محلی ذخیره</strong>{" "}
                  می‌شود.
                </p>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-1 text-[12px]">
                  <div className="p-2.5 rounded-xl bg-mab-surface border border-mab-border">
                    <strong className="text-white block mb-0.5">اصلاح مسیر پروژه:</strong>
                    می‌توانی اولویت تسک‌ها را تغییر دهی یا تسک جدید بسازی.
                  </div>
                  <div className="p-2.5 rounded-xl bg-mab-surface border border-mab-border">
                    <strong className="text-white block mb-0.5">حل موانع و تدقیق:</strong>
                    همیار با سوالات چندنوبتی گام بعدی را سبک و دقیق می‌کند.
                  </div>
                </div>
              </>
            )}
          </div>
        )}

        {chatHistory.map((m) => (
          <div
            key={m.id}
            className={`p-3.5 sm:p-4 rounded-2xl text-[13.5px] leading-[1.8] space-y-2.5 transition ${
              m.sender === "USER"
                ? "bg-blue-500/[0.08] border border-blue-500/20 rounded-tr-sm mr-2 sm:mr-5"
                : m.sender === "MAB_FOCUS_SYNC"
                ? "bg-emerald-500/[0.08] border border-emerald-500/20 rounded-2xl"
                : m.sender === "SYSTEM"
                ? "bg-amber-500/[0.08] border border-amber-500/25 ml-2 sm:ml-5 rounded-tl-sm"
                : "bg-mab-surface border border-mab-border ml-2 sm:ml-5 rounded-tl-sm"
            }`}
          >
            {/* Sender header & timestamp */}
            <div className="flex items-center justify-between text-[11px] text-mab-text-muted">
              <span className="font-semibold flex items-center gap-1.5">
                {m.sender === "USER" ? (
                  <span className="text-blue-400">شما (ثبت فوری در حافظه)</span>
                ) : m.sender === "MAB_FOCUS_SYNC" ? (
                  <span className="text-emerald-400">سینک خودکار از Mab-Focus</span>
                ) : m.sender === "SYSTEM" ? (
                  <span className="text-amber-400">اعلان سیستم • ذخیره در حافظه</span>
                ) : (
                  <span className="text-purple-400 flex items-center gap-1">
                    <Sparkles className="w-3 h-3" />
                    <span>{isVault ? "همیار اسناد و دانش" : "همیار هوشمند پروژه"}</span>
                  </span>
                )}
              </span>
              <span className="font-timer">{formatShortTime(m.created_at)}</span>
            </div>

            {/* Message Body */}
            <p className="text-mab-text-primary whitespace-pre-wrap break-words leading-relaxed font-sans">
              {m.message_text}
            </p>

            {/* System retry button */}
            {m.sender === "SYSTEM" && onRetrySystemMessage && (
              <div className="pt-1 flex items-center gap-2 flex-wrap">
                <button
                  type="button"
                  disabled={isSendingChat || retryingMessageId !== null}
                  onClick={() => onRetrySystemMessage(m.id)}
                  className="px-3 py-1 rounded-full bg-blue-500 hover:bg-blue-600 text-white text-[11.5px] font-semibold inline-flex items-center gap-1.5 transition disabled:opacity-50"
                >
                  <RefreshCw
                    className={`w-3 h-3 ${
                      retryingMessageId === m.id ? "animate-spin" : ""
                    }`}
                  />
                  <span>
                    {retryingMessageId === m.id
                      ? "در حال ارسال مجدد به مدل..."
                      : "ارسال مجدد به مدل"}
                  </span>
                </button>
                {onOpenSettings && (
                  <button
                    type="button"
                    onClick={onOpenSettings}
                    className="px-3 py-1 rounded-full bg-white/[0.05] border border-white/[0.1] text-amber-400 text-[11.5px] font-semibold inline-flex items-center gap-1.5 hover:bg-white/[0.08] transition"
                  >
                    <Settings className="w-3 h-3" />
                    <span>تنظیمات مدل هوش مصنوعی</span>
                  </button>
                )}
              </div>
            )}

            {/* Clarifying Questions */}
            {m.clarifying_questions && m.clarifying_questions.length > 0 && (
              <div className="p-3 rounded-xl bg-amber-500/[0.07] border border-amber-500/25 space-y-2">
                <div className="flex items-center gap-1.5 text-[11.5px] font-bold text-amber-400">
                  <HelpCircle className="w-3.5 h-3.5 shrink-0" />
                  <span>سوالات همیار برای تدقیق گام پیش‌رو (کلیک کنید):</span>
                </div>
                <div className="space-y-1.5">
                  {m.clarifying_questions.map((q, idx) => (
                    <button
                      key={`${m.id}-cq-${idx}`}
                      type="button"
                      onClick={() => handleAnswerQuestionClick(q, idx)}
                      className="w-full text-right p-2.5 rounded-lg bg-mab-surface border border-mab-border hover:border-amber-500/40 text-[12.5px] text-mab-text-primary transition flex items-start gap-2 group"
                    >
                      <span className="font-mono text-amber-400 font-bold shrink-0 text-[11px] mt-0.5">
                        {idx + 1}.
                      </span>
                      <span className="group-hover:text-white transition">{q}</span>
                    </button>
                  ))}
                </div>
              </div>
            )}

            {/* Mutation Badge */}
            {m.applied_mutations_badge && (
              <div className="pt-0.5 flex items-center gap-1.5 flex-wrap">
                <span className="px-2.5 py-0.5 rounded-md text-[10.5px] font-mono font-semibold bg-emerald-500/15 border border-emerald-500/30 text-emerald-300">
                  ✓ {m.applied_mutations_badge}
                </span>
              </div>
            )}
          </div>
        ))}

        {isSendingChat && (
          <div className="p-3.5 rounded-2xl bg-mab-surface border border-purple-500/30 ml-4 flex items-center gap-2.5 text-[12.5px] text-mab-text-secondary">
            <RefreshCw className="w-3.5 h-3.5 text-purple-400 animate-spin shrink-0" />
            <span>پیام در حافظه ثبت شد • همیار در حال تحلیل و سنتز پاسخ است...</span>
          </div>
        )}
        <div ref={chatEndRef} />
      </div>

      {/* Anytype Unified AI Composer */}
      <div className="p-3 border-t border-mab-border bg-mab-sidebar/70 space-y-2 shrink-0">
        {/* Quick prompt shortcuts */}
        {activePromptStarters.length > 0 && (
          <div className="flex items-center gap-1.5 overflow-x-auto pb-1 text-[11px] select-none scrollbar-none">
            <span className="text-mab-text-muted text-[10.5px] shrink-0 ml-1">
              پیشنهادهای سریع:
            </span>
            {activePromptStarters.map((item, idx) => (
              <button
                key={idx}
                type="button"
                onClick={() => insertPromptStarter(item.text)}
                className={`px-2.5 py-0.5 rounded-full border text-white/75 hover:text-white transition shrink-0 ${
                  item.variant === "success"
                    ? "bg-emerald-500/10 border-emerald-500/25 hover:border-emerald-500/40 hover:bg-emerald-500/20"
                    : item.variant === "warning"
                    ? "bg-rose-500/10 border-rose-500/25 hover:border-rose-500/40 hover:bg-rose-500/20 text-rose-300"
                    : "bg-white/[0.04] border-mab-border hover:border-blue-500/40 hover:bg-white/[0.08]"
                }`}
              >
                {item.label}
              </button>
            ))}
          </div>
        )}

        {/* Input box */}
        <div className="rounded-2xl bg-[#16171B] border border-white/[0.09] focus-within:border-blue-500/50 focus-within:ring-2 focus-within:ring-blue-500/20 p-2.5 shadow-sm transition-all space-y-2">
          <textarea
            ref={chatTextareaRef}
            value={chatInput}
            onChange={(e) => onChangeChatInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                onSendChat();
              }
            }}
            placeholder={
              isVault
                ? "سوال یا دستور برای استخراج از اسناد بنویسید... (Enter ارسال)"
                : "پیام یا دستور بنویس... (Enter ارسال • Shift+Enter خط جدید)"
            }
            rows={isComposerExpanded ? 7 : 3}
            className={`w-full bg-transparent text-[13.5px] leading-relaxed text-white placeholder-white/30 resize-none outline-none select-text transition-all ${
              isComposerExpanded ? "min-h-[160px] max-h-[40vh]" : "min-h-[70px] max-h-[180px]"
            }`}
          />

          {/* Composer Toolbar */}
          <div className="flex items-center justify-between gap-2 pt-1 border-t border-white/[0.04]">
            <div className="flex items-center gap-1.5">
              {onSendFiles && (
                <>
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
                    title="ضمیمه هر نوع فایل (PDF، سورس‌کد، متن)"
                    className="p-1.5 rounded-lg bg-white/[0.03] hover:bg-white/[0.08] text-white/60 hover:text-white transition disabled:opacity-40"
                  >
                    <Paperclip className="w-3.5 h-3.5 text-blue-400" />
                  </button>
                </>
              )}

              <button
                type="button"
                onClick={() => setIsComposerExpanded((v) => !v)}
                title={isComposerExpanded ? "کوچک کردن کادر" : "بزرگ کردن کادر"}
                className={`p-1.5 rounded-lg transition ${
                  isComposerExpanded
                    ? "bg-blue-500/15 text-blue-400"
                    : "bg-white/[0.03] hover:bg-white/[0.08] text-white/60 hover:text-white"
                }`}
              >
                <ChevronsUpDown className="w-3.5 h-3.5" />
              </button>

              <button
                type="button"
                onClick={() => setShowComposerModal(true)}
                title="ویرایشگر تمام‌صفحه"
                className="p-1.5 rounded-lg bg-white/[0.03] hover:bg-white/[0.08] text-white/60 hover:text-white transition"
              >
                <Expand className="w-3.5 h-3.5" />
              </button>
            </div>

            <button
              onClick={onSendChat}
              disabled={isSendingChat || !chatInput.trim()}
              className={`px-3.5 py-1.5 rounded-xl text-white text-[12px] font-semibold flex items-center gap-1.5 shadow-sm transition disabled:opacity-40 ${
                isVault
                  ? "bg-purple-600 hover:bg-purple-500"
                  : "bg-blue-600 hover:bg-blue-500"
              }`}
            >
              <Send className="w-3.5 h-3.5" />
              <span>{isSendingChat ? "ارسال..." : "ارسال"}</span>
            </button>
          </div>
        </div>
      </div>

      {/* Full-Screen Composer Modal */}
      {showComposerModal && (
        <div
          className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex items-center justify-center p-4"
          onClick={() => setShowComposerModal(false)}
        >
          <div
            className="bg-mab-surface border border-mab-border w-full max-w-3xl p-5 space-y-4 rounded-2xl shadow-2xl flex flex-col max-h-[88vh]"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between gap-2 border-b border-mab-border pb-3">
              <div className="flex items-center gap-2">
                <Sparkles
                  className={`w-4 h-4 ${isVault ? "text-purple-400" : "text-blue-400"}`}
                />
                <h3 className="text-[14px] font-bold text-white">
                  ویرایشگر گسترده همیار ({projectName})
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setShowComposerModal(false)}
                className="p-1.5 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] text-white/60 hover:text-white transition"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <textarea
              ref={modalTextareaRef}
              value={chatInput}
              onChange={(e) => onChangeChatInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  setShowComposerModal(false);
                } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                  e.preventDefault();
                  setShowComposerModal(false);
                  onSendChat();
                }
              }}
              placeholder="متن کامل پیام یا سند خود را با خیال راحت اینجا بنویسید..."
              className="w-full flex-1 min-h-[300px] sm:min-h-[380px] bg-transparent p-3 text-[14px] leading-relaxed text-white placeholder-white/30 resize-none outline-none select-text"
            />

            <div className="flex items-center justify-between gap-2 pt-3 border-t border-mab-border flex-wrap">
              <span className="text-[11.5px] text-mab-text-muted">
                برای ارسال سریع: <code className="text-blue-400">Ctrl+Enter</code> • برای خروج:{" "}
                <code className="text-blue-400">Esc</code>
              </span>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setShowComposerModal(false)}
                  className="px-4 py-1.5 rounded-lg bg-white/[0.05] hover:bg-white/[0.08] text-white/70 text-[12px] font-medium transition"
                >
                  انصراف
                </button>
                <button
                  type="button"
                  disabled={isSendingChat || !chatInput.trim()}
                  onClick={() => {
                    setShowComposerModal(false);
                    onSendChat();
                  }}
                  className={`px-5 py-1.5 rounded-lg text-white text-[12.5px] font-semibold inline-flex items-center gap-1.5 transition disabled:opacity-50 ${
                    isVault
                      ? "bg-purple-600 hover:bg-purple-500"
                      : "bg-blue-600 hover:bg-blue-500"
                  }`}
                >
                  <Send className="w-3.5 h-3.5" />
                  <span>ارسال به همیار</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default CopilotChatDock;
