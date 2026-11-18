import { useMemo, useState } from "react";
import {
  BookOpen,
  CheckCircle2,
  ChevronDown,
  ChevronLeft,
  Clock,
  Edit3,
  FileText,
  Filter,
  GitBranch,
  Layers,
  Lock,
  MoveRight,
  Play,
  Plus,
  Search,
  ShieldAlert,
  Target,
  Zap,
} from "lucide-react";
import {
  DashboardState,
  Outcome,
  WorkEntity,
  WorkLine,
} from "../lib/tauri_api";
import { formatPomos, formatShortDate } from "../lib/formatters";

interface WbsAndDocsViewProps {
  dash: DashboardState;
  onQuickComplete: (taskId: string, pomosSpent: number) => void;
  onStartTask?: (taskId: string) => void;
  onOpenNewOutcome?: () => void;
  onEditOutcome?: (outcome: Outcome) => void;
  onOpenNewWorkLine?: () => void;
  onEditWorkLine?: (workLine: WorkLine) => void;
  onOpenNewTask?: () => void;
  onOpenAssignTask?: (task: WorkEntity) => void;
}

type WbsTab = "ALL_TASKS" | "DECISIONS" | "SOPS" | "LOGS";

interface WorkLineGroup {
  key: string;
  workLine: WorkLine | null;
  tasks: WorkEntity[];
}

interface OutcomeGroup {
  key: string;
  outcome: Outcome | null;
  workLineGroups: WorkLineGroup[];
  totalOpenTasks: number;
}

const UNASSIGNED_OUTCOME_KEY = "__UNASSIGNED_OUTCOME__";
const UNASSIGNED_LINE_KEY = "__UNASSIGNED_LINE__";

