import { useState, type ChangeEvent } from "react";
import {
  Brain,
  Compass,
  FileText,
  FolderArchive,
  FolderGit2,
  Layers,
  Maximize2,
  Minimize2,
  Plus,
  Settings,
  Terminal,
  Workflow,
  X,
} from "lucide-react";
import {
  AppSettings,
  DashboardState,
  MethodPlaybook,
  MethodPlaybookInput,
  Outcome,
  WorkEntity,
  WorkLine,
  WrittenArtifactRef,
} from "../lib/tauri_api";
import { BuiltinCanvasViewId } from "./AntigravitySidebar";
import { PathAheadPanel } from "./PathAheadPanel";
import { MethodsAndBpmnView } from "./MethodsAndBpmnView";
import { WbsAndDocsView } from "./WbsAndDocsView";
import { KnowledgeArchivePanel } from "./KnowledgeArchivePanel";
import { EcosystemAndCardsView } from "./EcosystemAndCardsView";
import { ProjectStudioView } from "./ProjectStudioView";
import { SettingsView } from "./SettingsView";
import { ArtifactViewerTab } from "./ArtifactViewerTab";
import { SystemLogsPanel } from "./SystemLogsPanel";

export interface CanvasTab {
  id: string;
  kind: BuiltinCanvasViewId | "ARTIFACT";
  title: string;
  artifact?: WrittenArtifactRef;
}

interface AntigravityRightCanvasProps {
  dash: DashboardState;
  settingsForm: AppSettings;
  tabs: CanvasTab[];
  activeTabId: string | null;
  isMaximized: boolean;
  onSelectTab: (tabId: string) => void;
  onCloseTab: (tabId: string) => void;
  onOpenBuiltinView: (viewId: BuiltinCanvasViewId) => void;
  onOpenArtifactTab: (artifact: WrittenArtifactRef) => void;
  onOpenMethodTab: (method: MethodPlaybook) => void;
  onToggleMaximize: () => void;
  onCloseCanvas: () => void;
  // View-specific callbacks
  checkedGates: Record<number, boolean>;
  onToggleGate: (gateId: number, checked: boolean) => void;
  allGatesPassed: boolean;
  onQuickComplete: (taskId: string, pomosSpent: number) => void;
  onStartTask: (taskId: string) => void;
  onInsertChatPrompt: (text: string) => void;
  onUpsertMethod: (input: MethodPlaybookInput) => Promise<void>;
  onDeleteMethod: (methodId: string) => Promise<void>;
  onOpenNewOutcome: () => void;
  onEditOutcome: (outcome: Outcome) => void;
  onDeleteOutcome: (outcomeId: string) => void;
  onOpenNewWorkLine: () => void;
  onEditWorkLine: (wl: WorkLine) => void;
  onOpenNewTask: () => void;
  onOpenAssignTask: (task: WorkEntity) => void;
  docQueueStatus: string | null;
  onBatchFilesSelected: (e: ChangeEvent<HTMLInputElement>) => void;
  onBatchFilesDropped: (files: FileList) => void;
  onManualDocIngest: (title: string, docType: string, content: string) => Promise<void>;
  onRequeueDoc: (docId: number) => void;
  onDashboardUpdated: (dash: DashboardState) => void;
  onToast: (msg: string) => void;
  onCreateVaultCard: (params: {
    realm: string;
    domainCode: string;
    itemType: string;
    title: string;
    contentMarkdown: string;
    tags: string[];
    attachToActiveProject: boolean;
  }) => Promise<void>;
  onDeleteVaultCard: (itemId: string) => void;
  onResolveProposal: (proposalId: string, approve: boolean) => void;
  onUpsertCalendarDay: (params: {
    dateKey: string;
    totalCapacityMinutes: number;
    deepCapacityMinutes: number;
    dayNote: string;
    isOffDay: boolean;
  }) => Promise<void>;
  onSwitchProject: (projectId: string) => void;
  onCreateProject: (parentId?: string | null) => void;
  onUpdateProject: (payload: {
    projectId: string;
    name?: string;
    parentId?: string | null;
    clearParent?: boolean;
    realm?: string;
    lifecycleStatus?: string;
    color?: string;
    dailyQuotaMinutes?: number | null;
    weeklyQuotaMinutes?: number | null;
    monthlyQuotaMinutes?: number | null;
    hourlyRate?: number | null;
  }) => Promise<void>;
  onMergeProjects: (sourceId: string, targetId: string) => Promise<void>;
  onDeleteProject: (projectId: string) => Promise<void>;
  onChangeSettings: (next: AppSettings) => void;
  onSaveSettings: (override?: AppSettings) => void;
}

