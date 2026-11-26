import { useEffect, useMemo, useState, type DragEvent } from "react";
import {
  Archive,
  Check,
  CheckSquare,
  ChevronDown,
  ChevronLeft,
  Folder,
  FolderGit2,
  FolderOpen,
  FolderPlus,
  GitMerge,
  Layers,
  PauseCircle,
  PlayCircle,
  Search,
  Sparkles,
  Square,
  Trash2,
  Upload,
} from "lucide-react";
import { DashboardState, Project } from "../lib/tauri_api";

const PROJECT_COLORS = [
  "#3B82F6",
  "#10B981",
  "#8B5CF6",
  "#F59E0B",
  "#EF4444",
  "#06B6D4",
  "#EC4899",
];

interface ProjectStudioViewProps {
  dash: DashboardState;
  onSwitchProject: (projectId: string) => void;
  onCreateProject: (parentId?: string | null) => void;
  onUpdateProject: (params: {
    projectId: string;
    name: string;
    parentId: string | null;
    clearParent: boolean;
    realm: string;
    lifecycleStatus: string;
    color: string;
    dailyQuotaMinutes: number;
    weeklyQuotaMinutes: number;
    monthlyQuotaMinutes: number;
    hourlyRate: number;
  }) => Promise<void> | void;
  onMergeProjects: (sourceId: string, targetId: string) => Promise<void> | void;
  onDeleteProject: (projectId: string) => Promise<void> | void;
}

function getDescendantIds(rootId: string, projects: Project[]): Set<string> {
  const result = new Set<string>([rootId]);
  let changed = true;
  while (changed) {
    changed = false;
    for (const p of projects) {
      if (p.parent_id && result.has(p.parent_id) && !result.has(p.id)) {
        result.add(p.id);
        changed = true;
      }
    }
  }
  return result;
}

function getBreadcrumb(projectId: string, projects: Project[]): Project[] {
  const map = new Map(projects.map((p) => [p.id, p]));
  const chain: Project[] = [];
  const seen = new Set<string>();
  let cur: Project | undefined = map.get(projectId);
  while (cur && !seen.has(cur.id)) {
    seen.add(cur.id);
    chain.unshift(cur);
    cur = cur.parent_id ? map.get(cur.parent_id) : undefined;
  }
  return chain;
}

