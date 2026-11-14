import React, { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  Check,
  ChevronDown,
  ChevronsUpDown,
  Expand,
  FileCode2,
  FileText,
  HelpCircle,
  Maximize2,
  MessageSquarePlus,
  Minimize2,
  Plus,
  RefreshCw,
  Send,
  Settings,
  Sparkles,
  UploadCloud,
  X,
} from "lucide-react";
import { DashboardState } from "../lib/tauri_api";
import { formatShortTime } from "../lib/formatters";

interface KnowledgeVaultViewProps {
  dash: DashboardState;
  docQueueStatus: string | null;
  onBatchFilesSelected: (e: React.ChangeEvent<HTMLInputElement>) => void;
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
  const [isComposerExpanded, setIsComposerExpanded] = useState(false);
  const [showComposerModal, setShowComposerModal] = useState(false);
  const [isSessionDropdownOpen, setIsSessionDropdownOpen] = useState(false);

  const chatEndRef = useRef<HTMLDivElement | null>(null);
  const vaultTextareaRef = useRef<HTMLTextAreaElement | null>(null);
  const modalTextareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [dash.vault_chat_history.length, isSendingVaultChat, docQueueStatus, isFullVaultChat]);

  useEffect(() => {
    if (showComposerModal) {
      window.setTimeout(() => modalTextareaRef.current?.focus(), 0);
    }
  }, [showComposerModal]);

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

  const handleAnswerQuestionClick = (question: string, index: number) => {
    const prefix = `پاسخ سوال ${index + 1} («${question}»):\n`;
    const nextVal = vaultChatInput.trim()
      ? `${vaultChatInput.trimEnd()}\n${prefix}`
      : prefix;
    onChangeVaultChatInput(nextVal);
    window.setTimeout(() => vaultTextareaRef.current?.focus(), 0);
  };

  const handleDrop = (e: React.DragEvent<HTMLLabelElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    if (docQueueStatus) return;
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      onBatchFilesDropped(e.dataTransfer.files);
    }
  };

  /* -------------------------------------------------------------------------- */
  /* Knowledge Vault Chat Dock                                                  */
  /* -------------------------------------------------------------------------- */
  const vaultChatDock = (
    <div
      className={`${
        isFullVaultChat ? "flex-1" : "lg:col-span-6 xl:col-span-7"
      } mab-card flex flex-col min-h-[500px] lg:min-h-0 lg:h-full overflow-hidden`}
    >
      {/* Header of Vault Copilot */}
      <div className="h-[46px] px-3.5 border-b border-white/[0.06] flex items-center justify-between gap-2 bg-[#131418] shrink-0">
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
                {activeVaultSession?.title || "گفتگوی دانش"}
              </span>
              {activeVaultSession && (
                <span className="font-timer text-[10px] text-white/50 shrink-0">
                  ({activeVaultSession.message_count})
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
                    <span>گفتگوهای آرشیو دانش</span>
                    <button
                      type="button"
                      onClick={() => {
                        setIsSessionDropdownOpen(false);
                        onNewVaultChat();
                      }}
                      className="text-purple-400 hover:text-purple-300 text-[10.5px] flex items-center gap-1"
                    >
                      <Plus className="w-3 h-3" />
                      <span>چت جدید</span>
                    </button>
                  </div>
                  <div className="max-h-52 overflow-y-auto space-y-0.5 py-1">
                    {vaultSessions.map((s) => {
                      const isSelected = s.id === activeVaultSession?.id;
                      return (
                        <button
                          key={s.id}
                          type="button"
                          onClick={() => {
                            onSwitchVaultSession(s.id);
                            setIsSessionDropdownOpen(false);
                          }}
                          className={`w-full text-right px-2.5 py-1.5 rounded-lg text-[12px] flex items-center justify-between gap-1.5 transition ${
                            isSelected
                              ? "bg-purple-500/15 text-purple-300 font-medium"
                              : "text-mab-text-secondary hover:bg-white/[0.05] hover:text-white"
                          }`}
                        >
                          <span className="truncate">{s.title}</span>
                          <div className="flex items-center gap-1.5 shrink-0">
                            <span className="font-timer text-[10px] opacity-60">
                              {s.message_count}
                            </span>
                            {isSelected && <Check className="w-3 h-3 text-purple-400" />}
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
            onClick={onNewVaultChat}
            className="px-2.5 py-1 rounded-full bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/25 text-purple-400 text-[11px] font-medium flex items-center gap-1 transition"
            title="شروع گفتگوی جدید دانش"
          >
            <MessageSquarePlus className="w-3 h-3" />
            <span className="hidden sm:inline">گفتگوی جدید</span>
          </button>

          <button
            type="button"
            onClick={() => setIsFullVaultChat((v) => !v)}
            className={`px-2 py-1 rounded-full border text-[11px] font-medium flex items-center gap-1 transition ${
              isFullVaultChat
                ? "bg-white/[0.1] border-white/[0.2] text-white"
                : "bg-white/[0.03] border-white/[0.08] text-mab-text-secondary hover:text-white"
            }`}
            title={isFullVaultChat ? "نمایش ستون اسناد" : "حالت چت کامل"}
          >
            {isFullVaultChat ? (
              <Minimize2 className="w-3 h-3" />
            ) : (
              <Maximize2 className="w-3 h-3" />
            )}
          </button>

          <button
            type="button"
            onClick={onNavigateToPath}
            className="px-3 py-1 rounded-full bg-emerald-600/90 hover:bg-emerald-500 text-white text-[11px] font-semibold flex items-center gap-1 transition shadow-sm"
          >
            <span>مسیر پیش‌رو</span>
            <ArrowLeft className="w-3 h-3" />
          </button>
        </div>
      </div>

      {/* Messages Stream */}
      <div className="flex-1 p-3.5 sm:p-4 overflow-y-auto space-y-3.5 select-text min-h-0">
        {dash.vault_chat_history.length === 0 && (
          <div className="p-5 rounded-2xl bg-white/[0.02] border border-white/[0.06] text-[13px] text-mab-text-secondary leading-[1.8] space-y-3">
            <div className="flex items-center gap-2 font-bold text-white text-[14px]">
              <Sparkles className="w-4 h-4 text-purple-400" />
              <span>تحلیل اسناد و استخراج خودکار گراف تسک‌ها</span>
            </div>
            <p>
              یک فایل یا صورت‌جلسه را از پنل راست بارگذاری کن. هوش مصنوعی متن را تحلیل کرده و تسک‌های قابل‌اجرا و سوالات تدقیق را استخراج می‌کند.
            </p>
          </div>
        )}

        {dash.vault_chat_history.map((m) => (
          <div
            key={m.id}
            className={`p-3.5 sm:p-4 rounded-2xl text-[13.5px] leading-[1.8] space-y-2.5 transition ${
              m.sender === "USER"
                ? "bg-purple-500/[0.08] border border-purple-500/20 rounded-tr-sm mr-2 sm:mr-5"
                : m.sender === "SYSTEM"
                ? "bg-amber-500/[0.08] border border-amber-500/25 ml-2 sm:ml-5 rounded-tl-sm"
                : "bg-white/[0.03] border border-white/[0.07] ml-2 sm:ml-5 rounded-tl-sm"
            }`}
          >
            <div className="flex items-center justify-between text-[11px] text-mab-text-muted">
              <span className="font-semibold">
                {m.sender === "USER" ? (
                  <span className="text-purple-400">شما (ورودی / پاسخ تدقیق)</span>
                ) : m.sender === "SYSTEM" ? (
                  <span className="text-amber-400">اعلان سیستم • ذخیره در حافظه</span>
                ) : (
                  <span className="text-purple-300 flex items-center gap-1">
                    <Sparkles className="w-3 h-3" />
                    <span>معمار دانش و گراف تسک‌ها</span>
                  </span>
                )}
              </span>
              <span className="font-timer">{formatShortTime(m.created_at)}</span>
            </div>

            <p className="text-mab-text-primary whitespace-pre-wrap break-words leading-relaxed">
              {m.message_text}
            </p>

            {m.sender === "SYSTEM" && (
              <div className="pt-1 flex items-center gap-2 flex-wrap">
                <button
                  type="button"
                  disabled={isSendingVaultChat || !!docQueueStatus || retryingMessageId !== null}
                  onClick={() => onRetrySystemMessage(m.id)}
                  className="px-3.5 py-1 rounded-full bg-blue-500 hover:bg-blue-600 text-white text-[11.5px] font-semibold inline-flex items-center gap-1.5 transition disabled:opacity-50"
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
                  <span>تنظیمات موتور مدل</span>
                </button>
              </div>
            )}

            {m.clarifying_questions && m.clarifying_questions.length > 0 && (
              <div className="p-3 rounded-xl bg-amber-500/[0.07] border border-amber-500/25 space-y-2">
                <div className="flex items-center gap-1.5 text-[11.5px] font-bold text-amber-400">
                  <HelpCircle className="w-3.5 h-3.5 shrink-0" />
                  <span>سوالات تدقیق گراف (برای پاسخ کلیک کنید):</span>
                </div>
                <div className="space-y-1.5">
                  {m.clarifying_questions.map((q, idx) => (
                    <button
                      key={`${m.id}-q-${idx}`}
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

            {m.applied_mutations_badge && (
              <div className="pt-0.5 flex items-center gap-1.5">
                <span className="px-2.5 py-0.5 rounded-md text-[10.5px] font-mono font-semibold bg-emerald-500/15 border border-emerald-500/30 text-emerald-300">
                  ✓ {m.applied_mutations_badge}
                </span>
              </div>
            )}
          </div>
        ))}

        {(isSendingVaultChat || !!docQueueStatus) && (
          <div className="p-3.5 rounded-2xl bg-white/[0.03] border border-purple-500/30 ml-4 flex items-center gap-2.5 text-[12.5px] text-mab-text-secondary">
            <RefreshCw className="w-3.5 h-3.5 text-purple-400 animate-spin shrink-0" />
            <span>ورودی در حافظه ذخیره شد • در حال تحلیل و سنتز اسناد...</span>
          </div>
        )}
        <div ref={chatEndRef} />
      </div>

      {/* Composer */}
      <div className="p-3 border-t border-white/[0.06] bg-[#121316] space-y-2 shrink-0">
        <div className="rounded-2xl bg-[#16171B] border border-white/[0.09] focus-within:border-purple-500/50 focus-within:ring-2 focus-within:ring-purple-500/20 p-2.5 shadow-sm transition-all space-y-2">
          <textarea
            ref={vaultTextareaRef}
            value={vaultChatInput}
            onChange={(e) => onChangeVaultChatInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                onSendVaultChat();
              }
            }}
            placeholder="پاسخ به سوالات، تحلیل یا اصلاح مسیر پیش‌رو... (Enter ارسال • Shift+Enter خط جدید)"
            rows={isComposerExpanded ? 7 : 3}
            className={`w-full bg-transparent text-[13.5px] leading-relaxed text-white placeholder-white/30 resize-none outline-none select-text transition-all ${
              isComposerExpanded ? "min-h-[160px] max-h-[40vh]" : "min-h-[70px] max-h-[180px]"
            }`}
          />

          <div className="flex items-center justify-between gap-2 pt-1 border-t border-white/[0.04]">
            <div className="flex items-center gap-1.5">
              <button
                type="button"
                onClick={() => setIsComposerExpanded((v) => !v)}
                title={isComposerExpanded ? "کوچک کردن کادر" : "بزرگ کردن کادر"}
                className={`p-1.5 rounded-lg transition ${
                  isComposerExpanded
                    ? "bg-purple-500/15 text-purple-400"
                    : "bg-white/[0.03] hover:bg-white/[0.08] text-white/60 hover:text-white"
                }`}
              >
                <ChevronsUpDown className="w-3.5 h-3.5" />
              </button>

              <button
                type="button"
                onClick={() => setShowComposerModal(true)}
                title="نمای بزرگ"
                className="p-1.5 rounded-lg bg-white/[0.03] hover:bg-white/[0.08] text-white/60 hover:text-white transition"
              >
                <Expand className="w-3.5 h-3.5" />
              </button>
            </div>

            <button
              onClick={onSendVaultChat}
              disabled={isSendingVaultChat || !!docQueueStatus || !vaultChatInput.trim()}
              className="px-4 py-1.5 rounded-xl bg-purple-600 hover:bg-purple-500 text-white text-[12px] font-semibold flex items-center gap-1.5 shadow-sm transition disabled:opacity-40"
            >
              <Send className="w-3.5 h-3.5" />
              <span>{isSendingVaultChat ? "در حال تدقیق..." : "ارسال پاسخ"}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );

  /* -------------------------------------------------------------------------- */
  /* Composer Modal                                                             */
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
          <h3 className="text-[14px] font-bold text-white">
            ویرایشگر بزرگ تدقیق دانش ({dash.active_project.name})
          </h3>
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
          value={vaultChatInput}
          onChange={(e) => onChangeVaultChatInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              setShowComposerModal(false);
            } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              setShowComposerModal(false);
              onSendVaultChat();
            }
          }}
          placeholder="پاسخ کامل به سوالات یا توضیحات سند..."
          className="w-full flex-1 min-h-[300px] sm:min-h-[380px] bg-transparent p-3 text-[14px] leading-relaxed text-white placeholder-white/30 resize-none outline-none select-text"
        />

        <div className="flex items-center justify-between gap-2 pt-3 border-t border-white/[0.06] flex-wrap">
          <span className="text-[11.5px] text-white/40">
            ارسال سریع با <code className="text-purple-400">Ctrl+Enter</code>
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
              disabled={isSendingVaultChat || !!docQueueStatus || !vaultChatInput.trim()}
              onClick={() => {
                setShowComposerModal(false);
                onSendVaultChat();
              }}
              className="px-5 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white text-[12.5px] font-semibold inline-flex items-center gap-1.5 transition disabled:opacity-50"
            >
              <Send className="w-3.5 h-3.5" />
              <span>ارسال</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  ) : null;

  if (isFullVaultChat) {
    return (
      <>
        <div className="flex flex-col gap-3 h-full min-h-0">{vaultChatDock}</div>
        {composerModal}
      </>
    );
  }

  /* -------------------------------------------------------------------------- */
  /* Main Split View                                                            */
  /* -------------------------------------------------------------------------- */
  return (
    <>
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-3.5 xl:gap-4 h-full min-h-0">
        {/* Right Column: Upload, Manual input, Document archive */}
        <div className="lg:col-span-6 xl:col-span-5 flex flex-col gap-3.5 overflow-y-auto pl-0.5 min-h-0">
          {/* Card 1: Anytype Dropzone */}
          <div className="mab-card p-4 space-y-3">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2 text-[13.5px] font-bold text-white">
                <UploadCloud className="w-4 h-4 text-blue-400" />
                <span>بارگذاری هر نوع سند یا فایل</span>
              </div>
              <span className="px-2 py-0.5 rounded-md bg-white/[0.06] text-white/60 font-mono text-[10px]">
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
              <select
                value={manualDocType}
                onChange={(e) => setManualDocType(e.target.value)}
                className="w-full mab-input px-3 py-1.5 text-[12px] bg-[#141519]"
              >
                <option value="FREE_TEXT">متن آزاد / یادداشت سریع</option>
                <option value="MEETING_MINUTES">صورت‌جلسه (Meeting Minutes)</option>
                <option value="PROJECT_CHARTER">منشور پروژه (Charter / Spec)</option>
                <option value="TECHNICAL_DOC">مستند فنی و معماری</option>
                <option value="SOURCE_CODE">قطعه کد / لاگ فنی</option>
              </select>
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
                  className="p-3 rounded-xl bg-white/[0.02] hover:bg-white/[0.04] border border-white/[0.06] space-y-1.5 transition"
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

        {/* Left Column: Copilot Chat */}
        {vaultChatDock}
      </div>
      {composerModal}
    </>
  );
}