export function WbsAndDocsView({
  dash,
  onQuickComplete,
  onStartTask,
  onOpenNewOutcome,
  onEditOutcome,
  onOpenNewWorkLine,
  onEditWorkLine,
  onOpenNewTask,
  onOpenAssignTask,
}: WbsAndDocsViewProps) {
  const [activeTab, setActiveTab] = useState<WbsTab>("ALL_TASKS");
  const [searchQuery, setSearchQuery] = useState("");
  const [energyFilter, setEnergyFilter] = useState<string>("ALL");
  const [deviceFilter, setDeviceFilter] = useState<string>("ALL");
  const [showOnlyUnlocked, setShowOnlyUnlocked] = useState(false);
  const [collapsedOutcomes, setCollapsedOutcomes] = useState<Set<string>>(
    () => new Set()
  );
  const [collapsedWorkLines, setCollapsedWorkLines] = useState<Set<string>>(
    () => new Set()
  );

  const toggleOutcomeCollapse = (key: string) => {
    setCollapsedOutcomes((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const toggleWorkLineCollapse = (key: string) => {
    setCollapsedWorkLines((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const outcomeMap = useMemo(() => {
    const map = new Map<string, Outcome>();
    for (const o of dash.outcomes ?? []) {
      map.set(o.id, o);
    }
    return map;
  }, [dash.outcomes]);

  const workLineMap = useMemo(() => {
    const map = new Map<string, WorkLine>();
    for (const wl of dash.work_lines ?? []) {
      map.set(wl.id, wl);
    }
    return map;
  }, [dash.work_lines]);

  const normalizedQuery = searchQuery.trim().toLowerCase();

  const filteredTasks = useMemo(() => {
    return dash.open_tasks.filter((t) => {
      if (
        showOnlyUnlocked &&
        (t.is_locked_by_chain || t.status === "QUEUED_IN_CHAIN")
      ) {
        return false;
      }
      if (energyFilter !== "ALL" && t.energy_tag !== energyFilter) {
        return false;
      }
      if (
        deviceFilter !== "ALL" &&
        t.device_tag !== deviceFilter &&
        t.device_tag !== "ANY_DEVICE"
      ) {
        return false;
      }
      if (!normalizedQuery) return true;
      const wlTitle =
        t.work_line_title ||
        (t.work_line_id ? workLineMap.get(t.work_line_id)?.title : "") ||
        "";
      const outId =
        t.outcome_id ||
        (t.work_line_id ? workLineMap.get(t.work_line_id)?.outcome_id : "") ||
        "";
      const outTitle = outId ? outcomeMap.get(outId)?.title || "" : "";

      return (
        t.title.toLowerCase().includes(normalizedQuery) ||
        t.id.toLowerCase().includes(normalizedQuery) ||
        t.category.toLowerCase().includes(normalizedQuery) ||
        wlTitle.toLowerCase().includes(normalizedQuery) ||
        outTitle.toLowerCase().includes(normalizedQuery)
      );
    });
  }, [
    dash.open_tasks,
    normalizedQuery,
    workLineMap,
    outcomeMap,
    energyFilter,
    deviceFilter,
    showOnlyUnlocked,
  ]);

  // Hierarchical Grouping: Outcome -> WorkLine -> Tasks
  const outcomeGroups = useMemo<OutcomeGroup[]>(() => {
    const outcomeBuckets = new Map<
      string,
      {
        outcome: Outcome | null;
        lineBuckets: Map<string, { workLine: WorkLine | null; tasks: WorkEntity[] }>;
      }
    >();

    const ensureOutcomeBucket = (outId: string | null | undefined) => {
      const key =
        outId && outId.trim().length > 0 ? outId : UNASSIGNED_OUTCOME_KEY;
      let bucket = outcomeBuckets.get(key);
      if (!bucket) {
        bucket = {
          outcome: key === UNASSIGNED_OUTCOME_KEY ? null : outcomeMap.get(key) ?? null,
          lineBuckets: new Map(),
        };
        outcomeBuckets.set(key, bucket);
      }
      return bucket;
    };

    const ensureLineBucket = (
      outBucket: {
        outcome: Outcome | null;
        lineBuckets: Map<string, { workLine: WorkLine | null; tasks: WorkEntity[] }>;
      },
      wlId: string | null | undefined
    ) => {
      const key =
        wlId && wlId.trim().length > 0 ? wlId : UNASSIGNED_LINE_KEY;
      let lb = outBucket.lineBuckets.get(key);
      if (!lb) {
        lb = {
          workLine: key === UNASSIGNED_LINE_KEY ? null : workLineMap.get(key) ?? null,
          tasks: [],
        };
        outBucket.lineBuckets.set(key, lb);
      }
      return lb;
    };

    // Include project's active WorkLines even if currently empty when not searching
    if (!normalizedQuery) {
      for (const wl of dash.work_lines ?? []) {
        if (wl.status === "ARCHIVED") continue;
        const ob = ensureOutcomeBucket(wl.outcome_id);
        ensureLineBucket(ob, wl.id);
      }
    }

    for (const task of filteredTasks) {
      const wl = task.work_line_id ? workLineMap.get(task.work_line_id) : undefined;
      const effectiveOutcomeId = task.outcome_id || wl?.outcome_id || null;
      const ob = ensureOutcomeBucket(effectiveOutcomeId);
      const lb = ensureLineBucket(ob, task.work_line_id);
      lb.tasks.push(task);
    }

    const result: OutcomeGroup[] = [];
    for (const [oKey, ob] of outcomeBuckets.entries()) {
      const wlGroups: WorkLineGroup[] = [];
      let totalOpen = 0;

      for (const [lKey, lb] of ob.lineBuckets.entries()) {
        const sortedTasks = [...lb.tasks].sort((a, b) => {
          if (lb.workLine?.line_mode === "SEQUENTIAL_CHAIN") {
            return a.sequence_order - b.sequence_order;
          }
          const rank = (s: string) =>
            s === "IN_PROGRESS" ? 0 : s === "OPEN" ? 1 : s === "BLOCKED" ? 2 : 3;
          const dr = rank(a.status) - rank(b.status);
          if (dr !== 0) return dr;
          return (b.priority_score ?? 50) - (a.priority_score ?? 50);
        });

        totalOpen += sortedTasks.length;
        wlGroups.push({
          key: `${oKey}::${lKey}`,
          workLine: lb.workLine,
          tasks: sortedTasks,
        });
      }

      // Named WorkLines first, unassigned standalone tasks last
      wlGroups.sort((a, b) => {
        if (a.workLine && !b.workLine) return -1;
        if (!a.workLine && b.workLine) return 1;
        return (a.workLine?.title || "").localeCompare(b.workLine?.title || "", "fa");
      });

      result.push({
        key: oKey,
        outcome: ob.outcome,
        workLineGroups: wlGroups,
        totalOpenTasks: totalOpen,
      });
    }

    // Named Outcomes first, unassigned tasks last
    result.sort((a, b) => {
      if (a.key === UNASSIGNED_OUTCOME_KEY) return 1;
      if (b.key === UNASSIGNED_OUTCOME_KEY) return -1;
      return (a.outcome?.title || a.key).localeCompare(
        b.outcome?.title || b.key,
        "fa"
      );
    });

    return result;
  }, [dash.work_lines, filteredTasks, normalizedQuery, outcomeMap, workLineMap]);

  const filteredDecisions = dash.active_decisions.filter((d) =>
    normalizedQuery
      ? d.title.toLowerCase().includes(normalizedQuery) ||
        d.id.toLowerCase().includes(normalizedQuery)
      : true
  );

  const filteredSops = dash.active_sops.filter((s) =>
    normalizedQuery
      ? s.rule_statement.toLowerCase().includes(normalizedQuery) ||
        s.category.toLowerCase().includes(normalizedQuery)
      : true
  );

  const filteredLogs = dash.recent_logs.filter((l) =>
    normalizedQuery
      ? l.client_sanitized_note.toLowerCase().includes(normalizedQuery) ||
        l.category.toLowerCase().includes(normalizedQuery)
      : true
  );

  const totalWorkLinesCount = (dash.work_lines ?? []).length;

  return (
    <div className="flex flex-col gap-3.5 h-full min-h-0">
      {/* Top Database Header: Segmented Tabs & Search */}
      <div className="mab-card p-3 flex items-center justify-between gap-3 flex-wrap shrink-0 border-mab-border">
        {/* Segmented Tabs */}
        <div className="flex items-center gap-1 bg-mab-base/80 p-1 rounded-xl border border-mab-border overflow-x-auto">
          <button
            type="button"
            onClick={() => setActiveTab("ALL_TASKS")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeTab === "ALL_TASKS"
                ? "bg-white/[0.08] text-white font-medium border border-white/[0.1] shadow-sm"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <Layers className="w-3.5 h-3.5 text-blue-400" />
            <span>ساختار شکست کار و لاین‌ها (WBS)</span>
            <span className="font-timer px-1.5 py-0.2 rounded-full bg-white/[0.08] text-[10px]">
              {dash.open_tasks.length}
            </span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("DECISIONS")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeTab === "DECISIONS"
                ? "bg-white/[0.08] text-white font-medium border border-white/[0.1] shadow-sm"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <FileText className="w-3.5 h-3.5 text-purple-400" />
            <span>تصمیمات معماری (ADR)</span>
            <span className="font-timer px-1.5 py-0.2 rounded-full bg-white/[0.08] text-[10px]">
              {dash.active_decisions.length}
            </span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("SOPS")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeTab === "SOPS"
                ? "bg-white/[0.08] text-white font-medium border border-white/[0.1] shadow-sm"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <BookOpen className="w-3.5 h-3.5 text-emerald-400" />
            <span>قوانین اجرایی (SOP)</span>
            <span className="font-timer px-1.5 py-0.2 rounded-full bg-white/[0.08] text-[10px]">
              {dash.active_sops.length}
            </span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("LOGS")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeTab === "LOGS"
                ? "bg-white/[0.08] text-white font-medium border border-white/[0.1] shadow-sm"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <Clock className="w-3.5 h-3.5 text-amber-400" />
            <span>لاگ‌های Mab-Focus</span>
            <span className="font-timer px-1.5 py-0.2 rounded-full bg-white/[0.08] text-[10px]">
              {dash.recent_logs.length}
            </span>
          </button>
        </div>

        {/* Summary Counters + Search Filter */}
        <div className="flex items-center gap-2.5 flex-wrap">
          {activeTab === "ALL_TASKS" && totalWorkLinesCount > 0 && (
            <div className="hidden md:flex items-center gap-1.5 text-[11px] text-mab-text-secondary bg-white/[0.03] border border-mab-border px-2.5 py-1 rounded-lg">
              <GitBranch className="w-3.5 h-3.5 text-blue-400" />
              <span>{totalWorkLinesCount} لاین کاری</span>
              <span className="text-white/25">•</span>
              <span>{dash.closed_tasks_count} تکمیل‌شده</span>
            </div>
          )}

          <div className="relative min-w-[200px] sm:min-w-[240px]">
            <Search className="w-3.5 h-3.5 text-white/40 absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="جستجو در تسک، لاین کاری یا Outcome..."
              className="w-full mab-input pr-8 pl-3 py-1.5 text-[12px] select-text"
            />
          </div>
        </div>
      </div>

      {/* Main Database Content View */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* Tab 1: Hierarchical Outcome -> WorkLine -> Tasks */}
        {activeTab === "ALL_TASKS" && (
          <div className="space-y-4 pb-4">
            {/* Quick Creation & Context Filter Toolbar */}
            <div className="mab-card p-3 flex items-center justify-between gap-2.5 flex-wrap">
              <div className="flex items-center gap-2 flex-wrap">
                {onOpenNewOutcome && (
                  <button
                    type="button"
                    onClick={onOpenNewOutcome}
                    className="px-3 py-1.5 rounded-xl bg-amber-600/20 hover:bg-amber-600/35 border border-amber-500/35 text-amber-300 text-[11.5px] font-semibold flex items-center gap-1.5 transition"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>Outcome جدید</span>
                  </button>
                )}
                {onOpenNewWorkLine && (
                  <button
                    type="button"
                    onClick={onOpenNewWorkLine}
                    className="px-3 py-1.5 rounded-xl bg-blue-600/20 hover:bg-blue-600/35 border border-blue-500/35 text-blue-300 text-[11.5px] font-semibold flex items-center gap-1.5 transition"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>لاین کاری جدید</span>
                  </button>
                )}
                {onOpenNewTask && (
                  <button
                    type="button"
                    onClick={onOpenNewTask}
                    className="px-3 py-1.5 rounded-xl bg-emerald-600/20 hover:bg-emerald-600/35 border border-emerald-500/35 text-emerald-300 text-[11.5px] font-semibold flex items-center gap-1.5 transition"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>تسک جدید</span>
                  </button>
                )}
              </div>

              {/* 4D Context Quick Filters */}
              <div className="flex items-center gap-1.5 flex-wrap text-[11px]">
                <Filter className="w-3.5 h-3.5 text-white/40 shrink-0" />
                <select
                  value={energyFilter}
                  onChange={(e) => setEnergyFilter(e.target.value)}
                  className="mab-input px-2 py-1 text-[11px] bg-mab-base"
                  title="فیلتر سطح انرژی"
                >
                  <option value="ALL">همه انرژی‌ها</option>
                  <option value="HIGH_DEEP">انرژی بالا (عمیق)</option>
                  <option value="MEDIUM_NORMAL">انرژی متوسط</option>
                  <option value="LOW_ZOMBIE">انرژی کم (سبک)</option>
                </select>
                <select
                  value={deviceFilter}
                  onChange={(e) => setDeviceFilter(e.target.value)}
                  className="mab-input px-2 py-1 text-[11px] bg-mab-base"
                  title="فیلتر وسیله"
                >
                  <option value="ALL">همه وسایل</option>
                  <option value="LAPTOP_ONLY">لپ‌تاپ</option>
                  <option value="PHONE_OK">موبایل</option>
                </select>
                <button
                  type="button"
                  onClick={() => setShowOnlyUnlocked((v) => !v)}
                  className={`px-2 py-1 rounded-lg border transition ${
                    showOnlyUnlocked
                      ? "bg-blue-500/20 border-blue-500/40 text-blue-300 font-semibold"
                      : "bg-white/[0.03] border-mab-border text-white/65 hover:text-white"
                  }`}
                >
                  فقط قابل اجرا
                </button>
              </div>
            </div>

            {outcomeGroups.map((og) => {
              const isOutcomeCollapsed = collapsedOutcomes.has(og.key);
              const isUnassignedOutcome = og.key === UNASSIGNED_OUTCOME_KEY;

              return (
                <section
                  key={og.key}
                  className="mab-card overflow-hidden border-mab-border"
                >
                  {/* Outcome Header */}
                  <div className="w-full px-4 py-3 bg-white/[0.025] hover:bg-white/[0.045] transition flex items-center justify-between gap-3 border-b border-mab-border flex-wrap">
                    <button
                      type="button"
                      onClick={() => toggleOutcomeCollapse(og.key)}
                      className="flex items-center gap-2.5 min-w-0 flex-1 text-right"
                    >
                      {isOutcomeCollapsed ? (
                        <ChevronLeft className="w-4 h-4 text-white/50 shrink-0" />
                      ) : (
                        <ChevronDown className="w-4 h-4 text-white/50 shrink-0" />
                      )}
                      <Target
                        className={`w-4 h-4 shrink-0 ${
                          isUnassignedOutcome
                            ? "text-white/45"
                            : "text-amber-400"
                        }`}
                      />
                      <div className="min-w-0 flex items-center gap-2 flex-wrap">
                        {!isUnassignedOutcome && (
                          <span className="px-2 py-0.5 rounded-md bg-amber-500/15 border border-amber-500/30 text-amber-300 font-mono text-[10.5px] font-bold shrink-0">
                            {og.outcome?.id ?? og.key}
                          </span>
                        )}
                        <h3 className="text-[13.5px] font-bold text-white truncate">
                          {isUnassignedOutcome
                            ? "تسک‌های بدون Outcome (آزاد / عمومی)"
                            : og.outcome?.title || og.key}
                        </h3>
                        {og.outcome?.evidence_kr && (
                          <span className="hidden lg:inline text-[11px] text-mab-text-muted truncate max-w-md">
                            — سنجه: {og.outcome.evidence_kr}
                          </span>
                        )}
                      </div>
                    </button>

                    <div className="flex items-center gap-2 shrink-0">
                      {og.outcome && (
                        <span className="px-2 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/25 text-emerald-300 font-timer text-[10.5px]">
                          {og.outcome.progress_percent}% پیشرفت
                        </span>
                      )}
                      <span className="px-2 py-0.5 rounded-full bg-white/[0.06] text-white/70 font-timer text-[10.5px]">
                        {og.totalOpenTasks} تسک باز
                      </span>
                      {og.outcome && onEditOutcome && (
                        <button
                          type="button"
                          onClick={() => onEditOutcome(og.outcome!)}
                          title="ویرایش این Outcome"
                          className="p-1 rounded-md bg-white/[0.04] hover:bg-white/[0.1] text-white/60 hover:text-white transition"
                        >
                          <Edit3 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                  </div>

                  {/* WorkLines inside this Outcome */}
                  {!isOutcomeCollapsed && (
                    <div className="p-3 space-y-3">
                      {og.workLineGroups.map((wlg) => {
                        const wl = wlg.workLine;
                        const isWlCollapsed = collapsedWorkLines.has(wlg.key);
                        const lineColor = wl?.color_hex || "#64748B";
                        const isSequential =
                          wl?.line_mode === "SEQUENTIAL_CHAIN";
                        const totalTasks = wl
                          ? Math.max(wl.total_tasks, wlg.tasks.length)
                          : wlg.tasks.length;
                        const completedTasks = wl?.completed_tasks ?? 0;
                        const progressPct =
                          totalTasks > 0
                            ? Math.min(
                                100,
                                Math.round((completedTasks / totalTasks) * 100)
                              )
                            : 0;

                        return (
                          <div
                            key={wlg.key}
                            className="rounded-xl bg-mab-base/60 border border-mab-border overflow-hidden"
                            style={{
                              borderRightWidth: "3px",
                              borderRightColor: lineColor,
                            }}
                          >
                            {/* WorkLine Sub-Header */}
                            <div className="w-full px-3.5 py-2.5 hover:bg-white/[0.03] transition flex items-center justify-between gap-3 flex-wrap">
                              <button
                                type="button"
                                onClick={() => toggleWorkLineCollapse(wlg.key)}
                                className="flex items-center gap-2 min-w-0 flex-wrap flex-1 text-right"
                              >
                                {isWlCollapsed ? (
                                  <ChevronLeft className="w-3.5 h-3.5 text-white/50 shrink-0" />
                                ) : (
                                  <ChevronDown className="w-3.5 h-3.5 text-white/50 shrink-0" />
                                )}

                                <span
                                  className="w-2.5 h-2.5 rounded-full shrink-0"
                                  style={{ backgroundColor: lineColor }}
                                />

                                {wl ? (
                                  <>
                                    <span className="font-mono text-[10.5px] text-white/60 shrink-0">
                                      [{wl.id}]
                                    </span>
                                    <span className="text-[13px] font-bold text-white">
                                      {wl.title}
                                    </span>
                                    <span
                                      className="px-2 py-0.5 rounded-full text-[10px] font-semibold border shrink-0"
                                      style={{
                                        color: lineColor,
                                        borderColor: `${lineColor}55`,
                                        backgroundColor: `${lineColor}1A`,
                                      }}
                                    >
                                      {isSequential
                                        ? "زنجیره پشت‌به‌پشت"
                                        : "خوشه هم‌هدف"}
                                    </span>
                                    {wl.status === "COMPLETED" && (
                                      <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 border border-emerald-500/30 text-emerald-300 text-[10px] font-bold">
                                        تکمیل شد
                                      </span>
                                    )}
                                  </>
                                ) : (
                                  <span className="text-[12.5px] font-semibold text-mab-text-secondary">
                                    تسک‌های مستقل (بدون لاین کاری)
                                  </span>
                                )}
                              </button>

                              {/* Progress Bar & Counts + Edit WorkLine */}
                              <div className="flex items-center gap-2.5 shrink-0">
                                {wl && (
                                  <div className="flex items-center gap-2">
                                    <div className="w-20 sm:w-24 h-1.5 rounded-full bg-white/[0.08] overflow-hidden">
                                      <div
                                        className="h-full rounded-full transition-all duration-300"
                                        style={{
                                          width: `${progressPct}%`,
                                          backgroundColor: lineColor,
                                        }}
                                      />
                                    </div>
                                    <span className="font-timer text-[10.5px] text-white/75">
                                      {completedTasks}/{totalTasks} ({progressPct}
                                      %)
                                    </span>
                                    {onEditWorkLine && (
                                      <button
                                        type="button"
                                        onClick={() => onEditWorkLine(wl)}
                                        title="ویرایش لاین کاری"
                                        className="p-1 rounded-md bg-white/[0.04] hover:bg-white/[0.1] text-white/60 hover:text-white transition"
                                      >
                                        <Edit3 className="w-3 h-3" />
                                      </button>
                                    )}
                                  </div>
                                )}
                                {!wl && (
                                  <span className="font-timer text-[10.5px] text-white/60">
                                    {wlg.tasks.length} تسک
                                  </span>
                                )}
                              </div>
                            </div>

                            {/* WorkLine Goal Statement */}
                            {!isWlCollapsed && wl?.goal_statement && (
                              <div className="px-4 pb-2 text-[11.5px] text-mab-text-muted border-b border-white/[0.04]">
                                هدف لاین: {wl.goal_statement}
                              </div>
                            )}

                            {/* Tasks List inside WorkLine */}
                            {!isWlCollapsed && (
                              <div className="divide-y divide-white/[0.05]">
                                {wlg.tasks.map((t) => {
                                  const isLocked =
                                    t.is_locked_by_chain ||
                                    t.status === "QUEUED_IN_CHAIN";
                                  const isInProgress =
                                    t.status === "IN_PROGRESS";
                                  const isBlocked = t.status === "BLOCKED";

                                  return (
                                    <div
                                      key={t.id}
                                      className={`px-3.5 py-3 flex items-center justify-between gap-3 flex-wrap sm:flex-nowrap transition ${
                                        isLocked
                                          ? "bg-white/[0.01] opacity-70"
                                          : isInProgress
                                          ? "bg-blue-500/[0.05] hover:bg-blue-500/[0.08]"
                                          : "hover:bg-mab-hover/60"
                                      }`}
                                    >
                                      <div className="min-w-0 space-y-1 flex-1">
                                        <div className="flex items-center gap-2 flex-wrap">
                                          {isSequential && (
                                            <span
                                              className="px-1.5 py-0.5 rounded font-mono text-[10px] font-bold border"
                                              style={{
                                                color: isLocked
                                                  ? "rgba(255,255,255,0.45)"
                                                  : lineColor,
                                                borderColor: isLocked
                                                  ? "rgba(255,255,255,0.12)"
                                                  : `${lineColor}55`,
                                                backgroundColor: isLocked
                                                  ? "rgba(255,255,255,0.03)"
                                                  : `${lineColor}1A`,
                                              }}
                                            >
                                              گام #{t.sequence_order}
                                            </span>
                                          )}

                                          <span className="font-mono text-[11.5px] text-blue-400 font-semibold">
                                            [{t.id}]
                                          </span>

                                          <span className="px-1.5 py-0.2 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px]">
                                            {t.category}
                                          </span>

                                          <span
                                            className={`text-[13.5px] font-semibold ${
                                              isLocked
                                                ? "text-white/70"
                                                : "text-white"
                                            }`}
                                          >
                                            {t.title}
                                          </span>

                                          {/* Status Pill */}
                                          {isLocked ? (
                                            <span className="px-2 py-0.5 rounded-full bg-white/[0.06] border border-white/[0.12] text-white/65 text-[10px] font-semibold flex items-center gap-1">
                                              <Lock className="w-2.5 h-2.5 text-amber-300" />
                                              <span>
                                                قفل در زنجیره
                                                {t.depends_on_task_id
                                                  ? ` (در انتظار ${t.depends_on_task_id})`
                                                  : ""}
                                              </span>
                                            </span>
                                          ) : isBlocked ? (
                                            <span className="px-2 py-0.5 rounded-full bg-rose-500/15 border border-rose-500/30 text-rose-400 font-mono text-[10px] font-bold flex items-center gap-1">
                                              <ShieldAlert className="w-2.5 h-2.5" />
                                              <span>BLOCKED</span>
                                            </span>
                                          ) : isInProgress ? (
                                            <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 border border-emerald-500/35 text-emerald-300 text-[10px] font-bold flex items-center gap-1">
                                              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                                              <span>در حال اجرا</span>
                                            </span>
                                          ) : (
                                            <span className="px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-400 font-mono text-[10px]">
                                              OPEN
                                            </span>
                                          )}

                                          <span className="font-mono text-[11px] text-amber-300/90 font-semibold mr-1">
                                            {formatPomos(t.calibrated_pomos)}{" "}
                                            Pomo
                                          </span>
                                        </div>

                                        {t.first_physical_action && (
                                          <p className="text-[12px] text-mab-text-secondary flex items-center gap-1.5 pt-0.5 select-text">
                                            <Zap className="w-3 h-3 text-emerald-400 shrink-0" />
                                            <span>
                                              اقدام شروع:{" "}
                                              {t.first_physical_action}
                                            </span>
                                          </p>
                                        )}

                                        {t.definition_of_done && (
                                          <p className="text-[11.5px] text-mab-text-muted select-text">
                                            DoD: {t.definition_of_done}
                                          </p>
                                        )}
                                      </div>

                                      {/* Action Buttons */}
                                      <div className="flex items-center gap-1.5 shrink-0 flex-wrap">
                                        {onOpenAssignTask && (
                                          <button
                                            type="button"
                                            onClick={() => onOpenAssignTask(t)}
                                            title="تخصیص به لاین کاری / Outcome و زمان‌بندی"
                                            className="px-2.5 py-1.5 rounded-xl bg-white/[0.04] hover:bg-white/[0.1] border border-mab-border text-white/75 hover:text-white text-[11px] font-medium transition flex items-center gap-1"
                                          >
                                            <MoveRight className="w-3 h-3" />
                                            <span className="hidden sm:inline">
                                              تخصیص
                                            </span>
                                          </button>
                                        )}

                                        {isLocked ? (
                                          <span
                                            title="تا تکمیل گام قبلی در زنجیره قفل است"
                                            className="px-3 py-1.5 rounded-xl bg-white/[0.04] border border-white/[0.08] text-white/40 text-[11px] font-medium flex items-center gap-1.5 cursor-not-allowed"
                                          >
                                            <Lock className="w-3.5 h-3.5" />
                                            <span>قفل شده</span>
                                          </span>
                                        ) : (
                                          <>
                                            {t.status === "OPEN" &&
                                              onStartTask && (
                                                <button
                                                  type="button"
                                                  onClick={() =>
                                                    onStartTask(t.id)
                                                  }
                                                  className="px-3 py-1.5 rounded-xl bg-blue-600/20 hover:bg-blue-600/35 border border-blue-500/35 text-blue-300 text-[11.5px] font-semibold transition flex items-center gap-1"
                                                >
                                                  <Play className="w-3 h-3" />
                                                  <span>شروع</span>
                                                </button>
                                              )}
                                            <button
                                              type="button"
                                              onClick={() =>
                                                onQuickComplete(
                                                  t.id,
                                                  Math.round(
                                                    t.calibrated_pomos
                                                  ) || 1
                                                )
                                              }
                                              className="px-3.5 py-1.5 rounded-xl bg-emerald-600/90 hover:bg-emerald-500 text-white text-[11.5px] font-semibold transition flex items-center gap-1.5 shadow-sm"
                                            >
                                              <CheckCircle2 className="w-3.5 h-3.5" />
                                              <span>تکمیل شد</span>
                                            </button>
                                          </>
                                        )}
                                      </div>
                                    </div>
                                  );
                                })}

                                {wlg.tasks.length === 0 && (
                                  <div className="px-4 py-3 text-[12px] text-mab-text-muted">
                                    همه تسک‌های این لاین تکمیل شده‌اند یا تسک بازی در آن ثبت نشده است.
                                  </div>
                                )}
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </section>
              );
            })}

            {outcomeGroups.length === 0 && (
              <div className="py-16 text-center text-[13px] text-mab-text-muted mab-card">
                هیچ گامی مطابق جستجو یا در صف این پروژه یافت نشد.
              </div>
            )}
          </div>
        )}

        {/* Tab 2: Architecture Decisions (ADR) */}
        {activeTab === "DECISIONS" && (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {filteredDecisions.map((d) => (
              <div
                key={d.id}
                className="mab-card p-4 space-y-2 hover:bg-mab-hover/60 transition"
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <span className="px-2 py-0.5 rounded-md bg-purple-500/15 border border-purple-500/30 text-purple-300 font-mono text-[10.5px] font-bold">
                      [{d.id}]
                    </span>
                    <h4 className="text-[13.5px] font-bold text-white truncate">
                      {d.title}
                    </h4>
                  </div>
                  <span className="font-timer text-[10.5px] text-mab-text-muted">
                    {formatShortDate(d.updated_at)}
                  </span>
                </div>
                <p className="text-[12.5px] text-mab-text-secondary leading-relaxed select-text">
                  {d.definition_of_done}
                </p>
              </div>
            ))}

            {filteredDecisions.length === 0 && (
              <div className="col-span-full py-16 text-center text-[13px] text-mab-text-muted mab-card">
                تصمیم معماری (ADR) ثبت نشده است.
              </div>
            )}
          </div>
        )}

        {/* Tab 3: SOP Rules */}
        {activeTab === "SOPS" && (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {filteredSops.map((s) => (
              <div
                key={s.id}
                className="mab-card p-4 space-y-2 hover:bg-mab-hover/60 transition"
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="px-2 py-0.5 rounded-md bg-emerald-500/15 border border-emerald-500/30 text-emerald-300 font-mono text-[10.5px] font-bold">
                    {s.category}
                  </span>
                  <span className="font-timer text-[10.5px] text-mab-text-muted">
                    {formatShortDate(s.updated_at)}
                  </span>
                </div>
                <p className="text-[12.5px] text-white leading-relaxed select-text">
                  {s.rule_statement}
                </p>
              </div>
            ))}

            {filteredSops.length === 0 && (
              <div className="col-span-full py-16 text-center text-[13px] text-mab-text-muted mab-card">
                قانون SOP ثبت نشده است.
              </div>
            )}
          </div>
        )}

        {/* Tab 4: Work Logs */}
        {activeTab === "LOGS" && (
          <div className="space-y-2">
            {filteredLogs.map((l) => (
              <div
                key={l.id}
                className="mab-card p-3.5 space-y-1.5 hover:bg-mab-hover/60 transition text-[12.5px]"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-300 font-mono text-[10.5px] font-bold">
                      {l.effective_pomos} Pomo
                    </span>
                    <span className="text-white/60 font-mono text-[11px]">
                      {l.source_origin}
                    </span>
                  </div>
                  <span className="font-timer text-[11px] text-mab-text-muted">
                    {formatShortDate(l.logged_at)}
                  </span>
                </div>
                <p className="text-mab-text-primary select-text leading-relaxed">
                  {l.client_sanitized_note}
                </p>
              </div>
            ))}

            {filteredLogs.length === 0 && (
              <div className="py-16 text-center text-[13px] text-mab-text-muted mab-card">
                هنوز لاگی ثبت یا از Mab-Focus سینک نشده است.
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export default WbsAndDocsView;
