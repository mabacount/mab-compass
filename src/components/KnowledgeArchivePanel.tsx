import { useMemo, useState, type ChangeEvent, type DragEvent } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronsUpDown,
  Clock,
  ExternalLink,
  FileCode2,
  FileText,
  RefreshCw,
  RotateCcw,
  UploadCloud,
} from "lucide-react";
import {
  DashboardState,
  KnowledgeDocument,
  LegacyReprocessCandidate,
  WrittenArtifactRef,
} from "../lib/tauri_api";
import { MabSelect } from "./ui/MabSelect";
import { LegacyReprocessPanel } from "./LegacyReprocessPanel";
import { formatShortTime } from "../lib/formatters";

interface KnowledgeArchivePanelProps {
  dash: DashboardState;
  docQueueStatus: string | null;
  onBatchFilesSelected: (e: ChangeEvent<HTMLInputElement>) => void;
  onBatchFilesDropped: (files: FileList) => void;
  onManualDocIngest: (title: string, docType: string, content: string) => Promise<void>;
  onRequeueDoc: (docId: number) => void;
  onOpenArtifactTab: (artifact: WrittenArtifactRef) => void;
  onDashboardUpdated: (dash: DashboardState) => void;
  onToast: (msg: string) => void;
}

const DOC_TYPE_OPTIONS = [
  { value: "FREE_TEXT", label: "متن آزاد / یادداشت سریع" },
  { value: "MEETING_MINUTES", label: "صورت‌جلسه (Meeting Minutes)" },
  { value: "PROJECT_CHARTER", label: "منشور پروژه (Charter / Spec)" },
  { value: "TECHNICAL_DOC", label: "مستند فنی و معماری" },
  { value: "SOURCE_CODE", label: "قطعه کد / لاگ فنی" },
];

type DocStatusFilter = "ALL" | "UNPROCESSED" | "PROCESSED";