function getTabIcon(tab: CanvasTab) {
  switch (tab.kind) {
    case "PROJECTS_STUDIO":
      return <FolderGit2 className="w-3.5 h-3.5 text-emerald-400 shrink-0" />;
    case "PATH_AHEAD":
      return <Compass className="w-3.5 h-3.5 text-blue-400 shrink-0" />;
    case "METHODS_BPMN":
      return <Workflow className="w-3.5 h-3.5 text-purple-400 shrink-0" />;
    case "WBS":
      return <Layers className="w-3.5 h-3.5 text-emerald-400 shrink-0" />;
    case "KNOWLEDGE_ARCHIVE":
      return <FolderArchive className="w-3.5 h-3.5 text-amber-400 shrink-0" />;
    case "ECOSYSTEM_CARDS":
      return <Brain className="w-3.5 h-3.5 text-cyan-400 shrink-0" />;
    case "SETTINGS":
      return <Settings className="w-3.5 h-3.5 text-white/70 shrink-0" />;
    case "LOGS":
      return <Terminal className="w-3.5 h-3.5 text-blue-400 shrink-0" />;
    default:
      return tab.artifact?.artifact_type === "METHOD_PLAYBOOK" ||
        tab.artifact?.artifact_type === "WORK_LINE" ? (
        <Workflow className="w-3.5 h-3.5 text-purple-400 shrink-0" />
      ) : (
        <FileText className="w-3.5 h-3.5 text-white/75 shrink-0" />
      );
  }
}

