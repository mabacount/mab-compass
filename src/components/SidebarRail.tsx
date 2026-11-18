import { useEffect, useState } from "react";
import {
  Brain,
  ChevronDown,
  ChevronLeft,
  Compass,
  Database,
  FolderArchive,
  FolderOpen,
  Layers,
  MessageSquare,
  MessageSquarePlus,
  PanelRightClose,
  PanelRightOpen,
  Plus,
  Settings,
  Trash2,
} from "lucide-react";
import { ChatSession, DashboardState } from "../lib/tauri_api";
import { formatShortTime } from "../lib/formatters";

export type AppTab =
  | "PATH_AND_CHAT"
  | "KNOWLEDGE_VAULT"
  | "WBS_AND_DOCS"
  | "ECOSYSTEM_CARDS"
  | "SETTINGS";

interface SidebarRailProps {
  dash: DashboardState;
  activeTab: AppTab;
  isCollapsed: boolean;
  isNarrowViewport?: boolean;
  sidebarWidth?: number;
  onToggleCollapse: () => void;
  onSelectTab: (tab: AppTab) => void;
  onSwitchProject: (projectId: string) => void;
  onOpenNewProject: () => void;
  onCreateChatSession: (projectId: string, channel: "COPILOT" | "VAULT") => void;
  onSwitchChatSession: (session: ChatSession) => void;
  onDeleteChatSession: (sessionId: string) => void;
}

