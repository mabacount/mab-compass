import { useMemo, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  BookOpen,
  Edit3,
  Eye,
  FileCode2,
  GitBranch,
  Layers,
  Plus,
  Search,
  Sparkles,
  Trash2,
  Workflow,
  X,
} from "lucide-react";
import {
  DashboardState,
  MethodPlaybook,
  MethodPlaybookInput,
} from "../lib/tauri_api";
import { buildMethodStepsBpmn, buildProjectWorkMapBpmn } from "../lib/bpmnEngine";
import { BpmnViewer } from "./bpmn/BpmnViewer";
import { formatShortDate } from "../lib/formatters";

interface MethodsAndBpmnViewProps {
  dash: DashboardState;
  onOpenMethodTab: (method: MethodPlaybook) => void;
  onUpsertMethod: (input: MethodPlaybookInput) => Promise<void> | void;
  onDeleteMethod: (methodId: string) => Promise<void> | void;
  onStartTask?: (taskId: string) => void;
  onQuickCompleteTask?: (taskId: string, pomosSpent: number) => void;
  onAskCopilotToExtractMethod?: () => void;
}

type StudioSubTab = "PROJECT_MAP" | "METHODS_LIST";

interface DraftStep {
  id?: string;
  title: string;
  description: string;
  element_type: string;
  role: string;
  condition_label?: string;
  sub_steps_text: string;
}

