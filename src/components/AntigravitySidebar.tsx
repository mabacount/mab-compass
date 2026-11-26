import { useEffect, useState } from "react";
import {
  Brain,
  ChevronDown,
  ChevronRight,
  Compass,
  Database,
  Folder,
  FolderGit2,
  FolderOpen,
  FolderArchive,
  FolderPlus,
  Layers,
  MessageSquarePlus,
  Plus,
  Settings,
  Settings2,
  Terminal,
  Trash2,
  Workflow,
} from "lucide-react";
import { ChatSession, DashboardState, Project } from "../lib/tauri_api";

export type BuiltinCanvasViewId =
  | "PATH_AHEAD"
  | "PROJECTS_STUDIO"
  | "METHODS_BPMN"
  | "WBS"
  | "KNOWLEDGE_ARCHIVE"
  | "ECOSYSTEM_CARDS"
  | "SETTINGS"
  | "LOGS";

interface AntigravitySidebarProps {
  dash: DashboardState;
  width: number;
  activeCanvasTabId: string | null;
  isNarrowViewport?: boolean;
  onCloseMobile?: () => void;
  onNewConversation: () => void;
  onOpenBuiltinView: (viewId: BuiltinCanvasViewId) => void;
  onSwitchProject: (projectId: string) => void;
  onOpenNewProject: (parentId?: string | null) => void;
  onManageProject: (project: Project) => void;
  onCreateChatSession: (projectId: string, channel: "COPILOT" | "VAULT") => void;
  onSwitchChatSession: (session: ChatSession) => void;
  onDeleteChatSession: (sessionId: string) => void;
}