export function SidebarRail({
  dash,
  activeTab,
  isCollapsed,
  isNarrowViewport = false,
  sidebarWidth = 255,
  onToggleCollapse,
  onSelectTab,
  onSwitchProject,
  onOpenNewProject,
  onCreateChatSession,
  onSwitchChatSession,
  onDeleteChatSession,
}: SidebarRailProps) {
  const [expandedProjects, setExpandedProjects] = useState<Record<string, boolean>>({});

  useEffect(() => {
    setExpandedProjects((prev) => ({
      ...prev,
      [dash.active_project.id]: true,
    }));
  }, [dash.active_project.id]);

  const toggleProjectExpand = (pid: string) => {
    setExpandedProjects((prev) => ({
      ...prev,
      [pid]: !(prev[pid] ?? (pid === dash.active_project.id)),
    }));
  };

  const navItems: Array<{
    id: AppTab;
    label: string;
    icon: typeof Compass;
    badge: string | number | null;
  }> = [
    {
      id: "PATH_AND_CHAT",
      label: "مسیر پیش‌رو و همیار",
      icon: Compass,
      badge: dash.next_step_task ? "Active" : null,
    },
    {
      id: "KNOWLEDGE_VAULT",
      label: "آرشیو اسناد و دانش",
      icon: FolderArchive,
      badge: dash.knowledge_docs.length > 0 ? dash.knowledge_docs.length : null,
    },
    {
      id: "WBS_AND_DOCS",
      label: "نقشه WBS و لاین‌ها",
      icon: Layers,
      badge: dash.open_tasks.length > 0 ? dash.open_tasks.length : null,
    },
    {
      id: "ECOSYSTEM_CARDS",
      label: "اهداف و کارت‌های دانش",
      icon: Brain,
      badge:
        (dash.personal_vault_items?.length ?? 0) > 0
          ? dash.personal_vault_items!.length
          : dash.outcomes.length > 0
          ? dash.outcomes.length
          : null,
    },
    {
      id: "SETTINGS",
      label: "تنظیمات و سینک",
      icon: Settings,
      badge: null,
    },
  ];

  const handleSelectNav = (tab: AppTab) => {
    onSelectTab(tab);
    if (isNarrowViewport && !isCollapsed) {
      onToggleCollapse();
    }
  };

  /* -------------------------------------------------------------------------- */
  /* Collapsed Rail (Width 60px)                                                 */
  /* -------------------------------------------------------------------------- */
  if (isCollapsed) {
    return (
      <aside className="w-[60px] bg-mab-sidebar border-l border-mab-border flex flex-col items-center justify-between py-3 px-1.5 shrink-0 min-h-0 select-none z-20">
        <div className="flex flex-col items-center gap-2.5 w-full">
          {/* Toggle button */}
          <button
            type="button"
            onClick={onToggleCollapse}
            title="باز کردن نوار کناری"
            className="w-9 h-9 rounded-lg bg-white/[0.03] border border-mab-border text-mab-text-secondary hover:text-white hover:bg-white/[0.08] flex items-center justify-center transition"
          >
            <PanelRightOpen className="w-4 h-4" />
          </button>

          <div className="w-7 border-t border-mab-border" />

          {/* Navigation view icons */}
          <nav className="flex flex-col items-center gap-1.5 w-full">
            {navItems.map((item) => {
              const Icon = item.icon;
              const selected = activeTab === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onSelectTab(item.id)}
                  title={item.label}
                  className={`relative w-9 h-9 rounded-lg flex items-center justify-center transition ${
                    selected
                      ? "bg-blue-500/15 border border-blue-500/30 text-blue-400 shadow-sm"
                      : "text-mab-text-secondary hover:bg-white/[0.05] hover:text-white"
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  {item.badge !== null && (
                    <span className="absolute -top-1 -left-1 min-w-[15px] h-3.5 px-1 rounded-full bg-blue-500 text-white font-timer text-[9px] font-bold flex items-center justify-center">
                      {item.badge}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>

          <div className="w-7 border-t border-mab-border" />

          {/* Projects avatar list */}
          <div className="flex flex-col items-center gap-1.5 w-full">
            {dash.projects.map((p) => (
              <button
                key={p.id}
                type="button"
                onClick={() => onSwitchProject(p.id)}
                title={`${p.name} (${p.id})`}
                className={`w-9 h-9 rounded-lg font-mono text-[10.5px] font-bold flex items-center justify-center transition ${
                  p.is_active
                    ? "bg-white/[0.1] border border-white/[0.2] text-white shadow-sm"
                    : "bg-white/[0.02] border border-mab-border text-mab-text-muted hover:text-white hover:bg-white/[0.05]"
                }`}
              >
                {p.id.slice(0, 3)}
              </button>
            ))}

            <button
              type="button"
              onClick={onOpenNewProject}
              title="ساخت پروژه جدید"
              className="w-9 h-7 rounded-lg border border-dashed border-white/[0.12] text-mab-text-muted hover:text-white hover:border-white/[0.25] flex items-center justify-center transition"
            >
              <Plus className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        {/* Bottom memory dot */}
        <div
          title={`${dash.raw_events_count} رویداد در حافظه SQLite`}
          className="w-9 h-9 rounded-lg bg-white/[0.02] border border-mab-border flex flex-col items-center justify-center text-blue-400/80"
        >
          <Database className="w-3.5 h-3.5" />
          <span className="font-timer text-[8.5px] font-bold text-white/50">
            {dash.raw_events_count}
          </span>
        </div>
      </aside>
    );
  }

  /* -------------------------------------------------------------------------- */
  /* Expanded Sidebar (Drawer Overlay on Narrow Viewports < 768px)              */
  /* -------------------------------------------------------------------------- */
  return (
    <>
      {isNarrowViewport && (
        <div
          onClick={onToggleCollapse}
          className="fixed inset-0 top-[46px] bg-black/60 backdrop-blur-[2px] z-30 md:hidden"
        />
      )}
      <aside
        style={
          isNarrowViewport
            ? undefined
            : { width: `${sidebarWidth}px` }
        }
        className={`${
          isNarrowViewport
            ? "fixed top-[46px] right-0 bottom-0 z-40 shadow-2xl w-[260px]"
            : "z-20"
        } bg-mab-sidebar border-l border-mab-border flex flex-col justify-between p-3 shrink-0 min-h-0 select-none`}
      >
        <div className="flex-1 min-h-0 flex flex-col gap-3 overflow-y-auto pl-0.5">
          {/* Workspace Brand / Header */}
          <div className="px-1 py-0.5 flex items-center justify-between shrink-0 gap-2">
            <div className="flex items-center gap-2 min-w-0">
              <div className="w-6 h-6 rounded-lg bg-white/[0.05] border border-white/[0.08] flex items-center justify-center shadow-sm text-blue-400 shrink-0">
                <Compass className="w-3.5 h-3.5" />
              </div>
              <div className="min-w-0 flex items-center gap-1.5">
                <span className="font-bold text-[13px] text-white tracking-tight truncate">
                  Mab-Compass
                </span>
                <span className="px-1.5 py-0.5 rounded-full bg-blue-500/15 border border-blue-500/30 text-[9.5px] font-mono text-blue-300">
                  v3.0
                </span>
              </div>
            </div>

            <button
              type="button"
              onClick={onToggleCollapse}
              title="جمع‌کردن نوار کناری"
              className="w-6 h-6 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-mab-text-muted hover:text-white flex items-center justify-center transition"
            >
              <PanelRightClose className="w-3.5 h-3.5" />
            </button>
          </div>

          {/* Section 1: Primary Navigation Views */}
          <nav className="space-y-0.5">
            {navItems.map((item) => {
              const Icon = item.icon;
              const selected = activeTab === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => handleSelectNav(item.id)}
                className={`w-full flex items-center justify-between px-2.5 py-2 rounded-lg text-[12.5px] transition group ${
                  selected
                    ? "bg-white/[0.08] text-white font-medium border border-white/[0.1] shadow-sm"
                    : "text-mab-text-secondary hover:bg-white/[0.04] hover:text-white border border-transparent"
                }`}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <Icon
                    className={`w-4 h-4 shrink-0 transition ${
                      selected
                        ? "text-blue-400"
                        : "text-mab-text-muted group-hover:text-white"
                    }`}
                  />
                  <span className="truncate">{item.label}</span>
                </div>
                {item.badge !== null && (
                  <span
                    className={`px-1.5 py-0.5 rounded-full font-timer text-[10px] font-bold shrink-0 ${
                      selected
                        ? "bg-blue-500/25 text-blue-300"
                        : "bg-white/[0.06] text-mab-text-muted"
                    }`}
                  >
                    {item.badge}
                  </span>
                )}
              </button>
            );
          })}
        </nav>

        <div className="border-t border-mab-border" />

        {/* Section 2: Projects & Nested Chat History */}
        <div className="space-y-1.5 flex-1 min-h-0 flex flex-col">
          <div className="flex items-center justify-between px-1">
            <span className="text-[11px] font-semibold text-mab-text-muted uppercase tracking-wider">
              پروژه‌ها و گفتگوها
            </span>
            <button
              onClick={onOpenNewProject}
              className="p-1 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-mab-text-muted hover:text-white transition"
              title="ساخت پروژه جدید"
            >
              <Plus className="w-3 h-3" />
            </button>
          </div>

          <div className="space-y-1 overflow-y-auto flex-1 min-h-0 pl-0.5">
            {dash.projects.map((p) => {
              const isExpanded = expandedProjects[p.id] ?? p.is_active;
              const projSessions = dash.all_project_chat_sessions.filter(
                (s) => s.project_id === p.id
              );

              return (
                <div key={p.id} className="space-y-0.5">
                  {/* Project Row with clean Anytype subtle highlight */}
                  <div
                    className={`w-full flex items-center justify-between rounded-lg transition ${
                      p.is_active
                        ? "bg-white/[0.08] border border-white/[0.12] text-white font-medium shadow-sm"
                        : "text-mab-text-secondary hover:bg-white/[0.04] hover:text-white border border-transparent"
                    }`}
                  >
                    <button
                      type="button"
                      onClick={() => onSwitchProject(p.id)}
                      className="flex-1 flex items-center justify-between px-2.5 py-1.5 text-[12.5px] min-w-0 text-right"
                    >
                      <div className="flex items-center gap-1.5 truncate">
                        {p.is_active && (
                          <span className="w-1.5 h-1.5 rounded-full bg-blue-500 shrink-0" />
                        )}
                        <span className="truncate">{p.name}</span>
                      </div>
                      <span className="font-mono text-[10px] opacity-60 shrink-0 mr-1.5">
                        {p.id}
                      </span>
                    </button>
                    <button
                      type="button"
                      onClick={() => toggleProjectExpand(p.id)}
                      className="p-1.5 opacity-60 hover:opacity-100 text-mab-text-muted hover:text-white shrink-0"
                      title={isExpanded ? "بستن لیست چت‌ها" : "نمایش لیست چت‌ها"}
                    >
                      {isExpanded ? (
                        <ChevronDown className="w-3.5 h-3.5" />
                      ) : (
                        <ChevronLeft className="w-3.5 h-3.5" />
                      )}
                    </button>
                  </div>

                  {/* Sub-tree: Project's Chat Sessions */}
                  {isExpanded && (
                    <div className="pr-2 mr-2 border-r border-mab-border space-y-0.5 py-0.5">
                      <div className="flex items-center justify-between px-1.5 py-0.5">
                        <span className="text-[10.5px] text-mab-text-muted">
                          گفتگوها ({projSessions.length})
                        </span>
                        <button
                          type="button"
                          onClick={() => onCreateChatSession(p.id, "COPILOT")}
                          className="px-2 py-0.5 rounded-full bg-blue-500/10 hover:bg-blue-500/20 text-blue-400 text-[10.5px] font-medium flex items-center gap-1 transition"
                          title={`شروع گفتگوی جدید در ${p.name}`}
                        >
                          <MessageSquarePlus className="w-2.5 h-2.5" />
                          <span>چت جدید</span>
                        </button>
                      </div>

                      <div className="space-y-0.5 max-h-36 overflow-y-auto">
                        {projSessions.map((sess) => {
                          const isCurrent =
                            p.is_active &&
                            ((sess.channel === "COPILOT" &&
                              sess.id === dash.active_copilot_session_id) ||
                              (sess.channel === "VAULT" &&
                                sess.id === dash.active_vault_session_id));

                          return (
                            <div
                              key={sess.id}
                              className={`group flex items-center justify-between gap-1 px-2 py-1 rounded-md text-[11.5px] transition ${
                                isCurrent
                                  ? "bg-white/[0.08] text-white font-medium border border-white/[0.08]"
                                  : "text-mab-text-secondary hover:bg-white/[0.04] hover:text-white"
                              }`}
                            >
                              <button
                                type="button"
                                onClick={() => onSwitchChatSession(sess)}
                                className="flex-1 flex items-center gap-1.5 min-w-0 text-right"
                              >
                                <MessageSquare
                                  className={`w-3 h-3 shrink-0 ${
                                    sess.channel === "VAULT"
                                      ? "text-purple-400"
                                      : isCurrent
                                      ? "text-blue-400"
                                      : "text-mab-text-muted"
                                  }`}
                                />
                                <span className="truncate">{sess.title}</span>
                              </button>

                              <div className="flex items-center gap-1 shrink-0">
                                {sess.channel === "VAULT" && (
                                  <span className="px-1 py-0.2 rounded bg-purple-500/15 text-purple-300 font-mono text-[9px]">
                                    دانش
                                  </span>
                                )}
                                <span
                                  className="font-timer text-[9.5px] text-mab-text-muted"
                                  title={formatShortTime(sess.updated_at)}
                                >
                                  {sess.message_count}
                                </span>
                                {projSessions.length > 1 && (
                                  <button
                                    type="button"
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      onDeleteChatSession(sess.id);
                                    }}
                                    className="opacity-0 group-hover:opacity-100 p-0.5 text-mab-text-muted hover:text-rose-400 transition"
                                    title="حذف این گفتگو"
                                  >
                                    <Trash2 className="w-2.5 h-2.5" />
                                  </button>
                                )}
                              </div>
                            </div>
                          );
                        })}

                        {projSessions.length === 0 && (
                          <div className="px-2 py-1 text-[11px] text-mab-text-muted">
                            هنوز گفتگویی ثبت نشده است
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>

      {/* Bottom Minimalist Status Bar (Anytype style) with fixed path clipping */}
      <div className="pt-2 mt-2 border-t border-mab-border space-y-1.5 text-[11px] shrink-0">
        <div className="flex items-center justify-between text-mab-text-muted px-1">
          <span className="flex items-center gap-1.5">
            <Database className="w-3 h-3 text-blue-400" />
            <span>حافظه پروژه:</span>
          </span>
          <span className="font-timer text-blue-400 font-semibold">
            {dash.raw_events_count} رویداد
          </span>
        </div>

        <div
          dir="ltr"
          className="flex items-center gap-1.5 text-[10.5px] text-mab-text-muted px-1 rounded-md bg-white/[0.02] border border-mab-border py-1 overflow-hidden"
          title={dash.active_project.workspace_path}
        >
          <FolderOpen className="w-3.5 h-3.5 text-emerald-400/80 shrink-0" />
          <span className="truncate font-mono select-text" dir="ltr">
            {dash.active_project.workspace_path}
          </span>
        </div>
      </div>
    </aside>
    </>
  );
}

export default SidebarRail;
