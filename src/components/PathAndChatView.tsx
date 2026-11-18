import { useMemo } from "react";
import {
  BookOpen,
  CheckCircle2,
  Edit3,
  GitBranch,
  Play,
  ShieldAlert,
  Target,
  Zap,
} from "lucide-react";
import { DashboardState, Outcome, WorkLine } from "../lib/tauri_api";
import { formatPomos } from "../lib/formatters";
import { CopilotChatDock } from "./chat/CopilotChatDock";
import { MabCheckbox } from "./ui/MabCheckbox";
import { ResizableSplitPane } from "./ui/ResizableSplitPane";

interface PathAndChatViewProps {
  dash: DashboardState;
  checkedGates: Record<number, boolean>;
  onToggleGate: (gateId: number, checked: boolean) => void;
  allGatesPassed: boolean;
  onQuickComplete: (taskId: string, pomosSpent: number) => void;
  onStartTask?: (taskId: string) => void;
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
  onStartTask,
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
  const workLineMap = useMemo(() => {
    const map = new Map<string, WorkLine>();
    for (const wl of dash.work_lines ?? []) {
      map.set(wl.id, wl);
    }
    return map;
  }, [dash.work_lines]);

  const outcomeMap = useMemo(() => {
    const map = new Map<string, Outcome>();
    for (const o of dash.outcomes ?? []) {
      map.set(o.id, o);
    }
    return map;
  }, [dash.outcomes]);

  const topPrioritySteps = useMemo(() => {
    if (dash.top_priority_steps && dash.top_priority_steps.length > 0) {
      return dash.top_priority_steps.slice(0, 3);
    }
    return dash.next_step_task ? [dash.next_step_task] : [];
  }, [dash.top_priority_steps, dash.next_step_task]);

  const topPriorityIds = useMemo(
    () => new Set(topPrioritySteps.map((t) => t.id)),
    [topPrioritySteps]
  );

  const remainingHorizonTasks = useMemo(
    () => dash.horizon_tasks.filter((ht) => !topPriorityIds.has(ht.id)),
    [dash.horizon_tasks, topPriorityIds]
  );

  const activeCategory =
    topPrioritySteps[0]?.category ?? dash.next_step_task?.category;
  const relevantSops = activeCategory
    ? dash.active_sops.filter(
        (s) => s.category === activeCategory || s.category === "GENERAL"
      )
    : dash.active_sops.slice(0, 3);

  const copilotSessions = dash.chat_sessions.filter((s) => s.channel !== "VAULT");
  const activeSession =
    copilotSessions.find((s) => s.id === dash.active_copilot_session_id) ??
    copilotSessions[0];

  const activeStepId = topPrioritySteps[0]?.id ?? dash.next_step_task?.id;
  const firstHorizonId =
    remainingHorizonTasks[0]?.id ?? dash.horizon_tasks[0]?.id;

  const insertPromptStarter = (starter: string) => {
    const nextVal = chatInput.trim() ? `${chatInput.trimEnd()}\n${starter}` : starter;
    onChangeChatInput(nextVal);
  };

  /* -------------------------------------------------------------------------- */
  /* Full Chat Mode Return                                                      */
  /* -------------------------------------------------------------------------- */
  if (isFullChatMode) {
    return (
      <div className="flex flex-col gap-2.5 h-full min-h-0">
        {/* Compact Path Bar on top */}
        <div className="mab-card px-4 py-2 border-mab-border flex items-center justify-between gap-3 flex-wrap shrink-0">
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

        <CopilotChatDock
          channel="COPILOT"
          activeSession={activeSession}
          sessions={copilotSessions}
          onSwitchSession={onSwitchSession}
          onNewChat={onNewChat}
          chatHistory={dash.chat_history}
          chatInput={chatInput}
          onChangeChatInput={onChangeChatInput}
          isSendingChat={isSendingChat}
          onSendChat={onSendChat}
          onSendFiles={onSendCopilotFiles}
          isFullChatMode={isFullChatMode}
          onToggleFullChatMode={onToggleFullChatMode}
          rawEventsCount={dash.raw_events_count}
          recentRawEvents={dash.recent_raw_events}
          retryingMessageId={retryingMessageId}
          onRetrySystemMessage={onRetrySystemMessage}
          onOpenSettings={onOpenSettings}
          activeStepId={activeStepId}
          firstHorizonId={firstHorizonId}
          projectName={dash.active_project.name}
        />
      </div>
    );
  }