export function MethodsAndBpmnView({
  dash,
  onOpenMethodTab,
  onUpsertMethod,
  onDeleteMethod,
  onStartTask,
  onQuickCompleteTask,
  onAskCopilotToExtractMethod,
}: MethodsAndBpmnViewProps) {
  const [subTab, setSubTab] = useState<StudioSubTab>("PROJECT_MAP");
  const [searchQuery, setSearchQuery] = useState("");
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showLivePreview, setShowLivePreview] = useState(true);
  const [editingMethodId, setEditingMethodId] = useState<string | undefined>(undefined);

  // Manual method/diagram creation or edit state
  const [mTitle, setMTitle] = useState("");
  const [mCategory, setMCategory] = useState("GENERAL");
  const [mObjective, setMObjective] = useState("");
  const [mPrereqs, setMPrereqs] = useState("");
  const [mChecklist, setMChecklist] = useState("");
  const [mNotes, setMNotes] = useState("");
  const [mSteps, setMSteps] = useState<DraftStep[]>([
    { title: "", description: "", element_type: "userTask", role: "", condition_label: "", sub_steps_text: "" },
  ]);
  const [isSaving, setIsSaving] = useState(false);

  const openCreateNew = () => {
    setEditingMethodId(undefined);
    setMTitle("");
    setMCategory("GENERAL");
    setMObjective("");
    setMPrereqs("");
    setMChecklist("");
    setMNotes("");
    setMSteps([
      { title: "", description: "", element_type: "userTask", role: "", condition_label: "", sub_steps_text: "" },
    ]);
    setShowLivePreview(true);
    setShowCreateModal(true);
  };

  const openEditMethod = (m: MethodPlaybook) => {
    setEditingMethodId(m.id);
    setMTitle(m.title);
    setMCategory(m.category || "GENERAL");
    setMObjective(m.objective || "");
    setMPrereqs((m.prerequisites || []).join("\n"));
    setMChecklist((m.checklist || []).join("\n"));
    setMNotes(m.notes_markdown || "");
    setMSteps(
      m.steps && m.steps.length > 0
        ? m.steps.map((s) => ({
            id: s.id,
            title: s.title,
            description: s.description || "",
            element_type: s.sub_steps && s.sub_steps.length > 0 ? "subProcess" : s.element_type || "userTask",
            role: s.role || "",
            condition_label: s.condition_label || "",
            sub_steps_text: (s.sub_steps || []).map((ss) => ss.title).join("\n"),
          }))
        : [{ title: "", description: "", element_type: "userTask", role: "", condition_label: "", sub_steps_text: "" }]
    );
    setShowLivePreview(true);
    setShowCreateModal(true);
  };

  const moveStep = (idx: number, dir: -1 | 1) => {
    setMSteps((prev) => {
      const target = idx + dir;
      if (target < 0 || target >= prev.length) return prev;
      const next = [...prev];
      const [moved] = next.splice(idx, 1);
      next.splice(target, 0, moved);
      return next;
    });
  };

  const livePreviewXml = useMemo(
    () =>
      buildMethodStepsBpmn(
        mTitle,
        mSteps.map((s, idx) => ({
          id: s.id || `S${idx + 1}`,
          title: s.title,
          description: s.description,
          element_type: s.element_type,
          role: s.role,
          condition_label: s.condition_label,
          sub_steps: s.sub_steps_text
            .split("\n")
            .map((l) => l.trim())
            .filter(Boolean)
            .map((t, sidx) => ({ id: `S${idx + 1}_${sidx + 1}`, title: t })),
        })),
        mObjective
      ),
    [mTitle, mSteps, mObjective]
  );

  const methods = dash.methods_playbooks ?? [];
  const projectWorkMapXml = useMemo(() => buildProjectWorkMapBpmn(dash), [dash]);

  const filteredMethods = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return methods;
    return methods.filter(
      (m) =>
        m.title.toLowerCase().includes(q) ||
        m.summary.toLowerCase().includes(q) ||
        m.category.toLowerCase().includes(q) ||
        m.objective.toLowerCase().includes(q)
    );
  }, [methods, searchQuery]);

  const handleSaveNewMethod = async () => {
    if (!mTitle.trim() || isSaving) return;
    setIsSaving(true);
    try {
      const stepsPayload = mSteps
        .filter((s) => s.title.trim())
        .map((s, idx) => {
          const subSteps = s.sub_steps_text
            .split("\n")
            .map((l) => l.trim())
            .filter(Boolean)
            .map((subTitle, sidx) => ({
              id: `S${idx + 1}_${sidx + 1}`,
              order: sidx + 1,
              title: subTitle,
              description: "",
              element_type: "task",
              role: s.role.trim(),
              inputs: [],
              outputs: [],
              checklist: [],
              notes: "",
              sub_steps: [],
            }));
          return {
            id: `S${idx + 1}`,
            order: idx + 1,
            title: s.title.trim(),
            description: s.description.trim(),
            element_type: subSteps.length > 0 ? "subProcess" : s.element_type,
            role: s.role.trim(),
            inputs: [],
            outputs: [],
            checklist: [],
            notes: "",
            condition_label: s.condition_label?.trim() || undefined,
            sub_steps: subSteps,
          };
        });

      await onUpsertMethod({
        method_id: editingMethodId,
        title: mTitle.trim(),
        category: mCategory,
        objective: mObjective.trim(),
        prerequisites: mPrereqs
          .split("\n")
          .map((l) => l.trim())
          .filter(Boolean),
        steps: stepsPayload,
        checklist: mChecklist
          .split("\n")
          .map((l) => l.trim())
          .filter(Boolean),
        notes_markdown: mNotes.trim(),
      });

      setEditingMethodId(undefined);
      setMTitle("");
      setMObjective("");
      setMPrereqs("");
      setMChecklist("");
      setMNotes("");
      setMSteps([
        { title: "", description: "", element_type: "userTask", role: "", condition_label: "", sub_steps_text: "" },
      ]);
      setShowCreateModal(false);
      setSubTab("METHODS_LIST");
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div dir="rtl" className="flex flex-col h-full min-h-0 gap-3 p-3.5 overflow-hidden">
      {/* Top Studio Controls */}
      <div className="flex items-center justify-between gap-2.5 flex-wrap shrink-0">
        <div className="flex items-center gap-1 bg-[#121317] p-1 rounded-xl border border-white/[0.07]">
          <button
            type="button"
            onClick={() => setSubTab("PROJECT_MAP")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium flex items-center gap-1.5 transition ${
              subTab === "PROJECT_MAP"
                ? "bg-white/[0.09] text-white border border-white/[0.1]"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <Workflow className="w-3.5 h-3.5 text-purple-400" />
            <span>نقشه کار زنده پروژه (BPMN 2.0)</span>
          </button>

          <button
            type="button"
            onClick={() => setSubTab("METHODS_LIST")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium flex items-center gap-1.5 transition ${
              subTab === "METHODS_LIST"
                ? "bg-white/[0.09] text-white border border-white/[0.1]"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <BookOpen className="w-3.5 h-3.5 text-emerald-400" />
            <span>دیاگرام‌ها و دستورالعمل‌های فرآیند</span>
            <span className="font-mono text-[10px] px-1.5 py-[2px] rounded-full bg-white/[0.08]">
              {methods.length}
            </span>
          </button>
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          {subTab === "METHODS_LIST" && (
            <div className="relative min-w-[180px]">
              <Search className="w-3.5 h-3.5 text-white/40 absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="جستجو در دستورالعمل‌ها..."
                className="mab-input pr-8 pl-3 py-1 text-[12px] w-full select-text"
              />
            </div>
          )}

          {onAskCopilotToExtractMethod && (
            <button
              type="button"
              onClick={onAskCopilotToExtractMethod}
              className="px-2.5 py-1.5 rounded-lg bg-purple-500/15 hover:bg-purple-500/25 border border-purple-500/30 text-purple-300 text-[11.5px] font-medium inline-flex items-center gap-1.5 transition"
            >
              <Sparkles className="w-3.5 h-3.5" />
              <span>ترسیم فرآیند با همیار</span>
            </button>
          )}

          <button
            type="button"
            onClick={openCreateNew}
            className="px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[11.5px] font-semibold inline-flex items-center gap-1.5 transition"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>رسم دستورالعمل / BPMN</span>
          </button>
        </div>
      </div>

      {/* Main Content */}
      {subTab === "PROJECT_MAP" ? (
        <div className="flex-1 min-h-0 flex flex-col gap-2">
          <BpmnViewer
            bpmnXml={projectWorkMapXml}
            title={`نقشه کار پروژه ${dash.active_project.name}`}
            allMethods={methods}
            onOpenMethod={onOpenMethodTab}
            onStartTask={onStartTask}
            onQuickCompleteTask={
              onQuickCompleteTask ? (tid) => onQuickCompleteTask(tid, 1) : undefined
            }
            className="flex-1 min-h-0"
          />
        </div>
      ) : (
        <div className="flex-1 min-h-0 overflow-y-auto space-y-2.5 pl-0.5">
          {filteredMethods.length === 0 ? (
            <div className="p-8 rounded-2xl bg-[#131418] border border-white/[0.07] text-center space-y-3">
              <div className="w-11 h-11 rounded-2xl bg-purple-500/15 border border-purple-500/30 text-purple-400 flex items-center justify-center mx-auto">
                <Workflow className="w-5 h-5" />
              </div>
              <h3 className="text-[14.5px] font-bold text-white">
                هنوز متد یا دستورالعمل انجام کاری در این پروژه ثبت نشده است
              </h3>
              <p className="text-[12.5px] text-mab-text-secondary max-w-lg mx-auto leading-relaxed">
                هر زمان گزارش کاری در چت ارسال کنید، یا خرده‌گزارش‌های خود را بنویسید، یا از همیار بخواهید
                روش انجام کار را مستند کند، مدل به صورت خودکار دستورالعمل Markdown + نمودار BPMN 2.0 را در
                پوشه <code className="font-mono text-blue-300">04_methods_playbooks</code> ذخیره و به‌روزرسانی می‌کند.
              </p>
              <div className="flex items-center justify-center gap-2 pt-1">
                <button
                  type="button"
                  onClick={() => setShowCreateModal(true)}
                  className="px-3.5 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[12px] font-semibold inline-flex items-center gap-1.5"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>ساخت دستی اولین متد</span>
                </button>
                {onAskCopilotToExtractMethod && (
                  <button
                    type="button"
                    onClick={onAskCopilotToExtractMethod}
                    className="px-3.5 py-1.5 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] border border-white/[0.09] text-white/85 text-[12px] font-medium inline-flex items-center gap-1.5"
                  >
                    <Sparkles className="w-3.5 h-3.5 text-purple-400" />
                    <span>درخواست استخراج از همیار</span>
                  </button>
                )}
              </div>
            </div>
          ) : (
            <div className="grid grid-cols-1 xl:grid-cols-2 gap-3 pb-4">
              {filteredMethods.map((m) => {
                const subProcCount = m.steps.filter(
                  (s) => s.element_type === "subProcess" || (s.sub_steps && s.sub_steps.length > 0)
                ).length;

                return (
                  <div
                    key={m.id}
                    onClick={() => onOpenMethodTab(m)}
                    className="p-4 rounded-xl bg-[#141519] hover:bg-[#191B22] border border-white/[0.08] hover:border-white/[0.18] transition cursor-pointer space-y-2.5 group"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex items-center gap-2 min-w-0">
                        <div className="w-8 h-8 rounded-lg bg-purple-500/15 border border-purple-500/30 text-purple-300 flex items-center justify-center shrink-0">
                          <Workflow className="w-4 h-4" />
                        </div>
                        <div className="min-w-0">
                          <h4 className="text-[13.5px] font-bold text-white truncate group-hover:text-blue-300 transition">
                            {m.title}
                          </h4>
                          <div className="flex items-center gap-1.5 text-[10.5px] text-mab-text-muted mt-0.5 flex-wrap">
                            <span className="font-mono text-blue-400">{m.id}</span>
                            <span>•</span>
                            <span className="px-1.5 py-[2px] rounded bg-emerald-500/15 text-emerald-300 font-mono">
                              v{m.version}
                            </span>
                            <span className="px-1.5 py-[2px] rounded bg-white/[0.06] font-mono">
                              {m.category}
                            </span>
                          </div>
                        </div>
                      </div>

                      <div className="flex items-center gap-1 shrink-0">
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            openEditMethod(m);
                          }}
                          title="ویرایش دیاگرام و گام‌ها"
                          className="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg bg-white/[0.04] hover:bg-blue-500/20 text-white/60 hover:text-blue-300 transition shrink-0"
                        >
                          <Edit3 className="w-3.5 h-3.5" />
                        </button>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            void onDeleteMethod(m.id);
                          }}
                          title="حذف متد"
                          className="opacity-0 group-hover:opacity-100 p-1.5 rounded-lg bg-white/[0.03] hover:bg-rose-500/15 text-white/45 hover:text-rose-400 transition shrink-0"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>

                    <p className="text-[12px] text-mab-text-secondary line-clamp-2 leading-relaxed">
                      {m.summary || m.objective || "دستورالعمل گام‌به‌گام و نمودار BPMN 2.0"}
                    </p>

                    <div className="flex items-center justify-between gap-2 pt-1 border-t border-white/[0.05] text-[11px] text-mab-text-muted flex-wrap">
                      <div className="flex items-center gap-2">
                        <span className="inline-flex items-center gap-1 text-white/75">
                          <Layers className="w-3 h-3 text-blue-400" />
                          <span>{m.steps.length} گام</span>
                        </span>
                        {subProcCount > 0 && (
                          <span className="inline-flex items-center gap-1 text-purple-300">
                            <GitBranch className="w-3 h-3" />
                            <span>{subProcCount} زیرفرآیند</span>
                          </span>
                        )}
                        <span className="inline-flex items-center gap-1 text-emerald-400/80 font-mono text-[10px]">
                          <FileCode2 className="w-3 h-3" />
                          <span>MD + BPMN</span>
                        </span>
                      </div>

                      <span className="font-timer text-[10.5px]">
                        {formatShortDate(m.updated_at)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {/* Manual Method Playbook & Visual Diagram Builder Modal */}
      {showCreateModal && (
        <div
          className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4"
          onClick={() => setShowCreateModal(false)}
        >
          <div
            onClick={(e) => e.stopPropagation()}
            className="w-full max-w-4xl max-h-[90vh] bg-[#16171C] border border-white/[0.1] rounded-2xl shadow-2xl flex flex-col overflow-hidden"
          >
            <div className="px-5 py-3.5 border-b border-white/[0.08] flex items-center justify-between gap-2">
              <div className="flex items-center gap-2 text-white font-bold text-[14px]">
                <Workflow className="w-4 h-4 text-purple-400" />
                <span>
                  {editingMethodId
                    ? `ویرایش دستورالعمل و دیاگرام (${editingMethodId})`
                    : "رسم دستورالعمل و نقشه فرآیند BPMN 2.0 جدید"}
                </span>
              </div>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setShowLivePreview((v) => !v)}
                  className={`px-2.5 py-1 rounded-lg text-[11px] font-medium border inline-flex items-center gap-1 transition ${
                    showLivePreview
                      ? "bg-purple-500/20 border-purple-400/40 text-purple-200"
                      : "bg-white/[0.05] border-white/10 text-white/70"
                  }`}
                >
                  <Eye className="w-3.5 h-3.5" />
                  <span>{showLivePreview ? "مخفی‌سازی پیش‌نمایش دیاگرام" : "نمایش زنده دیاگرام"}</span>
                </button>
                <button
                  type="button"
                  onClick={() => setShowCreateModal(false)}
                  className="p-1 rounded-lg hover:bg-white/[0.08] text-white/60 hover:text-white"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
            </div>

            <div className="p-5 overflow-y-auto space-y-3.5 text-[12.5px]">
              {showLivePreview && (
                <div className="h-56 rounded-xl border border-purple-500/25 bg-[#0E0F13] overflow-hidden">
                  <BpmnViewer
                    bpmnXml={livePreviewXml}
                    title={mTitle || "پیش‌نمایش زنده فرآیند"}
                    className="h-full"
                  />
                </div>
              )}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                <div className="sm:col-span-2 space-y-1">
                  <label className="text-white/80 font-medium block">عنوان متد / دستورالعمل *</label>
                  <input
                    value={mTitle}
                    onChange={(e) => setMTitle(e.target.value)}
                    placeholder="مثلاً: متد استقرار و تست نسخه جدید سرور"
                    className="w-full mab-input px-3 py-2 select-text"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-white/80 font-medium block">دسته‌بندی</label>
                  <select
                    value={mCategory}
                    onChange={(e) => setMCategory(e.target.value)}
                    className="w-full mab-input px-3 py-2 bg-[#0D0E11]"
                  >
                    <option value="GENERAL">GENERAL</option>
                    <option value="BACKEND">BACKEND</option>
                    <option value="UI_UX">UI_UX</option>
                    <option value="DEPLOY">DEPLOY</option>
                    <option value="ARCHITECTURE">ARCHITECTURE</option>
                    <option value="VIDEO">VIDEO</option>
                  </select>
                </div>
              </div>

              <div className="space-y-1">
                <label className="text-white/80 font-medium block">هدف (Objective)</label>
                <input
                  value={mObjective}
                  onChange={(e) => setMObjective(e.target.value)}
                  placeholder="این متد چه خروجی استاندارد و بدون خطایی تضمین می‌کند؟"
                  className="w-full mab-input px-3 py-2 select-text"
                />
              </div>

              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <label className="text-white/80 font-medium">
                    گام‌های ورک‌فلو (برای مجموعه کار داخلی، زیرگام‌ها را خط‌به‌خط بنویسید)
                  </label>
                  <button
                    type="button"
                    onClick={() =>
                      setMSteps((prev) => [
                        ...prev,
                        {
                          title: "",
                          description: "",
                          element_type: "userTask",
                          role: "",
                          sub_steps_text: "",
                        },
                      ])
                    }
                    className="px-2.5 py-1 rounded-lg bg-blue-500/15 hover:bg-blue-500/25 text-blue-300 text-[11px] font-semibold inline-flex items-center gap-1"
                  >
                    <Plus className="w-3 h-3" />
                    <span>افزودن گام</span>
                  </button>
                </div>

                {mSteps.map((st, idx) => (
                  <div
                    key={idx}
                    className="p-3 rounded-xl bg-[#0E0F12] border border-white/[0.07] space-y-2"
                  >
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-[11px] text-blue-400 font-bold">
                        #{idx + 1}
                      </span>
                      <input
                        value={st.title}
                        onChange={(e) => {
                          const v = e.target.value;
                          setMSteps((prev) =>
                            prev.map((x, i) => (i === idx ? { ...x, title: v } : x))
                          );
                        }}
                        placeholder="عنوان گام..."
                        className="flex-1 mab-input px-2.5 py-1.5 text-[12px] select-text"
                      />
                      <select
                        value={st.element_type}
                        onChange={(e) => {
                          const v = e.target.value;
                          setMSteps((prev) =>
                            prev.map((x, i) => (i === idx ? { ...x, element_type: v } : x))
                          );
                        }}
                        className="mab-input px-2 py-1.5 text-[11px] bg-[#0D0E11]"
                      >
                        <option value="userTask">User Task (گام انسانی)</option>
                        <option value="serviceTask">Service Task (گام سیستمی)</option>
                        <option value="subProcess">SubProcess (زیرفرآیند قابل کلیک)</option>
                        <option value="exclusiveGateway">Exclusive Gateway (دروازه تصمیم XOR)</option>
                        <option value="parallelGateway">Parallel Gateway (اجرای موازی AND)</option>
                      </select>
                      {idx > 0 && (
                        <button
                          type="button"
                          onClick={() => moveStep(idx, -1)}
                          title="بالا"
                          className="p-1 text-white/45 hover:text-white"
                        >
                          <ArrowUp className="w-3.5 h-3.5" />
                        </button>
                      )}
                      {idx < mSteps.length - 1 && (
                        <button
                          type="button"
                          onClick={() => moveStep(idx, 1)}
                          title="پایین"
                          className="p-1 text-white/45 hover:text-white"
                        >
                          <ArrowDown className="w-3.5 h-3.5" />
                        </button>
                      )}
                      {mSteps.length > 1 && (
                        <button
                          type="button"
                          onClick={() =>
                            setMSteps((prev) => prev.filter((_, i) => i !== idx))
                          }
                          className="p-1 text-white/40 hover:text-rose-400"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>

                    <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                      <input
                        value={st.description}
                        onChange={(e) => {
                          const v = e.target.value;
                          setMSteps((prev) =>
                            prev.map((x, i) => (i === idx ? { ...x, description: v } : x))
                          );
                        }}
                        placeholder="شرح کوتاه گام..."
                        className="mab-input px-2.5 py-1.5 text-[11.5px] select-text"
                      />
                      <input
                        value={st.role}
                        onChange={(e) => {
                          const v = e.target.value;
                          setMSteps((prev) =>
                            prev.map((x, i) => (i === idx ? { ...x, role: v } : x))
                          );
                        }}
                        placeholder="نقش / مسئول (اختیاری)"
                        className="mab-input px-2.5 py-1.5 text-[11.5px] select-text"
                      />
                      <input
                        value={st.condition_label || ""}
                        onChange={(e) => {
                          const v = e.target.value;
                          setMSteps((prev) =>
                            prev.map((x, i) => (i === idx ? { ...x, condition_label: v } : x))
                          );
                        }}
                        placeholder="برچسب یال / شرط انتقال (اختیاری)"
                        className="mab-input px-2.5 py-1.5 text-[11.5px] select-text"
                      />
                    </div>

                    {st.element_type === "subProcess" && (
                      <textarea
                        value={st.sub_steps_text}
                        onChange={(e) => {
                          const v = e.target.value;
                          setMSteps((prev) =>
                            prev.map((x, i) => (i === idx ? { ...x, sub_steps_text: v } : x))
                          );
                        }}
                        rows={2}
                        placeholder="زیرگام‌های داخلی این مجموعه کار (هر خط یک زیرگام)..."
                        className="w-full mab-input px-2.5 py-1.5 text-[11.5px] select-text"
                      />
                    )}
                  </div>
                ))}
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                <div className="space-y-1">
                  <label className="text-white/80 font-medium block">پیش‌نیازها (هر خط یک مورد)</label>
                  <textarea
                    value={mPrereqs}
                    onChange={(e) => setMPrereqs(e.target.value)}
                    rows={2}
                    className="w-full mab-input p-2 text-[12px] select-text"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-white/80 font-medium block">چک‌لیست نهایی (هر خط یک مورد)</label>
                  <textarea
                    value={mChecklist}
                    onChange={(e) => setMChecklist(e.target.value)}
                    rows={2}
                    className="w-full mab-input p-2 text-[12px] select-text"
                  />
                </div>
              </div>

              <div className="space-y-1">
                <label className="text-white/80 font-medium block">نکات و هشدارها</label>
                <textarea
                  value={mNotes}
                  onChange={(e) => setMNotes(e.target.value)}
                  rows={2}
                  className="w-full mab-input p-2 text-[12px] select-text"
                />
              </div>
            </div>

            <div className="px-5 py-3 border-t border-white/[0.08] flex items-center justify-end gap-2 bg-[#121317]">
              <button
                type="button"
                onClick={() => setShowCreateModal(false)}
                className="px-4 py-1.5 rounded-lg bg-white/[0.05] hover:bg-white/[0.09] text-white/75 text-[12px]"
              >
                انصراف
              </button>
              <button
                type="button"
                disabled={!mTitle.trim() || isSaving}
                onClick={() => void handleSaveNewMethod()}
                className="px-4 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[12px] font-semibold disabled:opacity-40"
              >
                {isSaving ? "در حال ذخیره..." : "ذخیره متد و ساخت BPMN 2.0"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default MethodsAndBpmnView;
