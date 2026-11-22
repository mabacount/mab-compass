import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  AppSettings,
  ChatSession,
  CompassApi,
  DashboardState,
  LlmConnectionStatus,
  MethodPlaybook,
  MethodPlaybookInput,
  Outcome,
  WorkEntity,
  WorkLine,
  WrittenArtifactRef,
  fileToBase64,
} from "./lib/tauri_api";
import { AntigravityTitlebar } from "./components/AntigravityTitlebar";
import {
  AntigravitySidebar,
  BuiltinCanvasViewId,
} from "./components/AntigravitySidebar";
import { AntigravityChatPane } from "./components/AntigravityChatPane";
import {
  AntigravityRightCanvas,
  CanvasTab,
} from "./components/AntigravityRightCanvas";
import { NewProjectModal } from "./components/NewProjectModal";
import { OutcomeModal, OutcomeFormValues } from "./components/OutcomeModal";
import {
  WorkLineModal,
  WorkLineCreatePayload,
  WorkLineUpdatePayload,
} from "./components/WorkLineModal";
import {
  TaskModal,
  ManualTaskPayload,
  TaskAssignPayload,
} from "./components/TaskModal";
import { addAppLog } from "./lib/logStore";

const BUILTIN_TAB_TITLES: Record<BuiltinCanvasViewId, string> = {
  PATH_AHEAD: "مسیر پیش‌رو",
  METHODS_BPMN: "متدها و BPMN 2.0",
  WBS: "ساختار WBS و لاین‌ها",
  KNOWLEDGE_ARCHIVE: "اسناد و بازپردازش",
  ECOSYSTEM_CARDS: "اهداف و کارت‌ها",
  SETTINGS: "تنظیمات",
  LOGS: "لاگ‌های سیستم",
};

