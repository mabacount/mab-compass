// src/components/SystemLogsPanel.tsx
import { useEffect, useState, useMemo } from "react";
import {
  AlertCircle,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  ClipboardCopy,
  Info,
  Search,
  Terminal,
  Trash2,
  AlertTriangle,
} from "lucide-react";
import {
  AppLogEntry,
  clearAppLogs,
  exportLogsAsMarkdown,
  getAppLogs,
  LogLevel,
  LogCategory,
  subscribeAppLogs,
} from "../lib/logStore";

export function SystemLogsPanel() {
  const [logs, setLogs] = useState<AppLogEntry[]>(() => getAppLogs());
  const [levelFilter, setLevelFilter] = useState<"ALL" | LogLevel>("ALL");
  const [categoryFilter, setCategoryFilter] = useState<"ALL" | LogCategory>("ALL");
  const [searchQuery, setSearchQuery] = useState("");
  const [expandedLogIds, setExpandedLogIds] = useState<Set<string>>(new Set());
  const [copiedSuccess, setCopiedSuccess] = useState(false);

  useEffect(() => {
    return subscribeAppLogs((newLogs) => {
      setLogs([...newLogs]);
    });
  }, []);

  const filteredLogs = useMemo(() => {
    return logs
      .filter((l) => {
        if (levelFilter !== "ALL" && l.level !== levelFilter) return false;
        if (categoryFilter !== "ALL" && l.category !== categoryFilter) return false;
        if (searchQuery.trim()) {
          const q = searchQuery.toLowerCase();
          const matchMsg = l.message.toLowerCase().includes(q);
          const matchCat = l.category.toLowerCase().includes(q);
          const matchDetails = l.details?.toLowerCase().includes(q) ?? false;
          if (!matchMsg && !matchCat && !matchDetails) return false;
        }
        return true;
      })
      .reverse(); // Newest first
  }, [logs, levelFilter, categoryFilter, searchQuery]);

  const toggleExpand = (id: string) => {
    setExpandedLogIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleCopyLogs = async () => {
    const text = exportLogsAsMarkdown();
    try {
      await navigator.clipboard.writeText(text);
      setCopiedSuccess(true);
      setTimeout(() => setCopiedSuccess(false), 2500);
    } catch {
      // fallback
    }
  };

  const getLevelBadge = (level: LogLevel) => {
    switch (level) {
      case "ERROR":
        return (
          <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-rose-400 bg-rose-500/15 border border-rose-500/30 px-1.5 py-0.5 rounded">
            <AlertCircle className="w-3 h-3 shrink-0" />
            <span>ERROR</span>
          </span>
        );
      case "WARN":
        return (
          <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-amber-300 bg-amber-500/15 border border-amber-500/30 px-1.5 py-0.5 rounded">
            <AlertTriangle className="w-3 h-3 shrink-0" />
            <span>WARN</span>
          </span>
        );
      case "SUCCESS":
        return (
          <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-emerald-400 bg-emerald-500/15 border border-emerald-500/30 px-1.5 py-0.5 rounded">
            <CheckCircle2 className="w-3 h-3 shrink-0" />
            <span>OK</span>
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center gap-1 text-[11px] font-medium text-blue-300 bg-blue-500/15 border border-blue-500/30 px-1.5 py-0.5 rounded">
            <Info className="w-3 h-3 shrink-0" />
            <span>INFO</span>
          </span>
        );
    }
  };

  const counts = useMemo(() => {
    let err = 0;
    let warn = 0;
    for (const l of logs) {
      if (l.level === "ERROR") err++;
      else if (l.level === "WARN") warn++;
    }
    return { total: logs.length, err, warn };
  }, [logs]);

  return (
    <div dir="rtl" className="h-full flex flex-col min-h-0 bg-[#0E0F12] select-text">
      {/* Top Header & Actions Bar */}
      <div className="p-4 border-b border-white/[0.08] space-y-3 shrink-0 bg-[#121318]">
        <div className="flex items-center justify-between gap-3 flex-wrap">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-blue-500/15 border border-blue-500/30 flex items-center justify-center text-blue-400">
              <Terminal className="w-4 h-4" />
            </div>
            <div>
              <h2 className="text-[14px] font-bold text-white flex items-center gap-2">
                <span>لاگ‌ها و رویدادهای سیستم</span>
                <span className="text-[11px] px-2 py-0.5 rounded-full bg-white/[0.08] text-white/70 font-mono">
                  {counts.total} رخداد
                </span>
                {counts.err > 0 && (
                  <span className="text-[11px] px-2 py-0.5 rounded-full bg-rose-500/20 text-rose-400 border border-rose-500/30 font-mono">
                    {counts.err} خطا
                  </span>
                )}
              </h2>
              <p className="text-[11px] text-white/50">
                تاریخچه شفاف ارتباطات مدل‌های هوش مصنوعی، رویدادهای صف و عملیات دیتابیس
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleCopyLogs}
              className={`px-3 py-1.5 rounded-lg text-[12px] font-medium flex items-center gap-1.5 border transition ${
                copiedSuccess
                  ? "bg-emerald-500/20 border-emerald-500/40 text-emerald-300"
                  : "bg-blue-600/90 hover:bg-blue-500 border-blue-500/40 text-white shadow-sm"
              }`}
            >
              {copiedSuccess ? (
                <>
                  <Check className="w-3.5 h-3.5" />
                  <span>کپی شد! آماده برای ارسال</span>
                </>
              ) : (
                <>
                  <ClipboardCopy className="w-3.5 h-3.5" />
                  <span>کپی کل لاگ‌ها برای ارسال به AI</span>
                </>
              )}
            </button>

            <button
              type="button"
              onClick={clearAppLogs}
              title="پاکسازی تاریخچه لاگ‌ها"
              className="p-1.5 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] text-white/60 hover:text-rose-400 transition"
            >
              <Trash2 className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Filter controls */}
        <div className="flex items-center gap-2 flex-wrap pt-1">
          {/* Search box */}
          <div className="relative flex-1 min-w-[180px]">
            <Search className="w-3.5 h-3.5 absolute right-2.5 top-1/2 -translate-y-1/2 text-white/40" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="جستجو در پیام‌ها یا جزئیات خطای لاگ..."
              className="w-full pl-3 pr-8 py-1.5 bg-[#17181F] border border-white/[0.08] rounded-lg text-[12px] text-white placeholder-white/35 focus:outline-none focus:border-blue-500/60"
            />
          </div>

          {/* Level Filter */}
          <div className="flex items-center gap-1 bg-[#17181F] p-1 rounded-lg border border-white/[0.08] text-[11.5px]">
            {(["ALL", "ERROR", "WARN", "INFO", "SUCCESS"] as const).map((lvl) => (
              <button
                key={lvl}
                type="button"
                onClick={() => setLevelFilter(lvl)}
                className={`px-2 py-0.5 rounded transition ${
                  levelFilter === lvl
                    ? "bg-white/[0.14] text-white font-semibold"
                    : "text-white/55 hover:text-white"
                }`}
              >
                {lvl === "ALL" ? "همه" : lvl}
              </button>
            ))}
          </div>

          {/* Category Filter */}
          <div className="flex items-center gap-1 bg-[#17181F] p-1 rounded-lg border border-white/[0.08] text-[11.5px]">
            {(["ALL", "LLM", "QUEUE", "SYNC", "SYSTEM"] as const).map((cat) => (
              <button
                key={cat}
                type="button"
                onClick={() => setCategoryFilter(cat)}
                className={`px-2 py-0.5 rounded transition ${
                  categoryFilter === cat
                    ? "bg-blue-500/25 text-blue-300 font-semibold"
                    : "text-white/55 hover:text-white"
                }`}
              >
                {cat === "ALL" ? "همه بخش‌ها" : cat}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Logs Scrollable Stream */}
      <div className="flex-1 overflow-y-auto p-3 space-y-1.5 font-mono text-[12px]">
        {filteredLogs.length === 0 ? (
          <div className="py-16 text-center text-white/40 flex flex-col items-center gap-2">
            <Terminal className="w-8 h-8 opacity-30" />
            <span>هیچ رخدادی با فیلترهای انتخابی یافت نشد.</span>
          </div>
        ) : (
          filteredLogs.map((log) => {
            const isExpanded = expandedLogIds.has(log.id);
            const hasDetails = Boolean(log.details && log.details.trim().length > 0);

            return (
              <div
                key={log.id}
                className={`rounded-lg border transition ${
                  log.level === "ERROR"
                    ? "bg-rose-500/[0.04] border-rose-500/25 hover:border-rose-500/40"
                    : log.level === "WARN"
                    ? "bg-amber-500/[0.03] border-amber-500/20 hover:border-amber-500/35"
                    : "bg-[#14151B] border-white/[0.06] hover:border-white/[0.12]"
                }`}
              >
                <div
                  onClick={() => hasDetails && toggleExpand(log.id)}
                  className={`p-2.5 flex items-start gap-2.5 ${
                    hasDetails ? "cursor-pointer select-none" : ""
                  }`}
                >
                  <span className="text-white/40 font-mono text-[11px] shrink-0 pt-0.5">
                    {log.timeFormatted}
                  </span>

                  <div className="shrink-0">{getLevelBadge(log.level)}</div>

                  <span className="text-white/60 font-semibold text-[11px] px-1.5 py-0.5 rounded bg-white/[0.05] shrink-0">
                    {log.category}
                  </span>

                  <span className="flex-1 text-white/85 text-[12px] break-words font-sans leading-relaxed">
                    {log.message}
                  </span>

                  {hasDetails && (
                    <button
                      type="button"
                      className="p-1 rounded text-white/40 hover:text-white shrink-0"
                    >
                      {isExpanded ? (
                        <ChevronDown className="w-3.5 h-3.5" />
                      ) : (
                        <ChevronRight className="w-3.5 h-3.5" />
                      )}
                    </button>
                  )}
                </div>

                {/* Expandable Details Drawer */}
                {isExpanded && log.details && (
                  <div className="px-3 pb-3 pt-1 border-t border-white/[0.06] bg-black/30 text-[11.5px] rounded-b-lg">
                    <div className="flex items-center justify-between pb-1.5 text-white/40 text-[10.5px]">
                      <span>جزئیات رویداد / Stack Trace:</span>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          void navigator.clipboard.writeText(log.details || "");
                        }}
                        className="hover:text-blue-400 flex items-center gap-1"
                      >
                        <ClipboardCopy className="w-3 h-3" />
                        <span>کپی این بخش</span>
                      </button>
                    </div>
                    <pre className="p-2.5 rounded bg-[#090A0D] border border-white/[0.08] text-white/75 font-mono text-[11px] overflow-x-auto whitespace-pre-wrap select-all leading-relaxed max-h-60 overflow-y-auto">
                      {log.details}
                    </pre>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}

export default SystemLogsPanel;
