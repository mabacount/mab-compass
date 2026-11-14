import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Compass,
  FileSpreadsheet,
  Minus,
  PanelRightClose,
  PanelRightOpen,
  RefreshCw,
  RotateCcw,
  Square,
  X,
} from "lucide-react";
import { DashboardState, LlmConnectionStatus } from "../lib/tauri_api";

interface HeaderBarProps {
  dash: DashboardState;
  isSyncing: boolean;
  isSidebarCollapsed: boolean;
  connectionStatus: LlmConnectionStatus | null;
  isCheckingConnection: boolean;
  onToggleSidebar: () => void;
  onRecheckConnection: () => void;
  onUndo: () => void;
  onSyncMabFocus: () => void;
  onExportExcel: () => void;
}

export function HeaderBar({
  dash,
  isSyncing,
  isSidebarCollapsed,
  connectionStatus,
  isCheckingConnection,
  onToggleSidebar,
  onRecheckConnection,
  onUndo,
  onSyncMabFocus,
  onExportExcel,
}: HeaderBarProps) {
  const appWindow = getCurrentWindow();
  const isConnected = connectionStatus?.connected ?? false;

  return (
    <header
      data-tauri-drag-region
      className="h-[46px] bg-mab-sidebar border-b border-mab-border flex items-center justify-between px-2 sm:px-3 select-none shrink-0 z-30 tauri-drag"
    >
      {/* Right side (RTL start): Navigation, Breadcrumb & Project stats */}
      <div className="flex items-center gap-2 sm:gap-2.5 min-w-0 tauri-no-drag">
        <button
          type="button"
          onClick={onToggleSidebar}
          title={isSidebarCollapsed ? "باز کردن نوار کناری (Ctrl+B)" : "جمع‌کردن نوار کناری"}
          className="w-7 h-7 rounded-md bg-white/[0.04] border border-white/[0.08] text-mab-text-secondary hover:text-white hover:bg-white/[0.08] hover:border-white/[0.15] flex items-center justify-center transition shrink-0"
        >
          {isSidebarCollapsed ? (
            <PanelRightOpen className="w-3.5 h-3.5" />
          ) : (
            <PanelRightClose className="w-3.5 h-3.5" />
          )}
        </button>

        {/* Space Avatar & Breadcrumbs */}
        <div className="flex items-center gap-2 min-w-0">
          <div className="w-5 h-5 rounded-[6px] bg-gradient-to-br from-blue-500 to-indigo-600 flex items-center justify-center shadow-sm shrink-0">
            <Compass className="w-3 h-3 text-white" />
          </div>

          <div className="flex items-center gap-1.5 min-w-0">
            <span className="text-[12px] font-medium text-mab-text-muted hidden md:inline truncate">
              اکوسیستم ماب
            </span>
            <span className="text-[11px] text-mab-text-muted hidden md:inline">/</span>
            <span className="text-[13px] font-bold text-white truncate max-w-[150px] sm:max-w-[210px]">
              {dash.active_project.name}
            </span>
          </div>

          {/* Open/Closed Tasks Pill */}
          <div className="hidden lg:flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-white/[0.03] border border-white/[0.07] font-timer text-[11px] text-mab-text-secondary shrink-0">
            <span className="w-1.5 h-1.5 rounded-full bg-mab-focus" />
            <span className="font-semibold text-white/90">{dash.open_tasks.length}</span>
            <span>گام باز</span>
            <span className="text-white/20">|</span>
            <span className="text-white/60">{dash.closed_tasks_count} مختومه</span>
          </div>
        </div>
      </div>

      {/* Middle drag area */}
      <div
        data-tauri-drag-region
        className="flex-1 h-full flex items-center justify-center text-[11px] text-white/25 font-mono pointer-events-none px-2 truncate tauri-drag"
      >
        <span className="hidden xl:inline bidi-ltr truncate">
          Mab-Compass • {dash.active_project.id}
        </span>
      </div>

      {/* Left side (RTL end / LTR start): Actions + Window Controls */}
      <div className="flex items-center gap-1.5 sm:gap-2 shrink-0 tauri-no-drag">
        {/* Undo button if available */}
        {dash.can_undo && (
          <button
            type="button"
            onClick={onUndo}
            title={dash.last_undo_summary || "بازگردانی آخرین تغییر"}
            className="px-2.5 py-1 rounded-full bg-amber-500/10 border border-amber-500/30 text-amber-400 hover:bg-amber-500/20 text-[11px] font-medium flex items-center gap-1 transition"
          >
            <RotateCcw className="w-3 h-3" />
            <span className="hidden sm:inline">بازگردانی</span>
          </button>
        )}

        {/* LLM Connection Badge */}
        <button
          type="button"
          onClick={onRecheckConnection}
          disabled={isCheckingConnection}
          title={connectionStatus?.message || "بررسی وضعیت مدل هوش مصنوعی"}
          className={`px-2.5 py-1 rounded-full border text-[11px] font-medium inline-flex items-center gap-1.5 transition ${
            isCheckingConnection
              ? "bg-white/[0.04] border-white/[0.08] text-mab-text-muted"
              : isConnected
              ? "bg-emerald-500/10 border-emerald-500/25 text-emerald-400 hover:bg-emerald-500/15"
              : "bg-rose-500/10 border-rose-500/25 text-rose-400 hover:bg-rose-500/15"
          }`}
        >
          {isCheckingConnection ? (
            <RefreshCw className="w-3 h-3 animate-spin" />
          ) : isConnected ? (
            <span className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-60"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
            </span>
          ) : (
            <span className="w-2 h-2 rounded-full bg-rose-500" />
          )}
          <span className="hidden md:inline truncate max-w-[130px]">
            {isCheckingConnection
              ? "تست..."
              : isConnected
              ? `${dash.settings.active_llm_provider}`
              : "مدل قطع"}
          </span>
        </button>

        {/* Mab-Focus Sync Button */}
        <button
          type="button"
          onClick={onSyncMabFocus}
          disabled={isSyncing}
          title="همگام‌سازی لاگ‌ها و وضعیت با اپلیکیشن Mab-Focus"
          className="px-2.5 py-1 rounded-full bg-white/[0.04] border border-white/[0.08] text-mab-text-secondary hover:text-white hover:bg-white/[0.08] text-[11px] font-medium flex items-center gap-1.5 transition disabled:opacity-50"
        >
          <RefreshCw
            className={`w-3 h-3 text-blue-400 ${isSyncing ? "animate-spin" : ""}`}
          />
          <span className="hidden sm:inline">{isSyncing ? "سینک..." : "سینک با Focus"}</span>
        </button>

        {/* Export Excel Button */}
        <button
          type="button"
          onClick={onExportExcel}
          title="تولید شیت گزارش خروجی اکسل کارفرما"
          className="px-2.5 py-1 rounded-full bg-emerald-600/90 hover:bg-emerald-500 text-white text-[11px] font-semibold flex items-center gap-1.5 shadow-sm transition"
        >
          <FileSpreadsheet className="w-3.5 h-3.5" />
          <span className="hidden md:inline">اکسل کارفرما</span>
        </button>

        {/* Separator before window controls */}
        <div className="w-[1px] h-4 bg-white/[0.1] mx-0.5" />

        {/* Native Frameless Window Controls */}
        <div dir="ltr" className="flex items-center h-[46px] -mr-2 sm:-mr-3">
          <button
            type="button"
            onClick={() => appWindow.minimize()}
            className="mab-titlebar-btn"
            title="کوچک‌سازی پنجره"
          >
            <Minus className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={() => appWindow.toggleMaximize()}
            className="mab-titlebar-btn"
            title="بزرگ‌نمایی پنجره"
          >
            <Square className="w-2.5 h-2.5" />
          </button>
          <button
            type="button"
            onClick={() => appWindow.close()}
            className="mab-titlebar-btn mab-titlebar-btn-close"
            title="بستن برنامه"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </header>
  );
}
