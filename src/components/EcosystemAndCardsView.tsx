import { useMemo, useState } from "react";
import {
  BookMarked,
  Brain,
  Calendar,
  Check,
  CheckCircle2,
  Compass,
  Edit3,
  Eye,
  EyeOff,
  GitPullRequest,
  Plus,
  Search,
  ShieldAlert,
  Sparkles,
  Target,
  Trash2,
  X,
} from "lucide-react";
import { DashboardState, Outcome } from "../lib/tauri_api";
import { formatShortDate } from "../lib/formatters";
import { MabSelect } from "./ui/MabSelect";

type EcosystemTab = "CARDS" | "DOMAINS_OUTCOMES" | "PROPOSALS" | "CALENDAR";

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
}

const CARD_TYPE_META: Record<
  string,
  { label: string; colorClass: string }
> = {
  MEMORY_CARD: {
    label: "کارت حافظه",
    colorClass: "bg-blue-500/15 border-blue-500/30 text-blue-300",
  },
  LESSON: {
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
  INTEREST: {
    label: "علاقه / مطالعه",
    colorClass: "bg-pink-500/15 border-pink-500/30 text-pink-300",
  },
};

const CARD_TYPE_OPTIONS = [
  { value: "MEMORY_CARD", label: "کارت حافظه / دانش اتمیک" },
  { value: "LESSON", label: "درس‌آموخته (Lesson Learned)" },
  { value: "INSTRUCTION", label: "دستورالعمل و روش اجرا" },
  { value: "NOTE", label: "نکته کلیدی و یادداشت" },
];

export function EcosystemAndCardsView({
  dash,
  onOpenNewOutcome,
  onEditOutcome,
  onDeleteOutcome,
  onCreateVaultCard,
  onDeleteVaultCard,
  onResolveProposal,
  onUpsertCalendarDay,
}: EcosystemAndCardsViewProps) {
  const [activeTab, setActiveTab] = useState<EcosystemTab>("CARDS");
  const [searchQuery, setSearchQuery] = useState("");
  const [cardTypeFilter, setCardTypeFilter] = useState<string>("ALL");
  const [studyMode, setStudyMode] = useState(false);
  const [revealedCards, setRevealedCards] = useState<Set<string>>(
    () => new Set()
  );

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
  const domains = dash.domains ?? [];
  const outcomes = dash.outcomes ?? [];
  const proposals = dash.pending_proposals ?? [];
  const calendarDays = dash.calendar_days ?? [];

  const pendingProposalsCount = proposals.filter(
    (p) => p.status === "PENDING"
  ).length;

  const normalizedQuery = searchQuery.trim().toLowerCase();

  const filteredCards = useMemo(() => {
    return cards.filter((c) => {
      if (cardTypeFilter !== "ALL" && c.item_type !== cardTypeFilter) {
        return false;
      }
      if (!normalizedQuery) return true;
      return (
        c.title.toLowerCase().includes(normalizedQuery) ||
        c.content_markdown.toLowerCase().includes(normalizedQuery) ||
        c.tags.some((t) => t.toLowerCase().includes(normalizedQuery))
      );
    });
  }, [cards, cardTypeFilter, normalizedQuery]);

  const toggleRevealCard = (id: string) => {
    setRevealedCards((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
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
        {/* ================= TAB 1: ATOMIC KNOWLEDGE & MEMORY CARDS ================= */}
        {activeTab === "CARDS" && (
          <div className="space-y-4">
            {/* Filter & Action Toolbar */}
            <div className="mab-card p-3 flex items-center justify-between gap-2.5 flex-wrap">
              <div className="flex items-center gap-1.5 flex-wrap">
                {[
                  { id: "ALL", label: `همه (${cards.length})` },
                  { id: "MEMORY_CARD", label: "کارت حافظه" },
                  { id: "LESSON", label: "درس‌آموخته" },
                  { id: "INSTRUCTION", label: "دستورالعمل" },
                  { id: "NOTE", label: "نکته" },
                ].map((f) => (
                  <button
                    key={f.id}
                    type="button"
                    onClick={() => setCardTypeFilter(f.id)}
                    className={`px-2.5 py-1 rounded-lg text-[11.5px] font-medium transition border ${
                      cardTypeFilter === f.id
                        ? "bg-blue-500/20 border-blue-500/40 text-blue-300"
                        : "bg-white/[0.03] border-mab-border text-mab-text-secondary hover:text-white"
                    }`}
                  >
                    {f.label}
                  </button>
                ))}
              </div>

              <div className="flex items-center gap-2 flex-wrap">
                <button
                  type="button"
                  onClick={() => {
                    setStudyMode((v) => !v);
                    setRevealedCards(new Set());
                  }}
                  className={`px-3 py-1.5 rounded-xl border text-[11.5px] font-semibold flex items-center gap-1.5 transition ${
                    studyMode
                      ? "bg-purple-500/20 border-purple-500/40 text-purple-300"
                      : "bg-white/[0.04] border-mab-border text-white/75 hover:text-white"
                  }`}
                >
                  {studyMode ? (
                    <EyeOff className="w-3.5 h-3.5" />
                  ) : (
                    <Eye className="w-3.5 h-3.5" />
                  )}
                  <span>
                    {studyMode ? "خروج از حالت مرور حافظه" : "حالت مرور کارت حافظه"}
                  </span>
                </button>

                <button
                  type="button"
                  onClick={() => setShowNewCardForm((v) => !v)}
                  className="px-3 py-1.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-[11.5px] font-semibold flex items-center gap-1.5 transition shadow-sm"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>کارت دانش جدید</span>
                </button>
              </div>
            </div>

            {/* Collapsible New Card Form */}
            {showNewCardForm && (
              <div className="mab-card p-4 space-y-3 border-blue-500/30">
                <div className="flex items-center justify-between">
                  <span className="text-[13px] font-bold text-white flex items-center gap-1.5">
                    <Sparkles className="w-4 h-4 text-blue-400" />
                    <span>ثبت دستی کارت دانش / حافظه / درس‌آموخته</span>
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
                  placeholder="عنوان کارت یا سوال روی کارت حافظه..."
                  className="w-full mab-input px-3 py-2 text-[12.5px] select-text"
                />

                <textarea
                  value={cardContent}
                  onChange={(e) => setCardContent(e.target.value)}
                  rows={3}
                  placeholder="پاسخ، توضیح اتمیک، درس‌آموخته یا گام‌های دستورالعمل..."
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
                    {isCreatingCard ? "در حال ثبت..." : "ذخیره کارت"}
                  </button>
                </div>
              </div>
            )}

            {/* Cards Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
              {filteredCards.map((card) => {
                const meta = CARD_TYPE_META[card.item_type] || {
                  label: card.item_type,
                  colorClass: "bg-white/[0.08] border-mab-border text-white/75",
                };
                const isHiddenInStudy =
                  studyMode && !revealedCards.has(card.id);

                return (
                  <div
                    key={card.id}
                    className="mab-card p-4 flex flex-col justify-between gap-3 hover:border-white/[0.15] transition"
                  >
                    <div className="space-y-2">
                      <div className="flex items-center justify-between gap-2">
                        <div className="flex items-center gap-1.5 flex-wrap">
                          <span
                            className={`px-2 py-0.5 rounded-full border text-[10px] font-bold ${meta.colorClass}`}
                          >
                            {meta.label}
                          </span>
                          <span className="font-mono text-[10px] text-white/45">
                            {card.id}
                          </span>
                        </div>
                        <button
                          type="button"
                          onClick={() => onDeleteVaultCard(card.id)}
                          title="حذف کارت"
                          className="p-1 rounded-md text-white/35 hover:text-rose-400 hover:bg-rose-500/10 transition"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>

                      <h4 className="text-[13.5px] font-bold text-white leading-snug select-text">
                        {card.title}
                      </h4>

                      {isHiddenInStudy ? (
                        <button
                          type="button"
                          onClick={() => toggleRevealCard(card.id)}
                          className="w-full py-5 rounded-xl border border-dashed border-purple-500/35 bg-purple-500/[0.06] hover:bg-purple-500/[0.12] text-purple-300 text-[12px] font-semibold transition flex items-center justify-center gap-2"
                        >
                          <Eye className="w-4 h-4" />
                          <span>کلیک برای نمایش پاسخ کارت حافظه</span>
                        </button>
                      ) : (
                        <p
                          onClick={() =>
                            studyMode ? toggleRevealCard(card.id) : undefined
                          }
                          className={`text-[12.5px] text-mab-text-secondary leading-relaxed whitespace-pre-wrap select-text ${
                            studyMode ? "cursor-pointer" : ""
                          }`}
                        >
                          {card.content_markdown}
                        </p>
                      )}
                    </div>

                    <div className="flex items-center justify-between gap-2 pt-2 border-t border-white/[0.05] flex-wrap">
                      <div className="flex items-center gap-1 flex-wrap">
                        {card.tags.map((tg) => (
                          <span
                            key={tg}
                            className="px-1.5 py-0.5 rounded bg-white/[0.05] text-[10px] text-white/60"
                          >
                            #{tg}
                          </span>
                        ))}
                      </div>
                      <span className="font-timer text-[10px] text-mab-text-muted">
                        {formatShortDate(card.created_at)}
                      </span>
                    </div>
                  </div>
                );
              })}

              {filteredCards.length === 0 && (
                <div className="col-span-full py-12 text-center mab-card space-y-2">
                  <BookMarked className="w-8 h-8 text-blue-400/50 mx-auto" />
                  <p className="text-[13px] font-semibold text-white">
                    هنوز کارت دانش یا درس‌آموخته‌ای ثبت نشده است
                  </p>
                  <p className="text-[12px] text-mab-text-muted">
                    با پردازش اسناد توسط هوش مصنوعی یا کلیک روی «کارت دانش جدید»، کارت‌های حافظه و درس‌آموخته‌ها اینجا جمع می‌شوند.
                  </p>
                </div>
              )}
            </div>

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
