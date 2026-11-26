import { useMemo, useState } from "react";
import {
  AlertTriangle,
  BookOpen,
  Calendar,
  CheckCircle2,
  Edit3,
  FolderOpen,
  GitBranch,
  Layers,
  Play,
  ShieldAlert,
  Sparkles,
  Target,
  Workflow,
  Zap,
} from "lucide-react";
import { DashboardState, Outcome, WorkLine } from "../lib/tauri_api";
import { formatPomos } from "../lib/formatters";
import { MabCheckbox } from "./ui/MabCheckbox";

interface PathAheadPanelProps {
  dash: DashboardState;
  checkedGates: Record<number, boolean>;
  onToggleGate: (gateId: number, checked: boolean) => void;
  allGatesPassed: boolean;
  onQuickComplete: (taskId: string, pomosSpent: number) => void;
  onStartTask?: (taskId: string) => void;
  onInsertChatPrompt: (text: string) => void;
  onSwitchProject?: (projectId: string) => void;
  onOpenWorkLineBpmn?: (workLine: WorkLine) => void;
  onOpenProjectBpmn?: () => void;
}

export function PathAheadPanel({
  dash,
  checkedGates,
  onToggleGate,
  allGatesPassed,
  onQuickComplete,
  onStartTask,
  onInsertChatPrompt,
  onSwitchProject,
  onOpenWorkLineBpmn,
  onOpenProjectBpmn,
}: PathAheadPanelProps) {
  const portfolioProjects = dash.portfolio_projects ?? [];
  const crossPriorities = dash.cross_project_priorities ?? [];
  const hasMultipleProjects = portfolioProjects.length > 1;
  const [viewMode, setViewMode] = useState<"PORTFOLIO" | "ACTIVE_PROJECT">(() =>
    dash.active_chat_scope === "GLOBAL" && portfolioProjects.length > 1
      ? "PORTFOLIO"
      : "ACTIVE_PROJECT"
  );
  const projectColorMap = useMemo(() => {
    const map = new Map<string, string>();
    for (const p of dash.projects ?? []) {
      map.set(p.id, p.color || "#3B82F6");
    }
    return map;
  }, [dash.projects]);
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

  return (
    <div dir="rtl" className="h-full overflow-y-auto p-4 space-y-4">
      {/* Portfolio / Project Mode Bar */}
      {hasMultipleProjects && (
        <div className="flex items-center justify-between gap-2 flex-wrap bg-[#121318] border border-white/[0.07] rounded-xl p-1.5">
          <div className="flex items-center gap-1">
            <button
              type="button"
              onClick={() => setViewMode("PORTFOLIO")}
              className={`px-3 py-1.5 rounded-lg text-[12px] font-semibold flex items-center gap-1.5 transition ${
                viewMode === "PORTFOLIO"
                  ? "bg-cyan-500/20 border border-cyan-500/35 text-cyan-200"
                  : "text-white/60 hover:text-white hover:bg-white/[0.04]"
              }`}
            >
              <Layers className="w-3.5 h-3.5" />
              <span>همه پروژه‌ها (لایه راهبردی)</span>
            </button>
            <button
              type="button"
              onClick={() => setViewMode("ACTIVE_PROJECT")}
              className={`px-3 py-1.5 rounded-lg text-[12px] font-semibold flex items-center gap-1.5 transition ${
                viewMode === "ACTIVE_PROJECT"
                  ? "bg-blue-500/20 border border-blue-500/35 text-blue-200"
                  : "text-white/60 hover:text-white hover:bg-white/[0.04]"
              }`}
            >
              <FolderOpen className="w-3.5 h-3.5" />
              <span className="truncate max-w-[140px]">{dash.active_project.name}</span>
            </button>
          </div>

          <button
            type="button"
            onClick={() =>
              onInsertChatPrompt(
                "مسیر پیش‌رو در همه پروژه‌های فعالم را بررسی کن، گلوگاه‌ها را بگو و اولویت‌های امروز را بین پروژه‌ها تنظیم کن."
              )
            }
            className="px-2.5 py-1.5 rounded-lg bg-cyan-600/20 hover:bg-cyan-600/35 border border-cyan-500/30 text-cyan-200 text-[11.5px] font-medium flex items-center gap-1.5 transition"
          >
            <Sparkles className="w-3.5 h-3.5" />
            <span>اولویت‌بندی با مدل</span>
          </button>
        </div>
      )}

      {viewMode === "PORTFOLIO" && hasMultipleProjects ? (
        <div className="space-y-4">
          {/* Cross-Project Top Prioritized Queue */}
          <div className="mab-card p-4 space-y-3 border-cyan-500/25 bg-cyan-500/[0.02]">
            <div className="flex items-center justify-between gap-2 flex-wrap">
              <div className="flex items-center gap-2 text-cyan-300 font-bold text-[13px]">
                <Layers className="w-4 h-4 shrink-0" />
                <span>صف اولویت‌های برتر در کل پروژه‌ها</span>
              </div>
              <span className="font-mono text-[11px] text-white/50">
                {crossPriorities.length} گام باز
              </span>
            </div>

            {crossPriorities.length > 0 ? (
              <div className="space-y-2">
                {crossPriorities.slice(0, 12).map((task, idx) => {
                  const pColor = projectColorMap.get(task.project_id) || "#3B82F6";
                  const isInProgress = task.status === "IN_PROGRESS";
                  return (
                    <div
                      key={`${task.project_id}-${task.id}`}
                      className="p-3 rounded-xl bg-[#131419] border border-white/[0.08] hover:border-white/[0.16] transition space-y-2"
                      style={{ borderRightWidth: "3px", borderRightColor: pColor }}
                    >
                      <div className="flex items-center justify-between gap-2 flex-wrap">
                        <div className="flex items-center gap-1.5 flex-wrap min-w-0">
                          <span className="w-5 h-5 rounded-md bg-white/[0.06] text-white/75 font-mono text-[10.5px] font-bold flex items-center justify-center shrink-0">
                            {idx + 1}
                          </span>
                          <button
                            type="button"
                            onClick={() => {
                              onSwitchProject?.(task.project_id);
                              setViewMode("ACTIVE_PROJECT");
                            }}
                            className="px-2 py-0.5 rounded-full text-[10.5px] font-semibold border shrink-0 hover:brightness-125 transition"
                            style={{
                              color: pColor,
                              borderColor: `${pColor}55`,
                              backgroundColor: `${pColor}1A`,
                            }}
                          >
                            {task.project_name || task.project_id}
                          </button>
                          <span className="px-1.5 py-0.5 rounded bg-amber-500/15 border border-amber-500/30 text-amber-300 font-mono text-[10px] font-semibold">
                            P{task.priority_score}
                          </span>
                          {isInProgress && (
                            <span className="px-1.5 py-0.5 rounded-full bg-emerald-500/15 text-emerald-300 text-[10px] font-semibold">
                              در حال اجرا
                            </span>
                          )}
                          {task.is_on_critical_path && (
                            <span className="px-1.5 py-0.5 rounded bg-rose-500/15 text-rose-300 text-[10px]">
                              بحرانی
                            </span>
                          )}
                          {task.scheduled_date && (
                            <span className="px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-300 font-mono text-[10px] inline-flex items-center gap-1">
                              <Calendar className="w-2.5 h-2.5" />
                              {task.scheduled_date}
                            </span>
                          )}
                        </div>

                        <div className="flex items-center gap-1.5 shrink-0">
                          {!isInProgress && onStartTask && (
                            <button
                              type="button"
                              onClick={() => onStartTask(task.id)}
                              className="px-2 py-1 rounded-lg bg-blue-600/80 hover:bg-blue-500 text-white text-[10.5px] font-medium inline-flex items-center gap-1 transition"
                            >
                              <Play className="w-2.5 h-2.5" />
                              <span>شروع</span>
                            </button>
                          )}
                          <button
                            type="button"
                            onClick={() =>
                              onQuickComplete(task.id, Math.round(task.calibrated_pomos) || 1)
                            }
                            className="px-2 py-1 rounded-lg bg-emerald-600/20 hover:bg-emerald-600/35 border border-emerald-500/35 text-emerald-300 text-[10.5px] font-medium inline-flex items-center gap-1 transition"
                          >
                            <CheckCircle2 className="w-2.5 h-2.5" />
                            <span>تکمیل</span>
                          </button>
                          <button
                            type="button"
                            onClick={() =>
                              onInsertChatPrompt(
                                `درباره تسک [${task.id}] در پروژه «${task.project_name || task.project_id}»: `
                              )
                            }
                            className="p-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] text-white/60 hover:text-white transition"
                            title="اصلاح یا اولویت‌دهی در چت"
                          >
                            <Edit3 className="w-3 h-3" />
                          </button>
                        </div>
                      </div>

                      <div className="text-[13px] font-semibold text-white">
                        <span className="font-mono text-blue-400 ml-1.5 text-[11.5px]">
                          [{task.id}]
                        </span>
                        {task.title}
                      </div>
                      {task.first_physical_action && (
                        <p className="text-[11.5px] text-white/55 truncate">
                          اقدام اول: {task.first_physical_action}
                        </p>
                      )}
                    </div>
                  );
                })}
              </div>
            ) : (
              <p className="text-[12px] text-mab-text-muted py-2">
                هنوز تسک فعالی در پروژه‌ها ثبت نشده است؛ از دکمه «اولویت‌بندی با مدل» استفاده کنید.
              </p>
            )}
          </div>

          {/* Per-Project Path Ahead Overview Cards */}
          <div className="space-y-2.5">
            <div className="text-[12.5px] font-bold text-white/80 px-0.5">
              وضعیت مسیر پیش‌رو به تفکیک هر پروژه ({portfolioProjects.length} پروژه)
            </div>
            <div className="grid grid-cols-1 gap-2.5">
              {portfolioProjects.map((summary) => {
                const p = summary.project;
                const pColor = p.color || "#3B82F6";
                return (
                  <div
                    key={p.id}
                    className="mab-card p-3.5 space-y-2 hover:border-white/[0.16] transition"
                    style={{ borderRightWidth: "3px", borderRightColor: pColor }}
                  >
                    <div className="flex items-center justify-between gap-2 flex-wrap">
                      <button
                        type="button"
                        onClick={() => {
                          onSwitchProject?.(p.id);
                          setViewMode("ACTIVE_PROJECT");
                        }}
                        className="flex items-center gap-2 text-right min-w-0 group"
                      >
                        <span
                          className="w-2.5 h-2.5 rounded-full shrink-0"
                          style={{ backgroundColor: pColor }}
                        />
                        <span className="text-[13.5px] font-bold text-white group-hover:text-blue-300 transition truncate">
                          {p.name}
                        </span>
                        {p.lifecycle_status === "PAUSED" && (
                          <span className="px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300 text-[10px]">
                            معلق
                          </span>
                        )}
                      </button>

                      <div className="flex items-center gap-1.5 text-[10.5px] shrink-0">
                        <span className="px-2 py-0.5 rounded bg-white/[0.05] text-white/70 font-mono">
                          {summary.open_tasks_count} باز
                        </span>
                        {summary.blockers.length > 0 && (
                          <span className="px-2 py-0.5 rounded bg-rose-500/15 text-rose-300 inline-flex items-center gap-1">
                            <AlertTriangle className="w-3 h-3" />
                            {summary.blockers.length} مانع
                          </span>
                        )}
                        <button
                          type="button"
                          onClick={() => {
                            onSwitchProject?.(p.id);
                            setViewMode("ACTIVE_PROJECT");
                          }}
                          className="px-2 py-0.5 rounded bg-blue-500/15 hover:bg-blue-500/25 text-blue-300 transition"
                        >
                          جزئیات پروژه
                        </button>
                      </div>
                    </div>

                    {summary.next_step_task ? (
                      <div className="p-2.5 rounded-lg bg-white/[0.03] border border-white/[0.06] flex items-center justify-between gap-2">
                        <div className="min-w-0">
                          <div className="text-[12px] font-medium text-white truncate">
                            <span className="font-mono text-emerald-400 ml-1.5">
                              [{summary.next_step_task.id}]
                            </span>
                            {summary.next_step_task.title}
                          </div>
                          {summary.horizon_tasks.length > 0 && (
                            <div className="text-[11px] text-white/45 truncate mt-0.5">
                              بعدی: {summary.horizon_tasks.map((h) => h.title).join(" ← ")}
                            </div>
                          )}
                        </div>
                        <span className="px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-300 font-mono text-[10px] shrink-0">
                          P{summary.next_step_task.priority_score}
                        </span>
                      </div>
                    ) : (
                      <div className="text-[11.5px] text-white/40 px-1">
                        گام فعالی ثبت نشده است.
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      ) : (
        <>
      {/* 1. Pre-flight Guardrail Gate */}
      {dash.active_guardrails.length > 0 && (
        <div className="mab-card p-4 space-y-3 bg-rose-500/[0.04] border-rose-500/20">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-rose-400 font-bold text-[13px]">
              <ShieldAlert className="w-4 h-4 shrink-0" />
              <span>گیت پیشگیرانه ضدتکرار خطا (بررسی قبل از شروع)</span>
            </div>
            <span className="px-2 py-0.5 rounded-full bg-rose-500/15 text-rose-300 font-mono text-[10.5px]">
              {Object.values(checkedGates).filter(Boolean).length} از{" "}
              {dash.active_guardrails.length} تایید شده
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
        <div className="flex items-center justify-between gap-2 px-0.5 flex-wrap">
          <div className="flex items-center gap-2">
            <span className="px-2.5 py-0.5 rounded-full bg-blue-500/15 border border-blue-500/30 text-blue-400 text-[11.5px] font-bold flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-blue-400 animate-pulse" />
              <span>۳ اولویت برتر اجرایی (لاین‌های کاری فعال)</span>
            </span>
          </div>

          <div className="flex items-center gap-2">
            {onOpenProjectBpmn && (
              <button
                type="button"
                onClick={onOpenProjectBpmn}
                className="px-2.5 py-1 rounded-lg bg-purple-500/15 hover:bg-purple-500/25 border border-purple-500/30 text-purple-300 text-[11px] font-medium inline-flex items-center gap-1 transition"
              >
                <Workflow className="w-3 h-3" />
                <span>نقشه BPMN پروژه</span>
              </button>
            )}
            <span className="font-timer text-[11px] text-mab-text-muted">
              {topPrioritySteps.length} از ۳ اولویت
            </span>
          </div>
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

                      <button
                        type="button"
                        onClick={() => wl && onOpenWorkLineBpmn?.(wl)}
                        disabled={!wl || !onOpenWorkLineBpmn}
                        className="px-2 py-0.5 rounded-full text-[11px] font-semibold border flex items-center gap-1.5 shrink-0 hover:brightness-125 transition disabled:cursor-default"
                        style={{
                          color: lineColor,
                          borderColor: `${lineColor}55`,
                          backgroundColor: `${lineColor}1A`,
                        }}
                        title={wl ? "کلیک برای مشاهده ورک‌فلوی BPMN این لاین" : undefined}
                      >
                        <GitBranch className="w-3 h-3" />
                        <span>
                          {wl?.title || step.work_line_title || "تسک مستقل"}
                        </span>
                      </button>

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
                          onInsertChatPrompt(`گام [${step.id}] رو اصلاح کن: `)
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

                  <div
                    className={`font-bold text-white leading-snug ${
                      isPrimary ? "text-[16px]" : "text-[14.5px]"
                    }`}
                  >
                    <span className="font-mono text-blue-400 ml-2 font-semibold">
                      [{step.id}]
                    </span>
                    {step.title}
                  </div>

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
              در پنل چت وسط بنویس قصد داری چه کاری انجام دهی تا لاین‌های کاری و ۳ اولویت برتر تولید شود.
            </p>
          </div>
        )}
      </div>

      {/* 3. Next Steps Horizon */}
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
                      onInsertChatPrompt(
                        `گام [${ht.id}] را بگذار گام اول مسیر پیش‌رو و این‌طور اصلاحش کن: `
                      )
                    }
                    className="px-2.5 py-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-mab-border text-[11px] text-white/70 hover:text-white transition opacity-80 group-hover:opacity-100"
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
            {relevantSops.slice(0, 4).map((s) => (
              <div
                key={s.id}
                className="p-2.5 rounded-lg bg-white/[0.02] border border-mab-border flex items-center gap-2 text-[12px]"
              >
                <span className="px-1.5 py-0.5 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px] shrink-0">
                  {s.category}
                </span>
                <span className="text-mab-text-secondary select-text">
                  {s.rule_statement}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
        </>
      )}
    </div>
  );
}

export default PathAheadPanel;
