import { useMemo, useState } from "react";
import {
  ArrowUp,
  ArrowUpToLine,
  ArrowDown,
  ChevronDown,
  ChevronUp,
  Clock3,
  Layers,
  Loader2,
  Package,
  Play,
  RefreshCw,
  RotateCcw,
  Sparkles,
  Trash2,
  Wifi,
  WifiOff,
  Zap,
} from "lucide-react";
import { LlmConnectionStatus, PendingQueueItem } from "../lib/tauri_api";

interface PendingQueueBannerProps {
  queue: PendingQueueItem[];
  connectionStatus: LlmConnectionStatus | null;
  isCheckingConnection: boolean;
  isProcessingQueue: boolean;
  processingQueueId: number | null;
  autoQueueEnabled: boolean;
  cooldownSecondsLeft: number;
  onToggleAutoQueue: (enabled: boolean) => void;
  onRecheckConnection: () => void;
  onProcessNowAll: () => void;
  onProcessBackgroundBatch: () => void;
  onProcessSingleItem: (queueId: number) => void;
  onReorderItem: (queueId: number, direction: "TOP" | "UP" | "DOWN") => void;
  onResetQueueOrder: () => void;
  onDismissItem: (queueId: number) => void;
  onOpenSettings: () => void;
}

function sourceTypeLabel(sourceType: string): string {
  switch (sourceType) {
    case "USER_CHAT":
      return "دستور مستقیم";
    case "MAB_FOCUS_SYNC":
      return "یادداشت همگام‌سازی Mab-Focus";
    case "LEGACY_REPROCESS_DOC":
      return "بازپردازش سند آرشیو";
    case "LEGACY_REPROCESS":
      return "بازپردازش رویداد آرشیو";
    default:
      return sourceType;
  }
}

