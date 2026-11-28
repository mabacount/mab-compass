import { useMemo, useState } from "react";
import {
  BookMarked,
  BookOpen,
  Brain,
  Calendar,
  Check,
  CheckCircle2,
  Compass,
  Download,
  Edit3,
  ExternalLink,
  FileText,
  GitPullRequest,
  Plus,
  Search,
  ShieldAlert,
  Sparkles,
  Target,
  Trash2,
  Workflow,
  X,
} from "lucide-react";
import { DashboardState, Outcome, WrittenArtifactRef } from "../lib/tauri_api";
import { formatShortDate, formatShortTime } from "../lib/formatters";
import { MabSelect } from "./ui/MabSelect";

type EcosystemTab = "CARDS" | "DOMAINS_OUTCOMES" | "PROPOSALS" | "CALENDAR";
type DataOriginFilter = "ALL" | "CREATED" | "IMPORTED";

interface EcosystemAndCardsViewProps {
  dash: DashboardState;
  onOpenNewOutcome: () => void;
  onEditOutcome: (outcome: Outcome) => void;
  onDeleteOutcome: (outcomeId: string) => void;
  onCreateVaultCard: (params: {
    realm: string;
    domainCode: string;
    itemType: string;
    title: string;
    contentMarkdown: string;
    tags: string[];
    attachToActiveProject: boolean;
  }) => Promise<void> | void;
  onDeleteVaultCard: (itemId: string) => void;
  onResolveProposal: (proposalId: string, approve: boolean) => void;
  onUpsertCalendarDay: (params: {
    dateKey: string;
    totalCapacityMinutes: number;
    deepCapacityMinutes: number;
    dayNote: string;
    isOffDay: boolean;
  }) => Promise<void> | void;
  onOpenArtifactTab: (artifact: WrittenArtifactRef) => void;
  onInsertChatPrompt?: (text: string) => void;
}

const CARD_TYPE_META: Record<
  string,
  { label: string; colorClass: string }
> = {
  MEMORY_CARD: {
    label: "کارت دانش",
    colorClass: "bg-blue-500/15 border-blue-500/30 text-blue-300",
  },
  LESSON: {
    label: "درس‌آموخته",
    colorClass: "bg-amber-500/15 border-amber-500/30 text-amber-300",
  },
  LESSON_LEARNED: {
    label: "درس‌آموخته",
    colorClass: "bg-amber-500/15 border-amber-500/30 text-amber-300",
  },
  INSTRUCTION: {
    label: "دستورالعمل",
    colorClass: "bg-emerald-500/15 border-emerald-500/30 text-emerald-300",
  },
  NOTE: {
    label: "نکته کلیدی",
    colorClass: "bg-purple-500/15 border-purple-500/30 text-purple-300",
  },
  IDEA: {
    label: "ایده",
    colorClass: "bg-cyan-500/15 border-cyan-500/30 text-cyan-300",
  },
  BOOKMARK: {
    label: "نشانک",
    colorClass: "bg-indigo-500/15 border-indigo-500/30 text-indigo-300",
  },
  INTEREST: {
    label: "علاقه / مطالعه",
    colorClass: "bg-pink-500/15 border-pink-500/30 text-pink-300",
  },
};

const CARD_TYPE_OPTIONS = [
  { value: "MEMORY_CARD", label: "کارت دانش / دانش اتمیک" },
  { value: "LESSON", label: "درس‌آموخته (Lesson Learned)" },
  { value: "INSTRUCTION", label: "دستورالعمل و روش اجرا" },
  { value: "NOTE", label: "نکته کلیدی و یادداشت" },
];

