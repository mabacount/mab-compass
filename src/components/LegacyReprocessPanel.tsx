import { useCallback, useEffect, useMemo, useState } from "react";
import {
  CheckCircle2,
  Clock,
  DatabaseBackup,
  FileText,
  History,
  MessageSquare,
  Play,
  RefreshCw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import {
  CompassApi,
  DashboardState,
  LegacyReprocessCandidate,
} from "../lib/tauri_api";
import { formatShortTime } from "../lib/formatters";
import { MabCheckbox } from "./ui/MabCheckbox";

interface LegacyReprocessPanelProps {
  projectId: string;
  onDashboardUpdated: (dash: DashboardState) => void;
  onToast: (msg: string) => void;
  onOpenArtifact?: (candidate: LegacyReprocessCandidate) => void;
}

type FilterMode = "ALL" | "LEGACY_ONLY" | "DOCS" | "EVENTS";

export function LegacyReprocessPanel({
  projectId,
  onDashboardUpdated,
  onToast,
  onOpenArtifact,
}: LegacyReprocessPanelProps) {
  const [candidates, setCandidates] = useState<LegacyReprocessCandidate[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRunning, setIsRunning] = useState(false);
  const [isBackingUp, setIsBackingUp] = useState(false);
  const [filterMode, setFilterMode] = useState<FilterMode>("LEGACY_ONLY");
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(() => new Set());
  const [lastBackupInfo, setLastBackupInfo] = useState<{
    path: string;
    sizeKb: number;
  } | null>(null);

  const itemKey = (c: LegacyReprocessCandidate) => `${c.source_kind}:${c.source_id}`;

  const loadCandidates = useCallback(async () => {
    setIsLoading(true);
    try {
      const list = await CompassApi.listLegacyReprocessCandidates();
      setCandidates(list);
    } catch (e) {
      onToast(`خطا در اسکن حافظه قدیمی: ${String(e)}`);
    } finally {
      setIsLoading(false);
    }
  }, [onToast]);

  useEffect(() => {
    void loadCandidates();
    setSelectedKeys(new Set());
  }, [projectId, loadCandidates]);

  const unUpgradedCount = useMemo(
    () => candidates.filter((c) => !c.already_upgraded).length,
    [candidates]
  );
  const upgradedCount = candidates.length - unUpgradedCount;

  const filtered = useMemo(() => {
    return candidates.filter((c) => {
      if (filterMode === "LEGACY_ONLY") return !c.already_upgraded;
      if (filterMode === "DOCS") return c.source_kind === "KNOWLEDGE_DOC";
      if (filterMode === "EVENTS") return c.source_kind === "RAW_EVENT";
      return true;
    });
  }, [candidates, filterMode]);

  const toggleSelect = (key: string) => {
    setSelectedKeys((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const selectAllVisible = () => {
    setSelectedKeys(new Set(filtered.map(itemKey)));
  };

  const clearSelection = () => {
    setSelectedKeys(new Set());
  };

  const handleManualBackup = async () => {
    if (isBackingUp) return;
    setIsBackingUp(true);
    try {
      const res = await CompassApi.backupDatabaseNow();
      const sizeKb = Math.max(1, Math.round(res.backup_size_bytes / 1024));
      setLastBackupInfo({ path: res.backup_path, sizeKb });
      onToast(`پشتیبان اتمیک دیتابیس با موفقیت ذخیره شد (${sizeKb} KB).`);
    } catch (e) {
      onToast(`خطا در پشتیبان‌گیری: ${String(e)}`);
    } finally {
      setIsBackingUp(false);
    }
  };

  const handleEnqueueAll = async () => {
    if (isRunning || unUpgradedCount === 0) return;
    setIsRunning(true);
    try {
      const res = await CompassApi.enqueueLegacyReprocess("ALL_CHRONOLOGICAL", []);
      const sizeKb = Math.max(1, Math.round(res.backup_size_bytes / 1024));
      setLastBackupInfo({ path: res.backup_path, sizeKb });
      onDashboardUpdated(res.dashboard);
      onToast(
        `پشتیبان خودکار ذخیره شد (${sizeKb} KB) و ${res.enqueued_count} سند/رویداد قدیمی به ترتیب زمانی در صف بازپردازش بدون‌تکرار قرار گرفت.`
      );
      void loadCandidates();
    } catch (e) {
      onToast(`خطا در شروع بازپردازش کل حافظه: ${String(e)}`);
    } finally {
      setIsRunning(false);
    }
  };

  const handleEnqueueSelected = async () => {
    if (isRunning || selectedKeys.size === 0) return;
    setIsRunning(true);
    try {
      const selections = Array.from(selectedKeys).map((k) => {
        const [source_kind, idStr] = k.split(":");
        return { source_kind, source_id: Number(idStr) };
      });
      const res = await CompassApi.enqueueLegacyReprocess("SELECTED", selections);
      const sizeKb = Math.max(1, Math.round(res.backup_size_bytes / 1024));
      setLastBackupInfo({ path: res.backup_path, sizeKb });
      onDashboardUpdated(res.dashboard);
      setSelectedKeys(new Set());
      onToast(
        `پشتیبان خودکار ذخیره شد (${sizeKb} KB) و ${res.enqueued_count} مورد انتخاب‌شده به ترتیب زمانی وارد صف بازپردازش شدند.`
      );
      void loadCandidates();
    } catch (e) {
      onToast(`خطا در ارسال موارد انتخاب‌شده: ${String(e)}`);
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div dir="rtl" className="mab-card p-4 space-y-3.5 border-purple-500/25 bg-[#131419]">
      {/* Header */}
      <div className="flex items-start justify-between gap-3 flex-wrap">
        <div className="space-y-1">
          <div className="flex items-center gap-2 text-[13.5px] font-bold text-white">
            <History className="w-4 h-4 text-purple-400 shrink-0" />
            <span>بازپردازش هوشمند حافظه و اسناد قدیمی (ارتقا به متد + BPMN + کارت‌های دانش)</span>
          </div>
          <p className="text-[11.5px] text-mab-text-secondary leading-relaxed">
            قبل از شروع، به صورت خودکار یک نسخه پشتیبان کامل از دیتابیس ذخیره می‌شود و سپس موارد قدیمی با{" "}
            <strong className="text-emerald-300">حالت ضدتکرار (Idempotent)</strong> به ترتیب زمانی تحلیل می‌شوند تا
            بدون تکرار تسک‌های قبلی، متدها، نمودارهای BPMN 2.0، کارت‌های حافظه و درس‌آموخته‌های جاافتاده استخراج شوند.
          </p>
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          <button
            type="button"
            disabled={isBackingUp}
            onClick={() => void handleManualBackup()}
            className="px-2.5 py-1.5 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.09] text-white/80 text-[11px] font-medium inline-flex items-center gap-1.5 transition disabled:opacity-40"
          >
            <DatabaseBackup className="w-3.5 h-3.5 text-emerald-400" />
            <span>{isBackingUp ? "در حال بکاپ..." : "بکاپ دستی دیتابیس"}</span>
          </button>

          <button
            type="button"
            disabled={isLoading}
            onClick={() => void loadCandidates()}
            className="p-1.5 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.09] text-white/70 hover:text-white transition"
            title="بروزرسانی لیست حافظه قدیمی"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? "animate-spin" : ""}`} />
          </button>
        </div>
      </div>

      {/* Backup confirmation banner */}
      {lastBackupInfo && (
        <div className="p-2.5 rounded-xl bg-emerald-500/[0.08] border border-emerald-500/25 flex items-center justify-between gap-2 text-[11.5px] text-emerald-300 flex-wrap">
          <span className="inline-flex items-center gap-1.5 font-medium">
            <ShieldCheck className="w-4 h-4 shrink-0" />
            <span>پشتیبان اتمیک دیتابیس با موفقیت ذخیره شد ({lastBackupInfo.sizeKb} KB):</span>
          </span>
          <code dir="ltr" className="font-mono text-[10.5px] text-emerald-200/80 truncate max-w-full">
            {lastBackupInfo.path}
          </code>
        </div>
      )}

      {/* Action Buttons: Both Batch All + Selective */}
      <div className="flex items-center justify-between gap-2.5 flex-wrap bg-[#0E0F12] p-3 rounded-xl border border-white/[0.07]">
        <div className="flex items-center gap-2 flex-wrap">
          <button
            type="button"
            disabled={isRunning || unUpgradedCount === 0}
            onClick={() => void handleEnqueueAll()}
            className="px-3.5 py-2 rounded-xl bg-purple-600 hover:bg-purple-500 text-white text-[12px] font-semibold inline-flex items-center gap-1.5 shadow-sm transition disabled:opacity-40"
          >
            <Sparkles className="w-3.5 h-3.5" />
            <span>
              بازپردازش هوشمند کل حافظه قدیمی ({unUpgradedCount} مورد به ترتیب زمانی)
            </span>
          </button>

          <button
            type="button"
            disabled={isRunning || selectedKeys.size === 0}
            onClick={() => void handleEnqueueSelected()}
            className="px-3.5 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-[12px] font-semibold inline-flex items-center gap-1.5 shadow-sm transition disabled:opacity-40"
          >
            <Play className="w-3.5 h-3.5" />
            <span>بازپردازش موارد انتخاب‌شده ({selectedKeys.size})</span>
          </button>
        </div>

        <div className="flex items-center gap-1.5 text-[11px] text-mab-text-muted">
          <span className="px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-300 font-semibold">
            {unUpgradedCount} قدیمی
          </span>
          <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-300 font-semibold">
            {upgradedCount} ارتقایافته
          </span>
        </div>
      </div>

      {/* Filter & Selection Toolbar */}
      <div className="flex items-center justify-between gap-2 flex-wrap text-[11px]">
        <div className="flex items-center gap-1 bg-[#0E0F12] p-0.5 rounded-lg border border-white/[0.07]">
          {[
            { id: "LEGACY_ONLY" as const, label: `ارتقانیافته (${unUpgradedCount})` },
            { id: "ALL" as const, label: `همه (${candidates.length})` },
            { id: "DOCS" as const, label: "اسناد" },
            { id: "EVENTS" as const, label: "چت و گزارش‌ها" },
          ].map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setFilterMode(tab.id)}
              className={`px-2.5 py-1 rounded-md transition ${
                filterMode === tab.id
                  ? "bg-white/[0.1] text-white font-semibold"
                  : "text-mab-text-muted hover:text-white"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={selectAllVisible}
            className="text-blue-400 hover:underline font-medium"
          >
            انتخاب همه ({filtered.length})
          </button>
          {selectedKeys.size > 0 && (
            <button
              type="button"
              onClick={clearSelection}
              className="text-white/50 hover:text-white"
            >
              پاک کردن انتخاب
            </button>
          )}
        </div>
      </div>

      {/* Chronological Candidate List with Checkboxes */}
      <div className="space-y-1.5 max-h-[280px] overflow-y-auto pl-0.5">
        {filtered.map((item) => {
          const key = itemKey(item);
          const checked = selectedKeys.has(key);
          return (
            <div
              key={key}
              onClick={() => toggleSelect(key)}
              className={`p-2.5 rounded-xl border flex items-start gap-2.5 cursor-pointer transition select-none ${
                checked
                  ? "bg-blue-500/[0.08] border-blue-500/35"
                  : "bg-[#0E0F12]/80 hover:bg-white/[0.03] border-white/[0.06]"
              }`}
            >
              <MabCheckbox
                checked={checked}
                onChange={() => toggleSelect(key)}
                className="mt-0.5"
              />

              <div className="flex-1 min-w-0 space-y-0.5">
                <div className="flex items-center justify-between gap-2 flex-wrap">
                  <div className="flex items-center gap-1.5 min-w-0">
                    {item.source_kind === "KNOWLEDGE_DOC" ? (
                      <FileText className="w-3.5 h-3.5 text-purple-400 shrink-0" />
                    ) : (
                      <MessageSquare className="w-3.5 h-3.5 text-blue-400 shrink-0" />
                    )}
                    <span className="text-[12px] font-semibold text-white truncate">
                      {item.title}
                    </span>
                    <span className="px-1.5 py-[2px] rounded bg-white/[0.06] font-mono text-[9.5px] text-white/60 shrink-0">
                      {item.doc_type}
                    </span>
                  </div>

                  <div className="flex items-center gap-1.5 shrink-0">
                    {item.already_upgraded ? (
                      <span className="px-2 py-[2px] rounded-full bg-emerald-500/15 text-emerald-300 text-[10px] font-semibold inline-flex items-center gap-1">
                        <CheckCircle2 className="w-2.5 h-2.5" />
                        <span>ارتقایافته</span>
                      </span>
                    ) : (
                      <span className="px-2 py-[2px] rounded-full bg-amber-500/15 text-amber-300 text-[10px] font-semibold inline-flex items-center gap-1">
                        <Clock className="w-2.5 h-2.5" />
                        <span>نیازمند ارتقا</span>
                      </span>
                    )}
                    <span className="font-timer text-[10px] text-mab-text-muted">
                      {formatShortTime(item.created_at)}
                    </span>
                  </div>
                </div>

                <div className="flex items-center justify-between gap-2">
                  <p className="text-[11px] text-mab-text-secondary line-clamp-1">
                    {item.snippet}
                  </p>
                  {onOpenArtifact && item.archived_path && (
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onOpenArtifact(item);
                      }}
                      className="text-[10.5px] text-blue-400 hover:underline shrink-0"
                    >
                      مشاهده
                    </button>
                  )}
                </div>
              </div>
            </div>
          );
        })}

        {!isLoading && filtered.length === 0 && (
          <div className="py-6 text-center text-[12px] text-mab-text-muted">
            {filterMode === "LEGACY_ONLY"
              ? "تمام اسناد و رویدادهای قدیمی این پروژه با رویکرد جدید بازپردازش شده‌اند!"
              : "موردی برای نمایش در این فیلتر یافت نشد."}
          </div>
        )}
      </div>
    </div>
  );
}

export default LegacyReprocessPanel;