export function PendingQueueBanner({
  queue,
  connectionStatus,
  isCheckingConnection,
  isProcessingQueue,
  processingQueueId,
  autoQueueEnabled,
  cooldownSecondsLeft,
  onToggleAutoQueue,
  onRecheckConnection,
  onProcessNowAll,
  onProcessBackgroundBatch,
  onProcessSingleItem,
  onReorderItem,
  onResetQueueOrder,
  onDismissItem,
  onOpenSettings,
}: PendingQueueBannerProps) {
  const [expanded, setExpanded] = useState(false);

  const { userCommands, backgroundItems, failedCount } = useMemo(() => {
    const cmds = queue.filter((q) => q.queue_kind === "USER_COMMAND");
    const bgs = queue.filter((q) => q.queue_kind !== "USER_COMMAND");
    const fails = queue.filter((q) => q.status === "FAILED").length;
    return { userCommands: cmds, backgroundItems: bgs, failedCount: fails };
  }, [queue]);

  if (queue.length === 0) return null;

  const isConnected = !!connectionStatus?.connected;
  const activeItem = queue.find((q) => q.id === processingQueueId || q.status === "PROCESSING");

  const renderQueueRow = (item: PendingQueueItem, idx: number, listLen: number, isUserCmd: boolean) => {
    const isProcessingThis = processingQueueId === item.id || item.status === "PROCESSING";
    const isFailed = item.status === "FAILED";
    return (
      <div
        key={item.id}
        className={`rounded-xl border px-3 py-2 flex items-start justify-between gap-2.5 text-[11.5px] transition ${
          isProcessingThis
            ? "bg-blue-500/15 border-blue-400/50"
            : isFailed
            ? "bg-rose-500/10 border-rose-500/35"
            : isUserCmd
            ? "bg-indigo-500/[0.08] border-indigo-400/25"
            : "bg-white/[0.03] border-white/[0.07]"
        }`}
      >
        <div className="min-w-0 flex-1 space-y-1">
          <div className="flex items-center gap-1.5 flex-wrap">
            <span
              className={`px-1.5 py-0.5 rounded text-[9.5px] font-semibold ${
                isUserCmd
                  ? "bg-indigo-500/25 text-indigo-200 border border-indigo-400/35"
                  : "bg-amber-500/15 text-amber-200 border border-amber-400/25"
              }`}
            >
              {isUserCmd ? "مستقل • دستور کاربر" : `پکیج • ${sourceTypeLabel(item.source_type)}`}
            </span>
            <span className="px-1.5 py-0.5 rounded bg-white/[0.07] text-white/75 text-[9.5px]">
              {item.project_name || item.project_id}
            </span>
            <span className="text-[9.5px] font-mono text-white/40">#{item.id}</span>
            {item.knowledge_doc_id && (
              <span className="text-[9.5px] font-mono text-cyan-300/80">
                doc:{item.knowledge_doc_id}
              </span>
            )}
            <span className="text-[9.5px] font-mono text-white/40">
              {item.created_at.slice(0, 16).replace("T", " ")}
            </span>
          </div>

          <div className="text-white/90 font-medium leading-snug break-words">
            {item.title_preview}
          </div>

          {item.last_error && (
            <div className="text-[10.5px] text-rose-300 bg-rose-950/40 rounded px-2 py-1">
              خطا (تلاش {item.attempts}): {item.last_error}
            </div>
          )}
        </div>

        <div className="flex items-center gap-1 shrink-0 pt-0.5">
          {idx > 0 && (
            <>
              <button
                type="button"
                disabled={isProcessingQueue}
                onClick={() => onReorderItem(item.id, "TOP")}
                title="انتقال به ابتدای صف"
                className="p-1 rounded bg-white/[0.06] hover:bg-white/[0.14] text-white/65 disabled:opacity-40"
              >
                <ArrowUpToLine className="w-3 h-3" />
              </button>
              <button
                type="button"
                disabled={isProcessingQueue}
                onClick={() => onReorderItem(item.id, "UP")}
                title="اولویت بالاتر"
                className="p-1 rounded bg-white/[0.06] hover:bg-white/[0.14] text-white/65 disabled:opacity-40"
              >
                <ArrowUp className="w-3 h-3" />
              </button>
            </>
          )}
          {idx < listLen - 1 && (
            <button
              type="button"
              disabled={isProcessingQueue}
              onClick={() => onReorderItem(item.id, "DOWN")}
              title="اولویت پایین‌تر"
              className="p-1 rounded bg-white/[0.06] hover:bg-white/[0.14] text-white/65 disabled:opacity-40"
            >
              <ArrowDown className="w-3 h-3" />
            </button>
          )}
          <button
            type="button"
            disabled={isProcessingQueue}
            onClick={() => onProcessSingleItem(item.id)}
            title="پردازش تکی همین مورد"
            className="px-2 py-1 rounded bg-blue-600/80 hover:bg-blue-500 text-white text-[10px] font-semibold flex items-center gap-1 disabled:opacity-40"
          >
            {isProcessingThis ? <Loader2 className="w-3 h-3 animate-spin" /> : <Play className="w-3 h-3" />}
            <span>ارسال</span>
          </button>
          <button
            type="button"
            disabled={isProcessingThis}
            onClick={() => onDismissItem(item.id)}
            title="حذف از صف"
            className="p-1 rounded bg-rose-500/15 hover:bg-rose-500/30 text-rose-300 disabled:opacity-40"
          >
            <Trash2 className="w-3 h-3" />
          </button>
        </div>
      </div>
    );
  };

  return (
    <div className="rounded-2xl border border-white/[0.09] bg-[#151720] shadow-lg overflow-hidden">
      {/* Summary Header Bar */}
      <div className="px-3 py-2 flex items-center justify-between gap-2 flex-wrap">
        <button
          type="button"
          onClick={() => setExpanded((v) => !v)}
          className="flex items-center gap-2 text-right min-w-0 flex-1"
        >
          {isProcessingQueue ? (
            <Loader2 className="w-4 h-4 text-blue-400 animate-spin shrink-0" />
          ) : isConnected ? (
            <Wifi className="w-4 h-4 text-emerald-400 shrink-0" />
          ) : (
            <WifiOff className="w-4 h-4 text-amber-400 shrink-0" />
          )}

          <div className="flex items-center gap-1.5 flex-wrap text-[11.5px]">
            <span className="font-bold text-white">صف پردازش ({queue.length})</span>
            {userCommands.length > 0 && (
              <span className="px-2 py-0.5 rounded-full bg-indigo-500/20 border border-indigo-400/30 text-indigo-200 text-[10px] font-semibold">
                دستور مستقل: {userCommands.length}
              </span>
            )}
            {backgroundItems.length > 0 && (
              <span className="px-2 py-0.5 rounded-full bg-amber-500/15 border border-amber-400/30 text-amber-200 text-[10px] font-semibold flex items-center gap-1">
                <Package className="w-3 h-3" />
                <span>زمینه (پک ۲۰تایی): {backgroundItems.length}</span>
              </span>
            )}
            {failedCount > 0 && (
              <span className="px-2 py-0.5 rounded-full bg-rose-500/20 text-rose-200 text-[10px]">
                خطا: {failedCount}
              </span>
            )}
            {activeItem && (
              <span className="text-[10.5px] text-blue-300 truncate max-w-[200px]">
                در حال پردازش: {activeItem.title_preview}
              </span>
            )}
            {!isConnected && cooldownSecondsLeft > 0 && (
              <span className="inline-flex items-center gap-1 text-[10px] text-amber-300 font-mono">
                <Clock3 className="w-3 h-3" />
                {cooldownSecondsLeft}s
              </span>
            )}
          </div>

          {expanded ? (
            <ChevronUp className="w-4 h-4 text-white/50 shrink-0" />
          ) : (
            <ChevronDown className="w-4 h-4 text-white/50 shrink-0" />
          )}
        </button>

        {/* Action Controls */}
        <div className="flex items-center gap-1.5 flex-wrap shrink-0">
          <button
            type="button"
            onClick={() => onToggleAutoQueue(!autoQueueEnabled)}
            className={`px-2 py-1 rounded-lg text-[10.5px] border transition ${
              autoQueueEnabled
                ? "bg-emerald-500/15 border-emerald-400/35 text-emerald-200"
                : "bg-white/[0.05] border-white/10 text-white/55"
            }`}
            title="پردازش خودکار صف هنگام اتصال مدل"
          >
            {autoQueueEnabled ? "خودکار: روشن" : "خودکار: خاموش"}
          </button>

          {backgroundItems.length > 0 && (
            <button
              type="button"
              disabled={isProcessingQueue}
              onClick={onProcessBackgroundBatch}
              title="بسته‌بندی تا ۲۰ مورد زمینه با متادیتا و پردازش همه در یک نوبت مدل"
              className="px-2.5 py-1 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-400/35 text-amber-100 text-[10.5px] font-semibold flex items-center gap-1 disabled:opacity-40"
            >
              <Package className="w-3 h-3" />
              <span>پک و پردازش زمینه ({Math.min(20, backgroundItems.length)})</span>
            </button>
          )}

          <button
            type="button"
            disabled={isProcessingQueue}
            onClick={onProcessNowAll}
            className="px-2.5 py-1 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[10.5px] font-semibold flex items-center gap-1 disabled:opacity-40"
          >
            {isProcessingQueue ? <Loader2 className="w-3 h-3 animate-spin" /> : <Zap className="w-3 h-3" />}
            <span>اجرای کامل صف</span>
          </button>

          <button
            type="button"
            disabled={isCheckingConnection}
            onClick={onRecheckConnection}
            title="بررسی مجدد اتصال مدل"
            className="p-1.5 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] text-white/70"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isCheckingConnection ? "animate-spin" : ""}`} />
          </button>
        </div>
      </div>

      {/* Expanded Two-Track View */}
      {expanded && (
        <div className="px-3 pb-3 pt-2 border-t border-white/[0.07] space-y-3 max-h-80 overflow-y-auto">
          {!isConnected && (
            <div className="px-3 py-2 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-between gap-2 text-[11px] text-amber-200">
              <span>{connectionStatus?.message || "اتصال به مدل برقرار نیست."}</span>
              <button
                type="button"
                onClick={onOpenSettings}
                className="px-2 py-1 rounded bg-amber-500/20 hover:bg-amber-500/30 text-amber-100 font-semibold"
              >
                تنظیمات مدل
              </button>
            </div>
          )}

          {/* Track 1: Independent User Commands */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between text-[11px]">
              <span className="font-bold text-indigo-300 flex items-center gap-1.5">
                <Sparkles className="w-3.5 h-3.5" />
                <span>مسیر ۱: دستورات مستقیم من ({userCommands.length}) — پردازش مستقل و اولویت‌دار</span>
              </span>
              {userCommands.length > 1 && (
                <button
                  type="button"
                  onClick={onResetQueueOrder}
                  className="text-[10px] text-white/55 hover:text-white flex items-center gap-1"
                >
                  <RotateCcw className="w-3 h-3" />
                  <span>ترتیب زمانی</span>
                </button>
              )}
            </div>
            {userCommands.length === 0 ? (
              <div className="text-[11px] text-white/40 px-2">دستور مستقیم منتظری در صف نیست.</div>
            ) : (
              <div className="space-y-1.5">
                {userCommands.map((item, idx) => renderQueueRow(item, idx, userCommands.length, true))}
              </div>
            )}
          </div>

          {/* Track 2: Packed Background Items */}
          <div className="space-y-1.5 pt-2 border-t border-white/[0.06]">
            <div className="flex items-center justify-between gap-2 flex-wrap text-[11px]">
              <span className="font-bold text-amber-300 flex items-center gap-1.5">
                <Layers className="w-3.5 h-3.5" />
                <span>
                  مسیر ۲: پردازش زمینه ({backgroundItems.length}) — پک‌بندی تا ۲۰ مورد همراه با متادیتا در یک درخواست
                </span>
              </span>
            </div>
            {backgroundItems.length === 0 ? (
              <div className="text-[11px] text-white/40 px-2">مورد زمینه‌ای در صف نیست.</div>
            ) : (
              <div className="space-y-1.5">
                {backgroundItems.map((item, idx) =>
                  renderQueueRow(item, idx, backgroundItems.length, false)
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
