import { useEffect, useMemo, useState } from "react";
import {
  BookOpen,
  Check,
  Copy,
  FileText,
  FolderOpen,
  RefreshCw,
  Workflow,
} from "lucide-react";
import {
  CompassApi,
  DashboardState,
  MethodPlaybook,
  WrittenArtifactRef,
} from "../lib/tauri_api";
import { buildWorkLineBpmn } from "../lib/bpmnEngine";
import { BpmnViewer } from "./bpmn/BpmnViewer";
import { MarkdownRenderer } from "./ui/MarkdownRenderer";

interface ArtifactViewerTabProps {
  artifact: WrittenArtifactRef;
  dash: DashboardState;
  onOpenMethodTab?: (method: MethodPlaybook) => void;
  onStartTask?: (taskId: string) => void;
  onQuickCompleteTask?: (taskId: string, pomosSpent: number) => void;
}

type ViewMode = "MARKDOWN" | "BPMN" | "SPLIT";

export function ArtifactViewerTab({
  artifact,
  dash,
  onOpenMethodTab,
  onStartTask,
  onQuickCompleteTask,
}: ArtifactViewerTabProps) {
  const matchedMethod = useMemo(() => {
    if (artifact.artifact_type !== "METHOD_PLAYBOOK") return null;
    return (
      (dash.methods_playbooks ?? []).find(
        (m) =>
          m.id === artifact.entity_id ||
          (artifact.file_path && m.archived_path === artifact.file_path) ||
          m.title === artifact.title
      ) ?? null
    );
  }, [artifact, dash.methods_playbooks]);

  const matchedWorkLine = useMemo(() => {
    if (artifact.artifact_type !== "WORK_LINE") return null;
    return (
      (dash.work_lines ?? []).find(
        (wl) => wl.id === artifact.entity_id || wl.title === artifact.title
      ) ?? null
    );
  }, [artifact, dash.work_lines]);

  const matchedKnowledgeDoc = useMemo(() => {
    if (artifact.artifact_type !== "KNOWLEDGE_DOC") return null;
    return (
      (dash.knowledge_docs ?? []).find(
        (d) =>
          String(d.id) === String(artifact.entity_id) ||
          (artifact.file_path && d.archived_path === artifact.file_path)
      ) ?? null
    );
  }, [artifact, dash.knowledge_docs]);

  const matchedVaultCard = useMemo(() => {
    if (artifact.artifact_type !== "VAULT_CARD") return null;
    return (
      (dash.personal_vault_items ?? []).find((c) => c.id === artifact.entity_id) ??
      null
    );
  }, [artifact, dash.personal_vault_items]);

  const matchedDecision = useMemo(() => {
    if (artifact.artifact_type !== "DECISION_ADR") return null;
    return (
      (dash.active_decisions ?? []).find(
        (d) => d.id === artifact.entity_id || d.title === artifact.title
      ) ?? null
    );
  }, [artifact, dash.active_decisions]);

  const hasBpmn = Boolean(matchedMethod?.bpmn_xml || matchedWorkLine);
  const [viewMode, setViewMode] = useState<ViewMode>(
    matchedWorkLine ? "BPMN" : matchedMethod ? "MARKDOWN" : "MARKDOWN"
  );
  const [diskContent, setDiskContent] = useState<string | null>(null);
  const [fileLoadError, setFileLoadError] = useState<string | null>(null);
  const [isLoadingFile, setIsLoadingFile] = useState(false);
  const [copiedPath, setCopiedPath] = useState(false);

  const effectiveFilePath =
    artifact.file_path ||
    matchedMethod?.archived_path ||
    matchedKnowledgeDoc?.archived_path ||
    null;

  const loadFileFromDisk = async () => {
    if (!effectiveFilePath) return;
    setIsLoadingFile(true);
    setFileLoadError(null);
    try {
      const content = await CompassApi.readWorkspaceFile(effectiveFilePath);
      setDiskContent(content);
    } catch (e) {
      setDiskContent(null);
      setFileLoadError(String(e));
    } finally {
      setIsLoadingFile(false);
    }
  };

  useEffect(() => {
    setDiskContent(null);
    setFileLoadError(null);
    if (effectiveFilePath && !matchedMethod) {
      void loadFileFromDisk();
    }
  }, [effectiveFilePath, matchedMethod?.id]);

  const bpmnXml = useMemo(() => {
    if (matchedMethod?.bpmn_xml) return matchedMethod.bpmn_xml;
    if (matchedWorkLine) {
      const wlTasks = dash.open_tasks.filter(
        (t) => t.work_line_id === matchedWorkLine.id
      );
      return buildWorkLineBpmn(matchedWorkLine, wlTasks);
    }
    return "";
  }, [matchedMethod, matchedWorkLine, dash.open_tasks]);

  const markdownToDisplay = useMemo(() => {
    if (matchedMethod) {
      return matchedMethod.markdown_content;
    }
    if (diskContent) {
      return diskContent;
    }
    if (matchedVaultCard) {
      return `# ${matchedVaultCard.title}\n\n> **نوع:** \`${matchedVaultCard.item_type}\` | **حوزه:** \`${matchedVaultCard.domain_code}\`\n\n${matchedVaultCard.content_markdown}`;
    }
    if (matchedDecision) {
      return `# تصمیم معماری: ${matchedDecision.title}\n\n> **شناسه:** \`${matchedDecision.id}\` | **دسته:** \`${matchedDecision.category}\` | **وضعیت:** \`${matchedDecision.status}\`\n\n## شرح و معیار تصمیم\n${matchedDecision.definition_of_done || artifact.summary || "—"}\n\n${matchedDecision.first_physical_action ? `## اقدام اجرایی\n${matchedDecision.first_physical_action}` : ""}`;
    }
    if (matchedKnowledgeDoc) {
      return `# ${matchedKnowledgeDoc.title}\n\n> **فایل اصلی:** \`${matchedKnowledgeDoc.original_filename}\` | **وضعیت:** \`${matchedKnowledgeDoc.processing_status}\`\n\n## خلاصه مدیریتی\n${matchedKnowledgeDoc.executive_summary}`;
    }
    if (matchedWorkLine) {
      const wlTasks = dash.open_tasks
        .filter((t) => t.work_line_id === matchedWorkLine.id)
        .sort((a, b) => a.sequence_order - b.sequence_order);
      const listMd = wlTasks
        .map(
          (t, i) =>
            `${i + 1}. **[${t.id}] ${t.title}** (\`${t.status}\`)\n   - اقدام اول: ${t.first_physical_action}\n   - شرط خروج: ${t.definition_of_done}`
        )
        .join("\n");
      return `# لاین کاری: ${matchedWorkLine.title}\n\n> **هدف:** ${matchedWorkLine.goal_statement || "—"}\n\n## گام‌های این لاین\n${listMd || "*(گام بازی ثبت نشده است)*"}`;
    }
    return `# ${artifact.title}\n\n${artifact.summary || "محتوای سند در دسترس است."}`;
  }, [
    matchedMethod,
    diskContent,
    matchedVaultCard,
    matchedDecision,
    matchedKnowledgeDoc,
    matchedWorkLine,
    dash.open_tasks,
    artifact,
  ]);

  const handleCopyPath = async () => {
    if (!effectiveFilePath) return;
    try {
      await navigator.clipboard.writeText(effectiveFilePath);
      setCopiedPath(true);
      window.setTimeout(() => setCopiedPath(false), 1800);
    } catch {
      // ignore
    }
  };

  return (
    <div className="flex flex-col h-full min-h-0 bg-[#0E0F12]">
      {/* Antigravity-style Document Sub-Header */}
      <div
        dir="ltr"
        className="h-[38px] px-4 border-b border-white/[0.06] bg-[#101115] flex items-center justify-between gap-2 shrink-0"
      >
        <div className="flex items-center gap-2 min-w-0">
          {hasBpmn ? (
            <Workflow className="w-3.5 h-3.5 text-purple-400 shrink-0" />
          ) : (
            <FileText className="w-3.5 h-3.5 text-blue-400 shrink-0" />
          )}
          <span className="text-[12.5px] font-medium text-white/90 truncate" dir="auto">
            {matchedMethod?.title || artifact.title}
          </span>
          {matchedMethod && (
            <span className="px-1.5 py-[2px] rounded bg-emerald-500/15 border border-emerald-500/30 text-emerald-300 font-mono text-[10px] shrink-0">
              v{matchedMethod.version}
            </span>
          )}
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          {hasBpmn && (
            <div
              dir="rtl"
              className="flex items-center gap-0.5 bg-[#16171C] p-0.5 rounded-lg border border-white/[0.08] text-[11px]"
            >
              <button
                type="button"
                onClick={() => setViewMode("MARKDOWN")}
                className={`px-2 py-0.5 rounded-md inline-flex items-center gap-1 transition ${
                  viewMode === "MARKDOWN"
                    ? "bg-white/[0.1] text-white font-semibold"
                    : "text-white/55 hover:text-white"
                }`}
              >
                <BookOpen className="w-3 h-3" />
                <span>دستورالعمل</span>
              </button>
              <button
                type="button"
                onClick={() => setViewMode("BPMN")}
                className={`px-2 py-0.5 rounded-md inline-flex items-center gap-1 transition ${
                  viewMode === "BPMN"
                    ? "bg-purple-500/25 text-purple-200 font-semibold"
                    : "text-white/55 hover:text-white"
                }`}
              >
                <Workflow className="w-3 h-3" />
                <span>نمودار BPMN</span>
              </button>
              <button
                type="button"
                onClick={() => setViewMode("SPLIT")}
                className={`px-2 py-0.5 rounded-md hidden sm:inline-flex items-center gap-1 transition ${
                  viewMode === "SPLIT"
                    ? "bg-blue-500/20 text-blue-200 font-semibold"
                    : "text-white/55 hover:text-white"
                }`}
              >
                <span>هر دو</span>
              </button>
            </div>
          )}

          {effectiveFilePath && (
            <button
              type="button"
              onClick={() => void handleCopyPath()}
              title={effectiveFilePath}
              className="p-1.5 rounded-md text-white/55 hover:text-white hover:bg-white/[0.07] transition"
            >
              {copiedPath ? (
                <Check className="w-3.5 h-3.5 text-emerald-400" />
              ) : (
                <Copy className="w-3.5 h-3.5" />
              )}
            </button>
          )}

          {effectiveFilePath && !matchedMethod && (
            <button
              type="button"
              onClick={() => void loadFileFromDisk()}
              title="بارگذاری مجدد از دیسک"
              className="p-1.5 rounded-md text-white/55 hover:text-white hover:bg-white/[0.07] transition"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isLoadingFile ? "animate-spin" : ""}`} />
            </button>
          )}
        </div>
      </div>

      {/* Body */}
      <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
        {hasBpmn && viewMode === "BPMN" ? (
          <div className="flex-1 min-h-0 p-3">
            <BpmnViewer
              bpmnXml={bpmnXml}
              title={matchedMethod?.title || matchedWorkLine?.title || artifact.title}
              allMethods={dash.methods_playbooks ?? []}
              onOpenMethod={onOpenMethodTab}
              onStartTask={onStartTask}
              onQuickCompleteTask={
                onQuickCompleteTask ? (tid) => onQuickCompleteTask(tid, 1) : undefined
              }
            />
          </div>
        ) : hasBpmn && viewMode === "SPLIT" ? (
          <div className="flex-1 min-h-0 flex flex-col gap-2.5 p-3 overflow-hidden">
            <div className="h-[52%] min-h-[240px]">
              <BpmnViewer
                bpmnXml={bpmnXml}
                title={matchedMethod?.title || matchedWorkLine?.title || artifact.title}
                allMethods={dash.methods_playbooks ?? []}
                onOpenMethod={onOpenMethodTab}
                onStartTask={onStartTask}
                onQuickCompleteTask={
                  onQuickCompleteTask ? (tid) => onQuickCompleteTask(tid, 1) : undefined
                }
              />
            </div>
            <div className="flex-1 min-h-0 overflow-y-auto px-5 py-4 rounded-xl bg-[#111216] border border-white/[0.06]">
              <MarkdownRenderer content={markdownToDisplay} />
            </div>
          </div>
        ) : (
          <div className="flex-1 min-h-0 overflow-y-auto px-6 sm:px-8 py-5">
            {isLoadingFile ? (
              <div
                dir="rtl"
                className="flex items-center gap-2 text-[12.5px] text-mab-text-muted py-6"
              >
                <RefreshCw className="w-4 h-4 animate-spin text-blue-400" />
                <span>در حال خواندن سند از آرشیو...</span>
              </div>
            ) : (
              <div className="max-w-3xl mx-auto">
                {fileLoadError && !diskContent && (
                  <div
                    dir="rtl"
                    className="mb-4 px-3 py-2 rounded-lg bg-amber-500/[0.08] border border-amber-500/25 text-[11.5px] text-amber-200/90"
                  >
                    فایل آرشیو روی دیسک یافت نشد یا جابه‌جا شده است؛ نسخه ذخیره‌شده در حافظه SQLite نمایش داده می‌شود.
                  </div>
                )}
                <MarkdownRenderer content={markdownToDisplay} />
                {effectiveFilePath && (
                  <div
                    dir="ltr"
                    className="mt-6 pt-3 border-t border-white/[0.06] flex items-center gap-2 text-[11px] font-mono text-white/40"
                  >
                    <FolderOpen className="w-3.5 h-3.5 shrink-0" />
                    <span className="truncate select-text">{effectiveFilePath}</span>
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export default ArtifactViewerTab;