function stripMarkdownPreview(md: string): string {
  return (md || "")
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/[#>*_`~|-]+/g, " ")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 140);
}

export function EcosystemAndCardsView({
  dash,
  onOpenNewOutcome,
  onEditOutcome,
  onDeleteOutcome,
  onCreateVaultCard,
  onDeleteVaultCard,
  onResolveProposal,
  onUpsertCalendarDay,
  onOpenArtifactTab,
  onInsertChatPrompt,
}: EcosystemAndCardsViewProps) {
  const [activeTab, setActiveTab] = useState<EcosystemTab>("CARDS");
  const [searchQuery, setSearchQuery] = useState("");
  const [originFilter, setOriginFilter] = useState<DataOriginFilter>("ALL");
  const [cardTypeFilter, setCardTypeFilter] = useState<string>("ALL");

  // Manual Card creation form state
  const [showNewCardForm, setShowNewCardForm] = useState(false);
  const [cardType, setCardType] = useState("MEMORY_CARD");
  const [cardDomain, setCardDomain] = useState(
    dash.domains?.[0]?.code || "MIND_LEARNING"
  );
  const [cardTitle, setCardTitle] = useState("");
  const [cardContent, setCardContent] = useState("");
  const [cardTags, setCardTags] = useState("");
  const [isCreatingCard, setIsCreatingCard] = useState(false);

  // Calendar day form state
  const todayIso = new Date().toISOString().slice(0, 10);
  const [calDate, setCalDate] = useState(todayIso);
  const [calTotalMin, setCalTotalMin] = useState(300);
  const [calDeepMin, setCalDeepMin] = useState(150);
  const [calNote, setCalNote] = useState("");
  const [calIsOff, setCalIsOff] = useState(false);

  const cards = dash.personal_vault_items ?? [];
  const methods = dash.methods_playbooks ?? [];
  const decisions = dash.active_decisions ?? [];
  const importedDocs = dash.knowledge_docs ?? [];
  const rawInputs = dash.recent_raw_events ?? [];
  const domains = dash.domains ?? [];
  const outcomes = dash.outcomes ?? [];
  const proposals = dash.pending_proposals ?? [];
  const calendarDays = dash.calendar_days ?? [];

  const pendingProposalsCount = proposals.filter(
    (p) => p.status === "PENDING"
  ).length;

  const projectNameById = useMemo(() => {
    const map = new Map<string, string>();
    for (const p of dash.projects) map.set(p.id, p.name);
    return map;
  }, [dash.projects]);

  const normalizedQuery = searchQuery.trim().toLowerCase();

  const filteredCards = useMemo(() => {
    return cards.filter((c) => {
      if (cardTypeFilter !== "ALL") {
        if (cardTypeFilter === "LESSON") {
          if (c.item_type !== "LESSON" && c.item_type !== "LESSON_LEARNED") return false;
        } else if (c.item_type !== cardTypeFilter) {
          return false;
        }
      }
      if (!normalizedQuery) return true;
      return (
        c.title.toLowerCase().includes(normalizedQuery) ||
        c.content_markdown.toLowerCase().includes(normalizedQuery) ||
        c.tags.some((t) => t.toLowerCase().includes(normalizedQuery))
      );
    });
  }, [cards, cardTypeFilter, normalizedQuery]);

  const filteredMethods = useMemo(() => {
    if (!normalizedQuery) return methods;
    return methods.filter(
      (m) =>
        m.title.toLowerCase().includes(normalizedQuery) ||
        m.summary.toLowerCase().includes(normalizedQuery)
    );
  }, [methods, normalizedQuery]);

  const filteredDecisions = useMemo(() => {
    if (!normalizedQuery) return decisions;
    return decisions.filter(
      (d) =>
        d.title.toLowerCase().includes(normalizedQuery) ||
        d.definition_of_done.toLowerCase().includes(normalizedQuery)
    );
  }, [decisions, normalizedQuery]);

  const filteredImportedDocs = useMemo(() => {
    if (!normalizedQuery) return importedDocs;
    return importedDocs.filter(
      (d) =>
        d.title.toLowerCase().includes(normalizedQuery) ||
        d.executive_summary.toLowerCase().includes(normalizedQuery) ||
        d.original_filename.toLowerCase().includes(normalizedQuery)
    );
  }, [importedDocs, normalizedQuery]);

  const filteredRawInputs = useMemo(() => {
    if (!normalizedQuery) return rawInputs;
    return rawInputs.filter(
      (r) =>
        r.snippet.toLowerCase().includes(normalizedQuery) ||
        r.source_type.toLowerCase().includes(normalizedQuery)
    );
  }, [rawInputs, normalizedQuery]);

  const createdTotalCount = cards.length + methods.length + decisions.length;
  const importedTotalCount = importedDocs.length + rawInputs.length;

  const openVaultCardAsDoc = (card: (typeof cards)[number]) => {
    onOpenArtifactTab({
      artifact_type: "VAULT_CARD",
      entity_id: card.id,
      title: card.title,
      summary: stripMarkdownPreview(card.content_markdown),
      file_path: card.archived_path ?? null,
      action: "CREATED",
      version: null,
      project_id: card.project_id ?? undefined,
    });
  };

  const openMethodAsDoc = (m: (typeof methods)[number]) => {
    onOpenArtifactTab({
      artifact_type: "METHOD_PLAYBOOK",
      entity_id: m.id,
      title: m.title,
      summary: m.summary || m.objective,
      file_path: m.archived_path,
      action: "UPDATED",
      version: m.version,
      project_id: m.project_id,
    });
  };

  const openDecisionAsDoc = (d: (typeof decisions)[number]) => {
    onOpenArtifactTab({
      artifact_type: "DECISION_ADR",
      entity_id: d.id,
      title: d.title,
      summary: d.definition_of_done,
      action: "CREATED",
      version: null,
      project_id: d.project_id,
    });
  };

  const openImportedDocAsTab = (doc: (typeof importedDocs)[number]) => {
    onOpenArtifactTab({
      artifact_type: "KNOWLEDGE_DOC",
      entity_id: String(doc.id),
      title: doc.title,
      summary: doc.executive_summary,
      file_path: doc.archived_path,
      action: "UPDATED",
      version: null,
      project_id: doc.project_id,
    });
  };

  const openRawInputAsTab = (r: (typeof rawInputs)[number]) => {
    onOpenArtifactTab({
      artifact_type: "RAW_EVENT",
      entity_id: String(r.id),
      title: `ورودی خام #${r.id} (${r.source_type})`,
      summary: r.snippet,
      file_path: r.archived_path ?? null,
      action: "UPDATED",
      version: null,
    });
  };

  const handleSubmitCard = async () => {
    if (!cardTitle.trim() || !cardContent.trim() || isCreatingCard) return;
    setIsCreatingCard(true);
    try {
      const tags = cardTags
        .split(/[,،\s]+/)
        .map((s) => s.trim())
        .filter(Boolean);
      await onCreateVaultCard({
        realm: "WORK",
        domainCode: cardDomain,
        itemType: cardType,
        title: cardTitle.trim(),
        contentMarkdown: cardContent.trim(),
        tags,
        attachToActiveProject: true,
      });
      setCardTitle("");
      setCardContent("");
      setCardTags("");
      setShowNewCardForm(false);
    } finally {
      setIsCreatingCard(false);
    }
  };

  const handleSaveCalendarDay = async () => {
    if (!calDate.trim()) return;
    await onUpsertCalendarDay({
      dateKey: calDate.trim(),
      totalCapacityMinutes: calTotalMin,
      deepCapacityMinutes: calDeepMin,
      dayNote: calNote.trim(),
      isOffDay: calIsOff,
    });
    setCalNote("");
  };

  const domainOptions =
    domains.length > 0
      ? domains.map((d) => ({ value: d.code, label: d.title }))
      : [{ value: "MIND_LEARNING", label: "ذهن و یادگیری" }];

  return (
    <div className="flex flex-col gap-3.5 h-full min-h-0">
      {/* Top Bar: Sub-tabs + Search */}
      <div className="mab-card p-3 flex items-center justify-between gap-3 flex-wrap shrink-0 border-mab-border">
        <div className="flex items-center gap-1 bg-mab-base/80 p-1 rounded-xl border border-mab-border overflow-x-auto max-w-full">
          <button
            type="button"
            onClick={() => setActiveTab("CARDS")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeTab === "CARDS"
                ? "bg-white/[0.08] text-white border border-white/[0.1] shadow-sm"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <Brain className="w-3.5 h-3.5 text-blue-400" />
            <span>کارت‌های دانش و درس‌آموخته‌ها</span>
            <span className="font-timer px-1.5 rounded-full bg-white/[0.08] text-[10px]">
              {cards.length}
            </span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("DOMAINS_OUTCOMES")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeTab === "DOMAINS_OUTCOMES"
                ? "bg-white/[0.08] text-white border border-white/[0.1] shadow-sm"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <Target className="w-3.5 h-3.5 text-amber-400" />
            <span>۸ حوزه زندگی و Outcomeها</span>
            <span className="font-timer px-1.5 rounded-full bg-white/[0.08] text-[10px]">
              {outcomes.length}
            </span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("PROPOSALS")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeTab === "PROPOSALS"
                ? "bg-white/[0.08] text-white border border-white/[0.1] shadow-sm"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <GitPullRequest className="w-3.5 h-3.5 text-purple-400" />
            <span>پیشنهادهای زیرپروژه</span>
            {pendingProposalsCount > 0 && (
              <span className="font-timer px-1.5 rounded-full bg-amber-500/25 text-amber-300 text-[10px] font-bold">
                {pendingProposalsCount}
              </span>
            )}
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("CALENDAR")}
            className={`px-3 py-1.5 rounded-lg text-[12px] font-medium transition flex items-center gap-1.5 shrink-0 ${
              activeTab === "CALENDAR"
                ? "bg-white/[0.08] text-white border border-white/[0.1] shadow-sm"
                : "text-mab-text-secondary hover:text-white border border-transparent"
            }`}
          >
            <Calendar className="w-3.5 h-3.5 text-emerald-400" />
            <span>ظرفیت روزانه و تقویم</span>
            <span className="font-timer px-1.5 rounded-full bg-white/[0.08] text-[10px]">
              {calendarDays.length}
            </span>
          </button>
        </div>

        <div className="relative min-w-[180px] sm:min-w-[230px] flex-1 sm:flex-initial">
          <Search className="w-3.5 h-3.5 text-white/40 absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none" />
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="جستجو..."
            className="w-full mab-input pr-8 pl-3 py-1.5 text-[12px] select-text"
          />
        </div>
      </div>

      {/* Main Scrollable Body */}
      <div className="flex-1 min-h-0 overflow-y-auto space-y-4 pb-4">
        {/* ================= TAB 1: SEPARATED KNOWLEDGE & DATA REPOSITORY ================= */}
        {activeTab === "CARDS" && (
          <div className="space-y-4">
            {/* Origin Separation & Action Toolbar */}
            <div className="mab-card p-3 space-y-2.5">
              <div className="flex items-center justify-between gap-2.5 flex-wrap">
                <div className="flex items-center gap-1.5 flex-wrap">
                  {[
                    {
                      id: "ALL" as const,
                      label: `همه داده‌ها (${createdTotalCount + importedTotalCount})`,
                    },
                    {
                      id: "CREATED" as const,
                      label: `✨ داده‌های خلق‌شده توسط اپ و مدل (${createdTotalCount})`,
                    },
                    {
                      id: "IMPORTED" as const,
                      label: `📥 داده‌های واردشده به اپ (${importedTotalCount})`,
                    },
                  ].map((tab) => (
                    <button
                      key={tab.id}
                      type="button"
                      onClick={() => setOriginFilter(tab.id)}
                      className={`px-3 py-1.5 rounded-lg text-[11.5px] font-semibold transition border ${
                        originFilter === tab.id
                          ? "bg-blue-500/20 border-blue-500/45 text-blue-200"
                          : "bg-white/[0.03] border-mab-border text-mab-text-secondary hover:text-white"
                      }`}
                    >
                      {tab.label}
                    </button>
                  ))}
                </div>

                <button
                  type="button"
                  onClick={() => setShowNewCardForm((v) => !v)}
                  className="px-3 py-1.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-[11.5px] font-semibold flex items-center gap-1.5 transition shadow-sm"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>سند / کارت دانش جدید</span>
                </button>
              </div>

              {originFilter !== "IMPORTED" && (
                <div className="flex items-center gap-1.5 flex-wrap pt-1 border-t border-white/[0.06]">
                  {[
                    { id: "ALL", label: `همه کارت‌ها (${cards.length})` },
                    { id: "MEMORY_CARD", label: "کارت دانش" },
                    { id: "LESSON", label: "درس‌آموخته" },
                    { id: "INSTRUCTION", label: "دستورالعمل" },
                    { id: "NOTE", label: "نکته" },
                  ].map((f) => (
                    <button
                      key={f.id}
                      type="button"
                      onClick={() => setCardTypeFilter(f.id)}
                      className={`px-2.5 py-1 rounded-lg text-[11px] font-medium transition border ${
                        cardTypeFilter === f.id
                          ? "bg-emerald-500/15 border-emerald-500/40 text-emerald-200"
                          : "bg-white/[0.02] border-mab-border text-mab-text-secondary hover:text-white"
                      }`}
                    >
                      {f.label}
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* Collapsible New Card Form */}
            {showNewCardForm && (
              <div className="mab-card p-4 space-y-3 border-blue-500/30">
                <div className="flex items-center justify-between">
                  <span className="text-[13px] font-bold text-white flex items-center gap-1.5">
                    <Sparkles className="w-4 h-4 text-blue-400" />
                    <span>ثبت دستی سند دانش / درس‌آموخته / دستورالعمل</span>
                  </span>
                  <button
                    type="button"
                    onClick={() => setShowNewCardForm(false)}
                    className="p-1 rounded-lg text-white/50 hover:text-white"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <MabSelect
                    value={cardType}
                    onChange={setCardType}
                    options={CARD_TYPE_OPTIONS}
                    className="w-full"
                  />
                  <MabSelect
                    value={cardDomain}
                    onChange={setCardDomain}
                    options={domainOptions}
                    className="w-full"
                  />
                </div>

                <input
                  value={cardTitle}
                  onChange={(e) => setCardTitle(e.target.value)}
                  placeholder="عنوان سند یا کارت دانش..."
                  className="w-full mab-input px-3 py-2 text-[12.5px] select-text"
                />

                <textarea
                  value={cardContent}
                  onChange={(e) => setCardContent(e.target.value)}
                  rows={4}
                  placeholder="متن کامل (Markdown پشتیبانی می‌شود)..."
                  className="w-full mab-input px-3 py-2 text-[12.5px] select-text resize-none"
                />

                <div className="flex items-center gap-2 flex-wrap">
                  <input
                    value={cardTags}
                    onChange={(e) => setCardTags(e.target.value)}
                    placeholder="تگ‌ها (با کاما یا فاصله جدا کنید)..."
                    className="flex-1 min-w-[200px] mab-input px-3 py-1.5 text-[12px] select-text"
                  />
                  <button
                    type="button"
                    disabled={
                      !cardTitle.trim() || !cardContent.trim() || isCreatingCard
                    }
                    onClick={() => void handleSubmitCard()}
                    className="px-4 py-1.5 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white text-[12px] font-semibold transition disabled:opacity-40"
                  >
                    {isCreatingCard ? "در حال ثبت..." : "ذخیره"}
                  </button>
                </div>
              </div>
            )}

            {/* SECTION 1: APP & AI CREATED KNOWLEDGE */}
            {originFilter !== "IMPORTED" && (
              <div className="space-y-2.5">
                <div className="flex items-center justify-between gap-2 px-1">
                  <h3 className="text-[13px] font-bold text-emerald-300 flex items-center gap-2">
                    <Sparkles className="w-4 h-4" />
                    <span>داده‌های خلق‌شده توسط اپ و مدل ({createdTotalCount})</span>
                  </h3>
                  <span className="text-[11px] text-white/50">
                    برای مشاهده کامل محتوا و دیاگرام، روی عنوان هر مورد کلیک کنید تا مثل سند باز شود.
                  </span>
                </div>

                {filteredCards.length === 0 && filteredMethods.length === 0 && filteredDecisions.length === 0 ? (
                  <div className="py-8 text-center mab-card space-y-1.5">
                    <BookMarked className="w-7 h-7 text-blue-400/50 mx-auto" />
                    <p className="text-[12.5px] font-semibold text-white">
                      هنوز سند یا کارت دانشی با این فیلتر خلق نشده است
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {/* Vault Cards as Clean Clickable Title Documents */}
                    {filteredCards.map((card) => {
                      const meta = CARD_TYPE_META[card.item_type] || {
                        label: card.item_type,
                        colorClass: "bg-white/[0.08] border-mab-border text-white/75",
                      };
                      const projLabel = card.project_id
                        ? projectNameById.get(card.project_id) || card.project_id
                        : "فراپروژه‌ای";
                      const preview = stripMarkdownPreview(card.content_markdown);

                      return (
                        <div
                          key={card.id}
                          onClick={() => openVaultCardAsDoc(card)}
                          className="mab-card px-4 py-3 flex items-center justify-between gap-3 cursor-pointer hover:bg-white/[0.04] hover:border-blue-400/40 transition group"
                        >
                          <div className="min-w-0 flex-1 space-y-1">
                            <div className="flex items-center gap-2 flex-wrap">
                              <span
                                className={`px-2 py-0.5 rounded-full border text-[10px] font-bold shrink-0 ${meta.colorClass}`}
                              >
                                {meta.label}
                              </span>
                              <h4 className="text-[13.5px] font-bold text-white group-hover:text-blue-300 transition truncate">
                                {card.title}
                              </h4>
                              <span className="px-1.5 py-0.5 rounded bg-white/[0.06] text-[10px] text-white/65 shrink-0">
                                {projLabel}
                              </span>
                            </div>

                            {preview && (
                              <p className="text-[11.5px] text-mab-text-muted truncate">
                                {preview}
                              </p>
                            )}
                          </div>

                          <div className="flex items-center gap-2 shrink-0">
                            <span className="font-timer text-[10px] text-white/45 hidden sm:inline">
                              {formatShortDate(card.created_at)}
                            </span>
                            {onInsertChatPrompt && (
                              <button
                                type="button"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  onInsertChatPrompt(
                                    `برای «${card.title}» (${card.id}) دیاگرام و نقشه فرآیند BPMN 2.0 ترسیم کن.`
                                  );
                                }}
                                title="رسم دیاگرام با مدل"
                                className="px-2 py-1 rounded-lg bg-purple-500/15 hover:bg-purple-500/25 text-purple-200 text-[10.5px] hidden md:inline-flex items-center gap-1"
                              >
                                <Workflow className="w-3 h-3" />
                                <span>دیاگرام</span>
                              </button>
                            )}
                            <span className="px-2 py-1 rounded-lg bg-white/[0.05] group-hover:bg-blue-500/20 text-white/75 group-hover:text-blue-200 text-[11px] inline-flex items-center gap-1">
                              <BookOpen className="w-3.5 h-3.5" />
                              <span>باز کردن سند</span>
                            </span>
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onDeleteVaultCard(card.id);
                              }}
                              title="حذف"
                              className="p-1.5 rounded-lg text-white/35 hover:text-rose-400 hover:bg-rose-500/10 transition"
                            >
                              <Trash2 className="w-3.5 h-3.5" />
                            </button>
                          </div>
                        </div>
                      );
                    })}

                    {/* Methods & Diagrams (BPMN) created for active project */}
                    {filteredMethods.map((m) => (
                      <div
                        key={m.id}
                        onClick={() => openMethodAsDoc(m)}
                        className="mab-card px-4 py-3 flex items-center justify-between gap-3 cursor-pointer hover:bg-white/[0.04] hover:border-purple-400/40 transition group"
                      >
                        <div className="min-w-0 flex-1 space-y-1">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="px-2 py-0.5 rounded-full border bg-purple-500/15 border-purple-500/35 text-purple-200 text-[10px] font-bold shrink-0">
                              دستورالعمل + دیاگرام BPMN
                            </span>
                            <h4 className="text-[13.5px] font-bold text-white group-hover:text-purple-200 transition truncate">
                              {m.title}
                            </h4>
                            <span className="font-mono text-[10px] text-emerald-300 bg-emerald-500/10 px-1.5 py-0.5 rounded">
                              v{m.version} • {m.steps.length} گام
                            </span>
                          </div>
                          {(m.summary || m.objective) && (
                            <p className="text-[11.5px] text-mab-text-muted truncate">
                              {m.summary || m.objective}
                            </p>
                          )}
                        </div>
                        <span className="px-2.5 py-1 rounded-lg bg-purple-500/15 group-hover:bg-purple-500/25 text-purple-200 text-[11px] inline-flex items-center gap-1 shrink-0">
                          <Workflow className="w-3.5 h-3.5" />
                          <span>سند + دیاگرام</span>
                        </span>
                      </div>
                    ))}

                    {/* Architecture Decisions (ADRs) */}
                    {filteredDecisions.map((d) => (
                      <div
                        key={d.id}
                        onClick={() => openDecisionAsDoc(d)}
                        className="mab-card px-4 py-3 flex items-center justify-between gap-3 cursor-pointer hover:bg-white/[0.04] hover:border-cyan-400/40 transition group"
                      >
                        <div className="min-w-0 flex-1 space-y-1">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="px-2 py-0.5 rounded-full border bg-cyan-500/15 border-cyan-500/30 text-cyan-200 text-[10px] font-bold shrink-0">
                              تصمیم معماری (ADR)
                            </span>
                            <h4 className="text-[13.5px] font-bold text-white group-hover:text-cyan-200 transition truncate">
                              {d.title}
                            </h4>
                          </div>
                          {d.definition_of_done && (
                            <p className="text-[11.5px] text-mab-text-muted truncate">
                              {d.definition_of_done}
                            </p>
                          )}
                        </div>
                        <span className="px-2 py-1 rounded-lg bg-white/[0.05] text-white/75 text-[11px] inline-flex items-center gap-1 shrink-0">
                          <ExternalLink className="w-3.5 h-3.5" />
                          <span>باز کردن</span>
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* SECTION 2: DATA IMPORTED / INGESTED INTO THE APP */}
            {originFilter !== "CREATED" && (
              <div className="space-y-2.5 pt-2 border-t border-white/[0.07]">
                <div className="flex items-center justify-between gap-2 px-1">
                  <h3 className="text-[13px] font-bold text-amber-300 flex items-center gap-2">
                    <Download className="w-4 h-4" />
                    <span>داده‌های واردشده به اپ ({importedTotalCount})</span>
                  </h3>
                  <span className="text-[11px] text-white/50">
                    شامل فایل‌ها/متون آپلودی و ورودی‌های همگام‌سازی‌شده
                  </span>
                </div>

                {filteredImportedDocs.length === 0 && filteredRawInputs.length === 0 ? (
                  <div className="py-6 text-center mab-card text-[12px] text-mab-text-muted">
                    داده واردشده‌ای با این فیلتر یافت نشد.
                  </div>
                ) : (
                  <div className="space-y-2">
                    {filteredImportedDocs.map((doc) => (
                      <div
                        key={`doc-${doc.id}`}
                        onClick={() => openImportedDocAsTab(doc)}
                        className="mab-card px-4 py-3 flex items-center justify-between gap-3 cursor-pointer hover:bg-white/[0.04] hover:border-amber-400/40 transition group"
                      >
                        <div className="min-w-0 flex-1 space-y-1">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="px-2 py-0.5 rounded-full border bg-amber-500/15 border-amber-500/30 text-amber-200 text-[10px] font-bold shrink-0">
                              سند واردشده ({doc.doc_type})
                            </span>
                            <h4 className="text-[13.5px] font-bold text-white group-hover:text-amber-200 transition truncate">
                              {doc.title}
                            </h4>
                            <span className="text-[10px] font-mono text-white/50">
                              {doc.original_filename}
                            </span>
                          </div>
                          {doc.executive_summary && (
                            <p className="text-[11.5px] text-mab-text-muted truncate">
                              {doc.executive_summary}
                            </p>
                          )}
                        </div>
                        <div className="flex items-center gap-2 shrink-0">
                          <span className="font-timer text-[10px] text-white/45 hidden sm:inline">
                            {formatShortTime(doc.ingested_at)}
                          </span>
                          <span className="px-2 py-1 rounded-lg bg-white/[0.05] group-hover:bg-amber-500/20 text-white/75 group-hover:text-amber-200 text-[11px] inline-flex items-center gap-1">
                            <FileText className="w-3.5 h-3.5" />
                            <span>مشاهده محتوا</span>
                          </span>
                        </div>
                      </div>
                    ))}

                    {filteredRawInputs.map((r) => (
                      <div
                        key={`raw-${r.id}`}
                        onClick={() => openRawInputAsTab(r)}
                        className="mab-card px-4 py-2.5 flex items-center justify-between gap-3 cursor-pointer hover:bg-white/[0.04] hover:border-white/25 transition group"
                      >
                        <div className="min-w-0 flex-1 space-y-0.5">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="px-2 py-0.5 rounded-full border bg-white/[0.07] border-white/15 text-white/80 text-[10px] font-semibold shrink-0">
                              ورودی خام ({r.source_type})
                            </span>
                            <span className="text-[12.5px] font-semibold text-white truncate group-hover:text-blue-200">
                              {r.snippet}
                            </span>
                          </div>
                        </div>
                        <div className="flex items-center gap-2 shrink-0">
                          <span className="font-timer text-[10px] text-white/45">
                            {formatShortTime(r.created_at)}
                          </span>
                          <span className="px-2 py-1 rounded-lg bg-white/[0.05] text-white/75 text-[11px] inline-flex items-center gap-1">
                            <FileText className="w-3.5 h-3.5" />
                            <span>متن کامل</span>
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* Active Learned Antipatterns (Guardrails) */}
            {dash.active_guardrails.length > 0 && (
              <div className="mab-card p-4 space-y-3 border-rose-500/20 bg-rose-500/[0.03]">
                <div className="flex items-center gap-2 text-rose-400 font-bold text-[13px]">
                  <ShieldAlert className="w-4 h-4" />
                  <span>
                    خطاهای تکرارشونده و سدهای پیشگیرانه ثبت‌شده (
                    {dash.active_guardrails.length})
                  </span>
                </div>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5">
                  {dash.active_guardrails.map((g) => (
                    <div
                      key={g.id}
                      className="p-3 rounded-xl bg-mab-surface border border-mab-border space-y-1"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="px-1.5 py-0.5 rounded bg-rose-500/15 text-rose-300 font-mono text-[10px]">
                          {g.trigger_category}
                        </span>
                        <span className="font-timer text-[10px] text-white/50">
                          شدت {g.severity_score}/10
                        </span>
                      </div>
                      <p className="text-[12.5px] font-semibold text-white">
                        {g.preventive_gate_question}
                      </p>
                      <p className="text-[11.5px] text-mab-text-muted">
                        الگوی خطا: {g.mistake_pattern}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {/* ================= TAB 2: 8 CADM LIFE DOMAINS & OUTCOMES ================= */}
        {activeTab === "DOMAINS_OUTCOMES" && (
          <div className="space-y-5">
            {/* Outcomes Header + List */}
            <div className="space-y-3">
              <div className="flex items-center justify-between gap-2 flex-wrap">
                <h3 className="text-[14px] font-bold text-white flex items-center gap-2">
                  <Target className="w-4 h-4 text-amber-400" />
                  <span>دستاوردها و اهداف نهایی (Outcomes)</span>
                </h3>
                <button
                  type="button"
                  onClick={onOpenNewOutcome}
                  className="px-3.5 py-1.5 rounded-xl bg-amber-600 hover:bg-amber-500 text-white text-[12px] font-semibold flex items-center gap-1.5 transition shadow-sm"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>Outcome جدید</span>
                </button>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {outcomes.map((o) => {
                  const dom = domains.find((d) => d.code === o.domain_code);
                  const linkedLines = (dash.work_lines ?? []).filter(
                    (w) => w.outcome_id === o.id
                  ).length;

                  return (
                    <div
                      key={o.id}
                      className="mab-card p-4 space-y-2.5 border-amber-500/20 hover:border-amber-500/40 transition"
                    >
                      <div className="flex items-center justify-between gap-2 flex-wrap">
                        <div className="flex items-center gap-1.5 flex-wrap">
                          <span className="px-2 py-0.5 rounded-md bg-amber-500/15 border border-amber-500/30 text-amber-300 font-mono text-[10.5px] font-bold">
                            {o.id}
                          </span>
                          <span className="px-2 py-0.5 rounded-full bg-white/[0.06] text-white/75 text-[10.5px]">
                            {dom?.title || o.domain_code}
                          </span>
                          {o.time_window && (
                            <span className="text-[10.5px] text-mab-text-muted">
                              • {o.time_window}
                            </span>
                          )}
                        </div>

                        <div className="flex items-center gap-1">
                          <button
                            type="button"
                            onClick={() => onEditOutcome(o)}
                            className="p-1.5 rounded-lg bg-white/[0.04] hover:bg-white/[0.09] text-white/70 hover:text-white transition"
                            title="ویرایش Outcome"
                          >
                            <Edit3 className="w-3.5 h-3.5" />
                          </button>
                          <button
                            type="button"
                            onClick={() => onDeleteOutcome(o.id)}
                            className="p-1.5 rounded-lg bg-white/[0.04] hover:bg-rose-500/15 text-white/50 hover:text-rose-400 transition"
                            title="حذف Outcome"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>

                      <h4 className="text-[14px] font-bold text-white">
                        {o.title}
                      </h4>

                      {o.desired_reality_change && (
                        <p className="text-[12px] text-mab-text-secondary leading-relaxed">
                          {o.desired_reality_change}
                        </p>
                      )}

                      {o.evidence_kr && (
                        <div className="text-[11.5px] text-emerald-300/90 bg-emerald-500/[0.06] border border-emerald-500/20 rounded-lg px-2.5 py-1.5">
                          سنجه تحقق (KR): {o.evidence_kr}
                        </div>
                      )}

                      <div className="space-y-1 pt-1">
                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-mab-text-muted">
                            {linkedLines} لاین کاری متصل در این پروژه
                          </span>
                          <span className="font-timer font-bold text-emerald-400">
                            {o.progress_percent}%
                          </span>
                        </div>
                        <div className="w-full h-1.5 rounded-full bg-white/[0.08] overflow-hidden">
                          <div
                            className="h-full rounded-full bg-emerald-500 transition-all"
                            style={{ width: `${o.progress_percent}%` }}
                          />
                        </div>
                      </div>
                    </div>
                  );
                })}

                {outcomes.length === 0 && (
                  <div className="col-span-full py-10 text-center mab-card text-[12.5px] text-mab-text-muted">
                    هنوز Outcome‌ای ثبت نشده است. با دکمه «Outcome جدید» اولین دستاورد هدف‌مند خود را تعریف کنید.
                  </div>
                )}
              </div>
            </div>

            {/* 8 CADM Domains Grid */}
            <div className="space-y-3">
              <h3 className="text-[14px] font-bold text-white flex items-center gap-2">
                <Compass className="w-4 h-4 text-blue-400" />
                <span>۸ بُعد پایدار زندگی و کار (CADM Domains)</span>
              </h3>

              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
                {domains.map((d) => (
                  <div
                    key={d.code}
                    className={`mab-card p-3.5 space-y-2 ${
                      d.is_growth_focus
                        ? "border-blue-500/40 bg-blue-500/[0.05]"
                        : ""
                    }`}
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-[13px] font-bold text-white truncate">
                        {d.title}
                      </span>
                      {d.is_growth_focus && (
                        <span className="px-1.5 py-0.5 rounded-full bg-blue-500/20 text-blue-300 text-[9.5px] font-bold shrink-0">
                          تمرکز رشد
                        </span>
                      )}
                    </div>
                    <div className="font-mono text-[10px] text-white/40">
                      {d.code}
                    </div>
                    <div className="flex items-center gap-1 pt-1">
                      {[1, 2, 3, 4, 5].map((lvl) => (
                        <span
                          key={lvl}
                          className={`h-1.5 flex-1 rounded-full ${
                            lvl <= d.maturity_state
                              ? "bg-blue-500"
                              : "bg-white/[0.08]"
                          }`}
                        />
                      ))}
                      <span className="font-timer text-[10px] text-white/60 mr-1">
                        {d.maturity_state}/5
                      </span>
                    </div>
                    {d.maintenance_floor && (
                      <p className="text-[11px] text-mab-text-muted line-clamp-2">
                        کف نگهداری: {d.maintenance_floor}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* ================= TAB 3: PENDING PROJECT LIFECYCLE PROPOSALS ================= */}
        {activeTab === "PROPOSALS" && (
          <div className="space-y-3">
            {proposals.map((prop) => (
              <div
                key={prop.id}
                className="mab-card p-4 space-y-2.5 flex flex-col sm:flex-row sm:items-center justify-between gap-3"
              >
                <div className="space-y-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-mono text-[11px] text-purple-300 font-bold">
                      [{prop.id}]
                    </span>
                    <span className="px-2 py-0.5 rounded bg-purple-500/15 text-purple-300 font-mono text-[10px]">
                      {prop.proposal_type}
                    </span>
                    <span
                      className={`px-2 py-0.5 rounded-full text-[10px] font-bold ${
                        prop.status === "APPROVED"
                          ? "bg-emerald-500/15 text-emerald-300"
                          : prop.status === "REJECTED"
                          ? "bg-rose-500/15 text-rose-300"
                          : "bg-amber-500/15 text-amber-300"
                      }`}
                    >
                      {prop.status}
                    </span>
                  </div>
                  <h4 className="text-[13.5px] font-bold text-white">
                    {prop.title}
                  </h4>
                  <p className="text-[12px] text-mab-text-secondary leading-relaxed">
                    {prop.rationale_markdown}
                  </p>
                </div>

                {prop.status === "PENDING" && (
                  <div className="flex items-center gap-2 shrink-0">
                    <button
                      type="button"
                      onClick={() => onResolveProposal(prop.id, true)}
                      className="px-3 py-1.5 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white text-[11.5px] font-semibold flex items-center gap-1 transition"
                    >
                      <Check className="w-3.5 h-3.5" />
                      <span>تایید</span>
                    </button>
                    <button
                      type="button"
                      onClick={() => onResolveProposal(prop.id, false)}
                      className="px-3 py-1.5 rounded-xl bg-rose-600/20 hover:bg-rose-600/35 border border-rose-500/35 text-rose-300 text-[11.5px] font-semibold flex items-center gap-1 transition"
                    >
                      <X className="w-3.5 h-3.5" />
                      <span>رد</span>
                    </button>
                  </div>
                )}
              </div>
            ))}

            {proposals.length === 0 && (
              <div className="py-12 text-center mab-card text-[12.5px] text-mab-text-muted">
                هیچ پیشنهاد در انتظار تاییدی برای ساخت/تعلیق/تغییر زیرپروژه ثبت نشده است.
              </div>
            )}
          </div>
        )}

        {/* ================= TAB 4: DAILY CAPACITY CALENDAR ================= */}
        {activeTab === "CALENDAR" && (
          <div className="space-y-4">
            <div className="mab-card p-4 space-y-3">
              <div className="text-[13px] font-bold text-white flex items-center gap-2">
                <Calendar className="w-4 h-4 text-emerald-400" />
                <span>ثبت ظرفیت کاری روزانه</span>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
                <div className="space-y-1">
                  <label className="text-[11px] text-mab-text-muted">تاریخ:</label>
                  <input
                    type="date"
                    value={calDate}
                    onChange={(e) => setCalDate(e.target.value)}
                    className="w-full mab-input px-3 py-1.5 text-[12px] font-mono bidi-ltr"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-[11px] text-mab-text-muted">
                    ظرفیت کل (دقیقه):
                  </label>
                  <input
                    type="number"
                    min={0}
                    max={960}
                    value={calTotalMin}
                    onChange={(e) => setCalTotalMin(Number(e.target.value) || 0)}
                    className="w-full mab-input px-3 py-1.5 text-[12px] font-mono"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-[11px] text-mab-text-muted">
                    ظرفیت کار عمیق (دقیقه):
                  </label>
                  <input
                    type="number"
                    min={0}
                    max={720}
                    value={calDeepMin}
                    onChange={(e) => setCalDeepMin(Number(e.target.value) || 0)}
                    className="w-full mab-input px-3 py-1.5 text-[12px] font-mono"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-[11px] text-mab-text-muted">
                    یادداشت روز:
                  </label>
                  <input
                    value={calNote}
                    onChange={(e) => setCalNote(e.target.value)}
                    placeholder="تمرکز اصلی امروز..."
                    className="w-full mab-input px-3 py-1.5 text-[12px]"
                  />
                </div>
              </div>

              <div className="flex items-center justify-between gap-2 flex-wrap">
                <label className="flex items-center gap-2 text-[12px] text-white/75 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={calIsOff}
                    onChange={(e) => setCalIsOff(e.target.checked)}
                    className="rounded accent-emerald-500"
                  />
                  <span>روز استراحت / تعطیل</span>
                </label>

                <button
                  type="button"
                  onClick={() => void handleSaveCalendarDay()}
                  className="px-4 py-1.5 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white text-[12px] font-semibold flex items-center gap-1.5 transition"
                >
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  <span>ذخیره ظرفیت روز</span>
                </button>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
              {calendarDays.map((day) => {
                const pct =
                  day.total_capacity_minutes > 0
                    ? Math.min(
                        100,
                        Math.round(
                          (day.assigned_minutes / day.total_capacity_minutes) *
                            100
                        )
                      )
                    : 0;
                return (
                  <div
                    key={day.date_key}
                    className="mab-card p-3.5 space-y-2"
                  >
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-mono text-[13px] font-bold text-white bidi-ltr">
                        {day.date_key}
                      </span>
                      {day.is_off_day ? (
                        <span className="px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-300 text-[10px] font-bold">
                          تعطیل
                        </span>
                      ) : (
                        <span className="font-timer text-[11px] text-emerald-300">
                          {day.assigned_minutes}/{day.total_capacity_minutes} دقیقه
                        </span>
                      )}
                    </div>
                    <div className="w-full h-1.5 rounded-full bg-white/[0.08] overflow-hidden">
                      <div
                        className={`h-full rounded-full ${
                          pct > 95 ? "bg-rose-500" : "bg-emerald-500"
                        }`}
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                    <div className="flex items-center justify-between text-[11px] text-mab-text-muted">
                      <span>{day.assigned_tasks_count} تسک زمان‌بندی‌شده</span>
                      <span>کار عمیق: {day.deep_capacity_minutes}m</span>
                    </div>
                    {day.day_note && (
                      <p className="text-[11.5px] text-mab-text-secondary">
                        {day.day_note}
                      </p>
                    )}
                  </div>
                );
              })}

              {calendarDays.length === 0 && (
                <div className="col-span-full py-10 text-center mab-card text-[12.5px] text-mab-text-muted">
                  هنوز ظرفیت روزانه‌ای ثبت نشده است.
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default EcosystemAndCardsView;
