import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  AppSettings,
  ChatSession,
  CompassApi,
  DashboardState,
  LlmConnectionStatus,
  fileToBase64,
} from "./lib/tauri_api";
import { HeaderBar } from "./components/HeaderBar";
import { SidebarRail, AppTab } from "./components/SidebarRail";
import { PathAndChatView } from "./components/PathAndChatView";
import { KnowledgeVaultView } from "./components/KnowledgeVaultView";
import { WbsAndDocsView } from "./components/WbsAndDocsView";
import { SettingsView } from "./components/SettingsView";
import { NewProjectModal } from "./components/NewProjectModal";
import { PendingQueueBanner } from "./components/PendingQueueBanner";

export default function App() {
  const [dash, setDash] = useState<DashboardState | null>(null);
  const [activeTab, setActiveTab] = useState<AppTab>("PATH_AND_CHAT");
  const [isFullChatMode, setIsFullChatMode] = useState(false);
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState<boolean>(() =>
    typeof window !== "undefined" ? window.innerWidth < 960 : false
  );

  const [chatInput, setChatInput] = useState("");
  const [isSendingChat, setIsSendingChat] = useState(false);

  const [vaultChatInput, setVaultChatInput] = useState("");
  const [isSendingVaultChat, setIsSendingVaultChat] = useState(false);
  const [retryingMessageId, setRetryingMessageId] = useState<number | null>(null);

  const [isSyncing, setIsSyncing] = useState(false);

  const [checkedGates, setCheckedGates] = useState<Record<number, boolean>>({});
  const [showNewProjModal, setShowNewProjModal] = useState(false);
  const [docQueueStatus, setDocQueueStatus] = useState<string | null>(null);

  const [settingsForm, setSettingsForm] = useState<AppSettings | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  // وضعیت اتصال مدل و صف انتظار پردازش خودکار
  const [connectionStatus, setConnectionStatus] = useState<LlmConnectionStatus | null>(null);
  const [isCheckingConnection, setIsCheckingConnection] = useState(false);
  const [autoQueueEnabled, setAutoQueueEnabled] = useState(true);
  const [isProcessingQueue, setIsProcessingQueue] = useState(false);
  const [processingQueueId, setProcessingQueueId] = useState<number | null>(null);
  const [cooldownUntil, setCooldownUntil] = useState<number>(0);
  const [cooldownSecondsLeft, setCooldownSecondsLeft] = useState<number>(0);

  const queueRunningRef = useRef(false);
  const autoQueueRef = useRef(autoQueueEnabled);
  autoQueueRef.current = autoQueueEnabled;

  const isUserBusyRef = useRef(false);
  isUserBusyRef.current =
    isSendingChat || isSendingVaultChat || !!docQueueStatus || retryingMessageId !== null;

  const showToast = useCallback((msg: string) => {
    setToastMsg(msg);
    window.setTimeout(() => {
      setToastMsg((prev) => (prev === msg ? null : prev));
    }, 5500);
  }, []);

  const refreshConnection = useCallback(async (): Promise<LlmConnectionStatus | null> => {
    setIsCheckingConnection(true);
    try {
      const st = await CompassApi.checkLlmConnection();
      setConnectionStatus(st);
      return st;
    } catch {
      return null;
    } finally {
      setIsCheckingConnection(false);
    }
  }, []);

  // بارگذاری اولیه داشبورد + اسکن پیام‌ها/اسناد پردازش‌نشده + تست اولیه اتصال
  useEffect(() => {
    (async () => {
      try {
        const d = await CompassApi.scanUnprocessedQueue();
        setDash(d);
        setSettingsForm(d.settings);
        void refreshConnection();
      } catch (e) {
        try {
          const fallback = await CompassApi.getDashboard();
          setDash(fallback);
          setSettingsForm(fallback.settings);
          void refreshConnection();
        } catch (err2) {
          showToast(`خطا در بارگذاری داشبورد: ${String(err2 ?? e)}`);
        }
      }
    })();
  }, [refreshConnection, showToast]);

  // تایمر شمارش معکوس کول‌داون خطای صف
  useEffect(() => {
    if (cooldownUntil <= Date.now()) {
      setCooldownSecondsLeft(0);
      return;
    }
    const tick = () => {
      const rem = Math.max(0, Math.ceil((cooldownUntil - Date.now()) / 1000));
      setCooldownSecondsLeft(rem);
    };
    tick();
    const id = window.setInterval(tick, 1000);
    return () => window.clearInterval(id);
  }, [cooldownUntil]);

  // پردازشگر ترتیبی صف انتظار
  const runQueueDrain = useCallback(
    async (manualTrigger: boolean) => {
      if (queueRunningRef.current || isUserBusyRef.current) return;
      queueRunningRef.current = true;
      setIsProcessingQueue(true);
      try {
        const conn = await refreshConnection();
        if (!conn?.connected && !manualTrigger) {
          return;
        }

        let processedCount = 0;
        while (true) {
          if (!manualTrigger && !autoQueueRef.current) break;
          if (isUserBusyRef.current) break;

          const latestDash = await CompassApi.getDashboard();
          setDash(latestDash);
          const nextItem = latestDash.pending_queue[0];
          if (!nextItem) break;

          setProcessingQueueId(nextItem.id);
          const outcome = await CompassApi.processPendingQueueItem(nextItem.id);
          setDash(outcome.dashboard);
          setProcessingQueueId(null);

          if (outcome.error) {
            setCooldownUntil(Date.now() + 45_000);
            void refreshConnection();
            showToast(
              `ارسال مورد «${nextItem.title_preview}» متوقف شد (${outcome.error}). تلاش مجدد تا ۴۵ ثانیه دیگر.`
            );
            break;
          }

          if (outcome.processed) {
            processedCount += 1;
          } else {
            break;
          }
        }

        if (processedCount > 0) {
          showToast(
            `${processedCount} پیام/سند در انتظار بررسی با موفقیت توسط مدل تحلیل شد.`
          );
        }
      } catch (e: unknown) {
        setCooldownUntil(Date.now() + 45_000);
        showToast(`خطا در پردازش صف انتظار: ${String(e)}`);
      } finally {
        setProcessingQueueId(null);
        setIsProcessingQueue(false);
        queueRunningRef.current = false;
      }
    },
    [refreshConnection, showToast]
  );

  // اجرای خودکار صف هنگام برقراری اتصال و وجود آیتم در صف انتظار
  useEffect(() => {
    if (!dash || dash.pending_queue.length === 0) return;
    if (!autoQueueEnabled || queueRunningRef.current || isUserBusyRef.current) return;
    if (Date.now() < cooldownUntil) return;

    if (connectionStatus?.connected) {
      void runQueueDrain(false);
    }
  }, [
    dash?.pending_queue.length,
    autoQueueEnabled,
    connectionStatus?.connected,
    cooldownSecondsLeft,
    cooldownUntil,
    runQueueDrain,
    dash,
  ]);

  // بررسی دوره‌ای اتصال مدل وقتی موارد پردازش‌نشده در صف هستند (هر ۲۰ ثانیه)
  useEffect(() => {
    if (!dash || dash.pending_queue.length === 0) return;
    const id = window.setInterval(() => {
      if (document.visibilityState !== "visible") return;
      if (queueRunningRef.current || isUserBusyRef.current) return;
      void refreshConnection();
    }, 20_000);
    return () => window.clearInterval(id);
  }, [dash?.pending_queue.length, refreshConnection, dash]);

  // رفرش خودکار داشبورد هنگام تغییر دیتابیس از طریق فرایند بیرونی
  useEffect(() => {
    let lastVersion: number | null = null;
    const intervalId = window.setInterval(async () => {
      if (document.visibilityState !== "visible") return;
      try {
        const ver = await CompassApi.getDbDataVersion();
        if (lastVersion === null) {
          lastVersion = ver;
          return;
        }
        if (ver !== lastVersion) {
          lastVersion = ver;
          const updated = await CompassApi.getDashboard();
          setDash(updated);
        }
      } catch {
        // Ignore transient polling errors
      }
    }, 3000);
    return () => window.clearInterval(intervalId);
  }, []);

  if (!dash || !settingsForm) {
    return (
      <div
        dir="rtl"
        className="h-screen w-screen bg-mab-bg text-mab-text-primary flex items-center justify-center p-4"
      >
        <span className="text-[14px] text-mab-text-secondary">
          در حال بارگذاری Mab-Compass و همگام‌سازی با Mab-Focus...
        </span>
      </div>
    );
  }

  const allGatesPassed = dash.active_guardrails.every((g) => checkedGates[g.id]);

  // ارسال فوری در ۲ فاز: ابتدا ثبت قطعی در حافظه (UI + SQLite + 00_raw_inputs)، سپس پردازش همیار
  const handleSendChat = async () => {
    if (!chatInput.trim() || isSendingChat) return;
    const msg = chatInput;
    setChatInput("");
    setIsSendingChat(true);
    try {
      const committed = await CompassApi.commitUserChatMessage(
        "COPILOT",
        dash.active_copilot_session_id || null,
        msg
      );
      setDash(committed.dashboard);

      const outcome = await CompassApi.runAssistantTurn({
        channel: "COPILOT",
        sessionId: committed.session_id,
        messageText: msg,
        archivedPath: committed.archived_path,
        rawEventId: committed.raw_event_id,
        userMessageId: committed.user_message_id,
      });
      setDash(outcome.dashboard);
      if (outcome.assistant_error) {
        setCooldownUntil(Date.now() + 45_000);
        void refreshConnection();
        showToast(
          `پیام در حافظه و صف انتظار ثبت شد اما خطای مدل رخ داد: ${outcome.assistant_error}`
        );
      }
    } catch (e: unknown) {
      setChatInput(msg);
      showToast(`خطا در ثبت پیام: ${String(e)}`);
    } finally {
      setIsSendingChat(false);
    }
  };

  const handleSendCopilotFiles = async (files: FileList) => {
    if (!files || files.length === 0 || isSendingChat) return;
    const fileArray = Array.from(files);
    const note = chatInput;
    setChatInput("");
    setIsSendingChat(true);
    let succeeded = 0;
    let modelFailures = 0;
    try {
      for (let i = 0; i < fileArray.length; i++) {
        const file = fileArray[i];
        try {
          const base64 = await fileToBase64(file);
          const committed = await CompassApi.commitUserFileMessage({
            channel: "COPILOT",
            sessionId: dash.active_copilot_session_id || null,
            originalFilename: file.name,
            contentBase64: base64,
            userNote: i === 0 ? note : "",
          });
          setDash(committed.dashboard);
          succeeded += 1;

          const outcome = await CompassApi.runFileAssistantTurn({
            channel: "COPILOT",
            sessionId: committed.session_id,
            rawEventId: committed.raw_event_id,
            userMessageId: committed.user_message_id,
            effectiveTitle: committed.effective_title,
            effectiveFilename: committed.effective_filename,
            docType: committed.doc_type,
            extractedText: committed.extracted_text,
            combinedPromptText: committed.combined_prompt_text,
          });
          setDash(outcome.dashboard);
          if (outcome.assistant_error) {
            modelFailures += 1;
            setCooldownUntil(Date.now() + 45_000);
            showToast(
              `فایل ${file.name} در حافظه و صف انتظار ذخیره شد اما مدل خطا داد: ${outcome.assistant_error}`
            );
          }
        } catch (err: unknown) {
          showToast(`خطا در فایل ${file.name}: ${String(err)}`);
        }
      }
      if (succeeded > 0 && modelFailures === 0) {
        showToast(
          `${succeeded} فایل بلافاصله در حافظه ثبت و در گفتگوی همیار تحلیل شد.`
        );
      }
    } finally {
      setIsSendingChat(false);
    }
  };

  const handleSendVaultChat = async () => {
    if (!vaultChatInput.trim() || isSendingVaultChat) return;
    const msg = vaultChatInput;
    setVaultChatInput("");
    setIsSendingVaultChat(true);
    try {
      const committed = await CompassApi.commitUserChatMessage(
        "VAULT",
        dash.active_vault_session_id || null,
        msg
      );
      setDash(committed.dashboard);

      const outcome = await CompassApi.runAssistantTurn({
        channel: "VAULT",
        sessionId: committed.session_id,
        messageText: msg,
        archivedPath: committed.archived_path,
        rawEventId: committed.raw_event_id,
        userMessageId: committed.user_message_id,
      });
      setDash(outcome.dashboard);
      if (outcome.assistant_error) {
        setCooldownUntil(Date.now() + 45_000);
        void refreshConnection();
        showToast(
          `پیام در حافظه و صف انتظار ذخیره شد اما خطای مدل رخ داد: ${outcome.assistant_error}`
        );
      }
    } catch (e: unknown) {
      setVaultChatInput(msg);
      showToast(`خطا در گفتگوی Knowledge Vault: ${String(e)}`);
    } finally {
      setIsSendingVaultChat(false);
    }
  };

  const handleRetrySystemMessage = async (systemMessageId: number) => {
    if (
      retryingMessageId !== null ||
      isSendingChat ||
      isSendingVaultChat ||
      !!docQueueStatus ||
      isProcessingQueue
    ) {
      return;
    }
    setRetryingMessageId(systemMessageId);
    try {
      const outcome = await CompassApi.retrySystemMessage(systemMessageId);
      setDash(outcome.dashboard);
      if (outcome.assistant_error) {
        setCooldownUntil(Date.now() + 45_000);
        showToast(`خطا در ارسال مجدد به مدل: ${outcome.assistant_error}`);
      } else {
        showToast("پیام با موفقیت مجدد به مدل ارسال و پردازش شد.");
      }
    } catch (e: unknown) {
      showToast(`خطا در ارسال مجدد: ${String(e)}`);
    } finally {
      setRetryingMessageId(null);
    }
  };

  const handleProcessSingleQueueItem = async (queueId: number) => {
    if (isProcessingQueue || queueRunningRef.current || isUserBusyRef.current) return;
    setIsProcessingQueue(true);
    setProcessingQueueId(queueId);
    try {
      const outcome = await CompassApi.processPendingQueueItem(queueId);
      setDash(outcome.dashboard);
      if (outcome.error) {
        setCooldownUntil(Date.now() + 45_000);
        showToast(`خطا در ارسال از صف: ${outcome.error}`);
      } else if (outcome.processed) {
        showToast("آیتم صف با موفقیت توسط مدل تحلیل و ثبت شد.");
      }
    } catch (e: unknown) {
      showToast(`خطا در پردازش آیتم صف: ${String(e)}`);
    } finally {
      setProcessingQueueId(null);
      setIsProcessingQueue(false);
    }
  };

  const handleDismissQueueItem = async (queueId: number) => {
    try {
      const updated = await CompassApi.dismissPendingQueueItem(queueId);
      setDash(updated);
      showToast("آیتم از صف انتظار حذف شد (نسخه اصلی در حافظه محفوظ است).");
    } catch (e: unknown) {
      showToast(`خطا در حذف از صف: ${String(e)}`);
    }
  };

  const handleQuickComplete = async (taskId: string, pomosSpent: number) => {
    try {
      const updated = await CompassApi.quickCompleteStep(taskId, pomosSpent);
      setDash(updated);
      setCheckedGates({});
      showToast("گام فعلی مختومه شد و گام بعدی در ریل پیش‌رو قرار گرفت!");
    } catch (e: unknown) {
      showToast(`خطا در بستن گام: ${String(e)}`);
    }
  };

  const handleSwitchProject = async (projectId: string) => {
    try {
      const d = await CompassApi.switchProject(projectId);
      setDash(d);
      setCheckedGates({});
    } catch (e: unknown) {
      showToast(`خطا در تغییر پروژه: ${String(e)}`);
    }
  };

  const handleCreateProject = async (idCode: string, name: string) => {
    try {
      const d = await CompassApi.createProject(idCode, name);
      setDash(d);
      setSettingsForm(d.settings);
      setCheckedGates({});
      setShowNewProjModal(false);
      showToast(
        "پروژه جدید ساخته شد، پوشه‌های ۸گانه ایجاد شد و با Mab-Focus سینک گردید."
      );
    } catch (e: unknown) {
      showToast(`خطا در ساخت پروژه: ${String(e)}`);
    }
  };

  const handleCreateChatSession = async (
    projectId: string | null,
    channel: "COPILOT" | "VAULT"
  ) => {
    try {
      const d = await CompassApi.createChatSession(projectId, channel);
      setDash(d);
      setActiveTab(channel === "VAULT" ? "KNOWLEDGE_VAULT" : "PATH_AND_CHAT");
      showToast("گفتگوی جدید آغاز شد.");
    } catch (e: unknown) {
      showToast(`خطا در ایجاد گفتگوی جدید: ${String(e)}`);
    }
  };

  const handleSwitchChatSession = async (session: ChatSession) => {
    try {
      const d = await CompassApi.switchChatSession(session.id);
      setDash(d);
      setCheckedGates({});
      setActiveTab(
        session.channel === "VAULT" ? "KNOWLEDGE_VAULT" : "PATH_AND_CHAT"
      );
    } catch (e: unknown) {
      showToast(`خطا در بارگذاری گفتگو: ${String(e)}`);
    }
  };

  const handleSwitchCopilotSessionById = async (sessionId: string) => {
    if (!sessionId) return;
    try {
      const d = await CompassApi.switchChatSession(sessionId);
      setDash(d);
    } catch (e: unknown) {
      showToast(`خطا در تغییر گفتگو: ${String(e)}`);
    }
  };

  const handleDeleteChatSession = async (sessionId: string) => {
    try {
      const d = await CompassApi.deleteChatSession(sessionId);
      setDash(d);
      showToast("گفتگو از تاریخچه حذف شد.");
    } catch (e: unknown) {
      showToast(`خطا در حذف گفتگو: ${String(e)}`);
    }
  };

  const handleSyncMabFocus = async () => {
    if (isSyncing) return;
    setIsSyncing(true);
    try {
      const [rep, d] = await CompassApi.syncMabFocusNow();
      setDash(d);
      showToast(
        `سینک با Mab-Focus انجام شد: ${rep.notes_ingested_from_focus} یادداشت جدید و ${rep.projects_imported_from_focus} پروژه دریافت شد.`
      );
    } catch (e: unknown) {
      showToast(`خطای همگام‌سازی Mab-Focus: ${String(e)}`);
    } finally {
      setIsSyncing(false);
    }
  };

  const handleExportExcel = async () => {
    try {
      const path = await CompassApi.exportExcelReport();
      showToast(`گزارش اکسل کارفرما در ${path} ساخته شد.`);
    } catch (e: unknown) {
      showToast(`خطا در ساخت اکسل: ${String(e)}`);
    }
  };

  const handleUndo = async () => {
    try {
      const d = await CompassApi.undoLast();
      setDash(d);
      showToast("آخرین تغییر به حالت قبل بازگردانده شد.");
    } catch (e: unknown) {
      showToast(`خطا در بازگردانی: ${String(e)}`);
    }
  };

  const processBatchVaultFiles = async (files: FileList) => {
    if (!files || files.length === 0) return;

    const fileArray = Array.from(files);
    let succeeded = 0;
    let modelFailures = 0;
    for (let i = 0; i < fileArray.length; i++) {
      const file = fileArray[i];
      setDocQueueStatus(
        `در حال ثبت فوری و تحلیل سند (${i + 1} از ${fileArray.length}): ${file.name}...`
      );
      try {
        const contentBase64 = await fileToBase64(file);
        const title = file.name.replace(/\.[^/.]+$/, "") || file.name;
        const committed = await CompassApi.commitUserFileMessage({
          channel: "VAULT",
          sessionId: dash.active_vault_session_id || null,
          originalFilename: file.name,
          contentBase64,
          userNote: "",
          title,
          docType: "ONBOARDING_FILE",
        });
        setDash(committed.dashboard);
        succeeded += 1;

        const outcome = await CompassApi.runFileAssistantTurn({
          channel: "VAULT",
          sessionId: committed.session_id,
          rawEventId: committed.raw_event_id,
          userMessageId: committed.user_message_id,
          effectiveTitle: committed.effective_title,
          effectiveFilename: committed.effective_filename,
          docType: committed.doc_type,
          extractedText: committed.extracted_text,
          combinedPromptText: committed.combined_prompt_text,
        });
        setDash(outcome.dashboard);
        if (outcome.assistant_error) {
          modelFailures += 1;
          setCooldownUntil(Date.now() + 45_000);
          showToast(
            `فایل ${file.name} در حافظه و صف انتظار ذخیره شد اما مدل خطا داد: ${outcome.assistant_error}`
          );
        }
      } catch (err: unknown) {
        showToast(`خطا در فایل ${file.name}: ${String(err)}`);
      }
    }
    setDocQueueStatus(null);
    if (succeeded > 0 && modelFailures === 0) {
      showToast(
        `${succeeded} از ${fileArray.length} فایل در حافظه و آرشیو ثبت و تحلیل شد.`
      );
    }
  };

  const handleBatchFilesSelected = async (
    e: React.ChangeEvent<HTMLInputElement>
  ) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    await processBatchVaultFiles(files);
    e.target.value = "";
  };

  const handleManualDocIngest = async (
    title: string,
    docType: string,
    content: string
  ) => {
    const displayLabel = title.trim() || "متن آزاد";
    setDocQueueStatus(`در حال ثبت در حافظه و کالبدشکافی «${displayLabel}»...`);
    try {
      const committed = await CompassApi.commitVaultTextMessage({
        sessionId: dash.active_vault_session_id || null,
        title,
        docType,
        originalFilename: title.trim() ? `${title.trim()}.md` : "free_text.md",
        content,
      });
      setDash(committed.dashboard);

      const outcome = await CompassApi.runFileAssistantTurn({
        channel: "VAULT",
        sessionId: committed.session_id,
        rawEventId: committed.raw_event_id,
        userMessageId: committed.user_message_id,
        effectiveTitle: committed.effective_title,
        effectiveFilename: committed.effective_filename,
        docType: committed.doc_type,
        extractedText: committed.extracted_text,
        combinedPromptText: committed.combined_prompt_text,
      });
      setDash(outcome.dashboard);
      if (outcome.assistant_error) {
        setCooldownUntil(Date.now() + 45_000);
        void refreshConnection();
        showToast(
          `محتوا در حافظه و صف انتظار ثبت شد اما خطای مدل رخ داد: ${outcome.assistant_error}`
        );
      } else {
        showToast("محتوا با موفقیت تحلیل شد؛ سوالات تدقیق در گفتگو ثبت شدند.");
      }
    } catch (e: unknown) {
      showToast(`خطا: ${String(e)}`);
    } finally {
      setDocQueueStatus(null);
    }
  };

  const handleSaveSettings = async (overrideSettings?: AppSettings) => {
    try {
      let target = overrideSettings ?? settingsForm;
      if (
        overrideSettings &&
        target.active_llm_provider === "LOCAL_OPENAI" &&
        !target.local_model.trim()
      ) {
        try {
          const live = await CompassApi.fetchLiveModels(
            "LOCAL_OPENAI",
            target.local_endpoint,
            target.gemini_api_key
          );
          if (live.models.length > 0) {
            target = { ...target, local_model: live.models[0].id };
          }
        } catch {
          // Ignore live pre-fetch error and save provider setting cleanly
        }
      }
      const d = await CompassApi.saveSettings(target);
      setDash(d);
      setSettingsForm(d.settings);
      setCooldownUntil(0);
      showToast("تنظیمات با موفقیت در دیتابیس ذخیره شد.");
      void refreshConnection();
    } catch (e: unknown) {
      showToast(`خطا در ذخیره تنظیمات: ${String(e)}`);
    }
  };

  const isSplitDesktopWorkspace =
    activeTab === "PATH_AND_CHAT" || activeTab === "KNOWLEDGE_VAULT";

  return (
    <div
      dir="rtl"
      className="h-screen w-screen bg-mab-bg text-mab-text-primary flex flex-col overflow-hidden select-none"
    >
      <HeaderBar
        dash={dash}
        isSyncing={isSyncing}
        isSidebarCollapsed={isSidebarCollapsed}
        connectionStatus={connectionStatus}
        isCheckingConnection={isCheckingConnection}
        onToggleSidebar={() => setIsSidebarCollapsed((v) => !v)}
        onRecheckConnection={() => void refreshConnection()}
        onUndo={handleUndo}
        onSyncMabFocus={handleSyncMabFocus}
        onExportExcel={handleExportExcel}
      />

      <div className="flex-1 flex overflow-hidden min-h-0">
        <SidebarRail
          dash={dash}
          activeTab={activeTab}
          isCollapsed={isSidebarCollapsed}
          onToggleCollapse={() => setIsSidebarCollapsed((v) => !v)}
          onSelectTab={setActiveTab}
          onSwitchProject={handleSwitchProject}
          onOpenNewProject={() => setShowNewProjModal(true)}
          onCreateChatSession={handleCreateChatSession}
          onSwitchChatSession={handleSwitchChatSession}
          onDeleteChatSession={handleDeleteChatSession}
        />

        <main className="flex-1 p-3 sm:p-4 min-h-0 flex flex-col gap-3 overflow-hidden">
          <PendingQueueBanner
            queue={dash.pending_queue}
            connectionStatus={connectionStatus}
            isCheckingConnection={isCheckingConnection}
            isProcessingQueue={isProcessingQueue}
            processingQueueId={processingQueueId}
            autoQueueEnabled={autoQueueEnabled}
            cooldownSecondsLeft={cooldownSecondsLeft}
            onToggleAutoQueue={(enabled) => {
              setAutoQueueEnabled(enabled);
              if (enabled) setCooldownUntil(0);
            }}
            onRecheckConnection={() => void refreshConnection()}
            onProcessNowAll={() => {
              setCooldownUntil(0);
              void runQueueDrain(true);
            }}
            onProcessSingleItem={(id) => void handleProcessSingleQueueItem(id)}
            onDismissItem={(id) => void handleDismissQueueItem(id)}
            onOpenSettings={() => setActiveTab("SETTINGS")}
          />

          <div
            className={`flex-1 min-h-0 ${
              isSplitDesktopWorkspace
                ? "overflow-y-auto lg:overflow-hidden"
                : "overflow-y-auto"
            }`}
          >
            {activeTab === "PATH_AND_CHAT" && (
              <PathAndChatView
                dash={dash}
                checkedGates={checkedGates}
                onToggleGate={(id, checked) =>
                  setCheckedGates({ ...checkedGates, [id]: checked })
                }
                allGatesPassed={allGatesPassed}
                onQuickComplete={handleQuickComplete}
                chatInput={chatInput}
                onChangeChatInput={setChatInput}
                isSendingChat={isSendingChat}
                onSendChat={handleSendChat}
                onSendCopilotFiles={handleSendCopilotFiles}
                isFullChatMode={isFullChatMode}
                onToggleFullChatMode={setIsFullChatMode}
                onNewChat={() =>
                  void handleCreateChatSession(dash.active_project.id, "COPILOT")
                }
                onSwitchSession={(sid) => void handleSwitchCopilotSessionById(sid)}
                onOpenSettings={() => setActiveTab("SETTINGS")}
                retryingMessageId={retryingMessageId}
                onRetrySystemMessage={(id) => void handleRetrySystemMessage(id)}
              />
            )}

            {activeTab === "KNOWLEDGE_VAULT" && (
              <KnowledgeVaultView
                dash={dash}
                docQueueStatus={docQueueStatus}
                onBatchFilesSelected={handleBatchFilesSelected}
                onBatchFilesDropped={(files) => void processBatchVaultFiles(files)}
                onManualDocIngest={handleManualDocIngest}
                vaultChatInput={vaultChatInput}
                onChangeVaultChatInput={setVaultChatInput}
                isSendingVaultChat={isSendingVaultChat}
                onSendVaultChat={handleSendVaultChat}
                onNewVaultChat={() =>
                  void handleCreateChatSession(dash.active_project.id, "VAULT")
                }
                onSwitchVaultSession={(sid) =>
                  void handleSwitchCopilotSessionById(sid)
                }
                onNavigateToPath={() => setActiveTab("PATH_AND_CHAT")}
                onOpenSettings={() => setActiveTab("SETTINGS")}
                retryingMessageId={retryingMessageId}
                onRetrySystemMessage={(id) => void handleRetrySystemMessage(id)}
              />
            )}

            {activeTab === "WBS_AND_DOCS" && (
              <WbsAndDocsView dash={dash} onQuickComplete={handleQuickComplete} />
            )}

            {activeTab === "SETTINGS" && (
              <SettingsView
                settingsForm={settingsForm}
                onChange={setSettingsForm}
                onSave={() => void handleSaveSettings()}
                onQuickSaveSettings={(next) => void handleSaveSettings(next)}
              />
            )}
          </div>
        </main>
      </div>

      <NewProjectModal
        open={showNewProjModal}
        onClose={() => setShowNewProjModal(false)}
        onCreate={handleCreateProject}
      />

      {toastMsg && (
        <div className="fixed bottom-5 left-5 max-w-md bg-[#18191E] border border-blue-500/40 text-white px-4 py-3 rounded-2xl text-[12.5px] font-medium z-50 shadow-mab-modal">
          {toastMsg}
        </div>
      )}
    </div>
  );
}