export function KnowledgeArchivePanel({
  dash,
  docQueueStatus,
  onBatchFilesSelected,
  onBatchFilesDropped,
  onManualDocIngest,
  onRequeueDoc,
  onOpenArtifactTab,
  onDashboardUpdated,
  onToast,
}: KnowledgeArchivePanelProps) {
  const [manualDocTitle, setManualDocTitle] = useState("");
  const [manualDocType, setManualDocType] = useState("FREE_TEXT");
  const [manualDocContent, setManualDocContent] = useState("");
  const [isManualExpanded, setIsManualExpanded] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [docStatusFilter, setDocStatusFilter] = useState<DocStatusFilter>("ALL");

  const unprocessedDocsCount = useMemo(
    () => dash.knowledge_docs.filter((d) => d.processing_status !== "PROCESSED").length,
    [dash.knowledge_docs]
  );
  const processedDocsCount = dash.knowledge_docs.length - unprocessedDocsCount;

  const filteredDocs = useMemo(() => {
    return dash.knowledge_docs.filter((doc) => {
      const isDone = doc.processing_status === "PROCESSED";
      if (docStatusFilter === "PROCESSED") return isDone;
      if (docStatusFilter === "UNPROCESSED") return !isDone;
      return true;
    });
  }, [dash.knowledge_docs, docStatusFilter]);

  const handleSubmitManual = async () => {
    if (!manualDocContent.trim() || docQueueStatus) return;
    await onManualDocIngest(manualDocTitle, manualDocType, manualDocContent);
    setManualDocTitle("");
    setManualDocContent("");
  };

  const handleDrop = (e: DragEvent<HTMLLabelElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
    if (docQueueStatus) return;
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      onBatchFilesDropped(e.dataTransfer.files);
    }
  };

  const openDocInTab = (doc: KnowledgeDocument) => {
    onOpenArtifactTab({
      artifact_type: "KNOWLEDGE_DOC",
      entity_id: String(doc.id),
      title: doc.title,
      summary: doc.executive_summary,
      file_path: doc.archived_path,
      action: "UPDATED",
      version: null,
    });
  };

  const openLegacyCandidate = (c: LegacyReprocessCandidate) => {
    onOpenArtifactTab({
      artifact_type: c.source_kind === "KNOWLEDGE_DOC" ? "KNOWLEDGE_DOC" : "MARKDOWN_FILE",
      entity_id: String(c.source_id),
      title: c.title,
      summary: c.snippet,
      file_path: c.archived_path ?? null,
      action: "UPDATED",
      version: null,
    });
  };

  return (
    <div dir="rtl" className="h-full overflow-y-auto p-4 space-y-4">
      {/* 1. Legacy Memory & Document Reprocessing Engine */}
      <LegacyReprocessPanel
        projectId={dash.active_project.id}
        onDashboardUpdated={onDashboardUpdated}
        onToast={onToast}
        onOpenArtifact={openLegacyCandidate}
      />

      {/* 2. Dropzone + Manual Text Ingest side-by-side on wider canvases */}
      <div className="grid grid-cols-1 xl:grid-cols-2 gap-3.5">
        <div className="mab-card p-4 space-y-3">
          <div className="flex items-center justify-between gap-2 flex-wrap">
            <div className="flex items-center gap-2 text-[13px] font-bold text-white">
              <UploadCloud className="w-4 h-4 text-blue-400" />
              <span>بارگذاری هر نوع سند یا فایل</span>
            </div>
            <span className="px-2 py-0.5 rounded-md bg-white/[0.05] border border-mab-border text-white/60 font-mono text-[10px]">
              ALL FORMATS
            </span>
          </div>

          <label
            onDragOver={(e) => {
              e.preventDefault();
              setIsDragging(true);
            }}
            onDragLeave={() => setIsDragging(false)}
            onDrop={handleDrop}
            className={`w-full py-5 px-4 border border-dashed rounded-xl bg-white/[0.01] hover:bg-white/[0.03] flex flex-col items-center justify-center gap-2 cursor-pointer transition ${
              isDragging
                ? "border-blue-500 bg-blue-500/[0.06]"
                : "border-white/[0.12] hover:border-white/[0.25]"
            }`}
          >
            <div className="w-9 h-9 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400">
              <UploadCloud className="w-4 h-4" />
            </div>
            <span className="text-[12px] font-semibold text-white text-center">
              انتخاب یا رها کردن فایل‌ها (PDF، DOCX، کد، متن، صورت‌جلسه)
            </span>
            <span className="text-[11px] text-mab-text-muted text-center">
              ثبت فوری در حافظه و صف پردازش زمانی
            </span>
            <input
              type="file"
              multiple
              disabled={!!docQueueStatus}
              onChange={onBatchFilesSelected}
              className="hidden"
            />
          </label>

          {docQueueStatus && (
            <div className="p-2.5 rounded-xl bg-purple-500/10 border border-purple-500/30 text-purple-300 text-[11.5px] font-semibold flex items-center gap-2">
              <RefreshCw className="w-3.5 h-3.5 animate-spin shrink-0" />
              <span className="min-w-0 truncate">{docQueueStatus}</span>
            </div>
          )}
        </div>

        <div className="mab-card p-4 space-y-2.5">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 text-[13px] font-bold text-white">
              <FileText className="w-4 h-4 text-emerald-400" />
              <span>ورود متن آزاد یا گزارش کار</span>
            </div>
            <button
              type="button"
              onClick={() => setIsManualExpanded((v) => !v)}
              className="p-1 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-white/60 hover:text-white transition"
            >
              <ChevronsUpDown className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            <input
              value={manualDocTitle}
              onChange={(e) => setManualDocTitle(e.target.value)}
              placeholder="عنوان دلخواه..."
              className="w-full mab-input px-3 py-1.5 text-[12px] select-text"
            />
            <MabSelect
              value={manualDocType}
              onChange={setManualDocType}
              options={DOC_TYPE_OPTIONS}
              className="w-full"
            />
          </div>

          <textarea
            value={manualDocContent}
            onChange={(e) => setManualDocContent(e.target.value)}
            placeholder="متن، گزارش کار، دستورالعمل یا صورت‌جلسه را اینجا وارد کنید..."
            rows={isManualExpanded ? 7 : 3}
            className="w-full mab-input p-2.5 text-[12.5px] leading-relaxed select-text resize-none"
          />

          <button
            type="button"
            onClick={() => void handleSubmitManual()}
            disabled={!!docQueueStatus || !manualDocContent.trim()}
            className="w-full py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-[12px] transition disabled:opacity-40"
          >
            ثبت در حافظه و ارسال به صف پردازش هوشمند
          </button>
        </div>
      </div>

      {/* 3. Ingested Knowledge Documents List */}
      <div className="mab-card p-4 space-y-3">
        <div className="flex items-center justify-between gap-2 flex-wrap">
          <span className="text-[13.5px] font-bold text-white flex items-center gap-1.5">
            <FileCode2 className="w-4 h-4 text-purple-400" />
            <span>اسناد ثبت‌شده پروژه ({dash.knowledge_docs.length})</span>
          </span>

          <div className="flex items-center gap-1 bg-mab-base/80 p-0.5 rounded-lg border border-mab-border text-[10.5px]">
            <button
              type="button"
              onClick={() => setDocStatusFilter("ALL")}
              className={`px-2 py-0.5 rounded-md transition ${
                docStatusFilter === "ALL"
                  ? "bg-white/[0.1] text-white font-semibold"
                  : "text-mab-text-muted hover:text-white"
              }`}
            >
              همه ({dash.knowledge_docs.length})
            </button>
            <button
              type="button"
              onClick={() => setDocStatusFilter("UNPROCESSED")}
              className={`px-2 py-0.5 rounded-md transition ${
                docStatusFilter === "UNPROCESSED"
                  ? "bg-amber-500/20 text-amber-300 font-semibold"
                  : "text-mab-text-muted hover:text-white"
              }`}
            >
              پردازش‌نشده ({unprocessedDocsCount})
            </button>
            <button
              type="button"
              onClick={() => setDocStatusFilter("PROCESSED")}
              className={`px-2 py-0.5 rounded-md transition ${
                docStatusFilter === "PROCESSED"
                  ? "bg-emerald-500/20 text-emerald-300 font-semibold"
                  : "text-mab-text-muted hover:text-white"
              }`}
            >
              پردازش‌شده ({processedDocsCount})
            </button>
          </div>
        </div>

        <div className="space-y-2">
          {filteredDocs.map((doc) => {
            const st = (doc.processing_status || "UNPROCESSED").toUpperCase();
            const isProcessed = st === "PROCESSED";
            const isProcessing = st === "PROCESSING";
            const isFailed = st === "FAILED";

            return (
              <div
                key={doc.id}
                onClick={() => openDocInTab(doc)}
                className={`p-3 rounded-xl border space-y-1.5 cursor-pointer transition ${
                  isProcessing
                    ? "bg-blue-500/[0.06] border-blue-500/35"
                    : isFailed
                    ? "bg-rose-500/[0.04] border-rose-500/30"
                    : !isProcessed
                    ? "bg-amber-500/[0.04] border-amber-500/25"
                    : "bg-white/[0.02] hover:bg-white/[0.04] border-mab-border"
                }`}
              >
                <div className="flex items-center justify-between gap-2 flex-wrap">
                  <div className="flex items-center gap-1.5 min-w-0 flex-1">
                    <span className="px-1.5 py-0.5 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px] shrink-0">
                      {doc.doc_type}
                    </span>
                    <span className="text-[12.5px] font-bold text-white truncate">
                      {doc.title}
                    </span>
                  </div>

                  <div className="flex items-center gap-1.5 shrink-0">
                    {isProcessed ? (
                      <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 border border-emerald-500/30 text-emerald-300 text-[10px] font-bold inline-flex items-center gap-1">
                        <CheckCircle2 className="w-3 h-3" />
                        <span>پردازش‌شده (+{doc.extracted_items_count})</span>
                      </span>
                    ) : isProcessing ? (
                      <span className="px-2 py-0.5 rounded-full bg-blue-500/15 border border-blue-500/30 text-blue-300 text-[10px] font-bold inline-flex items-center gap-1">
                        <RefreshCw className="w-3 h-3 animate-spin" />
                        <span>در حال پردازش...</span>
                      </span>
                    ) : isFailed ? (
                      <span className="px-2 py-0.5 rounded-full bg-rose-500/15 border border-rose-500/30 text-rose-300 text-[10px] font-bold inline-flex items-center gap-1">
                        <AlertTriangle className="w-3 h-3" />
                        <span>خطا (پردازش‌نشده)</span>
                      </span>
                    ) : (
                      <span className="px-2 py-0.5 rounded-full bg-amber-500/15 border border-amber-500/30 text-amber-300 text-[10px] font-bold inline-flex items-center gap-1">
                        <Clock className="w-3 h-3" />
                        <span>پردازش‌نشده</span>
                      </span>
                    )}
                  </div>
                </div>

                <p className="text-[12px] text-mab-text-secondary leading-relaxed line-clamp-2 select-text">
                  {doc.executive_summary}
                </p>

                <div className="flex items-center justify-between gap-2 pt-0.5 flex-wrap">
                  <div className="flex items-center gap-2 min-w-0 text-[10px] text-mab-text-muted">
                    <span className="bidi-ltr truncate max-w-[180px] opacity-80">
                      {doc.original_filename}
                    </span>
                    <span className="font-timer shrink-0">
                      {formatShortTime(doc.ingested_at)}
                    </span>
                  </div>

                  <div className="flex items-center gap-1.5">
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        openDocInTab(doc);
                      }}
                      className="px-2 py-0.5 rounded-md bg-white/[0.04] hover:bg-white/[0.09] border border-mab-border text-[10.5px] text-white/75 hover:text-white inline-flex items-center gap-1 transition"
                    >
                      <ExternalLink className="w-2.5 h-2.5" />
                      <span>باز کردن در تب</span>
                    </button>

                    {!isProcessing && (
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onRequeueDoc(doc.id);
                        }}
                        className="px-2 py-0.5 rounded-md bg-white/[0.04] hover:bg-blue-500/20 border border-mab-border hover:border-blue-500/35 text-[10.5px] text-white/75 hover:text-blue-300 inline-flex items-center gap-1 transition"
                      >
                        <RotateCcw className="w-2.5 h-2.5" />
                        <span>{isProcessed ? "تحلیل مجدد" : "ارسال به صف"}</span>
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          })}

          {filteredDocs.length === 0 && (
            <div className="py-6 text-center text-[12px] text-mab-text-muted">
              سندی مطابق این فیلتر در آرشیو دانش این پروژه یافت نشد.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default KnowledgeArchivePanel;
