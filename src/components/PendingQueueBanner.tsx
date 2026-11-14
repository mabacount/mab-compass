import { useState } from "react";
import {
  AlertCircle,
  ChevronDown,
  ChevronUp,
  Clock,
  PauseCircle,
  Play,
  RefreshCw,
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
  onDismissItem,
  onOpenSettings,
}: PendingQueueBannerProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  if (queue.length === 0) return null;

  const activeProcessingItem = queue.find((q) => q.id === processingQueueId);
  const isConnected = connectionStatus?.connected ?? false;

  return (
    <div className="mab-card border-amber-500/30 bg-[#16171B] p-3 space-y-2.5 shrink-0 shadow-sm select-text">
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
                صف انتظار بررسی مدل: {queue.length} پیام/سند
              </span>

              <span
                className={`px-2 py-0.5 rounded-full text-[10.5px] font-medium inline-flex items-center gap-1 border ${
                  isCheckingConnection
                    ? "bg-white/[0.04] border-white/[0.08] text-white/50"
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
            </div>

            <p className="text-[11.5px] text-mab-text-muted truncate mt-0.5">
              {activeProcessingItem
                ? `در حال تحلیل خودکار: ${activeProcessingItem.title_preview}`
                : cooldownSecondsLeft > 0
                ? `تلاش مجدد تا ${cooldownSecondsLeft} ثانیه دیگر (یا دکمه ارسال فوری را بزنید)`
                : isConnected
                ? autoQueueEnabled
                  ? "اتصال برقرار است؛ موارد صف به‌ترتیب ارسال می‌شوند."
                  : "ارسال خودکار متوقف است (آماده پردازش دستی)."
                : connectionStatus?.message ||
                  "به محض برقراری اتصال، موارد ذخیره‌شده خودکار تحلیل خواهند شد."}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-1.5 shrink-0 flex-wrap">
          <button
            type="button"
            onClick={onRecheckConnection}
            disabled={isCheckingConnection || isProcessingQueue}
            className="px-2.5 py-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] text-white/70 hover:text-white text-[11px] font-medium inline-flex items-center gap-1 transition disabled:opacity-50"
          >
            <RefreshCw
              className={`w-3 h-3 ${isCheckingConnection ? "animate-spin" : ""}`}
            />
            <span className="hidden sm:inline">تست اتصال</span>
          </button>

          <button
            type="button"
            onClick={() => onToggleAutoQueue(!autoQueueEnabled)}
            className={`px-2.5 py-1 rounded-lg border text-[11px] font-medium inline-flex items-center gap-1 transition ${
              autoQueueEnabled
                ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-300"
                : "bg-white/[0.04] border-white/[0.08] text-white/70"
            }`}
          >
            {autoQueueEnabled ? (
              <Play className="w-3 h-3" />
            ) : (
              <PauseCircle className="w-3 h-3" />
            )}
            <span>{autoQueueEnabled ? "خودکار روشن" : "خودکار متوقف"}</span>
          </button>

          <button
            type="button"
            onClick={onProcessNowAll}
            disabled={isProcessingQueue}
            className="px-3 py-1 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[11px] font-semibold inline-flex items-center gap-1.5 shadow-sm transition disabled:opacity-50"
          >
            <Play className="w-3 h-3" />
            <span>{isProcessingQueue ? "در حال ارسال..." : "ارسال صف الان"}</span>
          </button>

          <button
            type="button"
            onClick={() => setIsExpanded((v) => !v)}
            className="p-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] text-white/70 hover:text-white transition"
            title={isExpanded ? "بستن لیست" : "مشاهده لیست موارد"}
          >
            {isExpanded ? (
              <ChevronUp className="w-3.5 h-3.5" />
            ) : (
              <ChevronDown className="w-3.5 h-3.5" />
            )}
          </button>
        </div>
      </div>

      {isExpanded && (
        <div className="pt-2 border-t border-white/[0.06] space-y-1.5 max-h-56 overflow-y-auto">
          {queue.map((item, idx) => {
            const isItemBusy = processingQueueId === item.id;
            return (
              <div
                key={item.id}
                className="p-2.5 rounded-xl bg-white/[0.02] border border-white/[0.06] flex items-center justify-between gap-3 flex-wrap sm:flex-nowrap"
              >
                <div className="min-w-0 flex-1 space-y-0.5">
                  <div className="flex items-center gap-1.5 flex-wrap text-[11.5px]">
                    <span className="font-mono px-1.5 py-0.2 rounded bg-white/[0.06] text-blue-400 font-bold text-[10.5px]">
                      #{idx + 1}
                    </span>
                    <span className="px-1.5 py-0.2 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px]">
                      {item.channel === "VAULT" ? "دانش" : "همیار"}
                    </span>
                    <span className="font-medium text-white truncate max-w-[340px]">
                      {item.title_preview}
                    </span>
                  </div>

                  <div className="flex items-center gap-2 text-[10.5px] text-mab-text-muted">
                    <span className="font-timer">{formatShortTime(item.created_at)}</span>
                    {item.last_error && (
                      <span className="text-rose-400 inline-flex items-center gap-1 truncate max-w-[350px]">
                        <AlertCircle className="w-3 h-3 shrink-0" />
                        <span className="truncate">{item.last_error}</span>
                      </span>
                    )}
                  </div>
                </div>

                <div className="flex items-center gap-1.5 shrink-0">
                  <button
                    type="button"
                    disabled={isProcessingQueue}
                    onClick={() => onProcessSingleItem(item.id)}
                    className="px-2.5 py-1 rounded-lg bg-blue-500/15 hover:bg-blue-500/25 border border-blue-500/30 text-blue-300 text-[11px] font-medium inline-flex items-center gap-1 transition disabled:opacity-50"
                  >
                    <RefreshCw
                      className={`w-3 h-3 ${isItemBusy ? "animate-spin" : ""}`}
                    />
                    <span>{isItemBusy ? "در حال ارسال..." : "تحلیل تکی"}</span>
                  </button>

                  <button
                    type="button"
                    disabled={isItemBusy}
                    onClick={() => onDismissItem(item.id)}
                    title="حذف از صف"
                    className="p-1 rounded-lg bg-white/[0.03] hover:bg-rose-500/15 text-white/50 hover:text-rose-400 transition disabled:opacity-50"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            );
          })}

          {!isConnected && (
            <div className="flex items-center justify-between gap-2 px-1 pt-1 text-[11px] text-mab-text-muted">
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
