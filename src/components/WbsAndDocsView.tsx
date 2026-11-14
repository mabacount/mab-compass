import { useState } from "react";
import {
  BookOpen,
  CheckCircle2,
  Clock,
  FileText,
  Layers,
  Search,
  ShieldAlert,
  Zap,
} from "lucide-react";
import { DashboardState } from "../lib/tauri_api";
import { formatPomos, formatShortDate } from "../lib/formatters";

interface WbsAndDocsViewProps {
  dash: DashboardState;
  onQuickComplete: (taskId: string, pomosSpent: number) => void;
}

type WbsTab = "ALL_TASKS" | "DECISIONS" | "SOPS" | "LOGS";

export function WbsAndDocsView({ dash, onQuickComplete }: WbsAndDocsViewProps) {
  const [activeTab, setActiveTab] = useState<WbsTab>("ALL_TASKS");
  const [searchQuery, setSearchQuery] = useState("");

  const filteredTasks = dash.open_tasks.filter((t) =>
    searchQuery
      ? t.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        t.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
        t.category.toLowerCase().includes(searchQuery.toLowerCase())
      : true
  );

  const filteredDecisions = dash.active_decisions.filter((d) =>
    searchQuery
      ? d.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        d.id.toLowerCase().includes(searchQuery.toLowerCase())
      : true
  );

  const filteredSops = dash.active_sops.filter((s) =>
    searchQuery
      ? s.rule_statement.toLowerCase().includes(searchQuery.toLowerCase()) ||
        s.category.toLowerCase().includes(searchQuery.toLowerCase())
      : true
  );

  const filteredLogs = dash.recent_logs.filter((l) =>
    searchQuery
      ? l.client_sanitized_note.toLowerCase().includes(searchQuery.toLowerCase()) ||
        l.category.toLowerCase().includes(searchQuery.toLowerCase())
      : true
  );

  return (
    <div className="flex flex-col gap-3.5 h-full min-h-0">
      {/* Top Anytype Database Header: Segmented Tabs & Search */}
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
            <span>گام‌های ساختار شکست کار (WBS)</span>
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

        {/* Search Filter */}
        <div className="relative min-w-[200px] sm:min-w-[240px]">
          <Search className="w-3.5 h-3.5 text-white/40 absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none" />
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="جستجو در این بخش..."
            className="w-full mab-input pr-8 pl-3 py-1.5 text-[12px] select-text"
          />
        </div>
      </div>

      {/* Main Database Content View */}
      <div className="flex-1 min-h-0 overflow-y-auto">
        {/* Tab 1: WBS Open Tasks */}
        {activeTab === "ALL_TASKS" && (
          <div className="space-y-2">
            {filteredTasks.map((t) => (
              <div
                key={t.id}
                className="mab-card p-3.5 hover:bg-mab-hover/60 transition flex items-center justify-between gap-3 flex-wrap sm:flex-nowrap group"
              >
                <div className="min-w-0 space-y-1">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-mono text-[11.5px] text-blue-400 font-semibold">
                      [{t.id}]
                    </span>
                    <span className="px-1.5 py-0.2 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px]">
                      {t.category}
                    </span>
                    <span className="text-[13.5px] font-semibold text-white">
                      {t.title}
                    </span>

                    {t.status === "BLOCKED" ? (
                      <span className="px-2 py-0.5 rounded-full bg-rose-500/15 border border-rose-500/30 text-rose-400 font-mono text-[10px] font-bold flex items-center gap-1">
                        <ShieldAlert className="w-2.5 h-2.5" />
                        <span>BLOCKED</span>
                      </span>
                    ) : (
                      <span className="px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-400 font-mono text-[10px]">
                        OPEN
                      </span>
                    )}

                    <span className="font-mono text-[11px] text-amber-300/90 font-semibold mr-1">
                      {formatPomos(t.calibrated_pomos)} Pomo
                    </span>
                  </div>

                  {t.first_physical_action && (
                    <p className="text-[12px] text-mab-text-secondary flex items-center gap-1.5 pt-0.5 select-text">
                      <Zap className="w-3 h-3 text-emerald-400 shrink-0" />
                      <span>اقدام شروع: {t.first_physical_action}</span>
                    </p>
                  )}

                  {t.definition_of_done && (
                    <p className="text-[11.5px] text-mab-text-muted select-text">
                      DoD: {t.definition_of_done}
                    </p>
                  )}
                </div>

                <button
                  type="button"
                  onClick={() =>
                    onQuickComplete(t.id, Math.round(t.calibrated_pomos) || 1)
                  }
                  className="px-3.5 py-1.5 rounded-xl bg-emerald-600/90 hover:bg-emerald-500 text-white text-[11.5px] font-semibold shrink-0 transition flex items-center gap-1.5 shadow-sm"
                >
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  <span>تکمیل شد</span>
                </button>
              </div>
            ))}

            {filteredTasks.length === 0 && (
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