export function ProjectStudioView({
  dash,
  onSwitchProject,
  onCreateProject,
  onUpdateProject,
  onMergeProjects,
  onDeleteProject,
}: ProjectStudioViewProps) {
  const projects = dash.projects;

  const [selectedId, setSelectedId] = useState<string>(dash.active_project.id);
  const [realmFilter, setRealmFilter] = useState<"ALL" | "WORK" | "LIFE">("ALL");
  const [searchQuery, setSearchQuery] = useState("");
  const [includeArchived, setIncludeArchived] = useState(true);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [draggedId, setDraggedId] = useState<string | null>(null);
  const [hoverDropId, setHoverDropId] = useState<string | null>(null);

  // Batch merge selection
  const [batchMode, setBatchMode] = useState(false);
  const [batchSelectedIds, setBatchSelectedIds] = useState<Set<string>>(
    () => new Set()
  );
  const [batchTargetId, setBatchTargetId] = useState<string>("");
  const [isBusy, setIsBusy] = useState(false);

  // Selected project editor fields
  const [editName, setEditName] = useState("");
  const [editParentId, setEditParentId] = useState("");
  const [editRealm, setEditRealm] = useState("WORK");
  const [editStatus, setEditStatus] = useState("ACTIVE");
  const [editColor, setEditColor] = useState("#3B82F6");
  const [editDaily, setEditDaily] = useState(0);
  const [editWeekly, setEditWeekly] = useState(0);
  const [editMonthly, setEditMonthly] = useState(0);
  const [editRate, setEditRate] = useState(0);

  // Single-project merge controls
  const [mergeTargetId, setMergeTargetId] = useState("");
  const [confirmSingleMerge, setConfirmSingleMerge] = useState(false);

  useEffect(() => {
    if (!projects.some((p) => p.id === selectedId)) {
      setSelectedId(dash.active_project.id);
    }
  }, [projects, selectedId, dash.active_project.id]);

  const selectedProject = useMemo(
    () => projects.find((p) => p.id === selectedId) ?? dash.active_project,
    [projects, selectedId, dash.active_project]
  );

  useEffect(() => {
    if (!selectedProject) return;
    setEditName(selectedProject.name);
    setEditParentId(selectedProject.parent_id ?? "");
    setEditRealm((selectedProject.realm || "WORK").toUpperCase());
    setEditStatus((selectedProject.lifecycle_status || "ACTIVE").toUpperCase());
    setEditColor(selectedProject.color || "#3B82F6");
    setEditDaily(selectedProject.daily_quota_minutes ?? 0);
    setEditWeekly(selectedProject.weekly_quota_minutes ?? 0);
    setEditMonthly(selectedProject.monthly_quota_minutes ?? 0);
    setEditRate(selectedProject.hourly_rate ?? 0);
    const other = projects.find((p) => p.id !== selectedProject.id);
    setMergeTargetId(other?.id ?? "");
    setConfirmSingleMerge(false);
  }, [selectedProject, projects]);

  useEffect(() => {
    if (!batchTargetId || !projects.some((p) => p.id === batchTargetId)) {
      setBatchTargetId(dash.active_project.id);
    }
  }, [batchTargetId, projects, dash.active_project.id]);

  const sessionCountByProject = useMemo(() => {
    const counts = new Map<string, number>();
    for (const s of dash.all_project_chat_sessions) {
      counts.set(s.project_id, (counts.get(s.project_id) ?? 0) + 1);
    }
    return counts;
  }, [dash.all_project_chat_sessions]);

  const normalizedQuery = searchQuery.trim().toLowerCase();

  const visibleProjects = useMemo(() => {
    return projects.filter((p) => {
      const st = (p.lifecycle_status || "ACTIVE").toUpperCase();
      if (!includeArchived && st === "ARCHIVED") return false;
      const rlm = (p.realm || "WORK").toUpperCase();
      if (realmFilter !== "ALL" && rlm !== realmFilter) return false;
      if (!normalizedQuery) return true;
      return (
        p.name.toLowerCase().includes(normalizedQuery) ||
        p.id.toLowerCase().includes(normalizedQuery)
      );
    });
  }, [projects, includeArchived, realmFilter, normalizedQuery]);

  const visibleIds = useMemo(
    () => new Set(visibleProjects.map((p) => p.id)),
    [visibleProjects]
  );

  const rootProjects = useMemo(
    () =>
      visibleProjects.filter(
        (p) => !p.parent_id || !visibleIds.has(p.parent_id)
      ),
    [visibleProjects, visibleIds]
  );

  // Similar-name pairs suggestion to clean up clutter
  const duplicateHints = useMemo(() => {
    const hints: Array<{ a: Project; b: Project }> = [];
    for (let i = 0; i < projects.length; i++) {
      for (let j = i + 1; j < projects.length; j++) {
        const a = projects[i];
        const b = projects[j];
        const na = a.name.trim().toLowerCase();
        const nb = b.name.trim().toLowerCase();
        if (
          na === nb ||
          (na.length >= 4 && nb.includes(na)) ||
          (nb.length >= 4 && na.includes(nb))
        ) {
          hints.push({ a, b });
        }
      }
    }
    return hints.slice(0, 4);
  }, [projects]);

  const canDropOnto = (sourceId: string, targetParentId: string | null): boolean => {
    if (!sourceId) return false;
    if (sourceId === targetParentId) return false;
    const src = projects.find((p) => p.id === sourceId);
    if (!src) return false;
    if ((src.parent_id ?? null) === targetParentId) return false;
    if (targetParentId === null) return src.parent_id != null;
    const descendants = getDescendantIds(sourceId, projects);
    return !descendants.has(targetParentId);
  };

  const handleDropReparent = async (
    e: DragEvent,
    targetParentId: string | null
  ) => {
    e.preventDefault();
    e.stopPropagation();
    const srcId = draggedId || e.dataTransfer.getData("text/plain");
    setDraggedId(null);
    setHoverDropId(null);
    if (!canDropOnto(srcId, targetParentId)) return;
    const src = projects.find((p) => p.id === srcId);
    if (!src) return;
    setIsBusy(true);
    try {
      await onUpdateProject({
        projectId: src.id,
        name: src.name,
        parentId: targetParentId,
        clearParent: targetParentId === null,
        realm: (src.realm || "WORK").toUpperCase(),
        lifecycleStatus: (src.lifecycle_status || "ACTIVE").toUpperCase(),
        color: src.color || "#3B82F6",
        dailyQuotaMinutes: src.daily_quota_minutes ?? 0,
        weeklyQuotaMinutes: src.weekly_quota_minutes ?? 0,
        monthlyQuotaMinutes: src.monthly_quota_minutes ?? 0,
        hourlyRate: src.hourly_rate ?? 0,
      });
      if (targetParentId) {
        setExpanded((prev) => ({ ...prev, [targetParentId]: true }));
      }
    } finally {
      setIsBusy(false);
    }
  };

  const handleSaveSelected = async () => {
    if (!selectedProject || !editName.trim()) return;
    setIsBusy(true);
    try {
      await onUpdateProject({
        projectId: selectedProject.id,
        name: editName.trim(),
        parentId: editParentId.trim() || null,
        clearParent: !editParentId.trim(),
        realm: editRealm,
        lifecycleStatus: editStatus,
        color: editColor,
        dailyQuotaMinutes: editDaily,
        weeklyQuotaMinutes: editWeekly,
        monthlyQuotaMinutes: editMonthly,
        hourlyRate: editRate,
      });
    } finally {
      setIsBusy(false);
    }
  };

  const handleQuickStatus = async (nextStatus: "ACTIVE" | "PAUSED" | "ARCHIVED") => {
    if (!selectedProject) return;
    setEditStatus(nextStatus);
    setIsBusy(true);
    try {
      await onUpdateProject({
        projectId: selectedProject.id,
        name: editName.trim() || selectedProject.name,
        parentId: editParentId.trim() || null,
        clearParent: !editParentId.trim(),
        realm: editRealm,
        lifecycleStatus: nextStatus,
        color: editColor,
        dailyQuotaMinutes: editDaily,
        weeklyQuotaMinutes: editWeekly,
        monthlyQuotaMinutes: editMonthly,
        hourlyRate: editRate,
      });
    } finally {
      setIsBusy(false);
    }
  };

  const handleBatchMerge = async () => {
    if (!batchTargetId || batchSelectedIds.size === 0) return;
    const sources = Array.from(batchSelectedIds).filter((id) => id !== batchTargetId);
    if (sources.length === 0) return;
    setIsBusy(true);
    try {
      for (const srcId of sources) {
        await onMergeProjects(srcId, batchTargetId);
      }
      setBatchSelectedIds(new Set());
      setBatchMode(false);
      setSelectedId(batchTargetId);
    } finally {
      setIsBusy(false);
    }
  };

  const selectedDescendants = useMemo(
    () =>
      selectedProject
        ? getDescendantIds(selectedProject.id, projects)
        : new Set<string>(),
    [selectedProject, projects]
  );

  const availableParents = useMemo(
    () => projects.filter((p) => !selectedDescendants.has(p.id)),
    [projects, selectedDescendants]
  );

  const mergeTargets = useMemo(
    () => projects.filter((p) => p.id !== selectedProject?.id),
    [projects, selectedProject]
  );

  const breadcrumb = useMemo(
    () => (selectedProject ? getBreadcrumb(selectedProject.id, projects) : []),
    [selectedProject, projects]
  );

  const renderTreeItem = (p: Project, depth: number) => {
    const children = visibleProjects.filter((c) => c.parent_id === p.id);
    const isExp = expanded[p.id] ?? true;
    const isSel = selectedProject?.id === p.id;
    const isDropHover = hoverDropId === p.id && draggedId !== p.id;
    const isChecked = batchSelectedIds.has(p.id);
    const statusUpper = (p.lifecycle_status || "ACTIVE").toUpperCase();
    const realmUpper = (p.realm || "WORK").toUpperCase();
    const sessCount = sessionCountByProject.get(p.id) ?? 0;

    return (
      <div key={p.id} className="space-y-0.5">
        <div
          draggable={!batchMode}
          onDragStart={(e) => {
            setDraggedId(p.id);
            e.dataTransfer.setData("text/plain", p.id);
            e.dataTransfer.effectAllowed = "move";
          }}
          onDragEnd={() => {
            setDraggedId(null);
            setHoverDropId(null);
          }}
          onDragOver={(e) => {
            if (draggedId && canDropOnto(draggedId, p.id)) {
              e.preventDefault();
              setHoverDropId(p.id);
            }
          }}
          onDragLeave={() => {
            if (hoverDropId === p.id) setHoverDropId(null);
          }}
          onDrop={(e) => void handleDropReparent(e, p.id)}
          style={{ paddingRight: `${depth * 14}px` }}
          onClick={() => {
            if (batchMode) {
              setBatchSelectedIds((prev) => {
                const next = new Set(prev);
                if (next.has(p.id)) next.delete(p.id);
                else next.add(p.id);
                return next;
              });
            } else {
              setSelectedId(p.id);
            }
          }}
          className={`group flex items-center justify-between gap-1.5 px-2 py-1.5 rounded-lg text-[12px] cursor-pointer border transition ${
            isDropHover
              ? "bg-blue-500/20 border-blue-400 text-white"
              : isSel
              ? "bg-blue-500/15 border-blue-500/40 text-white font-semibold"
              : "bg-transparent border-transparent text-white/80 hover:bg-white/[0.04]"
          }`}
        >
          <div className="flex items-center gap-1.5 min-w-0 flex-1">
            {batchMode ? (
              <span className="text-amber-300 shrink-0">
                {isChecked ? (
                  <CheckSquare className="w-3.5 h-3.5" />
                ) : (
                  <Square className="w-3.5 h-3.5 text-white/40" />
                )}
              </span>
            ) : children.length > 0 ? (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setExpanded((prev) => ({ ...prev, [p.id]: !isExp }));
                }}
                className="p-0.5 text-white/45 hover:text-white shrink-0"
              >
                {isExp ? (
                  <ChevronDown className="w-3.5 h-3.5" />
                ) : (
                  <ChevronLeft className="w-3.5 h-3.5" />
                )}
              </button>
            ) : (
              <span className="w-4 shrink-0" />
            )}

            <span
              className="w-2.5 h-2.5 rounded-full shrink-0"
              style={{ backgroundColor: p.color || "#3B82F6" }}
            />
            {isExp && children.length > 0 ? (
              <FolderOpen className="w-3.5 h-3.5 text-white/60 shrink-0" />
            ) : (
              <Folder className="w-3.5 h-3.5 text-white/50 shrink-0" />
            )}
            <span className="truncate">{p.name}</span>
          </div>

          <div className="flex items-center gap-1 shrink-0">
            {p.is_active && (
              <span className="px-1.5 py-0.2 rounded bg-blue-500/20 text-blue-300 text-[9.5px]">
                فعال
              </span>
            )}
            {realmUpper === "LIFE" && (
              <span className="px-1 rounded bg-emerald-500/15 text-emerald-300 text-[9px]">
                زندگی
              </span>
            )}
            {statusUpper === "PAUSED" && (
              <span className="px-1 rounded bg-amber-500/15 text-amber-300 text-[9px]">
                معلق
              </span>
            )}
            {statusUpper === "ARCHIVED" && (
              <span className="px-1 rounded bg-white/10 text-white/50 text-[9px]">
                آرشیو
              </span>
            )}
            {sessCount > 0 && (
              <span className="text-[10px] font-mono text-white/35">
                {sessCount}
              </span>
            )}
            {!batchMode && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onCreateProject(p.id);
                }}
                title={`افزودن زیرپروژه به «${p.name}»`}
                className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-white/10 text-blue-300 transition"
              >
                <FolderPlus className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </div>

        {isExp && children.length > 0 && (
          <div className="space-y-0.5">
            {children.map((c) => renderTreeItem(c, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div dir="rtl" className="h-full flex flex-col min-h-0 bg-[#0E0F12] overflow-hidden">
      {/* Top Studio Header */}
      <div className="px-4 py-3 border-b border-white/[0.07] bg-[#121317] flex items-center justify-between gap-2 flex-wrap shrink-0">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-xl bg-purple-500/15 border border-purple-500/30 flex items-center justify-center text-purple-300">
            <FolderGit2 className="w-4 h-4" />
          </div>
          <div>
            <h2 className="text-[13.5px] font-bold text-white">
              استودیو و ساماندهی یکپارچه پروژه‌ها
            </h2>
            <p className="text-[11px] text-white/50">
              {projects.length} پروژه در هسته مشترک کامپس و مب‌فوکس • برای جابه‌جایی، پروژه‌ها را روی هم بکشید و رها کنید (Drag & Drop)
            </p>
          </div>
        </div>

        <div className="flex items-center gap-1.5 flex-wrap">
          <button
            type="button"
            onClick={() => {
              setBatchMode((v) => !v);
              setBatchSelectedIds(new Set());
            }}
            className={`px-3 py-1.5 rounded-lg text-[11.5px] font-medium inline-flex items-center gap-1.5 border transition ${
              batchMode
                ? "bg-amber-500/20 border-amber-500/40 text-amber-200"
                : "bg-white/[0.04] hover:bg-white/[0.08] border-white/[0.08] text-white/75"
            }`}
          >
            <GitMerge className="w-3.5 h-3.5" />
            <span>{batchMode ? "خروج از ادغام گروهی" : "ادغام گروهی / پاک‌سازی"}</span>
          </button>

          <button
            type="button"
            onClick={() => onCreateProject(null)}
            className="px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[11.5px] font-semibold inline-flex items-center gap-1.5 transition"
          >
            <FolderPlus className="w-3.5 h-3.5" />
            <span>ریشه جدید</span>
          </button>
        </div>
      </div>

      {/* Similar/Duplicate Cleanup Banner */}
      {duplicateHints.length > 0 && (
        <div className="px-4 py-2 bg-amber-500/[0.06] border-b border-amber-500/20 flex items-center justify-between gap-2 flex-wrap shrink-0 text-[11.5px]">
          <div className="flex items-center gap-2 text-amber-200">
            <Sparkles className="w-3.5 h-3.5 text-amber-400 shrink-0" />
            <span>
              پروژه‌های هم‌نام یا مشابه شناسایی شدند (می‌توانی با «ادغام» آن‌ها را یکی کنی):
            </span>
          </div>
          <div className="flex items-center gap-1.5 flex-wrap">
            {duplicateHints.map((h) => (
              <button
                key={`${h.a.id}-${h.b.id}`}
                type="button"
                onClick={() => {
                  setSelectedId(h.a.id);
                  setMergeTargetId(h.b.id);
                  setBatchMode(false);
                }}
                className="px-2 py-0.5 rounded-md bg-amber-500/15 hover:bg-amber-500/25 border border-amber-500/30 text-amber-200 text-[11px] transition"
              >
                {h.a.name} ↔ {h.b.name}
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Batch Merge Action Bar */}
      {batchMode && (
        <div className="px-4 py-2.5 bg-[#171922] border-b border-amber-500/25 flex items-center justify-between gap-2 flex-wrap shrink-0">
          <div className="text-[12px] text-amber-200">
            {batchSelectedIds.size} پروژه برای ادغام انتخاب شده است
          </div>
          <div className="flex items-center gap-2">
            <select
              value={batchTargetId}
              onChange={(e) => setBatchTargetId(e.target.value)}
              className="mab-input px-2.5 py-1 text-[12px] bg-[#121317]"
            >
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  ادغام همه در: {p.name}
                </option>
              ))}
            </select>
            <button
              type="button"
              disabled={isBusy || batchSelectedIds.size === 0 || !batchTargetId}
              onClick={() => void handleBatchMerge()}
              className="px-3.5 py-1.5 rounded-lg bg-amber-600 hover:bg-amber-500 text-white text-[11.5px] font-semibold disabled:opacity-40 transition"
            >
              ادغام انتخاب‌شده‌ها
            </button>
          </div>
        </div>
      )}

      {/* Two-Pane Body */}
      <div className="flex-1 min-h-0 flex flex-col lg:flex-row overflow-hidden">
        {/* Left Pane: Tree Explorer */}
        <div className="w-full lg:w-[320px] shrink-0 border-b lg:border-b-0 lg:border-l border-white/[0.07] bg-[#111216] flex flex-col min-h-0 max-h-[45%] lg:max-h-none">
          <div className="p-2.5 border-b border-white/[0.06] space-y-2 shrink-0">
            <div className="relative">
              <Search className="w-3.5 h-3.5 text-white/40 absolute right-2.5 top-1/2 -translate-y-1/2" />
              <input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="جستجوی پروژه یا کد..."
                className="w-full mab-input pr-8 pl-2.5 py-1.5 text-[12px]"
              />
            </div>
            <div className="flex items-center justify-between gap-1 text-[11px]">
              <div className="flex items-center gap-1 bg-white/[0.03] p-0.5 rounded-lg border border-white/[0.06]">
                {(["ALL", "WORK", "LIFE"] as const).map((r) => (
                  <button
                    key={r}
                    type="button"
                    onClick={() => setRealmFilter(r)}
                    className={`px-2 py-0.5 rounded-md transition ${
                      realmFilter === r
                        ? "bg-blue-500/20 text-blue-300 font-semibold"
                        : "text-white/55 hover:text-white"
                    }`}
                  >
                    {r === "ALL" ? "همه" : r === "WORK" ? "کار" : "زندگی"}
                  </button>
                ))}
              </div>
              <label className="flex items-center gap-1 text-white/60 cursor-pointer">
                <input
                  type="checkbox"
                  checked={includeArchived}
                  onChange={(e) => setIncludeArchived(e.target.checked)}
                  className="accent-blue-500"
                />
                <span>آرشیوشده‌ها</span>
              </label>
            </div>
          </div>

          <div className="flex-1 min-h-0 overflow-y-auto p-2 space-y-0.5">
            {rootProjects.map((rp) => renderTreeItem(rp, 0))}
            {rootProjects.length === 0 && (
              <div className="py-8 text-center text-[12px] text-white/40">
                پروژه‌ای با این فیلتر یافت نشد.
              </div>
            )}

            {/* Root Drop Zone to Un-nest child projects */}
            {draggedId && canDropOnto(draggedId, null) && (
              <div
                onDragOver={(e) => {
                  e.preventDefault();
                  setHoverDropId("__ROOT__");
                }}
                onDragLeave={() => setHoverDropId(null)}
                onDrop={(e) => void handleDropReparent(e, null)}
                className={`mt-2 p-2.5 rounded-xl border border-dashed text-center text-[11.5px] flex items-center justify-center gap-1.5 transition ${
                  hoverDropId === "__ROOT__"
                    ? "bg-blue-500/20 border-blue-400 text-blue-200"
                    : "bg-white/[0.03] border-white/20 text-white/60"
                }`}
              >
                <Upload className="w-3.5 h-3.5" />
                <span>رها کردن اینجا برای تبدیل به پروژه ریشه</span>
              </div>
            )}
          </div>
        </div>

        {/* Right Pane: Selected Project Details, Edit & Merge Workspace */}
        <div className="flex-1 min-w-0 min-h-0 overflow-y-auto p-4 space-y-4">
          {selectedProject ? (
            <>
              {/* Breadcrumb & Quick Actions */}
              <div className="p-3.5 rounded-xl bg-[#13141A] border border-white/[0.08] flex items-center justify-between gap-3 flex-wrap">
                <div className="space-y-1 min-w-0">
                  <div className="flex items-center gap-1.5 text-[11px] text-white/50 flex-wrap">
                    <Layers className="w-3.5 h-3.5 text-blue-400 shrink-0" />
                    {breadcrumb.map((b, idx) => (
                      <span key={b.id} className="inline-flex items-center gap-1">
                        {idx > 0 && <span>/</span>}
                        <button
                          type="button"
                          onClick={() => setSelectedId(b.id)}
                          className="hover:text-white transition"
                        >
                          {b.name}
                        </button>
                      </span>
                    ))}
                  </div>
                  <div className="flex items-center gap-2">
                    <span
                      className="w-3 h-3 rounded-full shrink-0"
                      style={{ backgroundColor: selectedProject.color || "#3B82F6" }}
                    />
                    <h3 className="text-[15px] font-bold text-white truncate">
                      {selectedProject.name}
                    </h3>
                    <span className="font-mono text-[10.5px] text-white/40 bg-white/[0.04] px-1.5 py-0.5 rounded">
                      {selectedProject.id}
                    </span>
                  </div>
                  <p className="text-[10.5px] text-white/40 font-mono truncate bidi-ltr text-right">
                    {selectedProject.workspace_path}
                  </p>
                </div>

                <div className="flex items-center gap-1.5 flex-wrap">
                  {!selectedProject.is_active && (
                    <button
                      type="button"
                      onClick={() => onSwitchProject(selectedProject.id)}
                      className="px-3 py-1.5 rounded-lg bg-blue-600/20 hover:bg-blue-600/35 border border-blue-500/35 text-blue-200 text-[11.5px] font-semibold transition"
                    >
                      فعال‌سازی در بوم اصلی
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => onCreateProject(selectedProject.id)}
                    className="px-2.5 py-1.5 rounded-lg bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.08] text-white/80 text-[11.5px] inline-flex items-center gap-1 transition"
                  >
                    <FolderPlus className="w-3.5 h-3.5 text-emerald-400" />
                    <span>زیرپروژه</span>
                  </button>
                </div>
              </div>

              {/* Quick Status Toggles (just like Mab-Focus) */}
              <div className="grid grid-cols-3 gap-2">
                <button
                  type="button"
                  disabled={isBusy}
                  onClick={() => void handleQuickStatus("ACTIVE")}
                  className={`py-2 px-3 rounded-xl border text-[12px] font-medium inline-flex items-center justify-center gap-1.5 transition ${
                    editStatus === "ACTIVE"
                      ? "bg-emerald-500/20 border-emerald-500/45 text-emerald-200"
                      : "bg-[#13141A] border-white/[0.07] text-white/60 hover:text-white"
                  }`}
                >
                  <PlayCircle className="w-3.5 h-3.5" />
                  <span>فعال</span>
                </button>
                <button
                  type="button"
                  disabled={isBusy}
                  onClick={() => void handleQuickStatus("PAUSED")}
                  className={`py-2 px-3 rounded-xl border text-[12px] font-medium inline-flex items-center justify-center gap-1.5 transition ${
                    editStatus === "PAUSED"
                      ? "bg-amber-500/20 border-amber-500/45 text-amber-200"
                      : "bg-[#13141A] border-white/[0.07] text-white/60 hover:text-white"
                  }`}
                >
                  <PauseCircle className="w-3.5 h-3.5" />
                  <span>تعلیق</span>
                </button>
                <button
                  type="button"
                  disabled={isBusy}
                  onClick={() => void handleQuickStatus("ARCHIVED")}
                  className={`py-2 px-3 rounded-xl border text-[12px] font-medium inline-flex items-center justify-center gap-1.5 transition ${
                    editStatus === "ARCHIVED"
                      ? "bg-white/[0.14] border-white/30 text-white"
                      : "bg-[#13141A] border-white/[0.07] text-white/60 hover:text-white"
                  }`}
                >
                  <Archive className="w-3.5 h-3.5" />
                  <span>آرشیو</span>
                </button>
              </div>

              {/* Edit Form */}
              <div className="p-4 rounded-xl bg-[#13141A] border border-white/[0.08] space-y-3">
                <h4 className="text-[12.5px] font-bold text-white">
                  ویرایش مشخصات، جایگاه درختی و سهمیه
                </h4>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div className="space-y-1">
                    <label className="text-[11px] text-white/60">نام پروژه:</label>
                    <input
                      value={editName}
                      onChange={(e) => setEditName(e.target.value)}
                      className="w-full mab-input px-3 py-1.5 text-[12.5px]"
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-[11px] text-white/60">پروژه والد (جابه‌جایی در درخت):</label>
                    <select
                      value={editParentId}
                      onChange={(e) => setEditParentId(e.target.value)}
                      className="w-full mab-input px-2.5 py-1.5 text-[12px] bg-[#111216]"
                    >
                      <option value="">ریشه (بدون والد)</option>
                      {availableParents.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.name}
                        </option>
                      ))}
                    </select>
                  </div>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div className="space-y-1">
                    <label className="text-[11px] text-white/60">قلمرو (Realm):</label>
                    <select
                      value={editRealm}
                      onChange={(e) => setEditRealm(e.target.value)}
                      className="w-full mab-input px-2.5 py-1.5 text-[12px] bg-[#111216]"
                    >
                      <option value="WORK">کار (Work)</option>
                      <option value="LIFE">زندگی (Life)</option>
                    </select>
                  </div>
                  <div className="space-y-1">
                    <label className="text-[11px] text-white/60">رنگ پروژه:</label>
                    <div className="flex items-center gap-2 pt-1">
                      {PROJECT_COLORS.map((hex) => (
                        <button
                          key={hex}
                          type="button"
                          onClick={() => setEditColor(hex)}
                          style={{ backgroundColor: hex }}
                          className={`w-6 h-6 rounded-full flex items-center justify-center transition ${
                            editColor.toUpperCase() === hex.toUpperCase()
                              ? "ring-2 ring-white ring-offset-2 ring-offset-[#13141A] scale-110"
                              : "opacity-75 hover:opacity-100"
                          }`}
                        >
                          {editColor.toUpperCase() === hex.toUpperCase() && (
                            <Check className="w-3.5 h-3.5 text-white" />
                          )}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                  <div className="space-y-1">
                    <label className="text-[10.5px] text-white/55">روزانه (دقیقه)</label>
                    <input
                      type="number"
                      min={0}
                      value={editDaily}
                      onChange={(e) => setEditDaily(Number(e.target.value) || 0)}
                      className="w-full mab-input px-2.5 py-1 text-[12px] font-mono bidi-ltr"
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-[10.5px] text-white/55">هفتگی (دقیقه)</label>
                    <input
                      type="number"
                      min={0}
                      value={editWeekly}
                      onChange={(e) => setEditWeekly(Number(e.target.value) || 0)}
                      className="w-full mab-input px-2.5 py-1 text-[12px] font-mono bidi-ltr"
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-[10.5px] text-white/55">ماهانه (دقیقه)</label>
                    <input
                      type="number"
                      min={0}
                      value={editMonthly}
                      onChange={(e) => setEditMonthly(Number(e.target.value) || 0)}
                      className="w-full mab-input px-2.5 py-1 text-[12px] font-mono bidi-ltr"
                    />
                  </div>
                  <div className="space-y-1">
                    <label className="text-[10.5px] text-white/55">نرخ ساعتی</label>
                    <input
                      type="number"
                      min={0}
                      value={editRate}
                      onChange={(e) => setEditRate(Number(e.target.value) || 0)}
                      className="w-full mab-input px-2.5 py-1 text-[12px] font-mono bidi-ltr"
                    />
                  </div>
                </div>

                <div className="flex justify-end pt-1">
                  <button
                    type="button"
                    disabled={isBusy || !editName.trim()}
                    onClick={() => void handleSaveSelected()}
                    className="px-4 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[12px] font-semibold disabled:opacity-40 transition"
                  >
                    ذخیره و همگام‌سازی با مب‌فوکس
                  </button>
                </div>
              </div>

              {/* Merge Section */}
              {mergeTargets.length > 0 && (
                <div className="p-4 rounded-xl bg-amber-500/[0.06] border border-amber-500/25 space-y-3">
                  <div className="flex items-center gap-2 text-[13px] font-bold text-amber-300">
                    <GitMerge className="w-4 h-4" />
                    <span>ادغام «{selectedProject.name}» در یک پروژه دیگر</span>
                  </div>
                  <p className="text-[11.5px] text-white/65 leading-relaxed">
                    تمام تسک‌ها، لاین‌های کاری، گفتگوها، متدها/BPMN، اسناد دانش، کارت‌های حافظه، زیرپروژه‌ها و سوابق Mab-Focus این پروژه به‌صورت اتمیک به پروژه مقصد منتقل و در هر دو اپلیکیشن اعمال می‌شوند.
                  </p>
                  <div className="flex items-center gap-2 flex-wrap">
                    <select
                      value={mergeTargetId}
                      onChange={(e) => {
                        setMergeTargetId(e.target.value);
                        setConfirmSingleMerge(false);
                      }}
                      className="flex-1 min-w-[200px] mab-input px-3 py-1.5 text-[12px] bg-[#111216]"
                    >
                      {mergeTargets.map((p) => (
                        <option key={p.id} value={p.id}>
                          مقصد ادغام: {p.name} ({p.id})
                        </option>
                      ))}
                    </select>
                    {!confirmSingleMerge ? (
                      <button
                        type="button"
                        disabled={isBusy || !mergeTargetId}
                        onClick={() => setConfirmSingleMerge(true)}
                        className="px-4 py-1.5 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-500/40 text-amber-200 text-[12px] font-semibold transition"
                      >
                        ادغام پروژه
                      </button>
                    ) : (
                      <div className="flex items-center gap-1.5">
                        <button
                          type="button"
                          disabled={isBusy}
                          onClick={async () => {
                            if (!mergeTargetId) return;
                            setIsBusy(true);
                            try {
                              await onMergeProjects(selectedProject.id, mergeTargetId);
                              setSelectedId(mergeTargetId);
                              setConfirmSingleMerge(false);
                            } finally {
                              setIsBusy(false);
                            }
                          }}
                          className="px-4 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-[12px] font-semibold transition"
                        >
                          تایید ادغام نهایی
                        </button>
                        <button
                          type="button"
                          onClick={() => setConfirmSingleMerge(false)}
                          className="px-3 py-1.5 rounded-lg bg-white/[0.06] text-white/70 text-[12px]"
                        >
                          لغو
                        </button>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* Delete Project */}
              {projects.length > 1 && (
                <div className="flex justify-end">
                  <button
                    type="button"
                    disabled={isBusy}
                    onClick={async () => {
                      setIsBusy(true);
                      try {
                        await onDeleteProject(selectedProject.id);
                      } finally {
                        setIsBusy(false);
                      }
                    }}
                    className="px-3.5 py-1.5 rounded-lg bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/25 text-rose-300 text-[11.5px] inline-flex items-center gap-1.5 transition"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>حذف این پروژه</span>
                  </button>
                </div>
              )}
            </>
          ) : (
            <div className="h-full flex items-center justify-center text-white/40 text-[13px]">
              یک پروژه را از درخت سمت راست انتخاب کنید.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default ProjectStudioView;