export function AntigravityRightCanvas({
  dash,
  settingsForm,
  tabs,
  activeTabId,
  isMaximized,
  onSelectTab,
  onCloseTab,
  onOpenBuiltinView,
  onOpenArtifactTab,
  onOpenMethodTab,
  onToggleMaximize,
  onCloseCanvas,
  checkedGates,
  onToggleGate,
  allGatesPassed,
  onQuickComplete,
  onStartTask,
  onInsertChatPrompt,
  onUpsertMethod,
  onDeleteMethod,
  onOpenNewOutcome,
  onEditOutcome,
  onDeleteOutcome,
  onOpenNewWorkLine,
  onEditWorkLine,
  onOpenNewTask,
  onOpenAssignTask,
  docQueueStatus,
  onBatchFilesSelected,
  onBatchFilesDropped,
  onManualDocIngest,
  onRequeueDoc,
  onDashboardUpdated,
  onToast,
  onCreateVaultCard,
  onDeleteVaultCard,
  onResolveProposal,
  onUpsertCalendarDay,
  onSwitchProject,
  onCreateProject,
  onUpdateProject,
  onMergeProjects,
  onDeleteProject,
  onChangeSettings,
  onSaveSettings,
}: AntigravityRightCanvasProps) {
  const [showNewTabMenu, setShowNewTabMenu] = useState(false);

  const activeTab = tabs.find((t) => t.id === activeTabId) ?? tabs[0] ?? null;

  const openWorkLineBpmn = (wl: WorkLine) => {
    onOpenArtifactTab({
      artifact_type: "WORK_LINE",
      entity_id: wl.id,
      title: `BPMN: ${wl.title}`,
      summary: wl.goal_statement || wl.title,
      file_path: null,
      action: "UPDATED",
      version: null,
    });
  };

  return (
    <section className="flex flex-col h-full min-h-0 min-w-0 bg-[#0E0F12] overflow-hidden">
      {/* Antigravity Multi-Tab Strip (Exact top-right tab bar) */}
      <div
        dir="ltr"
        className="h-[38px] bg-[#121317] border-b border-white/[0.07] flex items-center justify-between pl-1 pr-2 shrink-0 select-none relative"
      >
        {/* Scrollable Tabs */}
        <div className="flex items-center gap-0.5 overflow-x-auto min-w-0 flex-1 h-full scrollbar-none">
          {tabs.map((tab) => {
            const isActive = activeTab?.id === tab.id;
            return (
              <div
                key={tab.id}
                onClick={() => onSelectTab(tab.id)}
                className={`group h-full px-3 flex items-center gap-2 text-[12px] cursor-pointer border-r border-white/[0.05] transition shrink-0 max-w-[210px] ${
                  isActive
                    ? "bg-[#0E0F12] text-white font-medium border-t-2 border-t-blue-500/80"
                    : "text-[#8E93A2] hover:bg-white/[0.04] hover:text-white border-t-2 border-t-transparent"
                }`}
              >
                {getTabIcon(tab)}
                <span dir="auto" className="truncate">
                  {tab.title}
                </span>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onCloseTab(tab.id);
                  }}
                  title="بستن تب"
                  className="p-0.5 rounded hover:bg-white/[0.12] text-white/40 hover:text-white opacity-60 group-hover:opacity-100 transition shrink-0"
                >
                  <X className="w-3 h-3" />
                </button>
              </div>
            );
          })}
        </div>

        {/* Right Toolbar Icons: '+' New Tab Menu (OUTSIDE overflow-x-auto so dropdown never clips!), Maximize, Close */}
        <div className="flex items-center gap-1 shrink-0 pl-1">
          <div className="relative flex items-center">
            <button
              type="button"
              onClick={() => setShowNewTabMenu((v) => !v)}
              title="باز کردن تب جدید"
              className="w-7 h-7 rounded-md hover:bg-white/[0.07] text-white/55 hover:text-white flex items-center justify-center transition"
            >
              <Plus className="w-3.5 h-3.5" />
            </button>

            {showNewTabMenu && (
              <>
                <div
                  className="fixed inset-0 z-40"
                  onClick={() => setShowNewTabMenu(false)}
                />
                <div
                  dir="rtl"
                  className="absolute top-full right-0 mt-1 w-56 rounded-xl bg-[#1A1B22] border border-white/[0.1] shadow-2xl z-50 p-1 space-y-0.5"
                >
                  {(
                    [
                      { id: "PATH_AHEAD", label: "مسیر پیش‌رو", icon: Compass },
                      { id: "PROJECTS_STUDIO", label: "استودیو و ادغام پروژه‌ها", icon: FolderGit2 },
                      { id: "METHODS_BPMN", label: "متدها و نقشه BPMN 2.0", icon: Workflow },
                      { id: "WBS", label: "ساختار WBS و لاین‌ها", icon: Layers },
                      {
                        id: "KNOWLEDGE_ARCHIVE",
                        label: "اسناد و بازپردازش حافظه",
                        icon: FolderArchive,
                      },
                      {
                        id: "ECOSYSTEM_CARDS",
                        label: "اهداف، کارت‌ها و تقویم",
                        icon: Brain,
                      },
                      { id: "SETTINGS", label: "تنظیمات سیستم", icon: Settings },
                      { id: "LOGS", label: "لاگ‌ها و رویدادهای سیستم", icon: Terminal },
                    ] as const
                  ).map((item) => {
                    const Icon = item.icon;
                    return (
                      <button
                        key={item.id}
                        type="button"
                        onClick={() => {
                          onOpenBuiltinView(item.id);
                          setShowNewTabMenu(false);
                        }}
                        className="w-full px-2.5 py-1.5 rounded-lg text-right text-[12px] text-white/80 hover:bg-white/[0.08] hover:text-white flex items-center gap-2 transition"
                      >
                        <Icon className="w-3.5 h-3.5 text-blue-400 shrink-0" />
                        <span>{item.label}</span>
                      </button>
                    );
                  })}
                </div>
              </>
            )}
          </div>

          <button
            type="button"
            onClick={onToggleMaximize}
            title={isMaximized ? "بازگردانی اندازه بوم" : "تمام‌صفحه کردن بوم"}
            className="w-7 h-7 rounded-md hover:bg-white/[0.07] text-white/55 hover:text-white flex items-center justify-center transition"
          >
            {isMaximized ? (
              <Minimize2 className="w-3.5 h-3.5" />
            ) : (
              <Maximize2 className="w-3.5 h-3.5" />
            )}
          </button>
          <button
            type="button"
            onClick={onCloseCanvas}
            title="بستن پنل بوم سمت راست"
            className="w-7 h-7 rounded-md hover:bg-white/[0.07] text-white/55 hover:text-white flex items-center justify-center transition"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Active Tab Content */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {!activeTab ? (
          <div
            dir="rtl"
            className="h-full flex flex-col items-center justify-center text-center p-6 text-mab-text-muted space-y-3"
          >
            <Compass className="w-8 h-8 text-white/20" />
            <p className="text-[13px] text-white/75 font-medium">هیچ تبی باز نیست</p>
            <div className="flex items-center gap-2 flex-wrap justify-center">
              <button
                type="button"
                onClick={() => onOpenBuiltinView("PATH_AHEAD")}
                className="px-3 py-1.5 rounded-lg bg-white/[0.06] hover:bg-white/[0.1] text-white text-[12px]"
              >
                مسیر پیش‌رو
              </button>
              <button
                type="button"
                onClick={() => onOpenBuiltinView("PROJECTS_STUDIO")}
                className="px-3 py-1.5 rounded-lg bg-emerald-500/15 hover:bg-emerald-500/25 border border-emerald-500/30 text-emerald-200 text-[12px]"
              >
                استودیو و ادغام پروژه‌ها
              </button>
              <button
                type="button"
                onClick={() => onOpenBuiltinView("METHODS_BPMN")}
                className="px-3 py-1.5 rounded-lg bg-purple-500/20 hover:bg-purple-500/30 text-purple-200 text-[12px]"
              >
                متدها و BPMN 2.0
              </button>
            </div>
          </div>
        ) : activeTab.kind === "PATH_AHEAD" ? (
          <PathAheadPanel
            dash={dash}
            checkedGates={checkedGates}
            onToggleGate={onToggleGate}
            allGatesPassed={allGatesPassed}
            onQuickComplete={onQuickComplete}
            onStartTask={onStartTask}
            onInsertChatPrompt={onInsertChatPrompt}
            onSwitchProject={onSwitchProject}
            onOpenWorkLineBpmn={openWorkLineBpmn}
            onOpenProjectBpmn={() => onOpenBuiltinView("METHODS_BPMN")}
          />
        ) : activeTab.kind === "PROJECTS_STUDIO" ? (
          <ProjectStudioView
            dash={dash}
            onSwitchProject={onSwitchProject}
            onCreateProject={onCreateProject}
            onUpdateProject={onUpdateProject}
            onMergeProjects={onMergeProjects}
            onDeleteProject={onDeleteProject}
          />
        ) : activeTab.kind === "METHODS_BPMN" ? (
          <MethodsAndBpmnView
            dash={dash}
            onOpenMethodTab={onOpenMethodTab}
            onUpsertMethod={onUpsertMethod}
            onDeleteMethod={onDeleteMethod}
            onStartTask={onStartTask}
            onQuickCompleteTask={onQuickComplete}
            onAskCopilotToExtractMethod={() =>
              onInsertChatPrompt(
                "از گزارش کار و تاریخچه این پروژه، دستورالعمل گام‌به‌گام (متد انجام کار) و نمودار BPMN 2.0 آن را استخراج و ذخیره کن."
              )
            }
          />
        ) : activeTab.kind === "WBS" ? (
          <div dir="rtl" className="h-full p-3.5 overflow-hidden">
            <WbsAndDocsView
              dash={dash}
              onQuickComplete={onQuickComplete}
              onStartTask={onStartTask}
              onOpenNewOutcome={onOpenNewOutcome}
              onEditOutcome={onEditOutcome}
              onOpenNewWorkLine={onOpenNewWorkLine}
              onEditWorkLine={onEditWorkLine}
              onOpenNewTask={onOpenNewTask}
              onOpenAssignTask={onOpenAssignTask}
            />
          </div>
        ) : activeTab.kind === "KNOWLEDGE_ARCHIVE" ? (
          <KnowledgeArchivePanel
            dash={dash}
            docQueueStatus={docQueueStatus}
            onBatchFilesSelected={onBatchFilesSelected}
            onBatchFilesDropped={onBatchFilesDropped}
            onManualDocIngest={onManualDocIngest}
            onRequeueDoc={onRequeueDoc}
            onOpenArtifactTab={onOpenArtifactTab}
            onDashboardUpdated={onDashboardUpdated}
            onToast={onToast}
          />
        ) : activeTab.kind === "ECOSYSTEM_CARDS" ? (
          <div dir="rtl" className="h-full p-3.5 overflow-hidden">
            <EcosystemAndCardsView
              dash={dash}
              onOpenNewOutcome={onOpenNewOutcome}
              onEditOutcome={onEditOutcome}
              onDeleteOutcome={onDeleteOutcome}
              onCreateVaultCard={onCreateVaultCard}
              onDeleteVaultCard={onDeleteVaultCard}
              onResolveProposal={onResolveProposal}
              onUpsertCalendarDay={onUpsertCalendarDay}
            />
          </div>
        ) : activeTab.kind === "SETTINGS" ? (
          <div dir="rtl" className="h-full overflow-y-auto p-5">
            <SettingsView
              settingsForm={settingsForm}
              onChange={onChangeSettings}
              onSave={() => onSaveSettings()}
              onQuickSaveSettings={(next) => onSaveSettings(next)}
            />
          </div>
        ) : activeTab.kind === "LOGS" ? (
          <SystemLogsPanel />
        ) : activeTab.kind === "ARTIFACT" && activeTab.artifact ? (
          <ArtifactViewerTab
            key={activeTab.id}
            artifact={activeTab.artifact}
            dash={dash}
            onOpenMethodTab={onOpenMethodTab}
            onStartTask={onStartTask}
            onQuickCompleteTask={onQuickComplete}
          />
        ) : null}
      </div>
    </section>
  );
}

export default AntigravityRightCanvas;
