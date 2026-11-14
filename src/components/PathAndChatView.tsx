import React, { useEffect, useRef, useState } from "react";
import {
  BookOpen,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronsUpDown,
  Database,
  Edit3,
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
  ShieldAlert,
  Sparkles,
  X,
  Zap,
} from "lucide-react";
import { DashboardState } from "../lib/tauri_api";
import { formatPomos, formatShortTime } from "../lib/formatters";

interface PathAndChatViewProps {
  dash: DashboardState;
  checkedGates: Record<number, boolean>;
  onToggleGate: (gateId: number, checked: boolean) => void;
  allGatesPassed: boolean;
  onQuickComplete: (taskId: string, pomosSpent: number) => void;
  chatInput: string;
  onChangeChatInput: (value: string) => void;
  isSendingChat: boolean;
  onSendChat: () => void;
  onSendCopilotFiles: (files: FileList) => void;
  isFullChatMode: boolean;
  onToggleFullChatMode: (full: boolean) => void;
  onNewChat: () => void;
  onSwitchSession: (sessionId: string) => void;
  onOpenSettings: () => void;
  retryingMessageId: number | null;
  onRetrySystemMessage: (systemMessageId: number) => void;
}

export function PathAndChatView({
  dash,
  checkedGates,
  onToggleGate,
  allGatesPassed,
  onQuickComplete,
  chatInput,
  onChangeChatInput,
  isSendingChat,
  onSendChat,
  onSendCopilotFiles,
  isFullChatMode,
  onToggleFullChatMode,
  onNewChat,
  onSwitchSession,
  onOpenSettings,
  retryingMessageId,
  onRetrySystemMessage,
}: PathAndChatViewProps) {
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
  }, [dash.chat_history.length, isSendingChat, isFullChatMode]);

  useEffect(() => {
    if (showComposerModal) {
      window.setTimeout(() => modalTextareaRef.current?.focus(), 0);
    }
  }, [showComposerModal]);

  const activeCategory = dash.next_step_task?.category;
  const relevantSops = activeCategory
    ? dash.active_sops.filter(
        (s) => s.category === activeCategory || s.category === "GENERAL"
      )
    : dash.active_sops.slice(0, 3);

  const copilotSessions = dash.chat_sessions.filter((s) => s.channel !== "VAULT");
  const activeSession =
    copilotSessions.find((s) => s.id === dash.active_copilot_session_id) ??
    copilotSessions[0];

  const insertPromptStarter = (starter: string) => {
    const nextVal = chatInput.trim() ? `${chatInput.trimEnd()}\n${starter}` : starter;
    onChangeChatInput(nextVal);
    window.setTimeout(() => chatTextareaRef.current?.focus(), 0);
  };

  const handleAnswerQuestionClick = (question: string, index: number) => {
    insertPromptStarter(`پاسخ سوال ${index + 1} («${question}»):\n`);
  };

  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDraggingFile(false);
    if (isSendingChat) return;
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      onSendCopilotFiles(e.dataTransfer.files);
    }
  };

  const activeStepId = dash.next_step_task?.id;
  const firstHorizonId = dash.horizon_tasks[0]?.id;

  /* -------------------------------------------------------------------------- */
  /* Anytype Copilot Chat Dock Component                                        */
  /* -------------------------------------------------------------------------- */
  const chatDock = (
    <div
      onDragOver={(e) => {
        e.preventDefault();
        setIsDraggingFile(true);
      }}
      onDragLeave={() => setIsDraggingFile(false)}
      onDrop={handleDrop}
      className={`mab-card flex flex-col min-h-[460px] lg:min-h-0 flex-1 overflow-hidden transition relative ${
        isDraggingFile ? "ring-2 ring-blue-500/50 border-blue-500" : ""
      }`}
    >
      {/* Drag & drop overlay hint */}
      {isDraggingFile && (
        <div className="absolute inset-0 bg-blue-500/10 backdrop-blur-sm z-30 flex items-center justify-center pointer-events-none">
          <div className="bg-[#18191E] border border-blue-500/40 rounded-xl px-5 py-3 shadow-xl flex items-center gap-2 text-white font-medium text-sm">
            <Paperclip className="w-4 h-4 text-blue-400" />
            <span>فایل‌ها را رها کنید تا بلافاصله به حافظه و همیار منتقل شوند</span>
          </div>
        </div>
      )}

      {/* Header of Copilot Panel */}
      <div className="h-[46px] px-3.5 border-b border-white/[0.06] flex items-center justify-between gap-2 bg-[#131418] shrink-0">
        {/* Custom Session Selector Dropdown */}
        <div className="relative flex items-center gap-2 min-w-0">
          <div className="w-5 h-5 rounded-md bg-purple-500/15 border border-purple-500/30 flex items-center justify-center shrink-0">
            <Sparkles className="w-3 h-3 text-purple-400" />
          </div>

          <div className="relative">
            <button
              type="button"
              onClick={() => setIsSessionDropdownOpen((v) => !v)}
              className="flex items-center gap-1.5 px-2 py-1 rounded-md bg-white/[0.03] hover:bg-white/[0.07] border border-white/[0.07] text-[12px] font-medium text-white transition max-w-[190px] sm:max-w-[240px]"
            >
              <span className="truncate">
                {activeSession?.title || "گفتگوی همیار"}
              </span>
              {activeSession && (
                <span className="font-timer text-[10px] text-white/50 shrink-0">
                  ({activeSession.message_count})
                </span>
              )}
              <ChevronDown className="w-3 h-3 text-white/50 shrink-0 mr-0.5" />
            </button>

            {isSessionDropdownOpen && (
              <>
                <div
                  className="fixed inset-0 z-40"
                  onClick={() => setIsSessionDropdownOpen(false)}
                />
                <div className="absolute top-full right-0 mt-1.5 w-64 bg-[#1E1F25] border border-white/[0.1] rounded-xl shadow-mab-dropdown z-50 p-1 space-y-0.5">
                  <div className="px-2.5 py-1.5 text-[11px] font-semibold text-white/50 border-b border-white/[0.06] flex items-center justify-between">
                    <span>تاریخچه گفتگوهای این پروژه</span>
                    <button
                      type="button"
                      onClick={() => {
                        setIsSessionDropdownOpen(false);
                        onNewChat();
                      }}
                      className="text-blue-400 hover:text-blue-300 text-[10.5px] flex items-center gap-1"
                    >
                      <Plus className="w-3 h-3" />
                      <span>چت جدید</span>
                    </button>
                  </div>
                  <div className="max-h-52 overflow-y-auto space-y-0.5 py-1">
                    {copilotSessions.map((s) => {
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
                              ? "bg-blue-500/15 text-blue-300 font-medium"
                              : "text-mab-text-secondary hover:bg-white/[0.05] hover:text-white"
                          }`}
                        >
                          <span className="truncate">{s.title}</span>
                          <div className="flex items-center gap-1.5 shrink-0">
                            <span className="font-timer text-[10px] opacity-60">
                              {s.message_count}
                            </span>
                            {isSelected && <Check className="w-3 h-3 text-blue-400" />}
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
            className="px-2.5 py-1 rounded-full bg-blue-500/10 hover:bg-blue-500/20 border border-blue-500/25 text-blue-400 text-[11px] font-medium flex items-center gap-1 transition"
            title="شروع گفتگوی جدید"
          >
            <MessageSquarePlus className="w-3 h-3" />
            <span className="hidden sm:inline">چت جدید</span>
          </button>

          <button
            type="button"
            onClick={() => setShowMemoryDrawer((v) => !v)}
            className={`px-2.5 py-1 rounded-full border text-[11px] font-medium flex items-center gap-1 transition ${
              showMemoryDrawer
                ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-400"
                : "bg-white/[0.03] border-white/[0.08] text-mab-text-secondary hover:text-white"
            }`}
            title="مشاهده رویدادها و اسناد ثبت‌شده در پایگاه SQLite"
          >
            <Database className="w-3 h-3" />
            <span className="font-timer">حافظه ({dash.raw_events_count})</span>
          </button>

          <button
            type="button"
            onClick={() => onToggleFullChatMode(!isFullChatMode)}
            className={`px-2 py-1 rounded-full border text-[11px] font-medium flex items-center gap-1 transition ${
              isFullChatMode
                ? "bg-white/[0.1] border-white/[0.2] text-white"
                : "bg-white/[0.03] border-white/[0.08] text-mab-text-secondary hover:text-white"
            }`}
            title={isFullChatMode ? "بازگشت به نمای دو ستونه" : "حالت چت تمام‌صفحه"}
          >
            {isFullChatMode ? (
              <Minimize2 className="w-3 h-3" />
            ) : (
              <Maximize2 className="w-3 h-3" />
            )}
          </button>
        </div>
      </div>

      {/* Live Memory Drawer */}
      {showMemoryDrawer && (
        <div className="p-3 border-b border-white/[0.08] bg-[#121316] space-y-2 max-h-52 overflow-y-auto shrink-0">
          <div className="flex items-center justify-between text-[11.5px]">
            <span className="font-semibold text-emerald-400 flex items-center gap-1.5">
              <Database className="w-3.5 h-3.5" />
              <span>حافظه بلادرنگ پروژه ({dash.raw_events_count} رویداد SQLite)</span>
            </span>
            <span className="text-white/40 text-[10.5px]">ثبت فوری بدون نیاز به پردازش مدل</span>
          </div>
          {dash.recent_raw_events.length === 0 ? (
            <p className="text-[12px] text-mab-text-muted">
              هنوز پیامی در حافظه این پروژه ثبت نشده است.
            </p>
          ) : (
            <div className="space-y-1.5">
              {dash.recent_raw_events.slice(0, 6).map((ev) => (
                <div
                  key={ev.id}
                  className="p-2.5 rounded-lg bg-white/[0.03] border border-white/[0.06] text-[12px] space-y-0.5 select-text hover:bg-white/[0.05] transition"
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
        {dash.chat_history.length === 0 && (
          <div className="p-5 rounded-2xl bg-white/[0.02] border border-white/[0.06] text-[13px] text-mab-text-secondary leading-[1.8] space-y-3">
            <div className="flex items-center gap-2 text-white font-bold text-[14px]">
              <Sparkles className="w-4 h-4 text-purple-400" />
              <span>همیار هوشمند سنتز مسیر پیش‌رو</span>
            </div>
            <p>
              به محض ارسال هر پیام، دستور یا فایل (PDF، کد، متون و گزارش‌ها)، اطلاعات شما{" "}
              <strong className="text-emerald-400">بلافاصله در پایگاه داده محلی ذخیره</strong>{" "}
              می‌شود.
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-1 text-[12px]">
              <div className="p-2.5 rounded-xl bg-white/[0.02] border border-white/[0.06]">
                <strong className="text-white block mb-0.5">اصلاح مسیر پروژه:</strong>
                می‌توانی اولویت تسک‌ها را تغییر دهی یا تسک جدید بسازی.
              </div>
              <div className="p-2.5 rounded-xl bg-white/[0.02] border border-white/[0.06]">
                <strong className="text-white block mb-0.5">حل موانع و تدقیق:</strong>
                همیار با سوالات چندنوبتی گام بعدی را سبک و دقیق می‌کند.
              </div>
            </div>
          </div>
        )}

        {dash.chat_history.map((m) => (
          <div
            key={m.id}
            className={`p-3.5 sm:p-4 rounded-2xl text-[13.5px] leading-[1.8] space-y-2.5 transition ${
              m.sender === "USER"
                ? "bg-blue-500/[0.08] border border-blue-500/20 rounded-tr-sm mr-2 sm:mr-5"
                : m.sender === "MAB_FOCUS_SYNC"
                ? "bg-emerald-500/[0.08] border border-emerald-500/20 rounded-2xl"
                : m.sender === "SYSTEM"
                ? "bg-amber-500/[0.08] border border-amber-500/25 ml-2 sm:ml-5 rounded-tl-sm"
                : "bg-white/[0.03] border border-white/[0.07] ml-2 sm:ml-5 rounded-tl-sm"
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
                    <span>همیار هوشمند پروژه</span>
                  </span>
                )}
              </span>
              <span className="font-timer">{formatShortTime(m.created_at)}</span>
            </div>

            {/* Message Body */}
            <p className="text-mab-text-primary whitespace-pre-wrap break-words leading-relaxed">
              {m.message_text}
            </p>

            {/* System retry button */}
            {m.sender === "SYSTEM" && (
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
                <button
                  type="button"
                  onClick={onOpenSettings}
                  className="px-3 py-1 rounded-full bg-white/[0.05] border border-white/[0.1] text-amber-400 text-[11.5px] font-semibold inline-flex items-center gap-1.5 hover:bg-white/[0.08] transition"
                >
                  <Settings className="w-3 h-3" />
                  <span>تنظیمات مدل هوش مصنوعی</span>
                </button>
              </div>
            )}

            {/* Clarifying Questions (Anytype interactive suggestion blocks) */}
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
                      className="w-full text-right p-2.5 rounded-lg bg-[#141519] border border-white/[0.07] hover:border-amber-500/40 text-[12.5px] text-mab-text-primary transition flex items-start gap-2 group"
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
          <div className="p-3.5 rounded-2xl bg-white/[0.03] border border-purple-500/30 ml-4 flex items-center gap-2.5 text-[12.5px] text-mab-text-secondary">
            <RefreshCw className="w-3.5 h-3.5 text-purple-400 animate-spin shrink-0" />
            <span>پیام در حافظه ثبت شد • همیار در حال تحلیل و سنتز پاسخ است...</span>
          </div>
        )}
        <div ref={chatEndRef} />
      </div>

      {/* Anytype Unified AI Composer */}
      <div className="p-3 border-t border-white/[0.06] bg-[#121316] space-y-2 shrink-0">
        {/* Quick prompt shortcuts */}
        <div className="flex items-center gap-1.5 overflow-x-auto pb-1 text-[11px] select-none">
          <span className="text-white/40 text-[10.5px] shrink-0 ml-1">اصلاح سریع:</span>
          <button
            type="button"
            onClick={() =>
              insertPromptStarter("مسیر پیش‌رو را طوری اصلاح کن که گام اول این باشد: ")
            }
            className="px-2.5 py-0.5 rounded-full bg-white/[0.04] border border-white/[0.08] hover:border-blue-500/40 hover:bg-white/[0.08] text-white/75 hover:text-white transition shrink-0"
          >
            تغییر مسیر
          </button>
          {activeStepId && (
            <button
              type="button"
              onClick={() =>
                insertPromptStarter(`گام فعلی [${activeStepId}] را این‌طور اصلاح کن: `)
              }
              className="px-2.5 py-0.5 rounded-full bg-white/[0.04] border border-white/[0.08] hover:border-blue-500/40 hover:bg-white/[0.08] text-white/75 hover:text-white transition shrink-0"
            >
              ویرایش [{activeStepId}]
            </button>
          )}
          {firstHorizonId && (
            <button
              type="button"
              onClick={() =>
                insertPromptStarter(
                  `گام [${firstHorizonId}] را همین الان بگذار گام اول مسیر پیش‌رو`
                )
              }
              className="px-2.5 py-0.5 rounded-full bg-white/[0.04] border border-white/[0.08] hover:border-emerald-500/40 hover:bg-white/[0.08] text-white/75 hover:text-white transition shrink-0"
            >
              اولویت به [{firstHorizonId}]
            </button>
          )}
          {activeStepId && (
            <button
              type="button"
              onClick={() =>
                insertPromptStarter(`تسک [${activeStepId}] اشتباه یا تکراری است، حذفش کن.`)
              }
              className="px-2.5 py-0.5 rounded-full bg-white/[0.04] border border-white/[0.08] hover:border-rose-500/40 hover:bg-rose-500/10 text-white/75 hover:text-rose-400 transition shrink-0"
            >
              حذف تسک
            </button>
          )}
        </div>

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
            placeholder="پیام یا دستور بنویس... (Enter ارسال • Shift+Enter خط جدید)"
            rows={isComposerExpanded ? 7 : 3}
            className={`w-full bg-transparent text-[13.5px] leading-relaxed text-white placeholder-white/30 resize-none outline-none select-text transition-all ${
              isComposerExpanded ? "min-h-[160px] max-h-[40vh]" : "min-h-[70px] max-h-[180px]"
            }`}
          />

          {/* Composer Toolbar */}
          <div className="flex items-center justify-between gap-2 pt-1 border-t border-white/[0.04]">
            <div className="flex items-center gap-1.5">
              <input
                ref={fileInputRef}
                type="file"
                multiple
                className="hidden"
                disabled={isSendingChat}
                onChange={(e) => {
                  if (e.target.files && e.target.files.length > 0) {
                    onSendCopilotFiles(e.target.files);
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
              className="px-3.5 py-1.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-[12px] font-semibold flex items-center gap-1.5 shadow-sm transition disabled:opacity-40"
            >
              <Send className="w-3.5 h-3.5" />
              <span>{isSendingChat ? "ارسال..." : "ارسال"}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );

  /* -------------------------------------------------------------------------- */
  /* Full-Screen Composer Modal                                                 */
  /* -------------------------------------------------------------------------- */
  const composerModal = showComposerModal ? (
    <div
      className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex items-center justify-center p-4"
      onClick={() => setShowComposerModal(false)}
    >
      <div
        className="bg-[#18191E] border border-white/[0.1] w-full max-w-3xl p-5 space-y-4 rounded-2xl shadow-mab-modal flex flex-col max-h-[88vh]"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between gap-2 border-b border-white/[0.06] pb-3">
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-purple-400" />
            <h3 className="text-[14px] font-bold text-white">
              ویرایشگر گسترده همیار ({dash.active_project.name})
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

        <div className="flex items-center justify-between gap-2 pt-3 border-t border-white/[0.06] flex-wrap">
          <span className="text-[11.5px] text-white/40">
            برای ارسال سریع: <code className="text-blue-400">Ctrl+Enter</code> • برای خروج: <code className="text-blue-400">Esc</code>
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
              className="px-5 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[12.5px] font-semibold inline-flex items-center gap-1.5 transition disabled:opacity-50"
            >
              <Send className="w-3.5 h-3.5" />
              <span>ارسال به همیار</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  ) : null;

  /* -------------------------------------------------------------------------- */
  /* Full Chat Mode Return                                                      */
  /* -------------------------------------------------------------------------- */
  if (isFullChatMode) {
    return (
      <>
        <div className="flex flex-col gap-2.5 h-full min-h-0">
          {/* Compact Path Bar on top */}
          <div className="mab-card px-4 py-2 border-white/[0.08] flex items-center justify-between gap-3 flex-wrap shrink-0">
            <div className="flex items-center gap-2.5 min-w-0 flex-1">
              <span className="px-2 py-0.5 rounded-full bg-blue-500/15 border border-blue-500/30 text-blue-400 text-[11px] font-bold shrink-0">
                گام کنونی
              </span>
              {dash.next_step_task ? (
                <div className="flex items-center gap-2 min-w-0">
                  <span className="font-mono text-[11.5px] font-bold text-white/70 shrink-0">
                    [{dash.next_step_task.id}]
                  </span>
                  <span className="text-[13px] font-bold text-white truncate">
                    {dash.next_step_task.title}
                  </span>
                  <span className="hidden md:inline text-[11.5px] text-emerald-400 truncate">
                    ← {dash.next_step_task.first_physical_action}
                  </span>
                </div>
              ) : (
                <span className="text-[12.5px] text-mab-text-muted">
                  مسیر پیش‌رو هنوز چیده نشده است
                </span>
              )}
            </div>

            <div className="flex items-center gap-2 shrink-0">
              {dash.next_step_task && (
                <button
                  type="button"
                  disabled={!allGatesPassed}
                  onClick={() =>
                    onQuickComplete(
                      dash.next_step_task!.id,
                      Math.round(dash.next_step_task!.calibrated_pomos) || 1
                    )
                  }
                  className="px-3 py-1 rounded-full bg-emerald-600/90 hover:bg-emerald-500 text-white text-[11px] font-bold transition disabled:opacity-40"
                >
                  ✓ تکمیل گام
                </button>
              )}
              <button
                type="button"
                onClick={() => onToggleFullChatMode(false)}
                className="px-3 py-1 rounded-full bg-white/[0.05] border border-white/[0.1] text-white text-[11px] font-medium hover:bg-white/[0.08] transition"
              >
                نمایش کارت‌های مسیر
              </button>
            </div>
          </div>

          {chatDock}
        </div>
        {composerModal}
      </>
    );
  }

  /* -------------------------------------------------------------------------- */
  /* Main Split View: Left = The Path Ahead | Right = Copilot Chat               */
  /* -------------------------------------------------------------------------- */
  return (
    <>
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-3.5 xl:gap-4 h-full min-h-0">
        {/* Left Side (RTL Start, 7 columns): The Path Ahead Engine */}
        <div className="lg:col-span-6 xl:col-span-7 flex flex-col gap-3.5 overflow-y-auto pl-0.5 min-h-0">
          {/* 1. Pre-flight Guardrail Gate (Anti-repetition Shield) */}
          {dash.active_guardrails.length > 0 && (
            <div className="mab-card p-4 space-y-3 bg-rose-500/[0.04] border-rose-500/20">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 text-rose-400 font-bold text-[13px]">
                  <ShieldAlert className="w-4 h-4 shrink-0" />
                  <span>گیت پیشگیرانه ضدتکرار خطا (بررسی قبل از شروع)</span>
                </div>
                <span className="px-2 py-0.5 rounded-full bg-rose-500/15 text-rose-300 font-mono text-[10.5px]">
                  {Object.values(checkedGates).filter(Boolean).length} از {dash.active_guardrails.length} تایید شده
                </span>
              </div>

              <div className="space-y-2">
                {dash.active_guardrails.map((g) => {
                  const isChecked = !!checkedGates[g.id];
                  return (
                    <div
                      key={g.id}
                      onClick={() => onToggleGate(g.id, !isChecked)}
                      className={`flex items-start gap-3 p-3 rounded-xl border transition cursor-pointer select-none ${
                        isChecked
                          ? "bg-emerald-500/[0.06] border-emerald-500/25"
                          : "bg-[#141519] border-white/[0.06] hover:border-white/[0.12]"
                      }`}
                    >
                      {/* Custom Styled Checkbox */}
                      <div
                        className={`w-4 h-4 rounded-md mt-0.5 flex items-center justify-center transition shrink-0 ${
                          isChecked
                            ? "bg-emerald-500 text-white"
                            : "border border-white/30 bg-white/[0.03]"
                        }`}
                      >
                        {isChecked && <Check className="w-3 h-3 stroke-[3]" />}
                      </div>

                      <div className="text-[12.5px] leading-relaxed">
                        <p className={`font-semibold ${isChecked ? "text-white line-through opacity-70" : "text-white"}`}>
                          {g.preventive_gate_question}
                        </p>
                        <p className="text-[11px] text-mab-text-muted mt-0.5">
                          تجربه قبلی: {g.mistake_pattern}
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* 2. Hero Card: The Active Next Step (NEXT_STEP.md) */}
          <div className="mab-card p-4 sm:p-5 space-y-4 border-blue-500/30">
            {/* Header Badges */}
            <div className="flex items-center justify-between gap-2 flex-wrap">
              <div className="flex items-center gap-2">
                <span className="px-2.5 py-0.5 rounded-full bg-blue-500/15 border border-blue-500/30 text-blue-400 text-[11.5px] font-bold flex items-center gap-1.5">
                  <span className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse" />
                  <span>گام فعال پیش‌رو</span>
                </span>

                {dash.next_step_task && (
                  <span className="px-2 py-0.5 rounded-md bg-purple-500/15 text-purple-300 font-mono text-[11px]">
                    {dash.next_step_task.category}
                  </span>
                )}
              </div>

              <div className="flex items-center gap-2 flex-wrap">
                {dash.next_step_task && (
                  <button
                    type="button"
                    onClick={() =>
                      insertPromptStarter(
                        `گام [${dash.next_step_task!.id}] رو اصلاح کن: `
                      )
                    }
                    className="px-2.5 py-0.5 rounded-full bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] text-[11px] text-white/70 hover:text-white flex items-center gap-1 transition"
                  >
                    <Edit3 className="w-3 h-3" />
                    <span>اصلاح در چت</span>
                  </button>
                )}

                {dash.next_step_task && (
                  <span className="px-2.5 py-0.5 rounded-md bg-amber-500/15 border border-amber-500/30 text-amber-300 font-mono text-[11px] font-semibold">
                    {formatPomos(dash.next_step_task.calibrated_pomos)} Pomo
                  </span>
                )}
              </div>
            </div>

            {dash.next_step_task ? (
              <>
                {/* Title */}
                <div className="text-[17px] sm:text-[18px] font-bold text-white leading-snug">
                  <span className="font-mono text-blue-400 ml-2 font-semibold">
                    [{dash.next_step_task.id}]
                  </span>
                  {dash.next_step_task.title}
                </div>

                {/* Callout 1: First 2-minute physical action */}
                <div className="p-3.5 rounded-xl bg-emerald-500/[0.06] border border-emerald-500/25 space-y-1">
                  <div className="text-[12px] font-bold text-emerald-400 flex items-center gap-1.5">
                    <Zap className="w-3.5 h-3.5" />
                    <span>اولین اقدام فیزیکی ۲ دقیقه‌ای (شروع فوری بدون تعلل):</span>
                  </div>
                  <p className="text-[13.5px] font-semibold text-white leading-relaxed select-text pt-0.5">
                    {dash.next_step_task.first_physical_action ||
                      "فایل مربوطه را باز کرده و اسکلت کار را بنویسید."}
                  </p>
                </div>

                {/* Callout 2: Definition of Done */}
                <div className="p-3.5 rounded-xl bg-white/[0.02] border border-white/[0.06] space-y-1">
                  <div className="text-[12px] font-bold text-blue-400 flex items-center gap-1.5">
                    <CheckCircle2 className="w-3.5 h-3.5" />
                    <span>شرط خروج قطعی (Definition of Done):</span>
                  </div>
                  <p className="text-[13px] text-mab-text-secondary leading-relaxed select-text pt-0.5">
                    {dash.next_step_task.definition_of_done}
                  </p>
                </div>

                {/* Completion Button */}
                <div className="pt-1">
                  <button
                    disabled={!allGatesPassed}
                    onClick={() =>
                      onQuickComplete(
                        dash.next_step_task!.id,
                        Math.round(dash.next_step_task!.calibrated_pomos) || 1
                      )
                    }
                    className={`w-full py-2.5 rounded-xl font-bold text-[13px] flex items-center justify-center gap-2 transition shadow-sm ${
                      allGatesPassed
                        ? "bg-emerald-600 hover:bg-emerald-500 text-white active:scale-[0.99]"
                        : "bg-white/[0.04] border border-white/[0.08] text-white/40 cursor-not-allowed"
                    }`}
                  >
                    <CheckCircle2 className="w-4 h-4" />
                    <span>
                      {allGatesPassed
                        ? "تکمیل شد • ثبت در سابقه و عبور به گام بعدی"
                        : "ابتدا چک‌باکس‌های سد پیشگیرانه بالا را تیک بزنید"}
                    </span>
                  </button>
                </div>
              </>
            ) : (
              <div className="py-8 text-center space-y-2">
                <p className="text-[14px] font-bold text-white">
                  مسیر پیش‌رو هنوز چیده نشده است
                </p>
                <p className="text-[12.5px] text-mab-text-muted">
                  در پنل چت همیار بنویس قصد داری چه کاری انجام دهی تا مسیر گام‌به‌گام تولید شود.
                </p>
              </div>
            )}
          </div>

          {/* 3. Next Steps Horizon (Anytype Collection List) */}
          <div className="mab-card p-4 space-y-3">
            <div className="flex items-center justify-between gap-2">
              <span className="text-[13.5px] font-bold text-white">
                افق گام‌های بعدی در پروژه
              </span>
              <span className="px-2 py-0.5 rounded-full bg-white/[0.04] text-white/50 text-[11px] font-timer">
                {dash.horizon_tasks.length} گام در صف
              </span>
            </div>

            {dash.horizon_tasks.length > 0 ? (
              <div className="space-y-1.5">
                {dash.horizon_tasks.map((ht, idx) => (
                  <div
                    key={ht.id}
                    className="p-3 rounded-xl bg-white/[0.02] hover:bg-white/[0.04] border border-white/[0.06] flex items-center justify-between gap-3 transition group"
                  >
                    <div className="flex items-center gap-2.5 min-w-0">
                      <span className="w-5 h-5 rounded-md bg-white/[0.06] text-white/60 font-mono text-[10.5px] font-bold flex items-center justify-center shrink-0">
                        {idx + 2}
                      </span>
                      <div className="min-w-0">
                        <div className="flex items-center gap-1.5">
                          <span className="font-mono text-[11.5px] text-blue-400 font-semibold shrink-0">
                            [{ht.id}]
                          </span>
                          <span className="text-[13px] font-medium text-white truncate">
                            {ht.title}
                          </span>
                        </div>
                        <p className="text-[11.5px] text-mab-text-muted truncate mt-0.5">
                          شروع: {ht.first_physical_action}
                        </p>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0">
                      <button
                        type="button"
                        onClick={() =>
                          insertPromptStarter(
                            `گام [${ht.id}] را بگذار گام اول مسیر پیش‌رو و این‌طور اصلاحش کن: `
                          )
                        }
                        className="px-2.5 py-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] text-[11px] text-white/70 hover:text-white transition opacity-80 group-hover:opacity-100"
                        title="ارتقا به صدر مسیر"
                      >
                        ارتقا به گام اول
                      </button>
                      <span className="font-mono text-[11.5px] text-amber-300/80">
                        {formatPomos(ht.calibrated_pomos)} Pomo
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-[12px] text-mab-text-muted py-2">
                گام دیگری در افق نزدیک این پروژه ثبت نشده است.
              </p>
            )}
          </div>

          {/* 4. Relevant SOPs */}
          {relevantSops.length > 0 && (
            <div className="mab-card p-4 space-y-2.5">
              <div className="flex items-center gap-2 text-[12.5px] font-bold text-purple-400">
                <BookOpen className="w-3.5 h-3.5 shrink-0" />
                <span>قوانین اجرایی (SOP) مرتبط با این مرحله</span>
              </div>
              <div className="space-y-1.5">
                {relevantSops.slice(0, 3).map((s) => (
                  <div
                    key={s.id}
                    className="p-2.5 rounded-lg bg-white/[0.02] border border-white/[0.06] flex items-center gap-2 text-[12px]"
                  >
                    <span className="px-1.5 py-0.5 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px] shrink-0">
                      {s.category}
                    </span>
                    <span className="text-mab-text-secondary select-text truncate">
                      {s.rule_statement}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Right Side (RTL End, 5 columns): Copilot Chat Dock */}
        <div className="lg:col-span-6 xl:col-span-5 flex flex-col min-h-[500px] lg:min-h-0 lg:h-full">
          {chatDock}
        </div>
      </div>
      {composerModal}
    </>
  );
}