export default function App() {
  const [dash, setDash] = useState<DashboardState | null>(null);

  // Antigravity 3-Column Layout State (LTR: Left Sidebar | Center Chat | Right Tabbed Canvas)
  const [isNarrowViewport, setIsNarrowViewport] = useState<boolean>(() =>
    typeof window !== "undefined" ? window.innerWidth < 860 : false
  );
  const [isSidebarOpen, setIsSidebarOpen] = useState<boolean>(() =>
    typeof window !== "undefined" ? window.innerWidth >= 960 : true
  );
  const [sidebarWidth, setSidebarWidth] = useState<number>(() => {
    if (typeof window === "undefined") return 244;
    const saved = Number(window.localStorage.getItem("mab_ag_sidebar_width"));
    return Number.isFinite(saved) && saved >= 190 && saved <= 380 ? saved : 244;
  });
  const [isDraggingSidebar, setIsDraggingSidebar] = useState(false);
  const sidebarDragRef = useRef<{ startX: number; startWidth: number } | null>(null);

  const [isRightCanvasOpen, setIsRightCanvasOpen] = useState<boolean>(() =>
    typeof window !== "undefined" ? window.innerWidth >= 900 : true
  );
  const [isRightCanvasMaximized, setIsRightCanvasMaximized] = useState(false);
  const [rightCanvasPct, setRightCanvasPct] = useState<number>(() => {
    if (typeof window === "undefined") return 48;
    const saved = Number(window.localStorage.getItem("mab_ag_canvas_pct"));
    return Number.isFinite(saved) && saved >= 26 && saved <= 72 ? saved : 48;
  });
  const [isDraggingCanvas, setIsDraggingCanvas] = useState(false);
  const workspaceBodyRef = useRef<HTMLDivElement | null>(null);

  // Right Tabbed Canvas State + Back/Forward Tab History
  const [canvasTabs, setCanvasTabs] = useState<CanvasTab[]>([
    { id: "PATH_AHEAD", kind: "PATH_AHEAD", title: BUILTIN_TAB_TITLES.PATH_AHEAD },
    { id: "METHODS_BPMN", kind: "METHODS_BPMN", title: BUILTIN_TAB_TITLES.METHODS_BPMN },
  ]);
  const [activeCanvasTabId, setActiveCanvasTabId] = useState<string | null>("PATH_AHEAD");
  const [tabHistory, setTabHistory] = useState<string[]>(["PATH_AHEAD"]);
  const [tabHistoryIdx, setTabHistoryIdx] = useState<number>(0);

  // Active Chat Channel inside Center Column
  const [activeChatChannel, setActiveChatChannel] = useState<"COPILOT" | "VAULT">("COPILOT");
  const [chatInput, setChatInput] = useState("");
  const [isSendingChat, setIsSendingChat] = useState(false);
  const [retryingMessageId, setRetryingMessageId] = useState<number | null>(null);

  // Manual 5-Layer Modals
  const [showNewProjModal, setShowNewProjModal] = useState(false);
  const [showOutcomeModal, setShowOutcomeModal] = useState(false);
  const [editingOutcome, setEditingOutcome] = useState<Outcome | null>(null);
  const [showWorkLineModal, setShowWorkLineModal] = useState(false);
  const [editingWorkLine, setEditingWorkLine] = useState<WorkLine | null>(null);
  const [showTaskModal, setShowTaskModal] = useState(false);
  const [editingTask, setEditingTask] = useState<WorkEntity | null>(null);

  const [isSyncing, setIsSyncing] = useState(false);
  const [checkedGates, setCheckedGates] = useState<Record<number, boolean>>({});
  const [docQueueStatus, setDocQueueStatus] = useState<string | null>(null);
  const [settingsForm, setSettingsForm] = useState<AppSettings | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  // LLM Connection & Chronological Pending Queue State
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
    isSendingChat || !!docQueueStatus || retryingMessageId !== null;

  const showToast = useCallback((msg: string) => {
    setToastMsg(msg);
    window.setTimeout(() => {
      setToastMsg((prev) => (prev === msg ? null : prev));
    }, 5500);
  }, []);

  const pushTabHistory = useCallback(
    (tabId: string) => {
      setTabHistory((prev) => {
        const sliced = prev.slice(0, tabHistoryIdx + 1);
        if (sliced[sliced.length - 1] === tabId) return sliced;
        const next = [...sliced, tabId].slice(-30);
        setTabHistoryIdx(next.length - 1);
        return next;
      });
    },
    [tabHistoryIdx]
  );

  const activateTab = useCallback(
    (tabId: string) => {
      setActiveCanvasTabId(tabId);
      setIsRightCanvasOpen(true);
      pushTabHistory(tabId);
    },
    [pushTabHistory]
  );

  const openBuiltinView = useCallback(
    (viewId: BuiltinCanvasViewId) => {
      setCanvasTabs((prev) => {
        if (prev.some((t) => t.id === viewId)) return prev;
        return [
          ...prev,
          { id: viewId, kind: viewId, title: BUILTIN_TAB_TITLES[viewId] },
        ];
      });
      activateTab(viewId);
    },
    [activateTab]
  );

  const openArtifactTab = useCallback(
    (artifact: WrittenArtifactRef) => {
      const tabId = `ART:${artifact.artifact_type}:${artifact.entity_id}`;
      setCanvasTabs((prev) => {
        const existingIdx = prev.findIndex((t) => t.id === tabId);
        if (existingIdx >= 0) {
          const copy = [...prev];
          copy[existingIdx] = {
            ...copy[existingIdx],
            title: artifact.title,
            artifact,
          };
          return copy;
        }
        return [
          ...prev,
          { id: tabId, kind: "ARTIFACT", title: artifact.title, artifact },
        ];
      });
      activateTab(tabId);
    },
    [activateTab]
  );

  const openMethodTab = useCallback(
    (method: MethodPlaybook) => {
      openArtifactTab({
        artifact_type: "METHOD_PLAYBOOK",
        entity_id: method.id,
        title: method.title,
        summary: method.summary,
        file_path: method.archived_path,
        action: "UPDATED",
        version: method.version,
      });
    },
    [openArtifactTab]
  );

  const closeCanvasTab = useCallback(
    (tabId: string) => {
      setCanvasTabs((prev) => {
        const idx = prev.findIndex((t) => t.id === tabId);
        if (idx < 0) return prev;
        const next = prev.filter((t) => t.id !== tabId);
        if (activeCanvasTabId === tabId) {
          const fallback = next[Math.max(0, idx - 1)]?.id ?? null;
          setActiveCanvasTabId(fallback);
          if (!fallback) {
            setIsRightCanvasMaximized(false);
          }
        }
        return next;
      });
      setTabHistory((prev) => {
        const filtered = prev.filter((id) => id !== tabId);
        setTabHistoryIdx((curIdx) =>
          Math.max(0, Math.min(curIdx, Math.max(0, filtered.length - 1)))
        );
        return filtered;
      });
    },
    [activeCanvasTabId]
  );

  const handleGoBackTab = () => {
    for (let idx = tabHistoryIdx - 1; idx >= 0; idx--) {
      const targetId = tabHistory[idx];
      if (targetId && canvasTabs.some((t) => t.id === targetId)) {
        setTabHistoryIdx(idx);
        setActiveCanvasTabId(targetId);
        setIsRightCanvasOpen(true);
        return;
      }
    }
  };

  const handleGoForwardTab = () => {
    for (let idx = tabHistoryIdx + 1; idx < tabHistory.length; idx++) {
      const targetId = tabHistory[idx];
      if (targetId && canvasTabs.some((t) => t.id === targetId)) {
        setTabHistoryIdx(idx);
        setActiveCanvasTabId(targetId);
        setIsRightCanvasOpen(true);
        return;
      }
    }
  };

  const refreshConnection = useCallback(async (): Promise<LlmConnectionStatus | null> => {
    setIsCheckingConnection(true);
    try {
      const st = await CompassApi.checkLlmConnection();
      setConnectionStatus(st);
      if (st.connected) {
        addAppLog("SUCCESS", "LLM", `ارتباط با مدل برقرار است: ${st.provider} (${st.model})`, st.message);
      } else {
        addAppLog("WARN", "LLM", `عدم دسترسی به مدل: ${st.provider}`, st.message);
      }
      return st;
    } catch (err: unknown) {
      addAppLog("ERROR", "LLM", "خطا در تست اتصال هوش مصنوعی", String(err));
      return null;
    } finally {
      setIsCheckingConnection(false);
    }
  }, []);

  // Responsive viewport listener
  useEffect(() => {
    const onResize = () => {
      const w = window.innerWidth;
      setIsNarrowViewport(w < 860);
      if (w < 960) {
        setIsSidebarOpen(false);
      }
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  // Left Sidebar LTR drag resizer
  useEffect(() => {
    if (!isDraggingSidebar) return;
    const handleMouseMove = (e: MouseEvent) => {
      const ref = sidebarDragRef.current;
      if (!ref) return;
      // LTR: sidebar is on the left, dragging right increases width
      const nextWidth = Math.max(
        190,
        Math.min(380, ref.startWidth + (e.clientX - ref.startX))
      );
      setSidebarWidth(nextWidth);
    };
    const handleMouseUp = () => {
      setIsDraggingSidebar(false);
      sidebarDragRef.current = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isDraggingSidebar]);

  useEffect(() => {
    if (!isDraggingSidebar) {
      try {
        window.localStorage.setItem("mab_ag_sidebar_width", String(Math.round(sidebarWidth)));
      } catch {
        // ignore
      }
    }
  }, [isDraggingSidebar, sidebarWidth]);

  // Right Tabbed Canvas LTR drag resizer (between Center Chat & Right Canvas)
  useEffect(() => {
    if (!isDraggingCanvas) return;
    const handleMouseMove = (e: MouseEvent) => {
      const el = workspaceBodyRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0) return;
      // LTR: right canvas width is measured from right edge
      const rawPct = ((rect.right - e.clientX) / rect.width) * 100;
      const clamped = Math.max(26, Math.min(72, rawPct));
      setRightCanvasPct(clamped);
    };
    const handleMouseUp = () => {
      setIsDraggingCanvas(false);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isDraggingCanvas]);

  useEffect(() => {
    if (!isDraggingCanvas) {
      try {
        window.localStorage.setItem(
          "mab_ag_canvas_pct",
          String(Math.round(rightCanvasPct * 10) / 10)
        );
      } catch {
        // ignore
      }
    }
  }, [isDraggingCanvas, rightCanvasPct]);

  // Initial load + queue scan + LLM check
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

  // Cooldown countdown
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

  // Chronological Pending Queue Drain
  const runQueueDrain = useCallback(
    async (manualTrigger: boolean) => {
      if (queueRunningRef.current || isUserBusyRef.current) return;
      queueRunningRef.current = true;
      setIsProcessingQueue(true);
      try {
        const conn = await refreshConnection();
        if (!conn?.connected && !manualTrigger) return;

        let processedCount = 0;
        while (true) {
          if (!manualTrigger && !autoQueueRef.current) break;
          if (isUserBusyRef.current) break;

          const latestDash = await CompassApi.getDashboard();
          setDash(latestDash);
          const nextItem = latestDash.pending_queue[0];
          if (!nextItem) break;

          addAppLog(
            "INFO",
            "QUEUE",
            `آغاز پردازش نوبتی مورد ${nextItem.id} از صف: «${nextItem.title_preview}»`
          );
          setProcessingQueueId(nextItem.id);
          const outcome = await CompassApi.processPendingQueueItem(nextItem.id);
          setDash(outcome.dashboard);
          setProcessingQueueId(null);

          if (outcome.error) {
            addAppLog(
              "ERROR",
              "QUEUE",
              `توقف پردازش مورد «${nextItem.title_preview}»`,
              outcome.error
            );
            setCooldownUntil(Date.now() + 45_000);
            void refreshConnection();
            showToast(
              `ارسال مورد «${nextItem.title_preview}» متوقف شد (${outcome.error}). تلاش مجدد تا ۴۵ ثانیه دیگر.`
            );
            break;
          }

          if (outcome.processed) {
            addAppLog(
              "SUCCESS",
              "QUEUE",
              `مورد «${nextItem.title_preview}» با موفقیت توسط مدل پردازش و ثبت شد.`
            );
            processedCount += 1;
          } else {
            break;
          }
        }

        if (processedCount > 0) {
          showToast(`${processedCount} مورد از صف با موفقیت توسط مدل تحلیل و ثبت شد.`);
        }
      } catch (e: unknown) {
        addAppLog("ERROR", "QUEUE", "خطای پیش‌بینی‌نشده در پردازش صف", String(e));
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

  // Auto queue runner
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

  // Auto refresh when external MCP process mutates SQLite
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
        // ignore
      }
    }, 3000);
    return () => window.clearInterval(intervalId);
  }, []);

  if (!dash || !settingsForm) {
    return (
      <div
        dir="rtl"
        className="h-screen w-screen bg-[#0D0E11] text-mab-text-primary flex items-center justify-center p-4"
      >
        <span className="text-[13.5px] text-mab-text-secondary">
          در حال بارگذاری Mab-Compass و همگام‌سازی با Mab-Focus...
        </span>
      </div>
    );
  }

  const allGatesPassed = dash.active_guardrails.every((g) => checkedGates[g.id]);
  const activeChatHistory =
    activeChatChannel === "VAULT" ? dash.vault_chat_history : dash.chat_history;
  const channelSessions = dash.chat_sessions.filter((s) =>
    activeChatChannel === "VAULT" ? s.channel === "VAULT" : s.channel !== "VAULT"
  );
  const activeSession =
    channelSessions.find(
      (s) =>
        s.id ===
        (activeChatChannel === "VAULT"
          ? dash.active_vault_session_id
          : dash.active_copilot_session_id)
    ) ?? channelSessions[0];

  // Unified 2-phase Chat Send (supports both COPILOT & VAULT channels seamlessly)
  const handleSendChat = async () => {
    if (!chatInput.trim() || isSendingChat) return;
    const msg = chatInput;
    setChatInput("");
    setIsSendingChat(true);
    const sessionId =
      activeChatChannel === "VAULT"
        ? dash.active_vault_session_id || null
        : dash.active_copilot_session_id || null;
    try {
      const committed = await CompassApi.commitUserChatMessage(
        activeChatChannel,
        sessionId,
        msg
      );
      setDash(committed.dashboard);

      const outcome = await CompassApi.runAssistantTurn({
        channel: activeChatChannel,
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

  const handleSendChatFiles = async (files: FileList) => {
    if (!files || files.length === 0 || isSendingChat) return;
    const fileArray = Array.from(files);
    const note = chatInput;
    setChatInput("");
    setIsSendingChat(true);
    const sessionId =
      activeChatChannel === "VAULT"
        ? dash.active_vault_session_id || null
        : dash.active_copilot_session_id || null;
    let succeeded = 0;
    let modelFailures = 0;
    try {
      for (let i = 0; i < fileArray.length; i++) {
        const file = fileArray[i];
        try {
          const base64 = await fileToBase64(file);
          const committed = await CompassApi.commitUserFileMessage({
            channel: activeChatChannel,
            sessionId,
            originalFilename: file.name,
            contentBase64: base64,
            userNote: i === 0 ? note : "",
          });
          setDash(committed.dashboard);
          succeeded += 1;

          const outcome = await CompassApi.runFileAssistantTurn({
            channel: activeChatChannel,
            sessionId: committed.session_id,
            rawEventId: committed.raw_event_id,
            userMessageId: committed.user_message_id,
            effectiveTitle: committed.effective_title,
            effectiveFilename: committed.effective_filename,
            docType: committed.doc_type,
            extractedText: committed.extracted_text,
            combinedPromptText: committed.combined_prompt_text,
            knowledgeDocId: committed.knowledge_doc_id ?? null,
          });
          setDash(outcome.dashboard);
          if (outcome.assistant_error) {
            modelFailures += 1;
            setCooldownUntil(Date.now() + 45_000);
            showToast(
              `فایل ${file.name} در حافظه و صف ذخیره شد اما مدل خطا داد: ${outcome.assistant_error}`
            );
          }
        } catch (err: unknown) {
          showToast(`خطا در فایل ${file.name}: ${String(err)}`);
        }
      }
      if (succeeded > 0 && modelFailures === 0) {
        showToast(`${succeeded} فایل بلافاصله در حافظه ثبت و توسط مدل تحلیل شد.`);
      }
    } finally {
      setIsSendingChat(false);
    }
  };

  const handleRetrySystemMessage = async (systemMessageId: number) => {
    if (
      retryingMessageId !== null ||
      isSendingChat ||
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
        showToast(`خطا در ارسال مجدد: ${outcome.assistant_error}`);
      } else {
        showToast("پیام با موفقیت مجدد پردازش شد.");
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
        showToast("آیتم صف با موفقیت توسط مدل تحلیل شد.");
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
      showToast("آیتم از صف انتظار حذف شد.");
    } catch (e: unknown) {
      showToast(`خطا در حذف از صف: ${String(e)}`);
    }
  };

  const handleReorderQueueItem = async (
    queueId: number,
    direction: "TOP" | "UP" | "DOWN"
  ) => {
    try {
      const updated = await CompassApi.reorderPendingQueueItem(queueId, direction);
      setDash(updated);
    } catch (e: unknown) {
      showToast(`خطا در تغییر اولویت صف: ${String(e)}`);
    }
  };

  const handleResetQueueOrder = async () => {
    try {
      const updated = await CompassApi.resetPendingQueueOrder();
      setDash(updated);
      showToast("ترتیب صف به ترتیب زمانی ایجاد بازنشانی شد.");
    } catch (e: unknown) {
      showToast(`خطا در بازنشانی صف: ${String(e)}`);
    }
  };

  const handleQuickComplete = async (taskId: string, pomosSpent: number) => {
    try {
      const updated = await CompassApi.quickCompleteStep(taskId, pomosSpent);
      setDash(updated);
      setCheckedGates({});
      showToast("گام فعلی تکمیل شد و گام بعدی در مسیر پیش‌رو قرار گرفت!");
    } catch (e: unknown) {
      showToast(`خطا در تکمیل گام: ${String(e)}`);
    }
  };

  const handleStartTask = async (taskId: string) => {
    try {
      const updated = await CompassApi.startTaskStep(taskId);
      setDash(updated);
      showToast(`گام [${taskId}] وارد وضعیت «در حال اجرا» شد.`);
    } catch (e: unknown) {
      showToast(`خطا در شروع گام: ${String(e)}`);
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
      showToast("پروژه جدید با پوشه‌های استاندارد ساخته و با Mab-Focus سینک شد.");
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
      setActiveChatChannel(channel);
      showToast("گفتگوی جدید آغاز شد.");
    } catch (e: unknown) {
      showToast(`خطا در ایجاد گفتگو: ${String(e)}`);
    }
  };

  const handleSwitchChatSession = async (session: ChatSession) => {
    try {
      const d = await CompassApi.switchChatSession(session.id);
      setDash(d);
      setCheckedGates({});
      setActiveChatChannel(session.channel === "VAULT" ? "VAULT" : "COPILOT");
    } catch (e: unknown) {
      showToast(`خطا در بارگذاری گفتگو: ${String(e)}`);
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
        `سینک Mab-Focus: ${rep.notes_ingested_from_focus} یادداشت جدید و ${rep.projects_imported_from_focus} پروژه دریافت شد.`
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
      showToast("آخرین تغییر بازگردانی شد.");
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
          knowledgeDocId: committed.knowledge_doc_id ?? null,
        });
        setDash(outcome.dashboard);
        if (outcome.assistant_error) {
          modelFailures += 1;
          setCooldownUntil(Date.now() + 45_000);
          showToast(
            `فایل ${file.name} در حافظه و صف ذخیره شد اما مدل خطا داد: ${outcome.assistant_error}`
          );
        }
      } catch (err: unknown) {
        showToast(`خطا در فایل ${file.name}: ${String(err)}`);
      }
    }
    setDocQueueStatus(null);
    if (succeeded > 0 && modelFailures === 0) {
      showToast(`${succeeded} فایل با موفقیت ثبت و تحلیل شد.`);
    }
  };

  const handleBatchFilesSelected = async (e: React.ChangeEvent<HTMLInputElement>) => {
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
    setDocQueueStatus(`در حال ثبت و تحلیل «${displayLabel}»...`);
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
        knowledgeDocId: committed.knowledge_doc_id ?? null,
      });
      setDash(outcome.dashboard);
      if (outcome.assistant_error) {
        setCooldownUntil(Date.now() + 45_000);
        void refreshConnection();
        showToast(
          `محتوا در حافظه و صف ثبت شد اما خطای مدل رخ داد: ${outcome.assistant_error}`
        );
      } else {
        showToast("محتوا با موفقیت تحلیل و ذخیره شد.");
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
          // ignore
        }
      }
      const d = await CompassApi.saveSettings(target);
      setDash(d);
      setSettingsForm(d.settings);
      setCooldownUntil(0);
      showToast("تنظیمات با موفقیت ذخیره شد.");
      addAppLog("INFO", "SYSTEM", `تنظیمات با موفقیت ذخیره شد (ارائه‌دهنده: ${target.active_llm_provider})`);
      void refreshConnection();
    } catch (e: unknown) {
      addAppLog("ERROR", "SYSTEM", "خطا در ذخیره تنظیمات", String(e));
      showToast(`خطا در ذخیره تنظیمات: ${String(e)}`);
    }
  };

  const handleQuickChangeModel = async (provider: string, modelId: string, label: string) => {
    if (!settingsForm) return;
    const next: AppSettings = {
      ...settingsForm,
      active_llm_provider: provider,
      ...(provider === "AGY_CLI" ? { agy_model: modelId } : {}),
      ...(provider === "LOCAL_OPENAI" ? { local_model: modelId } : {}),
      ...(provider === "GEMINI_REST" ? { gemini_model: modelId } : {}),
    };
    addAppLog("INFO", "LLM", `تغییر سریع مدل به «${label}» (${provider} / ${modelId})`);
    showToast(`مدل فعال به «${label}» تغییر کرد.`);
    await handleSaveSettings(next);
  };

  const handleRequeueDoc = async (docId: number) => {
    try {
      const updated = await CompassApi.requeueKnowledgeDoc(docId);
      setDash(updated);
      showToast("سند وارد صف پردازش شد.");
    } catch (e: unknown) {
      showToast(`خطا در ارسال به صف: ${String(e)}`);
    }
  };

  const handleUpsertMethod = async (input: MethodPlaybookInput) => {
    try {
      const updated = await CompassApi.upsertMethodPlaybook(input);
      setDash(updated);
      showToast("متد و نمودار BPMN 2.0 با موفقیت در SQLite و پوشه 04_methods_playbooks ذخیره شد.");
    } catch (e: unknown) {
      showToast(`خطا در ذخیره متد: ${String(e)}`);
    }
  };

  const handleDeleteMethod = async (methodId: string) => {
    try {
      const updated = await CompassApi.deleteMethodPlaybook(methodId);
      setDash(updated);
      closeCanvasTab(`ART:METHOD_PLAYBOOK:${methodId}`);
      showToast("متد از کتابخانه حذف شد.");
    } catch (e: unknown) {
      showToast(`خطا در حذف متد: ${String(e)}`);
    }
  };

  const handleUpsertOutcome = async (values: OutcomeFormValues) => {
    try {
      const updated = await CompassApi.upsertOutcome(values);
      setDash(updated);
      showToast(values.outcomeId ? "Outcome ویرایش شد." : "Outcome جدید ساخته شد.");
    } catch (e: unknown) {
      showToast(`خطا در ذخیره Outcome: ${String(e)}`);
    }
  };

  const handleDeleteOutcome = async (outcomeId: string) => {
    try {
      const updated = await CompassApi.deleteOutcome(outcomeId);
      setDash(updated);
      showToast("Outcome حذف شد.");
    } catch (e: unknown) {
      showToast(`خطا در حذف Outcome: ${String(e)}`);
    }
  };

  const handleCreateWorkLine = async (payload: WorkLineCreatePayload) => {
    try {
      const res = await CompassApi.createWorkLineChain({
        project_id: dash.active_project.id,
        ...payload,
      });
      setDash(res.dashboard ?? (await CompassApi.getDashboard()));
      showToast("لاین کاری جدید ساخته شد.");
    } catch (e: unknown) {
      showToast(`خطا در ساخت لاین کاری: ${String(e)}`);
    }
  };

  const handleUpdateWorkLine = async (payload: WorkLineUpdatePayload) => {
    try {
      const updated = await CompassApi.updateWorkLine(payload);
      setDash(updated);
      showToast("لاین کاری به‌روزرسانی شد.");
    } catch (e: unknown) {
      showToast(`خطا در ویرایش لاین کاری: ${String(e)}`);
    }
  };

  const handleCreateManualTask = async (payload: ManualTaskPayload) => {
    try {
      const updated = await CompassApi.createTaskManual(payload);
      setDash(updated);
      showToast("تسک جدید ساخته شد.");
    } catch (e: unknown) {
      showToast(`خطا در ساخت تسک: ${String(e)}`);
    }
  };

  const handleAssignTaskGrouping = async (payload: TaskAssignPayload) => {
    try {
      const updated = await CompassApi.assignTaskGrouping(payload);
      setDash(updated);
      showToast("تخصیص تسک ذخیره شد.");
    } catch (e: unknown) {
      showToast(`خطا در تخصیص تسک: ${String(e)}`);
    }
  };

  const handleCreateVaultCard = async (params: {
    realm: string;
    domainCode: string;
    itemType: string;
    title: string;
    contentMarkdown: string;
    tags: string[];
    attachToActiveProject: boolean;
  }) => {
    try {
      const updated = await CompassApi.createPersonalVaultItem(params);
      setDash(updated);
      showToast("کارت دانش جدید ثبت شد.");
    } catch (e: unknown) {
      showToast(`خطا در ثبت کارت دانش: ${String(e)}`);
    }
  };

  const handleDeleteVaultCard = async (itemId: string) => {
    try {
      const updated = await CompassApi.deletePersonalVaultItem(itemId);
      setDash(updated);
      showToast("کارت دانش حذف شد.");
    } catch (e: unknown) {
      showToast(`خطا در حذف کارت دانش: ${String(e)}`);
    }
  };

  const handleResolveProposal = async (proposalId: string, approve: boolean) => {
    try {
      const updated = await CompassApi.resolvePendingProposal(proposalId, approve);
      setDash(updated);
      showToast(approve ? "پیشنهاد تایید شد." : "پیشنهاد رد شد.");
    } catch (e: unknown) {
      showToast(`خطا در پردازش پیشنهاد: ${String(e)}`);
    }
  };

  const handleUpsertCalendarDay = async (params: {
    dateKey: string;
    totalCapacityMinutes: number;
    deepCapacityMinutes: number;
    dayNote: string;
    isOffDay: boolean;
  }) => {
    try {
      const updated = await CompassApi.upsertCalendarDay(params);
      setDash(updated);
      showToast(`ظرفیت روز ${params.dateKey} ذخیره شد.`);
    } catch (e: unknown) {
      showToast(`خطا در ذخیره تقویم: ${String(e)}`);
    }
  };

  return (
    <div
      dir="ltr"
      className="h-screen w-screen bg-[#0D0E11] text-[#F3F4F6] flex flex-col overflow-hidden select-none"
    >
      {/* Top Antigravity Frameless Titlebar */}
      <AntigravityTitlebar
        dash={dash}
        activeSessionTitle={activeSession?.title}
        isSidebarOpen={isSidebarOpen}
        isRightCanvasOpen={isRightCanvasOpen}
        canGoBack={tabHistoryIdx > 0}
        canGoForward={tabHistoryIdx + 1 < tabHistory.length}
        isSyncing={isSyncing}
        connectionStatus={connectionStatus}
        isCheckingConnection={isCheckingConnection}
        onToggleSidebar={() => setIsSidebarOpen((v) => !v)}
        onToggleRightCanvas={() => {
          setIsRightCanvasOpen((v) => {
            if (v) setIsRightCanvasMaximized(false);
            return !v;
          });
        }}
        onGoBack={handleGoBackTab}
        onGoForward={handleGoForwardTab}
        onRecheckConnection={() => void refreshConnection()}
        onUndo={() => void handleUndo()}
        onSyncMabFocus={() => void handleSyncMabFocus()}
        onExportExcel={() => void handleExportExcel()}
        onOpenLogs={() => openBuiltinView("LOGS")}
      />

      {/* Antigravity 3-Column LTR Workbench Body: Left Sidebar | Center Chat | Right Tabbed Canvas */}
      <div className="flex-1 flex min-h-0 overflow-hidden relative">
        {/* Column 1: Left Sidebar */}
        {isSidebarOpen && (
          <AntigravitySidebar
            dash={dash}
            width={sidebarWidth}
            activeCanvasTabId={isRightCanvasOpen ? activeCanvasTabId : null}
            isNarrowViewport={isNarrowViewport}
            onCloseMobile={() => setIsSidebarOpen(false)}
            onNewConversation={() =>
              void handleCreateChatSession(dash.active_project.id, activeChatChannel)
            }
            onOpenBuiltinView={openBuiltinView}
            onSwitchProject={(pid) => void handleSwitchProject(pid)}
            onOpenNewProject={() => setShowNewProjModal(true)}
            onCreateChatSession={(pid, ch) => void handleCreateChatSession(pid, ch)}
            onSwitchChatSession={(s) => void handleSwitchChatSession(s)}
            onDeleteChatSession={(sid) => void handleDeleteChatSession(sid)}
          />
        )}

        {/* Draggable Splitter 1: Sidebar <-> Main Workspace */}
        {isSidebarOpen && !isNarrowViewport && (
          <div
            role="separator"
            aria-orientation="vertical"
            title="Drag to resize sidebar (double-click to reset)"
            onMouseDown={(e) => {
              e.preventDefault();
              sidebarDragRef.current = {
                startX: e.clientX,
                startWidth: sidebarWidth,
              };
              setIsDraggingSidebar(true);
            }}
            onDoubleClick={() => setSidebarWidth(244)}
            className="w-[5px] -ml-[2px] -mr-[2px] z-20 shrink-0 cursor-col-resize flex items-center justify-center group"
          >
            <div
              className={`w-[2px] h-full transition-colors ${
                isDraggingSidebar
                  ? "bg-blue-500"
                  : "bg-transparent group-hover:bg-blue-500/70"
              }`}
            />
          </div>
        )}

        {/* Columns 2 & 3 Container: Center Chat + Right Tabbed Canvas */}
        <div
          ref={workspaceBodyRef}
          className="flex-1 flex min-w-0 min-h-0 overflow-hidden"
        >
          {/* Column 2: Center Chat Stream (Hidden when Right Canvas is Maximized or on narrow screens when Right Canvas is open) */}
          {(!isRightCanvasOpen || (!isRightCanvasMaximized && !isNarrowViewport)) && (
            <AntigravityChatPane
              dash={dash}
              activeChannel={activeChatChannel}
              onChangeChannel={setActiveChatChannel}
              activeSession={activeSession}
              chatHistory={activeChatHistory}
              chatInput={chatInput}
              onChangeChatInput={setChatInput}
              isSendingChat={isSendingChat}
              onSendChat={() => void handleSendChat()}
              onSendFiles={(files) => void handleSendChatFiles(files)}
              retryingMessageId={retryingMessageId}
              onRetrySystemMessage={(id) => void handleRetrySystemMessage(id)}
              onOpenArtifactTab={openArtifactTab}
              onOpenSettingsTab={() => openBuiltinView("SETTINGS")}
              onQuickChangeModel={(prov, mid, lbl) => void handleQuickChangeModel(prov, mid, lbl)}
              onOpenReviewTab={() => openBuiltinView("WBS")}
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
              onProcessSingleQueueItem={(id) => void handleProcessSingleQueueItem(id)}
              onReorderQueueItem={(id, dir) => void handleReorderQueueItem(id, dir)}
              onResetQueueOrder={() => void handleResetQueueOrder()}
              onDismissQueueItem={(id) => void handleDismissQueueItem(id)}
            />
          )}

          {/* Draggable Splitter 2: Center Chat <-> Right Tabbed Canvas */}
          {isRightCanvasOpen && !isRightCanvasMaximized && !isNarrowViewport && (
            <div
              role="separator"
              aria-orientation="vertical"
              title="Drag to resize chat & right canvas (double-click to reset)"
              onMouseDown={(e) => {
                e.preventDefault();
                setIsDraggingCanvas(true);
              }}
              onDoubleClick={() => setRightCanvasPct(48)}
              className="w-[5px] -ml-[2px] -mr-[2px] z-20 shrink-0 cursor-col-resize flex items-center justify-center group border-l border-white/[0.07]"
            >
              <div
                className={`w-[2px] h-full transition-colors ${
                  isDraggingCanvas
                    ? "bg-blue-500"
                    : "bg-transparent group-hover:bg-blue-500/70"
                }`}
              />
            </div>
          )}

          {/* Column 3: Right Tabbed Canvas */}
          {isRightCanvasOpen && (
            <div
              style={
                isRightCanvasMaximized || isNarrowViewport
                  ? { flex: 1 }
                  : { width: `${rightCanvasPct}%` }
              }
              className={`min-w-0 min-h-0 flex flex-col overflow-hidden ${
                isRightCanvasMaximized || isNarrowViewport ? "flex-1 w-full" : "min-w-[280px] shrink-0"
              }`}
            >
              <AntigravityRightCanvas
                dash={dash}
                settingsForm={settingsForm}
                tabs={canvasTabs}
                activeTabId={activeCanvasTabId}
                isMaximized={isRightCanvasMaximized}
                onSelectTab={activateTab}
                onCloseTab={closeCanvasTab}
                onOpenBuiltinView={openBuiltinView}
                onOpenArtifactTab={openArtifactTab}
                onOpenMethodTab={openMethodTab}
                onToggleMaximize={() => setIsRightCanvasMaximized((v) => !v)}
                onCloseCanvas={() => {
                  setIsRightCanvasOpen(false);
                  setIsRightCanvasMaximized(false);
                }}
                checkedGates={checkedGates}
                onToggleGate={(id, checked) =>
                  setCheckedGates({ ...checkedGates, [id]: checked })
                }
                allGatesPassed={allGatesPassed}
                onQuickComplete={(tid, p) => void handleQuickComplete(tid, p)}
                onStartTask={(tid) => void handleStartTask(tid)}
                onInsertChatPrompt={(text) => {
                  setChatInput((prev) =>
                    prev.trim() ? `${prev.trimEnd()}\n${text}` : text
                  );
                  if (isRightCanvasMaximized) setIsRightCanvasMaximized(false);
                }}
                onUpsertMethod={handleUpsertMethod}
                onDeleteMethod={handleDeleteMethod}
                onOpenNewOutcome={() => {
                  setEditingOutcome(null);
                  setShowOutcomeModal(true);
                }}
                onEditOutcome={(o) => {
                  setEditingOutcome(o);
                  setShowOutcomeModal(true);
                }}
                onDeleteOutcome={(id) => void handleDeleteOutcome(id)}
                onOpenNewWorkLine={() => {
                  setEditingWorkLine(null);
                  setShowWorkLineModal(true);
                }}
                onEditWorkLine={(wl) => {
                  setEditingWorkLine(wl);
                  setShowWorkLineModal(true);
                }}
                onOpenNewTask={() => {
                  setEditingTask(null);
                  setShowTaskModal(true);
                }}
                onOpenAssignTask={(t) => {
                  setEditingTask(t);
                  setShowTaskModal(true);
                }}
                docQueueStatus={docQueueStatus}
                onBatchFilesSelected={handleBatchFilesSelected}
                onBatchFilesDropped={(files) => void processBatchVaultFiles(files)}
                onManualDocIngest={handleManualDocIngest}
                onRequeueDoc={(docId) => void handleRequeueDoc(docId)}
                onDashboardUpdated={setDash}
                onToast={showToast}
                onCreateVaultCard={handleCreateVaultCard}
                onDeleteVaultCard={(id) => void handleDeleteVaultCard(id)}
                onResolveProposal={(id, ok) => void handleResolveProposal(id, ok)}
                onUpsertCalendarDay={handleUpsertCalendarDay}
                onChangeSettings={setSettingsForm}
                onSaveSettings={(override) => void handleSaveSettings(override)}
              />
            </div>
          )}
        </div>
      </div>

      {/* Modals (RTL inside modals) */}
      <div dir="rtl">
        <NewProjectModal
          open={showNewProjModal}
          onClose={() => setShowNewProjModal(false)}
          onCreate={handleCreateProject}
        />

        <OutcomeModal
          open={showOutcomeModal}
          domains={dash.domains ?? []}
          initialOutcome={editingOutcome}
          onClose={() => setShowOutcomeModal(false)}
          onSubmit={handleUpsertOutcome}
        />

        <WorkLineModal
          open={showWorkLineModal}
          outcomes={dash.outcomes ?? []}
          initialWorkLine={editingWorkLine}
          onClose={() => setShowWorkLineModal(false)}
          onCreate={handleCreateWorkLine}
          onUpdate={handleUpdateWorkLine}
        />

        <TaskModal
          open={showTaskModal}
          workLines={dash.work_lines ?? []}
          outcomes={dash.outcomes ?? []}
          editingTask={editingTask}
          onClose={() => setShowTaskModal(false)}
          onCreate={handleCreateManualTask}
          onAssign={handleAssignTaskGrouping}
        />
      </div>

      {/* Toast Notification */}
      {toastMsg && (
        <div
          dir="rtl"
          className="fixed bottom-4 right-4 max-w-md bg-[#18191E] border border-blue-500/40 text-white px-4 py-3 rounded-2xl text-[12.5px] font-medium z-50 shadow-2xl"
        >
          {toastMsg}
        </div>
      )}
    </div>
  );
}
