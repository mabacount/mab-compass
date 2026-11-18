import { useState } from "react";
import {
  AlertCircle,
  ArrowDown,
  ArrowUp,
  ArrowUpToLine,
  ChevronDown,
  ChevronUp,
  Clock,
  PauseCircle,
  Play,
  RefreshCw,
  RotateCcw,
  Trash2,
  Wifi,
  WifiOff,
} from "lucide-react";
import { LlmConnectionStatus, PendingQueueItem } from "../lib/tauri_api";
import { formatShortTime } from "../lib/formatters";

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
  onProcessSingleItem: (queueId: number) => void;
  onReorderItem: (queueId: number, direction: "TOP" | "UP" | "DOWN") => void;
  onResetQueueOrder: () => void;
  onDismissItem: (queueId: number) => void;
  onOpenSettings: () => void;
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
  onProcessSingleItem,
  onReorderItem,
  onResetQueueOrder,
  onDismissItem,
  onOpenSettings,
}: PendingQueueBannerProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  if (queue.length === 0) return null;

  const activeProcessingItem = queue.find((q) => q.id === processingQueueId);
  const isConnected = connectionStatus?.connected ?? false;
  const hasManualOrder = queue.some((q) => (q.priority_order ?? 0) > 0);

  return (
    <div className="mab-card border-amber-500/30 bg-mab-surface p-3 sm:p-3.5 space-y-2.5 shrink-0 shadow-sm select-text">
      <div className="flex items-center justify-between gap-2.5 flex-wrap">
        <div className="flex items-center gap-2.5 min-w-0 flex-1">
          <div className="w-8 h-8 rounded-lg bg-amber-500/15 border border-amber-500/30 text-amber-400 flex items-center justify-center shrink-0">
            {isProcessingQueue ? (
              <RefreshCw className="w-4 h-4 animate-spin" />
            ) : (
              <Clock className="w-4 h-4" />
            )}
          </div>

          <div className="min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-[12.5px] font-bold text-white">
                صف پردازش ورودی‌ها: {queue.length} مورد
              </span>

              <span
                className={`px-2 py-0.5 rounded-full text-[10.5px] font-medium inline-flex items-center gap-1 border ${
                  isCheckingConnection
                    ? "bg-white/[0.04] border-mab-border text-white/50"
                    : isConnected
                    ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-300"
                    : "bg-rose-500/15 border-rose-500/30 text-rose-300"
                }`}
              >
                {isCheckingConnection ? (
                  <RefreshCw className="w-3 h-3 animate-spin" />
                ) : isConnected ? (
                  <Wifi className="w-3 h-3" />
                ) : (
                  <WifiOff className="w-3 h-3" />
                )}
                <span>
                  {isCheckingConnection
                    ? "تست اتصال..."
                    : isConnected
                    ? `متصل (${connectionStatus?.provider})`
                    : "مدل آفلاین"}
                </span>
              </span>

              {hasManualOrder && (
                <span className="px-2 py-0.5 rounded-full bg-purple-500/15 border border-purple-500/30 text-purple-300 text-[10px] font-medium">
                  اولویت دستی
                </span>
              )}
            </div>

            <p className="text-[11.5px] text-mab-text-muted truncate mt-0.5">
              {activeProcessingItem
                ? `در حال تحلیل توسط مدل: ${activeProcessingItem.title_preview}`
                : cooldownSecondsLeft > 0
                ? `تلاش مجدد تا ${cooldownSecondsLeft} ثانیه دیگر (یا دکمه ارسال فوری را بزنید)`
                : isConnected
                ? autoQueueEnabled
                  ? "اتصال برقرار است؛ موارد یکی‌یکی به ترتیب زمان ایجاد (و اولویت دستی) ارسال می‌شوند."
                  : "پردازش خودکار صف متوقف شده است (می‌توانید اولویت را تغییر دهید یا دستی اجرا کنید)."
                : connectionStatus?.message ||
                  "به محض برقراری اتصال، موارد ذخیره‌شده یکی‌یکی به ترتیب زمان ایجاد تحلیل خواهند شد."}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-1.5 shrink-0 flex-wrap">
          <button
            type="button"
            onClick={onRecheckConnection}
            disabled={isCheckingConnection || isProcessingQueue}
            className="px-2.5 py-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-mab-border text-white/70 hover:text-white text-[11px] font-medium inline-flex items-center gap-1 transition disabled:opacity-50"
          >
            <RefreshCw
              className={`w-3 h-3 ${isCheckingConnection ? "animate-spin" : ""}`}
            />
            <span className="hidden sm:inline">تست اتصال</span>
          </button>

          <button
            type="button"
            onClick={() => onToggleAutoQueue(!autoQueueEnabled)}
            className={`px-2.5 py-1 rounded-lg border text-[11px] font-semibold inline-flex items-center gap-1 transition ${
              autoQueueEnabled
                ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-300 hover:bg-emerald-500/25"
                : "bg-amber-500/15 border-amber-500/35 text-amber-300 hover:bg-amber-500/25"
            }`}
            title={
              autoQueueEnabled
                ? "توقف موقت پردازش خودکار صف"
                : "ادامه پردازش خودکار صف"
            }
          >
            {autoQueueEnabled ? (
              <PauseCircle className="w-3 h-3" />
            ) : (
              <Play className="w-3 h-3" />
            )}
            <span>{autoQueueEnabled ? "توقف صف" : "ادامه صف"}</span>
          </button>

          <button
            type="button"
            onClick={onProcessNowAll}
            disabled={isProcessingQueue}
            className="px-3 py-1 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[11px] font-semibold inline-flex items-center gap-1.5 shadow-sm transition disabled:opacity-50"
          >
            <Play className="w-3 h-3" />
            <span>{isProcessingQueue ? "در حال ارسال..." : "پردازش صف الان"}</span>
          </button>

          <button
            type="button"
            onClick={() => setIsExpanded((v) => !v)}
            className="px-2 py-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-mab-border text-white/75 hover:text-white text-[11px] inline-flex items-center gap-1 transition"
            title={isExpanded ? "بستن لیست صف" : "مشاهده و اولویت‌بندی صف"}
          >
            <span className="hidden md:inline">
              {isExpanded ? "بستن لیست" : "اولویت‌بندی"}
            </span>
            {isExpanded ? (
              <ChevronUp className="w-3.5 h-3.5" />
            ) : (
              <ChevronDown className="w-3.5 h-3.5" />
            )}
          </button>
        </div>
      </div>

      {isExpanded && (
        <div className="pt-2 border-t border-mab-border space-y-2 max-h-64 overflow-y-auto">
          <div className="flex items-center justify-between gap-2 px-1 flex-wrap text-[11px] text-mab-text-muted">
            <span>
              ترتیب پیش‌فرض: <strong className="text-white/80">قدیمی‌ترین زمان ایجاد ← جدیدترین</strong> (با دکمه‌های فلش می‌توانید اولویت را تغییر دهید)
            </span>
            {hasManualOrder && (
              <button
                type="button"
                disabled={isProcessingQueue}
                onClick={onResetQueueOrder}
                className="px-2 py-0.5 rounded-md bg-white/[0.04] hover:bg-white/[0.08] border border-mab-border text-white/75 hover:text-white inline-flex items-center gap-1 transition disabled:opacity-40"
              >
                <RotateCcw className="w-3 h-3" />
                <span>بازنشانی به ترتیب زمانی</span>
              </button>
            )}
          </div>

          {queue.map((item, idx) => {
            const isItemBusy = processingQueueId === item.id;
            const isFirst = idx === 0;
            const isLast = idx === queue.length - 1;

            return (
              <div
                key={item.id}
                className={`p-2.5 rounded-xl border flex items-center justify-between gap-2.5 flex-wrap sm:flex-nowrap transition ${
                  isItemBusy
                    ? "bg-blue-500/[0.08] border-blue-500/35"
                    : "bg-white/[0.02] border-mab-border"
                }`}
              >
                <div className="min-w-0 flex-1 space-y-0.5">
                  <div className="flex items-center gap-1.5 flex-wrap text-[11.5px]">
                    <span className="font-mono px-1.5 py-0.2 rounded bg-white/[0.06] text-blue-400 font-bold text-[10.5px]">
                      #{idx + 1}
                    </span>
                    <span
                      className={`px-1.5 py-0.2 rounded font-mono text-[10px] ${
                        item.source_type === "MAB_FOCUS_SYNC"
                          ? "bg-cyan-500/15 text-cyan-300"
                          : item.channel === "VAULT"
                          ? "bg-purple-500/15 text-purple-300"
                          : "bg-blue-500/15 text-blue-300"
                      }`}
                    >
                      {item.source_type === "MAB_FOCUS_SYNC"
                        ? "Mab-Focus"
                        : item.channel === "VAULT"
                        ? "سند/دانش"
                        : "همیار"}
                    </span>
                    <span className="font-medium text-white truncate max-w-[300px] sm:max-w-[380px]">
                      {item.title_preview}
                    </span>
                  </div>

                  <div className="flex items-center gap-2 text-[10.5px] text-mab-text-muted flex-wrap">
                    <span className="font-timer">
                      زمان ایجاد: {formatShortTime(item.created_at)}
                    </span>
                    {item.last_error && (
                      <span className="text-rose-400 inline-flex items-center gap-1 truncate max-w-[320px]">
                        <AlertCircle className="w-3 h-3 shrink-0" />
                        <span className="truncate">{item.last_error}</span>
                      </span>
                    )}
                  </div>
                </div>

                <div className="flex items-center gap-1 shrink-0 flex-wrap">
                  {/* Reorder Buttons */}
                  {queue.length > 1 && (
                    <div className="flex items-center gap-0.5 bg-white/[0.03] border border-mab-border rounded-lg p-0.5 ml-1">
                      <button
                        type="button"
                        disabled={isFirst || isProcessingQueue}
                        onClick={() => onReorderItem(item.id, "TOP")}
                        title="انتقال به اولویت اول صف"
                        className="p-1 rounded hover:bg-white/[0.08] text-white/65 hover:text-amber-300 disabled:opacity-25 transition"
                      >
                        <ArrowUpToLine className="w-3 h-3" />
                      </button>
                      <button
                        type="button"
                        disabled={isFirst || isProcessingQueue}
                        onClick={() => onReorderItem(item.id, "UP")}
                        title="یک رتبه بالاتر"
                        className="p-1 rounded hover:bg-white/[0.08] text-white/65 hover:text-white disabled:opacity-25 transition"
                      >
                        <ArrowUp className="w-3 h-3" />
                      </button>
                      <button
                        type="button"
                        disabled={isLast || isProcessingQueue}
                        onClick={() => onReorderItem(item.id, "DOWN")}
                        title="یک رتبه پایین‌تر"
                        className="p-1 rounded hover:bg-white/[0.08] text-white/65 hover:text-white disabled:opacity-25 transition"
                      >
                        <ArrowDown className="w-3 h-3" />
                      </button>
                    </div>
                  )}

                  <button
                    type="button"
                    disabled={isProcessingQueue}
                    onClick={() => onProcessSingleItem(item.id)}
                    className="px-2.5 py-1 rounded-lg bg-blue-500/15 hover:bg-blue-500/25 border border-blue-500/30 text-blue-300 text-[11px] font-medium inline-flex items-center gap-1 transition disabled:opacity-50"
                  >
                    <RefreshCw
                      className={`w-3 h-3 ${isItemBusy ? "animate-spin" : ""}`}
                    />
                    <span>{isItemBusy ? "در حال تحلیل..." : "تحلیل الان"}</span>
                  </button>

                  <button
                    type="button"
                    disabled={isItemBusy}
                    onClick={() => onDismissItem(item.id)}
                    title="حذف از صف انتظار"
                    className="p-1 rounded-lg bg-white/[0.03] hover:bg-rose-500/15 text-white/50 hover:text-rose-400 transition disabled:opacity-50"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            );
          })}

          {!isConnected && (
            <div className="flex items-center justify-between gap-2 px-1 pt-1 text-[11px] text-mab-text-muted flex-wrap">
              <span>جهت استفاده از مدل دیگر، تنظیمات را باز کنید:</span>
              <button
                type="button"
                onClick={onOpenSettings}
                className="text-blue-400 font-semibold hover:underline"
              >
                تنظیمات مدل هوش مصنوعی
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default PendingQueueBanner;