  /* -------------------------------------------------------------------------- */
  /* Main VS Code–Style Resizable Split View                                    */
  /* -------------------------------------------------------------------------- */
  return (
    <ResizableSplitPane
      storageKey="mab_compass_path_split_pct"
      defaultPrimaryPercent={56}
      minPrimaryPercent={28}
      maxPrimaryPercent={74}
      primary={
        <div className="flex flex-col gap-3.5 overflow-y-auto pl-1 min-h-0 h-full">
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
                        : "bg-mab-surface border-mab-border hover:border-white/[0.12] hover:bg-mab-hover"
                    }`}
                  >
                    <MabCheckbox
                      checked={isChecked}
                      onChange={(nextChecked) => onToggleGate(g.id, nextChecked)}
                      className="mt-0.5"
                    />

                    <div className="text-[12.5px] leading-relaxed flex-1">
                      <p
                        className={`font-semibold transition-colors ${
                          isChecked ? "text-white/70 line-through" : "text-white"
                        }`}
                      >
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

        {/* 2. Top 3 Priorities across Active WorkLines */}
        <div className="space-y-3">
          <div className="flex items-center justify-between gap-2 px-0.5">
            <div className="flex items-center gap-2">
              <span className="px-2.5 py-0.5 rounded-full bg-blue-500/15 border border-blue-500/30 text-blue-400 text-[11.5px] font-bold flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse" />
                <span>۳ اولویت برتر اجرایی (لاین‌های کاری فعال)</span>
              </span>
            </div>
            <span className="font-timer text-[11px] text-mab-text-muted">
              {topPrioritySteps.length} از ۳ اولویت
            </span>
          </div>

          {topPrioritySteps.length > 0 ? (
            <div className="space-y-3">
              {topPrioritySteps.map((step, idx) => {
                const wl = step.work_line_id
                  ? workLineMap.get(step.work_line_id)
                  : undefined;
                const lineColor = wl?.color_hex || "#3B82F6";
                const effectiveOutcomeId =
                  step.outcome_id || wl?.outcome_id || null;
                const outcome = effectiveOutcomeId
                  ? outcomeMap.get(effectiveOutcomeId)
                  : undefined;
                const isInProgress = step.status === "IN_PROGRESS";
                const isPrimary = idx === 0;

                return (
                  <div
                    key={step.id}
                    className={`mab-card p-4 space-y-3 transition ${
                      isPrimary
                        ? "border-blue-500/35"
                        : "border-mab-border hover:border-white/[0.14]"
                    }`}
                    style={{
                      borderRightWidth: "3px",
                      borderRightColor: lineColor,
                    }}
                  >
                    {/* Card Header: Priority #, WorkLine Badge, Outcome Badge, Status & Pomo */}
                    <div className="flex items-center justify-between gap-2 flex-wrap">
                      <div className="flex items-center gap-1.5 flex-wrap min-w-0">
                        <span
                          className="w-5 h-5 rounded-md font-mono text-[11px] font-bold flex items-center justify-center shrink-0"
                          style={{
                            color: lineColor,
                            backgroundColor: `${lineColor}22`,
                            border: `1px solid ${lineColor}55`,
                          }}
                        >
                          {idx + 1}
                        </span>

                        {/* WorkLine Pill */}
                        <span
                          className="px-2 py-0.5 rounded-full text-[11px] font-semibold border flex items-center gap-1.5 shrink-0"
                          style={{
                            color: lineColor,
                            borderColor: `${lineColor}55`,
                            backgroundColor: `${lineColor}1A`,
                          }}
                        >
                          <GitBranch className="w-3 h-3" />
                          <span>
                            {wl?.title ||
                              step.work_line_title ||
                              "تسک مستقل"}
                          </span>
                        </span>

                        {/* Outcome Pill */}
                        {(outcome || effectiveOutcomeId) && (
                          <span className="px-2 py-0.5 rounded-full bg-amber-500/10 border border-amber-500/25 text-amber-300 text-[10.5px] font-medium flex items-center gap-1 max-w-[200px] truncate">
                            <Target className="w-3 h-3 shrink-0" />
                            <span className="truncate">
                              {outcome?.title || effectiveOutcomeId}
                            </span>
                          </span>
                        )}

                        <span className="px-2 py-0.5 rounded-md bg-purple-500/15 text-purple-300 font-mono text-[10.5px]">
                          {step.category}
                        </span>

                        {isInProgress ? (
                          <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 border border-emerald-500/35 text-emerald-300 text-[10px] font-bold flex items-center gap-1">
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                            <span>در حال اجرا</span>
                          </span>
                        ) : (
                          <span className="px-2 py-0.5 rounded-full bg-blue-500/10 border border-blue-500/25 text-blue-300 font-mono text-[10px]">
                            OPEN
                          </span>
                        )}
                      </div>

                      <div className="flex items-center gap-2 shrink-0">
                        <button
                          type="button"
                          onClick={() =>
                            insertPromptStarter(
                              `گام [${step.id}] رو اصلاح کن: `
                            )
                          }
                          className="px-2 py-0.5 rounded-full bg-white/[0.04] hover:bg-white/[0.08] border border-mab-border text-[10.5px] text-white/70 hover:text-white flex items-center gap-1 transition"
                        >
                          <Edit3 className="w-3 h-3" />
                          <span>اصلاح در چت</span>
                        </button>
                        <span className="px-2 py-0.5 rounded-md bg-amber-500/15 border border-amber-500/30 text-amber-300 font-mono text-[10.5px] font-semibold">
                          {formatPomos(step.calibrated_pomos)} Pomo
                        </span>
                      </div>
                    </div>

                    {/* Title */}
                    <div
                      className={`font-bold text-white leading-snug ${
                        isPrimary
                          ? "text-[16.5px] sm:text-[17.5px]"
                          : "text-[14.5px]"
                      }`}
                    >
                      <span className="font-mono text-blue-400 ml-2 font-semibold">
                        [{step.id}]
                      </span>
                      {step.title}
                    </div>

                    {/* First 2-minute physical action */}
                    <div className="p-3 rounded-xl bg-emerald-500/[0.06] border border-emerald-500/25 space-y-1">
                      <div className="text-[11.5px] font-bold text-emerald-400 flex items-center gap-1.5">
                        <Zap className="w-3.5 h-3.5" />
                        <span>اولین اقدام فیزیکی ۲ دقیقه‌ای:</span>
                      </div>
                      <p className="text-[13px] font-semibold text-white leading-relaxed select-text">
                        {step.first_physical_action ||
                          "فایل مربوطه را باز کرده و اسکلت کار را بنویسید."}
                      </p>
                    </div>

                    {/* Definition of Done (always shown on primary, compact on secondary) */}
                    {step.definition_of_done && (
                      <div className="p-3 rounded-xl bg-white/[0.02] border border-mab-border space-y-1">
                        <div className="text-[11.5px] font-bold text-blue-400 flex items-center gap-1.5">
                          <CheckCircle2 className="w-3.5 h-3.5" />
                          <span>شرط خروج قطعی (Definition of Done):</span>
                        </div>
                        <p className="text-[12.5px] text-mab-text-secondary leading-relaxed select-text">
                          {step.definition_of_done}
                        </p>
                      </div>
                    )}

                    {/* Action Row: Complete for IN_PROGRESS, Start for OPEN */}
                    <div className="pt-0.5 flex items-center gap-2">
                      {isInProgress ? (
                        <button
                          type="button"
                          disabled={!allGatesPassed}
                          onClick={() =>
                            onQuickComplete(
                              step.id,
                              Math.round(step.calibrated_pomos) || 1
                            )
                          }
                          className={`flex-1 py-2 rounded-xl font-bold text-[12.5px] flex items-center justify-center gap-2 transition shadow-sm ${
                            allGatesPassed
                              ? "bg-emerald-600 hover:bg-emerald-500 text-white active:scale-[0.99]"
                              : "bg-white/[0.04] border border-mab-border text-white/40 cursor-not-allowed"
                          }`}
                        >
                          <CheckCircle2 className="w-4 h-4" />
                          <span>
                            {allGatesPassed
                              ? "تکمیل گام • ثبت در سابقه و باز کردن گام بعدی لاین"
                              : "ابتدا چک‌باکس‌های سد پیشگیرانه بالا را تیک بزنید"}
                          </span>
                        </button>
                      ) : (
                        <>
                          {onStartTask && (
                            <button
                              type="button"
                              onClick={() => onStartTask(step.id)}
                              className="flex-1 py-2 rounded-xl bg-blue-600/90 hover:bg-blue-500 text-white font-bold text-[12.5px] flex items-center justify-center gap-2 transition shadow-sm active:scale-[0.99]"
                            >
                              <Play className="w-3.5 h-3.5" />
                              <span>شروع گام (انتقال به در حال اجرا)</span>
                            </button>
                          )}
                          <button
                            type="button"
                            disabled={!allGatesPassed}
                            onClick={() =>
                              onQuickComplete(
                                step.id,
                                Math.round(step.calibrated_pomos) || 1
                              )
                            }
                            className={`px-3.5 py-2 rounded-xl font-semibold text-[12px] flex items-center justify-center gap-1.5 transition ${
                              allGatesPassed
                                ? "bg-emerald-600/20 hover:bg-emerald-600/35 border border-emerald-500/35 text-emerald-300"
                                : "bg-white/[0.04] border border-mab-border text-white/40 cursor-not-allowed"
                            }`}
                          >
                            <CheckCircle2 className="w-3.5 h-3.5" />
                            <span>تکمیل</span>
                          </button>
                        </>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="mab-card p-6 text-center space-y-2 border-blue-500/30">
              <p className="text-[14px] font-bold text-white">
                مسیر پیش‌رو هنوز چیده نشده است
              </p>
              <p className="text-[12.5px] text-mab-text-muted">
                در پنل چت همیار بنویس قصد داری چه کاری انجام دهی تا لاین‌های کاری و ۳ اولویت برتر تولید شود.
              </p>
            </div>
          )}
        </div>

        {/* 3. Next Steps Horizon (Remaining unblocked tasks beyond Top 3) */}
        <div className="mab-card p-4 space-y-3">
          <div className="flex items-center justify-between gap-2">
            <span className="text-[13.5px] font-bold text-white">
              سایر گام‌های افق پروژه
            </span>
            <span className="px-2 py-0.5 rounded-full bg-white/[0.04] border border-mab-border text-white/50 text-[11px] font-timer">
              {remainingHorizonTasks.length} گام در صف
            </span>
          </div>

          {remainingHorizonTasks.length > 0 ? (
            <div className="space-y-1.5">
              {remainingHorizonTasks.map((ht, idx) => (
                <div
                  key={ht.id}
                  className="p-3 rounded-xl bg-white/[0.02] hover:bg-white/[0.04] border border-mab-border flex items-center justify-between gap-3 transition group"
                >
                  <div className="flex items-center gap-2.5 min-w-0">
                    <span className="w-5 h-5 rounded-md bg-white/[0.06] text-white/60 font-mono text-[10.5px] font-bold flex items-center justify-center shrink-0">
                      {idx + topPrioritySteps.length + 1}
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
                      className="px-2.5 py-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-mab-border text-[11px] text-white/70 hover:text-white transition opacity-80 group-hover:opacity-100"
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
              گام دیگری خارج از ۳ اولویت برتر در افق نزدیک این پروژه ثبت نشده است.
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
                  className="p-2.5 rounded-lg bg-white/[0.02] border border-mab-border flex items-center gap-2 text-[12px]"
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

      }
      secondary={
        <div className="flex flex-col min-h-[460px] lg:min-h-0 lg:h-full">
          <CopilotChatDock
            channel="COPILOT"
            activeSession={activeSession}
            sessions={copilotSessions}
            onSwitchSession={onSwitchSession}
            onNewChat={onNewChat}
            chatHistory={dash.chat_history}
            chatInput={chatInput}
            onChangeChatInput={onChangeChatInput}
            isSendingChat={isSendingChat}
            onSendChat={onSendChat}
            onSendFiles={onSendCopilotFiles}
            isFullChatMode={isFullChatMode}
            onToggleFullChatMode={onToggleFullChatMode}
            rawEventsCount={dash.raw_events_count}
            recentRawEvents={dash.recent_raw_events}
            retryingMessageId={retryingMessageId}
            onRetrySystemMessage={onRetrySystemMessage}
            onOpenSettings={onOpenSettings}
            activeStepId={activeStepId}
            firstHorizonId={firstHorizonId}
            projectName={dash.active_project.name}
          />
        </div>
      }
    />
  );
}

export default PathAndChatView;