function formatCompactAge(iso: string): string {
  if (!iso) return "";
  const dt = new Date(iso);
  if (Number.isNaN(dt.getTime())) return "";
  const diffSec = Math.max(0, Math.floor((Date.now() - dt.getTime()) / 1000));
  if (diffSec < 60) return "now";
  const mins = Math.floor(diffSec / 60);
  if (mins < 60) return `${mins}m`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h`;
  const days = Math.floor(hrs / 24);
  if (days < 30) return `${days}d`;
  return `${Math.floor(days / 30)}mo`;
}

export function AntigravitySidebar({
  dash,
  width,
  activeCanvasTabId,
  isNarrowViewport = false,
  onCloseMobile,
  onNewConversation,
  onOpenBuiltinView,
  onSwitchProject,
  onOpenNewProject,
  onManageProject,
  onCreateChatSession,
  onSwitchChatSession,
  onDeleteChatSession,
}: AntigravitySidebarProps) {
  const [expandedProjects, setExpandedProjects] = useState<Record<string, boolean>>({});

  useEffect(() => {
    setExpandedProjects((prev) => {
      const next = { ...prev, [dash.active_project.id]: true };
      let curParent = dash.active_project.parent_id;
      let guard = 0;
      while (curParent && guard < 10) {
        next[curParent] = true;
        const parentObj = dash.projects.find((p) => p.id === curParent);
        curParent = parentObj?.parent_id ?? null;
        guard += 1;
      }
      return next;
    });
  }, [dash.active_project.id, dash.active_project.parent_id, dash.projects]);

  const toggleExpand = (pid: string) => {
    setExpandedProjects((prev) => ({
      ...prev,
      [pid]: !(prev[pid] ?? pid === dash.active_project.id),
    }));
  };

  const quickLinks: Array<{
    id: BuiltinCanvasViewId;
    label: string;
    icon: typeof Compass;
    badge: number | string | null;
    accentClass: string;
  }> = [
    {
      id: "PATH_AHEAD",
      label: "مسیر پیش‌رو",
      icon: Compass,
      badge: dash.top_priority_steps?.length || (dash.next_step_task ? 1 : null),
      accentClass: "text-blue-400",
    },
    {
      id: "PROJECTS_STUDIO",
      label: "استودیو و ادغام پروژه‌ها",
      icon: FolderGit2,
      badge: dash.projects.length,
      accentClass: "text-indigo-400",
    },
    {
      id: "METHODS_BPMN",
      label: "متدها و BPMN 2.0",
      icon: Workflow,
      badge: (dash.methods_playbooks?.length ?? 0) > 0 ? dash.methods_playbooks!.length : null,
      accentClass: "text-purple-400",
    },
    {
      id: "WBS",
      label: "ساختار WBS و لاین‌ها",
      icon: Layers,
      badge: dash.open_tasks.length > 0 ? dash.open_tasks.length : null,
      accentClass: "text-emerald-400",
    },
    {
      id: "KNOWLEDGE_ARCHIVE",
      label: "اسناد و بازپردازش",
      icon: FolderArchive,
      badge: dash.knowledge_docs.length > 0 ? dash.knowledge_docs.length : null,
      accentClass: "text-amber-400",
    },
    {
      id: "ECOSYSTEM_CARDS",
      label: "اهداف، کارت‌ها و تقویم",
      icon: Brain,
      badge:
        (dash.personal_vault_items?.length ?? 0) > 0
          ? dash.personal_vault_items!.length
          : dash.outcomes.length > 0
          ? dash.outcomes.length
          : null,
      accentClass: "text-cyan-400",
    },
  ];

  const handleSelectQuickLink = (viewId: BuiltinCanvasViewId) => {
    onOpenBuiltinView(viewId);
    if (isNarrowViewport && onCloseMobile) onCloseMobile();
  };

  return (
    <>
      {isNarrowViewport && onCloseMobile && (
        <div
          onClick={onCloseMobile}
          className="fixed inset-0 top-[38px] bg-black/60 backdrop-blur-[2px] z-30 md:hidden"
        />
      )}
      <aside
        dir="ltr"
        style={isNarrowViewport ? undefined : { width: `${width}px` }}
        className={`${
          isNarrowViewport
            ? "fixed top-[38px] left-0 bottom-0 z-40 w-[260px] shadow-2xl"
            : "shrink-0"
        } bg-[#121317] border-r border-white/[0.07] flex flex-col justify-between min-h-0 select-none`}
      >
        {/* Top Scrollable Area */}
        <div className="flex-1 min-h-0 flex flex-col overflow-hidden">
          {/* + New Conversation button (Antigravity signature top button) */}
          <div className="p-2.5 pb-1.5 shrink-0">
            <button
              type="button"
              onClick={() => {
                onNewConversation();
                if (isNarrowViewport && onCloseMobile) onCloseMobile();
              }}
              className="w-full px-3 py-2 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.08] text-[12.5px] font-medium text-white flex items-center gap-2 transition"
            >
              <Plus className="w-3.5 h-3.5 text-white/80 shrink-0" />
              <span dir="rtl" className="truncate">
                گفتگوی جدید
              </span>
            </button>
          </div>

          {/* Quick Links (replaces Automations / Customizations — opens Right Canvas Tabs!) */}
          <nav className="px-2 py-1 space-y-0.5 shrink-0">
            {quickLinks.map((item) => {
              const Icon = item.icon;
              const isSelected = activeCanvasTabId === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => handleSelectQuickLink(item.id)}
                  className={`w-full px-2.5 py-1.5 rounded-lg text-[12.5px] flex items-center justify-between gap-2 transition group ${
                    isSelected
                      ? "bg-white/[0.09] text-white font-medium"
                      : "text-[#A1A6B4] hover:bg-white/[0.04] hover:text-white"
                  }`}
                >
                  <div className="flex items-center gap-2.5 min-w-0">
                    <Icon
                      className={`w-3.5 h-3.5 shrink-0 ${
                        isSelected ? item.accentClass : "text-white/50 group-hover:text-white/80"
                      }`}
                    />
                    <span dir="rtl" className="truncate">
                      {item.label}
                    </span>
                  </div>
                  {item.badge !== null && (
                    <span className="font-mono text-[10px] text-white/45 shrink-0">
                      {item.badge}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>

          <div className="mx-3 my-2 border-t border-white/[0.06] shrink-0" />

          {/* Projects & Conversations Tree (exact Antigravity Projects section) */}
          <div className="flex-1 min-h-0 flex flex-col px-2 overflow-hidden">
            {/* Cross-Project Global Chats Section */}
            {(() => {
              const globalSessions = dash.all_project_chat_sessions.filter(
                (s) => s.project_id === "GLOBAL"
              );
              return (
                <div className="mb-2.5 shrink-0">
                  <div className="px-2 pb-1 flex items-center justify-between text-[11px] font-medium text-cyan-300/85">
                    <span dir="rtl" className="truncate">
                      گفتگوها (خارج از پروژه)
                    </span>
                    <button
                      type="button"
                      onClick={() => onCreateChatSession("GLOBAL", "COPILOT")}
                      title="گفتگوی فراپروژه‌ای جدید"
                      className="p-1 rounded hover:bg-white/[0.07] text-cyan-300/70 hover:text-cyan-200 transition"
                    >
                      <MessageSquarePlus className="w-3.5 h-3.5" />
                    </button>
                  </div>
                  <div className="space-y-0.5 max-h-48 overflow-y-auto pr-0.5">
                    {globalSessions.map((sess) => {
                      const isCurrent =
                        (sess.channel === "COPILOT" &&
                          sess.id === dash.active_copilot_session_id) ||
                        (sess.channel === "VAULT" &&
                          sess.id === dash.active_vault_session_id);
                      return (
                        <div
                          key={sess.id}
                          className={`group flex items-center justify-between gap-1.5 px-2.5 py-1.5 rounded-lg text-[12px] transition ${
                            isCurrent
                              ? "bg-cyan-500/15 text-white font-medium border border-cyan-500/25"
                              : "text-[#9CA3AF] hover:bg-white/[0.04] hover:text-white"
                          }`}
                        >
                          <button
                            type="button"
                            onClick={() => {
                              onSwitchChatSession(sess);
                              if (isNarrowViewport && onCloseMobile) onCloseMobile();
                            }}
                            className="flex-1 min-w-0 text-left truncate"
                            title={sess.title}
                          >
                            <span dir="auto" className="truncate block">
                              {sess.title}
                            </span>
                          </button>
                          <div className="flex items-center gap-1 shrink-0">
                            <span className="font-mono text-[10.5px] text-white/35 group-hover:hidden">
                              {formatCompactAge(sess.updated_at)}
                            </span>
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onDeleteChatSession(sess.id);
                              }}
                              title="حذف گفتگو"
                              className="hidden group-hover:inline-flex p-0.5 text-white/45 hover:text-rose-400 transition"
                            >
                              <Trash2 className="w-3 h-3" />
                            </button>
                          </div>
                        </div>
                      );
                    })}
                    {globalSessions.length === 0 && (
                      <div className="px-2.5 py-1 text-[11px] text-white/35" dir="rtl">
                        برای چت هم‌زمان روی چند پروژه، + را بزنید
                      </div>
                    )}
                  </div>
                </div>
              );
            })()}

            <div className="px-2 pb-1.5 flex items-center justify-between text-[11px] font-medium text-white/45 shrink-0">
              <span>Projects</span>
              <div className="flex items-center gap-0.5">
                <button
                  type="button"
                  onClick={() => handleSelectQuickLink("PROJECTS_STUDIO")}
                  title="استودیو و ادغام پروژه‌ها"
                  className="p-1 rounded hover:bg-white/[0.07] text-indigo-300/75 hover:text-indigo-200 transition"
                >
                  <FolderGit2 className="w-3.5 h-3.5" />
                </button>
                <button
                  type="button"
                  onClick={() => onOpenNewProject(null)}
                  title="ساخت پروژه جدید"
                  className="p-1 rounded hover:bg-white/[0.07] text-white/50 hover:text-white transition"
                >
                  <Plus className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            <div className="flex-1 min-h-0 overflow-y-auto space-y-1 pr-0.5 pb-2">
              {(() => {
                const visibleSidebarProjects = dash.projects.filter(
                  (p) => p.is_active || (p.lifecycle_status || "ACTIVE").toUpperCase() !== "ARCHIVED"
                );
                const projectIds = new Set(visibleSidebarProjects.map((p) => p.id));
                const rootProjects = visibleSidebarProjects.filter(
                  (p) => !p.parent_id || !projectIds.has(p.parent_id)
                );

                const renderProjectNode = (p: Project, depth: number) => {
                  const isExpanded = expandedProjects[p.id] ?? p.is_active;
                  const projSessions = dash.all_project_chat_sessions.filter(
                    (s) => s.project_id === p.id
                  );
                  const children = visibleSidebarProjects.filter((c) => c.parent_id === p.id);
                  const dotColor = p.color || "#3B82F6";

                  return (
                    <div
                      key={p.id}
                      style={depth > 0 ? { paddingLeft: `${Math.min(depth, 4) * 10}px` } : undefined}
                      className="space-y-0.5"
                    >
                      {/* Project Folder Row */}
                      <div
                        className={`group flex items-center justify-between gap-1 px-2 py-1.5 rounded-lg text-[12.5px] transition ${
                          p.is_active
                            ? "text-white font-medium bg-white/[0.04]"
                            : "text-[#9CA3AF] hover:bg-white/[0.04] hover:text-white"
                        }`}
                      >
                        <button
                          type="button"
                          onClick={() => {
                            if (!p.is_active) onSwitchProject(p.id);
                            else toggleExpand(p.id);
                          }}
                          className="flex-1 flex items-center gap-1.5 min-w-0 text-left"
                        >
                          <span
                            className="w-2 h-2 rounded-full shrink-0"
                            style={{ backgroundColor: dotColor }}
                          />
                          {isExpanded ? (
                            <FolderOpen
                              className={`w-3.5 h-3.5 shrink-0 ${
                                p.is_active ? "text-blue-400" : "text-white/45"
                              }`}
                            />
                          ) : (
                            <Folder className="w-3.5 h-3.5 shrink-0 text-white/45" />
                          )}
                          <span dir="auto" className="truncate">
                            {p.name}
                          </span>
                          {p.lifecycle_status === "PAUSED" && (
                            <span className="px-1 rounded bg-amber-500/15 text-amber-300 text-[9px] shrink-0">
                              معلق
                            </span>
                          )}
                          {p.lifecycle_status === "ARCHIVED" && (
                            <span className="px-1 rounded bg-white/10 text-white/50 text-[9px] shrink-0">
                              آرشیو
                            </span>
                          )}
                        </button>

                        <div className="flex items-center gap-0.5 shrink-0">
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              onOpenNewProject(p.id);
                            }}
                            title="ساخت زیرپروژه"
                            className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-white/[0.09] text-white/55 hover:text-emerald-300 transition"
                          >
                            <FolderPlus className="w-3 h-3" />
                          </button>
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              onManageProject(p);
                            }}
                            title="تنظیمات و ادغام پروژه"
                            className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-white/[0.09] text-white/55 hover:text-purple-300 transition"
                          >
                            <Settings2 className="w-3 h-3" />
                          </button>
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              onCreateChatSession(p.id, "COPILOT");
                            }}
                            title="گفتگوی جدید در این پروژه"
                            className="opacity-0 group-hover:opacity-100 p-1 rounded hover:bg-white/[0.09] text-white/55 hover:text-blue-300 transition"
                          >
                            <MessageSquarePlus className="w-3 h-3" />
                          </button>
                          <button
                            type="button"
                            onClick={(e) => {
                              e.stopPropagation();
                              toggleExpand(p.id);
                            }}
                            className="p-0.5 text-white/40 hover:text-white"
                          >
                            {isExpanded ? (
                              <ChevronDown className="w-3 h-3" />
                            ) : (
                              <ChevronRight className="w-3 h-3" />
                            )}
                          </button>
                        </div>
                      </div>

                      {/* Conversations & Child Sub-Projects */}
                      {isExpanded && (
                        <div className="pl-3 space-y-0.5 border-l border-white/[0.06] ml-2.5">
                          {children.map((child) => renderProjectNode(child, depth + 1))}
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
                                className={`group flex items-center justify-between gap-1.5 px-2.5 py-1.5 rounded-lg text-[12px] transition ${
                                  isCurrent
                                    ? "bg-white/[0.08] text-white font-medium"
                                    : "text-[#9CA3AF] hover:bg-white/[0.04] hover:text-white"
                                }`}
                              >
                                <button
                                  type="button"
                                  onClick={() => {
                                    onSwitchChatSession(sess);
                                    if (isNarrowViewport && onCloseMobile) onCloseMobile();
                                  }}
                                  className="flex-1 min-w-0 text-left truncate"
                                  title={sess.title}
                                >
                                  <span dir="auto" className="truncate block">
                                    {sess.title}
                                  </span>
                                </button>

                                <div className="flex items-center gap-1 shrink-0">
                                  {sess.channel === "VAULT" && (
                                    <span className="px-1 rounded bg-purple-500/20 text-purple-300 text-[9px]">
                                      دانش
                                    </span>
                                  )}
                                  <span className="font-mono text-[10.5px] text-white/35 group-hover:hidden">
                                    {formatCompactAge(sess.updated_at)}
                                  </span>
                                  {projSessions.length > 1 && (
                                    <button
                                      type="button"
                                      onClick={(e) => {
                                        e.stopPropagation();
                                        onDeleteChatSession(sess.id);
                                      }}
                                      title="حذف گفتگو"
                                      className="hidden group-hover:inline-flex p-0.5 text-white/45 hover:text-rose-400 transition"
                                    >
                                      <Trash2 className="w-3 h-3" />
                                    </button>
                                  )}
                                </div>
                              </div>
                            );
                          })}
                        </div>
                      )}
                    </div>
                  );
                };

                return rootProjects.map((rootProj) => renderProjectNode(rootProj, 0));
              })()}
            </div>
          </div>
        </div>

        {/* Bottom Footer (Settings & System Logs + subtle memory count) */}
        <div className="p-2 border-t border-white/[0.06] space-y-1 shrink-0">
          <button
            type="button"
            onClick={() => handleSelectQuickLink("LOGS")}
            className={`w-full px-2.5 py-1.5 rounded-lg text-[12.5px] flex items-center justify-between transition ${
              activeCanvasTabId === "LOGS"
                ? "bg-white/[0.09] text-white font-medium"
                : "text-[#9CA3AF] hover:bg-white/[0.04] hover:text-white"
            }`}
          >
            <span className="flex items-center gap-2.5">
              <Terminal className="w-3.5 h-3.5 text-blue-400" />
              <span dir="rtl">لاگ‌های سیستم</span>
            </span>
            <span className="font-mono text-[10px] text-blue-400/80 bg-blue-500/10 px-1.5 py-0.5 rounded">
              Logs
            </span>
          </button>

          <button
            type="button"
            onClick={() => handleSelectQuickLink("SETTINGS")}
            className={`w-full px-2.5 py-1.5 rounded-lg text-[12.5px] flex items-center justify-between transition ${
              activeCanvasTabId === "SETTINGS"
                ? "bg-white/[0.09] text-white font-medium"
                : "text-[#9CA3AF] hover:bg-white/[0.04] hover:text-white"
            }`}
          >
            <span className="flex items-center gap-2.5">
              <Settings className="w-3.5 h-3.5 text-white/55" />
              <span dir="rtl">تنظیمات</span>
            </span>
            <span
              title={`${dash.raw_events_count} رویداد در SQLite`}
              className="inline-flex items-center gap-1 font-mono text-[10px] text-white/40"
            >
              <Database className="w-2.5 h-2.5 text-blue-400/75" />
              {dash.raw_events_count}
            </span>
          </button>
        </div>
      </aside>
    </>
  );
}

export default AntigravitySidebar;
