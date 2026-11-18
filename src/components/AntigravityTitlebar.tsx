import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ArrowLeft,
  ArrowRight,
  Compass,
  FileSpreadsheet,
  Minus,
  PanelLeft,
  PanelRight,
  RefreshCw,
  RotateCcw,
  Square,
  X,
} from "lucide-react";
import { DashboardState, LlmConnectionStatus } from "../lib/tauri_api";

interface AntigravityTitlebarProps {
  dash: DashboardState;
  activeSessionTitle?: string;
  isSidebarOpen: boolean;
  isRightCanvasOpen: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  isSyncing: boolean;
  connectionStatus: LlmConnectionStatus | null;
  isCheckingConnection: boolean;
  onToggleSidebar: () => void;
  onToggleRightCanvas: () => void;
  onGoBack: () => void;
  onGoForward: () => void;
  onRecheckConnection: () => void;
  onUndo: () => void;
  onSyncMabFocus: () => void;
  onExportExcel: () => void;
}

export function AntigravityTitlebar({
  dash,
  activeSessionTitle,
  isSidebarOpen,
  isRightCanvasOpen,
  canGoBack,
  canGoForward,
  isSyncing,
  connectionStatus,
  isCheckingConnection,
  onToggleSidebar,
  onToggleRightCanvas,
  onGoBack,
  onGoForward,
  onRecheckConnection,
  onUndo,
  onSyncMabFocus,
  onExportExcel,
}: AntigravityTitlebarProps) {
  const isConnected = connectionStatus?.connected ?? false;
  const runWindowAction = (action: "minimize" | "toggleMaximize" | "close") => {
    try {
      const win = getCurrentWindow();
      void win[action]();
    } catch {
      // Safe fallback when previewed outside Tauri WebView
    }
  };

  return (
    <header
      dir="ltr"
      data-tauri-drag-region
      className="h-[38px] bg-[#111216] border-b border-white/[0.07] flex items-center justify-between pl-2.5 pr-0 select-none shrink-0 z-30 tauri-drag"
    >
      {/* Left: Brand + Sidebar Toggle + Tab History */}
      <div className="flex items-center gap-1.5 tauri-no-drag shrink-0">
        <div className="flex items-center gap-1.5 pr-1.5">
          <div className="w-6 h-6 rounded-md bg-white/[0.06] border border-white/[0.09] flex items-center justify-center text-white">
            <Compass className="w-3.5 h-3.5 text-blue-400" />
          </div>
          <span className="text-[12px] font-semibold text-white/90 tracking-tight hidden sm:inline">
            Mab-Compass
          </span>
        </div>

        <button
          type="button"
          onClick={onToggleSidebar}
          title={isSidebarOpen ? "بستن سایدبار چپ" : "باز کردن سایدبار چپ"}
          className={`w-7 h-7 rounded-md flex items-center justify-center transition ${
            isSidebarOpen
              ? "text-white/80 hover:bg-white/[0.07]"
              : "text-white/45 hover:text-white hover:bg-white/[0.07]"
          }`}
        >
          <PanelLeft className="w-3.5 h-3.5" />
        </button>

        <button
          type="button"
          disabled={!canGoBack}
          onClick={onGoBack}
          title="تب قبلی"
          className="w-6 h-6 rounded-md flex items-center justify-center text-white/60 hover:text-white hover:bg-white/[0.07] disabled:opacity-25 transition"
        >
          <ArrowLeft className="w-3.5 h-3.5" />
        </button>

        <button
          type="button"
          disabled={!canGoForward}
          onClick={onGoForward}
          title="تب بعدی"
          className="w-6 h-6 rounded-md flex items-center justify-center text-white/60 hover:text-white hover:bg-white/[0.07] disabled:opacity-25 transition"
        >
          <ArrowRight className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Center: Project / Conversation Breadcrumb (drag region) */}
      <div
        data-tauri-drag-region
        className="flex-1 h-full flex items-center justify-center px-3 min-w-0 tauri-drag"
      >
        <div className="flex items-center gap-1.5 text-[12px] text-white/65 truncate max-w-[440px] pointer-events-none">
          <span className="font-medium text-white/85 truncate" dir="auto">
            {dash.active_project.name}
          </span>
          {activeSessionTitle && (
            <>
              <span className="text-white/30">/</span>
              <span className="text-white/60 truncate" dir="auto">
                {activeSessionTitle}
              </span>
            </>
          )}
        </div>
      </div>

      {/* Right: Compact Utility Actions + Right Canvas Toggle + Native Window Controls */}
      <div className="flex items-center gap-1 tauri-no-drag shrink-0 h-full">
        {dash.can_undo && (
          <button
            type="button"
            onClick={onUndo}
            title={dash.last_undo_summary || "بازگردانی آخرین تغییر"}
            className="px-2 py-1 rounded-md bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/25 text-amber-300 text-[11px] font-medium flex items-center gap-1 transition"
          >
            <RotateCcw className="w-3 h-3" />
            <span className="hidden lg:inline">بازگردانی</span>
          </button>
        )}

        <button
          type="button"
          onClick={onRecheckConnection}
          disabled={isCheckingConnection}
          title={connectionStatus?.message || "وضعیت اتصال مدل هوش مصنوعی"}
          className={`px-2 py-1 rounded-md border text-[11px] font-medium inline-flex items-center gap-1.5 transition ${
            isCheckingConnection
              ? "bg-white/[0.04] border-white/[0.07] text-white/50"
              : isConnected
              ? "bg-emerald-500/10 border-emerald-500/25 text-emerald-300 hover:bg-emerald-500/20"
              : "bg-rose-500/10 border-rose-500/25 text-rose-300 hover:bg-rose-500/20"
          }`}
        >
          {isCheckingConnection ? (
            <RefreshCw className="w-3 h-3 animate-spin" />
          ) : (
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                isConnected ? "bg-emerald-400" : "bg-rose-400"
              }`}
            />
          )}
          <span className="hidden xl:inline font-mono text-[10.5px]">
            {isConnected ? dash.settings.active_llm_provider : "Offline"}
          </span>
        </button>

        <button
          type="button"
          onClick={onSyncMabFocus}
          disabled={isSyncing}
          title="همگام‌سازی با Mab-Focus"
          className="w-7 h-7 rounded-md text-white/65 hover:text-white hover:bg-white/[0.07] flex items-center justify-center transition disabled:opacity-40"
        >
          <RefreshCw className={`w-3.5 h-3.5 text-blue-400 ${isSyncing ? "animate-spin" : ""}`} />
        </button>

        <button
          type="button"
          onClick={onExportExcel}
          title="خروجی اکسل کارفرما"
          className="w-7 h-7 rounded-md text-emerald-400/80 hover:text-emerald-300 hover:bg-emerald-500/10 flex items-center justify-center transition"
        >
          <FileSpreadsheet className="w-3.5 h-3.5" />
        </button>

        <button
          type="button"
          onClick={onToggleRightCanvas}
          title={isRightCanvasOpen ? "بستن بوم سمت راست" : "باز کردن بوم سمت راست"}
          className={`w-7 h-7 rounded-md flex items-center justify-center transition ${
            isRightCanvasOpen
              ? "bg-white/[0.08] text-white"
              : "text-white/55 hover:text-white hover:bg-white/[0.07]"
          }`}
        >
          <PanelRight className="w-3.5 h-3.5" />
        </button>

        <div className="w-[1px] h-4 bg-white/[0.09] mx-0.5" />

        {/* Native Windows Controls on Far Right (LTR) */}
        <div className="flex items-center h-full">
          <button
            type="button"
            onClick={() => runWindowAction("minimize")}
            className="mab-titlebar-btn"
            title="Minimize"
          >
            <Minus className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={() => runWindowAction("toggleMaximize")}
            className="mab-titlebar-btn"
            title="Maximize"
          >
            <Square className="w-2.5 h-2.5" />
          </button>
          <button
            type="button"
            onClick={() => runWindowAction("close")}
            className="mab-titlebar-btn mab-titlebar-btn-close"
            title="Close"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </header>
  );
}

export default AntigravityTitlebar;
