import { useMemo, useState } from "react";
import {
  Calendar,
  CalendarDays,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Clock,
  Link2,
  MapPin,
  MessageSquare,
  Play,
  Plus,
  Sparkles,
  Trash2,
  Unlink,
  X,
} from "lucide-react";
import { CalendarDay, CalendarEvent, DashboardState, WorkEntity } from "../lib/tauri_api";

interface CalendarWorkspaceViewProps {
  dash: DashboardState;
  onQuickComplete: (taskId: string, pomosSpent: number) => void;
  onStartTask: (taskId: string) => void;
  onUpsertCalendarDay: (params: {
    dateKey: string;
    totalCapacityMinutes: number;
    deepCapacityMinutes: number;
    dayNote: string;
    isOffDay: boolean;
  }) => Promise<void>;
  onUpsertCalendarEvent: (params: {
    eventId?: string;
    projectId?: string;
    title: string;
    eventType: string;
    eventDate: string;
    startTime?: string;
    endTime?: string;
    locationOrLink?: string;
    notes?: string;
    status?: string;
    colorHex?: string;
  }) => Promise<void>;
  onDeleteCalendarEvent: (eventId: string) => Promise<void>;
  onLinkTasksToCalendarEvent: (params: {
    eventId: string;
    taskIds: string[];
    syncDateToEvent?: boolean;
  }) => Promise<void>;
  onUnlinkTaskFromCalendarEvent: (taskId: string) => Promise<void>;
  onCreateTaskForEvent: (params: {
    projectId: string;
    title: string;
    category: string;
    durationMinutes: number;
    scheduledDate: string;
    calendarEventId: string;
  }) => Promise<void>;
  onInsertChatPrompt: (text: string) => void;
}

const EVENT_TYPE_META: Record<string, { label: string; badgeClass: string; defaultColor: string }> = {
  MEETING: {
    label: "جلسه",
    badgeClass: "bg-indigo-500/20 text-indigo-200 border-indigo-400/35",
    defaultColor: "#6366f1",
  },
  DEADLINE: {
    label: "ددلاین",
    badgeClass: "bg-rose-500/20 text-rose-200 border-rose-400/35",
    defaultColor: "#f43f5e",
  },
  MILESTONE: {
    label: "مایلستون",
    badgeClass: "bg-emerald-500/20 text-emerald-200 border-emerald-400/35",
    defaultColor: "#10b981",
  },
  FOCUS_BLOCK: {
    label: "بلوک تمرکز",
    badgeClass: "bg-amber-500/20 text-amber-200 border-amber-400/35",
    defaultColor: "#f59e0b",
  },
  REMINDER: {
    label: "یادآور",
    badgeClass: "bg-cyan-500/20 text-cyan-200 border-cyan-400/35",
    defaultColor: "#06b6d4",
  },
};

function toIsoDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

function formatPersianDateLabel(isoDate: string): string {
  try {
    const dt = new Date(`${isoDate}T12:00:00`);
    if (Number.isNaN(dt.getTime())) return isoDate;
    return new Intl.DateTimeFormat("fa-IR-u-ca-persian", {
      weekday: "short",
      month: "long",
      day: "numeric",
    }).format(dt);
  } catch {
    return isoDate;
  }
}

function formatPersianDayNumber(isoDate: string): string {
  try {
    const dt = new Date(`${isoDate}T12:00:00`);
    if (Number.isNaN(dt.getTime())) return isoDate.slice(-2);
    return new Intl.DateTimeFormat("fa-IR-u-ca-persian", {
      day: "numeric",
    }).format(dt);
  } catch {
    return isoDate.slice(-2);
  }
}

function formatPersianMonthTitle(year: number, monthIndex: number): string {
  try {
    const mid = new Date(year, monthIndex, 15, 12, 0, 0);
    const jalali = new Intl.DateTimeFormat("fa-IR-u-ca-persian", {
      month: "long",
      year: "numeric",
    }).format(mid);
    const greg = new Intl.DateTimeFormat("en-US", {
      month: "short",
      year: "numeric",
    }).format(new Date(year, monthIndex, 1, 12, 0, 0));
    return `${jalali} • ${greg}`;
  } catch {
    return `${year}-${String(monthIndex + 1).padStart(2, "0")}`;
  }
}

export function CalendarWorkspaceView({
  dash,
  onQuickComplete,
  onStartTask,
  onUpsertCalendarDay,
  onUpsertCalendarEvent,
  onDeleteCalendarEvent,
  onLinkTasksToCalendarEvent,
  onUnlinkTaskFromCalendarEvent,
  onCreateTaskForEvent,
  onInsertChatPrompt,
}: CalendarWorkspaceViewProps) {
  const todayIso = useMemo(() => toIsoDate(new Date()), []);
  const [selectedDate, setSelectedDate] = useState<string>(todayIso);
  const [monthCursor, setMonthCursor] = useState<{ year: number; month: number }>(() => {
    const now = new Date();
    return { year: now.getFullYear(), month: now.getMonth() };
  });
  const [filterProjectId, setFilterProjectId] = useState<string>("ALL");

  // Create / Edit event modal state
  const [showEventForm, setShowEventForm] = useState(false);
  const [editingEventId, setEditingEventId] = useState<string | undefined>(undefined);
  const [evtTitle, setEvtTitle] = useState("");
  const [evtType, setEvtType] = useState("MEETING");
  const [evtDate, setEvtDate] = useState(todayIso);
  const [evtStartTime, setEvtStartTime] = useState("10:00");
  const [evtEndTime, setEvtEndTime] = useState("11:30");
  const [evtProjectId, setEvtProjectId] = useState<string>("ALL");
  const [evtLocation, setEvtLocation] = useState("");
  const [evtNotes, setEvtNotes] = useState("");
  const [evtStatus, setEvtStatus] = useState("UPCOMING");
  const [isSavingEvent, setIsSavingEvent] = useState(false);

  // Task linker drawer state (for any selected event)
  const [linkingEventId, setLinkingEventId] = useState<string | null>(null);
  const [linkSearch, setLinkSearch] = useState("");
  const [selectedTaskIdsToLink, setSelectedTaskIdsToLink] = useState<Record<string, boolean>>({});
  const [syncDatesOnLink, setSyncDatesOnLink] = useState(true);
  const [isLinkingTasks, setIsLinkingTasks] = useState(false);

  // Quick-add new task directly under an event
  const [quickTaskForEventId, setQuickTaskForEventId] = useState<string | null>(null);
  const [quickTaskTitle, setQuickTaskTitle] = useState("");
  const [quickTaskProjectId, setQuickTaskProjectId] = useState<string>(dash.active_project.id);
  const [quickTaskMinutes, setQuickTaskMinutes] = useState<number>(30);
  const [isCreatingQuickTask, setIsCreatingQuickTask] = useState(false);

  // Day capacity editor
  const currentDayRecord: CalendarDay | undefined = useMemo(
    () => (dash.calendar_days || []).find((d) => d.date_key === selectedDate),
    [dash.calendar_days, selectedDate]
  );
  const [editingDayCap, setEditingDayCap] = useState(false);
  const [capTotal, setCapTotal] = useState(300);
  const [capDeep, setCapDeep] = useState(150);
  const [capNote, setCapNote] = useState("");
  const [capOff, setCapOff] = useState(false);

  // Build map of all candidate tasks across projects (scheduled + current project's open tasks)
  const allKnownTasks = useMemo(() => {
    const map = new Map<string, WorkEntity>();
    for (const t of dash.open_tasks || []) map.set(t.id, t);
    for (const t of dash.horizon_tasks || []) map.set(t.id, t);
    if (dash.next_step_task) map.set(dash.next_step_task.id, dash.next_step_task);
    for (const t of dash.calendar_scheduled_tasks || []) map.set(t.id, t);
    for (const ev of dash.calendar_events || []) {
      for (const lt of ev.linked_tasks || []) map.set(lt.id, lt);
    }
    return Array.from(map.values());
  }, [dash]);

  const projectNameById = useMemo(() => {
    const m = new Map<string, string>();
    for (const p of dash.projects) m.set(p.id, p.name);
    return m;
  }, [dash.projects]);

  const filteredEvents = useMemo(() => {
    const list = dash.calendar_events || [];
    if (filterProjectId === "ALL") return list;
    return list.filter((ev) => ev.project_id === filterProjectId || ev.project_id === "ALL");
  }, [dash.calendar_events, filterProjectId]);

  const eventsByDate = useMemo(() => {
    const map = new Map<string, CalendarEvent[]>();
    for (const ev of filteredEvents) {
      const arr = map.get(ev.event_date) ?? [];
      arr.push(ev);
      map.set(ev.event_date, arr);
    }
    return map;
  }, [filteredEvents]);

  const scheduledTasksByDate = useMemo(() => {
    const map = new Map<string, WorkEntity[]>();
    for (const t of dash.calendar_scheduled_tasks || []) {
      if (!t.scheduled_date) continue;
      if (filterProjectId !== "ALL" && t.project_id !== filterProjectId) continue;
      const arr = map.get(t.scheduled_date) ?? [];
      arr.push(t);
      map.set(t.scheduled_date, arr);
    }
    return map;
  }, [dash.calendar_scheduled_tasks, filterProjectId]);

  // Month grid cells (Saturday-first Persian week: شنبه = 0 ... جمعه = 6)
  const monthCells = useMemo(() => {
    const first = new Date(monthCursor.year, monthCursor.month, 1, 12, 0, 0);
    const daysInMonth = new Date(monthCursor.year, monthCursor.month + 1, 0, 12, 0, 0).getDate();
    // JS getDay(): 0=Sun..6=Sat -> Persian offset (Sat=0, Sun=1, Mon=2, Tue=3, Wed=4, Thu=5, Fri=6)
    const offset = (first.getDay() + 1) % 7;
    const cells: Array<{ iso: string | null; dayNum: number | null }> = [];
    for (let i = 0; i < offset; i++) cells.push({ iso: null, dayNum: null });
    for (let d = 1; d <= daysInMonth; d++) {
      const iso = toIsoDate(new Date(monthCursor.year, monthCursor.month, d, 12, 0, 0));
      cells.push({ iso, dayNum: d });
    }
    while (cells.length % 7 !== 0) cells.push({ iso: null, dayNum: null });
    return cells;
  }, [monthCursor]);

  const selectedDayEvents = eventsByDate.get(selectedDate) ?? [];
  const selectedDayTasks = scheduledTasksByDate.get(selectedDate) ?? [];
  const selectedDayBookedMinutes = selectedDayTasks
    .filter((t) => t.status !== "CLOSED")
    .reduce((acc, t) => acc + (t.duration_minutes || 25), 0);

  const upcomingEvents = useMemo(
    () =>
      filteredEvents
        .filter((ev) => ev.status === "UPCOMING" || ev.event_date >= todayIso)
        .slice(0, 16),
    [filteredEvents, todayIso]
  );

  const openNewEventForm = (dateOverride?: string) => {
    setEditingEventId(undefined);
    setEvtTitle("");
    setEvtType("MEETING");
    setEvtDate(dateOverride || selectedDate || todayIso);
    setEvtStartTime("10:00");
    setEvtEndTime("11:30");
    setEvtProjectId(filterProjectId !== "ALL" ? filterProjectId : dash.active_project.id);
    setEvtLocation("");
    setEvtNotes("");
    setEvtStatus("UPCOMING");
    setShowEventForm(true);
  };

  const openEditEventForm = (ev: CalendarEvent) => {
    setEditingEventId(ev.id);
    setEvtTitle(ev.title);
    setEvtType(ev.event_type || "MEETING");
    setEvtDate(ev.event_date);
    setEvtStartTime(ev.start_time || "");
    setEvtEndTime(ev.end_time || "");
    setEvtProjectId(ev.project_id || "ALL");
    setEvtLocation(ev.location_or_link || "");
    setEvtNotes(ev.notes || "");
    setEvtStatus(ev.status || "UPCOMING");
    setShowEventForm(true);
  };

  const handleSaveEvent = async () => {
    if (!evtTitle.trim() || !evtDate.trim()) return;
    setIsSavingEvent(true);
    try {
      await onUpsertCalendarEvent({
        eventId: editingEventId,
        projectId: evtProjectId,
        title: evtTitle.trim(),
        eventType: evtType,
        eventDate: evtDate.trim(),
        startTime: evtStartTime.trim() || undefined,
        endTime: evtEndTime.trim() || undefined,
        locationOrLink: evtLocation.trim() || undefined,
        notes: evtNotes.trim() || undefined,
        status: evtStatus,
        colorHex: EVENT_TYPE_META[evtType]?.defaultColor,
      });
      setSelectedDate(evtDate.trim());
      setShowEventForm(false);
    } finally {
      setIsSavingEvent(false);
    }
  };

  const openDayCapEditor = () => {
    setCapTotal(currentDayRecord?.total_capacity_minutes ?? 300);
    setCapDeep(currentDayRecord?.deep_capacity_minutes ?? 150);
    setCapNote(currentDayRecord?.day_note ?? "");
    setCapOff(currentDayRecord?.is_off_day ?? false);
    setEditingDayCap(true);
  };

  const handleSaveDayCap = async () => {
    await onUpsertCalendarDay({
      dateKey: selectedDate,
      totalCapacityMinutes: Number(capTotal) || 300,
      deepCapacityMinutes: Number(capDeep) || 150,
      dayNote: capNote,
      isOffDay: capOff,
    });
    setEditingDayCap(false);
  };

  const openTaskLinker = (ev: CalendarEvent) => {
    setLinkingEventId(ev.id);
    setLinkSearch("");
    setSelectedTaskIdsToLink({});
    setSyncDatesOnLink(true);
  };

  const linkingEvent = useMemo(
    () => (dash.calendar_events || []).find((e) => e.id === linkingEventId) ?? null,
    [dash.calendar_events, linkingEventId]
  );

  const candidateTasksForLinking = useMemo(() => {
    if (!linkingEvent) return [];
    const alreadyIds = new Set((linkingEvent.linked_tasks || []).map((t) => t.id));
    const q = linkSearch.trim().toLowerCase();
    return allKnownTasks
      .filter((t) => !alreadyIds.has(t.id) && t.status !== "SUPERSEDED")
      .filter((t) => {
        if (!q) return true;
        return (
          t.title.toLowerCase().includes(q) ||
          t.id.toLowerCase().includes(q) ||
          (projectNameById.get(t.project_id) || "").toLowerCase().includes(q)
        );
      })
      .slice(0, 60);
  }, [allKnownTasks, linkingEvent, linkSearch, projectNameById]);

  const handleConfirmLinkSelected = async () => {
    if (!linkingEventId) return;
    const ids = Object.entries(selectedTaskIdsToLink)
      .filter(([, checked]) => checked)
      .map(([id]) => id);
    if (ids.length === 0) return;
    setIsLinkingTasks(true);
    try {
      await onLinkTasksToCalendarEvent({
        eventId: linkingEventId,
        taskIds: ids,
        syncDateToEvent: syncDatesOnLink,
      });
      setLinkingEventId(null);
      setSelectedTaskIdsToLink({});
    } finally {
      setIsLinkingTasks(false);
    }
  };

  const handleCreateQuickTask = async (ev: CalendarEvent) => {
    if (!quickTaskTitle.trim()) return;
    setIsCreatingQuickTask(true);
    try {
      const targetPid =
        ev.project_id && ev.project_id !== "ALL"
          ? ev.project_id
          : quickTaskProjectId || dash.active_project.id;
      await onCreateTaskForEvent({
        projectId: targetPid,
        title: quickTaskTitle.trim(),
        category: "GENERAL",
        durationMinutes: quickTaskMinutes || 30,
        scheduledDate: ev.event_date,
        calendarEventId: ev.id,
      });
      setQuickTaskTitle("");
      setQuickTaskForEventId(null);
    } finally {
      setIsCreatingQuickTask(false);
    }
  };

  const renderEventCard = (ev: CalendarEvent) => {
    const meta = EVENT_TYPE_META[ev.event_type] ?? EVENT_TYPE_META.MEETING;
    const linkedTasks = ev.linked_tasks || [];
    const totalCount = ev.linked_tasks_count ?? linkedTasks.length;
    const doneCount = ev.linked_completed_count ?? linkedTasks.filter((t) => t.status === "CLOSED").length;
    const pct = totalCount > 0 ? Math.round((doneCount / totalCount) * 100) : 0;

    return (
      <div
        key={ev.id}
        className="rounded-xl border border-white/[0.08] bg-[#161820] p-3.5 space-y-2.5 shadow-sm"
      >
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0 flex-1 space-y-1">
            <div className="flex items-center gap-1.5 flex-wrap">
              <span className={`px-2 py-0.5 rounded-full text-[10px] border font-semibold ${meta.badgeClass}`}>
                {meta.label}
              </span>
              <span className="px-2 py-0.5 rounded-md bg-white/[0.06] text-white/70 text-[10px] font-mono">
                {ev.event_date}
              </span>
              <span className="text-[11px] text-indigo-300 font-medium">
                {formatPersianDateLabel(ev.event_date)}
              </span>
              {(ev.start_time || ev.end_time) && (
                <span className="inline-flex items-center gap-1 text-[10.5px] text-white/65 font-mono">
                  <Clock className="w-3 h-3 text-white/40" />
                  {ev.start_time || "--:--"}
                  {ev.end_time ? ` - ${ev.end_time}` : ""}
                </span>
              )}
              <span className="px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-200/90 text-[10px]">
                {ev.project_id === "ALL" ? "فراپروژه‌ای / همه پروژه‌ها" : ev.project_name || ev.project_id}
              </span>
            </div>

            <div className="flex items-center gap-2">
              <span
                className="w-2.5 h-2.5 rounded-full shrink-0"
                style={{ backgroundColor: ev.color_hex || meta.defaultColor }}
              />
              <h4 className="text-[13.5px] font-bold text-white leading-snug">{ev.title}</h4>
            </div>

            {ev.location_or_link && (
              <div className="flex items-center gap-1 text-[11px] text-white/55">
                <MapPin className="w-3 h-3 shrink-0" />
                <span className="truncate">{ev.location_or_link}</span>
              </div>
            )}
            {ev.notes && (
              <p className="text-[11.5px] text-white/65 leading-relaxed whitespace-pre-wrap">{ev.notes}</p>
            )}
          </div>

          <div className="flex items-center gap-1 shrink-0">
            <button
              type="button"
              onClick={() =>
                onInsertChatPrompt(
                  `درباره رویداد تقویم «${ev.title}» (${ev.id}) در تاریخ ${ev.event_date} که ${totalCount} کار به آن متصل است بررسی کن، وضعیت آمادگی را تحلیل کن و اگر کار جدیدی لازم است به آن لینک کن.`
                )
              }
              title="بررسی و برنامه‌ریزی با مدل"
              className="px-2 py-1 rounded-lg bg-purple-500/15 hover:bg-purple-500/25 text-purple-200 text-[10.5px] flex items-center gap-1 transition"
            >
              <MessageSquare className="w-3 h-3" />
              <span>مشورت با مدل</span>
            </button>
            <button
              type="button"
              onClick={() => openEditEventForm(ev)}
              className="px-2 py-1 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] text-white/75 text-[10.5px]"
            >
              ویرایش
            </button>
            <button
              type="button"
              onClick={() => {
                if (window.confirm(`حذف رویداد «${ev.title}»؟ کارهای متصل از حالت لینک خارج می‌شوند.`)) {
                  void onDeleteCalendarEvent(ev.id);
                }
              }}
              title="حذف رویداد"
              className="p-1 rounded-lg bg-rose-500/10 hover:bg-rose-500/25 text-rose-300 transition"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        {/* Linked Tasks Header & Progress Bar */}
        <div className="pt-2 border-t border-white/[0.07] space-y-2">
          <div className="flex items-center justify-between gap-2 flex-wrap">
            <div className="flex items-center gap-2">
              <span className="text-[11px] font-semibold text-white/80 flex items-center gap-1">
                <Link2 className="w-3.5 h-3.5 text-emerald-400" />
                <span>کارهای متصل به این رویداد ({totalCount} کار)</span>
              </span>
              {totalCount > 0 && (
                <span className="text-[10.5px] font-mono text-emerald-300 bg-emerald-500/10 px-1.5 py-0.5 rounded">
                  {doneCount}/{totalCount} انجام‌شده ({pct}%)
                </span>
              )}
            </div>

            <div className="flex items-center gap-1.5">
              <button
                type="button"
                onClick={() => openTaskLinker(ev)}
                className="px-2.5 py-1 rounded-lg bg-emerald-500/15 hover:bg-emerald-500/25 border border-emerald-500/30 text-emerald-200 text-[11px] font-medium flex items-center gap-1"
              >
                <Link2 className="w-3 h-3" />
                <span>اتصال کارهای موجود</span>
              </button>
              <button
                type="button"
                onClick={() => {
                  setQuickTaskForEventId(quickTaskForEventId === ev.id ? null : ev.id);
                  setQuickTaskTitle("");
                  setQuickTaskProjectId(
                    ev.project_id && ev.project_id !== "ALL"
                      ? ev.project_id
                      : dash.active_project.id
                  );
                }}
                className="px-2.5 py-1 rounded-lg bg-blue-500/15 hover:bg-blue-500/25 border border-blue-500/30 text-blue-200 text-[11px] font-medium flex items-center gap-1"
              >
                <Plus className="w-3 h-3" />
                <span>کار جدید برای این رویداد</span>
              </button>
            </div>
          </div>

          {totalCount > 0 && (
            <div className="h-1.5 w-full rounded-full bg-white/[0.06] overflow-hidden">
              <div
                className="h-full bg-emerald-500 transition-all"
                style={{ width: `${pct}%` }}
              />
            </div>
          )}

          {/* Quick-add task inline form */}
          {quickTaskForEventId === ev.id && (
            <div className="p-2.5 rounded-lg bg-[#111319] border border-blue-500/30 flex items-center gap-2 flex-wrap">
              <input
                type="text"
                value={quickTaskTitle}
                onChange={(e) => setQuickTaskTitle(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void handleCreateQuickTask(ev);
                }}
                placeholder={`عنوان کار متصل به «${ev.title}»...`}
                className="flex-1 min-w-[180px] bg-black/40 border border-white/10 rounded-lg px-2.5 py-1.5 text-[12px] text-white outline-none focus:border-blue-400"
              />
              {ev.project_id === "ALL" && (
                <select
                  value={quickTaskProjectId}
                  onChange={(e) => setQuickTaskProjectId(e.target.value)}
                  className="bg-black/40 border border-white/10 rounded-lg px-2 py-1.5 text-[11px] text-white"
                >
                  {dash.projects.map((p) => (
                    <option key={p.id} value={p.id} className="bg-[#18191e]">
                      {p.name}
                    </option>
                  ))}
                </select>
              )}
              <input
                type="number"
                min={5}
                step={5}
                value={quickTaskMinutes}
                onChange={(e) => setQuickTaskMinutes(Number(e.target.value) || 25)}
                className="w-16 bg-black/40 border border-white/10 rounded-lg px-2 py-1.5 text-[11px] text-white font-mono text-center"
                title="مدت (دقیقه)"
              />
              <button
                type="button"
                disabled={isCreatingQuickTask || !quickTaskTitle.trim()}
                onClick={() => void handleCreateQuickTask(ev)}
                className="px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 disabled:opacity-40 text-white text-[11px] font-semibold"
              >
                {isCreatingQuickTask ? "..." : "ساخت و اتصال"}
              </button>
            </div>
          )}

          {/* List of linked tasks */}
          {linkedTasks.length === 0 ? (
            <div className="text-[11px] text-white/40 py-1">
              هنوز کاری به این رویداد لینک نشده است. از دکمه‌های بالا کارها را به این زمان متصل کنید یا در چت به مدل بگویید.
            </div>
          ) : (
            <div className="space-y-1.5 max-h-60 overflow-y-auto pr-0.5">
              {linkedTasks.map((t) => {
                const isDone = t.status === "CLOSED";
                const isInProgress = t.status === "IN_PROGRESS";
                return (
                  <div
                    key={t.id}
                    className={`px-2.5 py-2 rounded-lg border flex items-center justify-between gap-2 text-[11.5px] ${
                      isDone
                        ? "bg-emerald-500/[0.05] border-emerald-500/20 text-white/55"
                        : isInProgress
                        ? "bg-blue-500/[0.08] border-blue-500/30 text-white"
                        : "bg-white/[0.03] border-white/[0.06] text-white/85"
                    }`}
                  >
                    <div className="min-w-0 flex-1 flex items-center gap-2">
                      <span
                        className={`w-2 h-2 rounded-full shrink-0 ${
                          isDone ? "bg-emerald-400" : isInProgress ? "bg-blue-400 animate-pulse" : "bg-white/35"
                        }`}
                      />
                      <span className={`truncate ${isDone ? "line-through" : ""}`}>{t.title}</span>
                      <span className="px-1.5 py-0.5 rounded bg-white/[0.06] text-[9.5px] text-white/55 shrink-0">
                        {projectNameById.get(t.project_id) || t.project_id}
                      </span>
                      <span className="text-[10px] font-mono text-white/45 shrink-0">
                        {t.duration_minutes}m
                      </span>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      {!isDone && !isInProgress && (
                        <button
                          type="button"
                          onClick={() => onStartTask(t.id)}
                          title="شروع این کار"
                          className="p-1 rounded bg-blue-500/15 hover:bg-blue-500/30 text-blue-300"
                        >
                          <Play className="w-3 h-3" />
                        </button>
                      )}
                      {!isDone && (
                        <button
                          type="button"
                          onClick={() => onQuickComplete(t.id, Math.max(1, Math.round((t.duration_minutes || 25) / 25)))}
                          title="تکمیل سریع"
                          className="p-1 rounded bg-emerald-500/15 hover:bg-emerald-500/30 text-emerald-300"
                        >
                          <CheckCircle2 className="w-3 h-3" />
                        </button>
                      )}
                      <button
                        type="button"
                        onClick={() => void onUnlinkTaskFromCalendarEvent(t.id)}
                        title="قطع اتصال از رویداد"
                        className="p-1 rounded bg-white/[0.06] hover:bg-rose-500/20 text-white/50 hover:text-rose-300"
                      >
                        <Unlink className="w-3 h-3" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    );
  };

  return (
    <div dir="rtl" className="h-full flex flex-col bg-[#0F1117] text-white overflow-hidden">
      {/* Header Toolbar */}
      <div className="px-4 py-3 border-b border-white/[0.07] bg-[#14161E] flex items-center justify-between gap-2 flex-wrap shrink-0">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-xl bg-indigo-500/20 border border-indigo-400/30 flex items-center justify-center">
            <CalendarDays className="w-4 h-4 text-indigo-300" />
          </div>
          <div>
            <h2 className="text-[13.5px] font-bold text-white flex items-center gap-2">
              <span>تقویم، رویدادهای مهم و اتصال کارها به زمان</span>
              <span className="px-2 py-0.5 rounded-full bg-indigo-500/15 text-indigo-200 text-[10.5px] font-mono">
                {(dash.calendar_events || []).length} رویداد
              </span>
            </h2>
            <p className="text-[11px] text-white/55">
              کاربر، اپلیکیشن و مدل هوش مصنوعی رویدادها (مثل جلسات و ددلاین‌ها) و کارهای متصل به آن‌ها را یکجا می‌بینند.
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          <select
            value={filterProjectId}
            onChange={(e) => setFilterProjectId(e.target.value)}
            className="bg-[#1C1F2A] border border-white/10 rounded-lg px-2.5 py-1.5 text-[11.5px] text-white"
          >
            <option value="ALL" className="bg-[#18191e]">همه پروژه‌ها (نمای جامع)</option>
            {dash.projects.map((p) => (
              <option key={p.id} value={p.id} className="bg-[#18191e]">
                {p.name}
              </option>
            ))}
          </select>

          <button
            type="button"
            onClick={() => {
              const now = new Date();
              setMonthCursor({ year: now.getFullYear(), month: now.getMonth() });
              setSelectedDate(todayIso);
            }}
            className="px-2.5 py-1.5 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] text-[11.5px] text-white/80"
          >
            امروز
          </button>

          <button
            type="button"
            onClick={() => openNewEventForm(selectedDate)}
            className="px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-[11.5px] font-semibold flex items-center gap-1.5 shadow-sm"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>رویداد جدید</span>
          </button>
        </div>
      </div>

      {/* Main Scrollable Content */}
      <div className="flex-1 min-h-0 overflow-y-auto p-4 space-y-4">
        {/* Top Grid: Interactive Month Grid + Selected Day Workspace */}
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-4">
          {/* Month Grid (7 cols) */}
          <div className="xl:col-span-7 rounded-2xl border border-white/[0.08] bg-[#14161E] p-3.5 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Calendar className="w-4 h-4 text-indigo-400" />
                <span className="text-[13px] font-bold text-white">
                  {formatPersianMonthTitle(monthCursor.year, monthCursor.month)}
                </span>
              </div>
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={() =>
                    setMonthCursor((prev) =>
                      prev.month === 0
                        ? { year: prev.year - 1, month: 11 }
                        : { year: prev.year, month: prev.month - 1 }
                    )
                  }
                  className="p-1.5 rounded-lg bg-white/[0.05] hover:bg-white/[0.12] text-white/75"
                >
                  <ChevronRight className="w-4 h-4" />
                </button>
                <button
                  type="button"
                  onClick={() =>
                    setMonthCursor((prev) =>
                      prev.month === 11
                        ? { year: prev.year + 1, month: 0 }
                        : { year: prev.year, month: prev.month + 1 }
                    )
                  }
                  className="p-1.5 rounded-lg bg-white/[0.05] hover:bg-white/[0.12] text-white/75"
                >
                  <ChevronLeft className="w-4 h-4" />
                </button>
              </div>
            </div>

            <div className="grid grid-cols-7 gap-1 text-center text-[10.5px] font-semibold text-white/50">
              {["شنبه", "یکشنبه", "دوشنبه", "سه‌شنبه", "چهارشنبه", "پنجشنبه", "جمعه"].map((wd) => (
                <div key={wd} className="py-1">
                  {wd}
                </div>
              ))}
            </div>

            <div className="grid grid-cols-7 gap-1.5">
              {monthCells.map((cell, idx) => {
                if (!cell.iso) {
                  return <div key={`empty-${idx}`} className="h-20 rounded-xl bg-white/[0.015]" />;
                }
                const dayEvents = eventsByDate.get(cell.iso) ?? [];
                const dayTasks = scheduledTasksByDate.get(cell.iso) ?? [];
                const isSelected = cell.iso === selectedDate;
                const isToday = cell.iso === todayIso;
                return (
                  <button
                    key={cell.iso}
                    type="button"
                    onClick={() => setSelectedDate(cell.iso!)}
                    className={`h-20 rounded-xl p-1.5 text-right flex flex-col justify-between border transition overflow-hidden ${
                      isSelected
                        ? "bg-indigo-600/20 border-indigo-400/60 shadow-[0_0_0_1px_rgba(129,140,248,0.3)]"
                        : isToday
                        ? "bg-emerald-500/10 border-emerald-400/40 hover:bg-emerald-500/15"
                        : "bg-[#1A1C26]/70 border-white/[0.06] hover:bg-white/[0.06]"
                    }`}
                  >
                    <div className="flex items-center justify-between w-full">
                      <span className="text-[11.5px] font-bold text-white">
                        {formatPersianDayNumber(cell.iso)}
                      </span>
                      <span className="text-[9.5px] font-mono text-white/40">{cell.dayNum}</span>
                    </div>

                    <div className="space-y-0.5 w-full overflow-hidden">
                      {dayEvents.slice(0, 2).map((ev) => (
                        <div
                          key={ev.id}
                          className="px-1 py-0.5 rounded text-[9px] font-medium truncate text-white"
                          style={{
                            backgroundColor: `${ev.color_hex || EVENT_TYPE_META[ev.event_type]?.defaultColor || "#6366f1"}33`,
                            borderRight: `2px solid ${ev.color_hex || EVENT_TYPE_META[ev.event_type]?.defaultColor || "#6366f1"}`,
                          }}
                        >
                          {ev.title}
                        </div>
                      ))}
                      {dayEvents.length > 2 && (
                        <div className="text-[9px] text-indigo-300 font-mono">
                          +{dayEvents.length - 2} رویداد
                        </div>
                      )}
                      {dayTasks.length > 0 && (
                        <div className="text-[9px] text-emerald-300/90 font-mono truncate">
                          {dayTasks.length} کار زمان‌بندی‌شده
                        </div>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Selected Day Panel (5 cols) */}
          <div className="xl:col-span-5 rounded-2xl border border-white/[0.08] bg-[#14161E] p-3.5 space-y-3 flex flex-col">
            <div className="flex items-center justify-between gap-2 pb-2 border-b border-white/[0.07]">
              <div>
                <div className="text-[13px] font-bold text-white flex items-center gap-2">
                  <span>{formatPersianDateLabel(selectedDate)}</span>
                  <span className="text-[10.5px] font-mono text-white/50">{selectedDate}</span>
                </div>
                <div className="text-[11px] text-white/55 mt-0.5">
                  ظرفیت روز: {currentDayRecord?.total_capacity_minutes ?? 300} دقیقه (رزرو: {selectedDayBookedMinutes}m)
                  {currentDayRecord?.is_off_day ? " • روز تعطیل" : ""}
                </div>
              </div>
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  onClick={openDayCapEditor}
                  className="px-2 py-1 rounded-lg bg-white/[0.06] hover:bg-white/[0.12] text-[11px] text-white/75"
                >
                  ظرفیت روز
                </button>
                <button
                  type="button"
                  onClick={() => openNewEventForm(selectedDate)}
                  className="px-2.5 py-1 rounded-lg bg-indigo-500/20 hover:bg-indigo-500/30 text-indigo-200 text-[11px] font-medium"
                >
                  + رویداد این روز
                </button>
              </div>
            </div>

            {editingDayCap && (
              <div className="p-3 rounded-xl bg-[#101219] border border-white/10 space-y-2 text-[11.5px]">
                <div className="grid grid-cols-2 gap-2">
                  <label className="space-y-1">
                    <span className="text-white/60 text-[10.5px]">ظرفیت کل (دقیقه)</span>
                    <input
                      type="number"
                      value={capTotal}
                      onChange={(e) => setCapTotal(Number(e.target.value))}
                      className="w-full bg-black/40 border border-white/10 rounded-lg px-2 py-1 text-white"
                    />
                  </label>
                  <label className="space-y-1">
                    <span className="text-white/60 text-[10.5px]">تمرکز عمیق (دقیقه)</span>
                    <input
                      type="number"
                      value={capDeep}
                      onChange={(e) => setCapDeep(Number(e.target.value))}
                      className="w-full bg-black/40 border border-white/10 rounded-lg px-2 py-1 text-white"
                    />
                  </label>
                </div>
                <input
                  type="text"
                  value={capNote}
                  onChange={(e) => setCapNote(e.target.value)}
                  placeholder="یادداشت روز..."
                  className="w-full bg-black/40 border border-white/10 rounded-lg px-2.5 py-1.5 text-white"
                />
                <div className="flex items-center justify-between">
                  <label className="flex items-center gap-1.5 text-white/75 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={capOff}
                      onChange={(e) => setCapOff(e.target.checked)}
                    />
                    <span>روز تعطیل / استراحت</span>
                  </label>
                  <div className="flex gap-1.5">
                    <button
                      type="button"
                      onClick={() => setEditingDayCap(false)}
                      className="px-2.5 py-1 rounded bg-white/10 text-white/75"
                    >
                      انصراف
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleSaveDayCap()}
                      className="px-3 py-1 rounded bg-emerald-600 text-white font-semibold"
                    >
                      ذخیره
                    </button>
                  </div>
                </div>
              </div>
            )}

            {/* Events for Selected Date */}
            <div className="space-y-2.5">
              <div className="text-[11px] font-semibold text-indigo-300 flex items-center gap-1.5">
                <Sparkles className="w-3.5 h-3.5" />
                <span>رویدادهای {formatPersianDateLabel(selectedDate)} ({selectedDayEvents.length})</span>
              </div>
              {selectedDayEvents.length === 0 ? (
                <div className="p-3 rounded-xl border border-dashed border-white/10 text-[11.5px] text-white/45 text-center">
                  رویدادی در این تاریخ ثبت نشده است.
                </div>
              ) : (
                <div className="space-y-2.5">{selectedDayEvents.map(renderEventCard)}</div>
              )}
            </div>

            {/* Tasks directly scheduled on Selected Date */}
            <div className="space-y-2 pt-2 border-t border-white/[0.07]">
              <div className="text-[11px] font-semibold text-emerald-300">
                کارهای زمان‌بندی‌شده در این تاریخ ({selectedDayTasks.length})
              </div>
              {selectedDayTasks.length === 0 ? (
                <div className="text-[11px] text-white/40">کاری مستقیماً روی این تاریخ ست نشده است.</div>
              ) : (
                <div className="space-y-1.5 max-h-48 overflow-y-auto">
                  {selectedDayTasks.map((t) => (
                    <div
                      key={t.id}
                      className="px-2.5 py-2 rounded-lg bg-[#1A1C26] border border-white/[0.06] flex items-center justify-between gap-2 text-[11.5px]"
                    >
                      <div className="min-w-0 flex-1 space-y-0.5">
                        <div className="truncate font-medium text-white">{t.title}</div>
                        <div className="flex items-center gap-1.5 text-[10px] text-white/50 flex-wrap">
                          <span>{projectNameById.get(t.project_id) || t.project_id}</span>
                          {t.scheduled_slot && <span>• {t.scheduled_slot}</span>}
                          {t.calendar_event_title && (
                            <span className="px-1.5 py-0.2 rounded bg-indigo-500/20 text-indigo-200">
                              رویداد: {t.calendar_event_title}
                            </span>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-1 shrink-0">
                        {t.status !== "CLOSED" && (
                          <button
                            type="button"
                            onClick={() => onQuickComplete(t.id, Math.max(1, Math.round((t.duration_minutes || 25) / 25)))}
                            className="p-1 rounded bg-emerald-500/15 hover:bg-emerald-500/30 text-emerald-300"
                            title="تکمیل"
                          >
                            <CheckCircle2 className="w-3.5 h-3.5" />
                          </button>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* All Upcoming Events & Linked Work */}
        <div className="rounded-2xl border border-white/[0.08] bg-[#14161E] p-4 space-y-3">
          <div className="flex items-center justify-between gap-2">
            <h3 className="text-[13px] font-bold text-white flex items-center gap-2">
              <CalendarDays className="w-4 h-4 text-emerald-400" />
              <span>نمای یکپارچه رویدادهای پیش‌رو و کارهای متصل ({upcomingEvents.length})</span>
            </h3>
            <span className="text-[11px] text-white/50">
              مثلاً اگر ۱۰ کار به جلسه یکشنبه متصل باشد، پیشرفت و تمام کارهای آن اینجا و در دید مدل قابل مشاهده است.
            </span>
          </div>

          {upcomingEvents.length === 0 ? (
            <div className="p-6 rounded-xl border border-dashed border-white/10 text-center space-y-2">
              <p className="text-[12.5px] text-white/65">هنوز رویداد مهمی در تقویم ثبت نشده است.</p>
              <button
                type="button"
                onClick={() => openNewEventForm(todayIso)}
                className="px-3.5 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white text-[12px] font-semibold inline-flex items-center gap-1.5"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>ثبت اولین رویداد (مثلاً جلسه یکشنبه)</span>
              </button>
            </div>
          ) : (
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
              {upcomingEvents.map(renderEventCard)}
            </div>
          )}
        </div>
      </div>

      {/* Create / Edit Event Modal */}
      {showEventForm && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="w-full max-w-lg rounded-2xl bg-[#181A22] border border-white/15 p-4 space-y-3 shadow-2xl">
            <div className="flex items-center justify-between">
              <h3 className="text-[14px] font-bold text-white">
                {editingEventId ? "ویرایش رویداد تقویم" : "ثبت رویداد جدید در تقویم"}
              </h3>
              <button
                type="button"
                onClick={() => setShowEventForm(false)}
                className="p-1 rounded-lg hover:bg-white/10 text-white/60"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-2.5 text-[12px]">
              <input
                type="text"
                value={evtTitle}
                onChange={(e) => setEvtTitle(e.target.value)}
                placeholder="عنوان رویداد (مثلاً: جلسه یکشنبه بررسی اسپرینت)"
                className="w-full bg-black/40 border border-white/10 rounded-xl px-3 py-2 text-white outline-none focus:border-indigo-400"
              />

              <div className="grid grid-cols-2 gap-2">
                <label className="space-y-1">
                  <span className="text-[10.5px] text-white/60">نوع رویداد</span>
                  <select
                    value={evtType}
                    onChange={(e) => setEvtType(e.target.value)}
                    className="w-full bg-black/40 border border-white/10 rounded-xl px-2.5 py-2 text-white"
                  >
                    {Object.entries(EVENT_TYPE_META).map(([k, v]) => (
                      <option key={k} value={k} className="bg-[#18191e]">
                        {v.label}
                      </option>
                    ))}
                  </select>
                </label>

                <label className="space-y-1">
                  <span className="text-[10.5px] text-white/60">محدوده پروژه</span>
                  <select
                    value={evtProjectId}
                    onChange={(e) => setEvtProjectId(e.target.value)}
                    className="w-full bg-black/40 border border-white/10 rounded-xl px-2.5 py-2 text-white"
                  >
                    <option value="ALL" className="bg-[#18191e]">فراپروژه‌ای (همه پروژه‌ها)</option>
                    {dash.projects.map((p) => (
                      <option key={p.id} value={p.id} className="bg-[#18191e]">
                        {p.name}
                      </option>
                    ))}
                  </select>
                </label>
              </div>

              <div className="grid grid-cols-3 gap-2">
                <label className="space-y-1">
                  <span className="text-[10.5px] text-white/60">تاریخ میلادی</span>
                  <input
                    type="date"
                    value={evtDate}
                    onChange={(e) => setEvtDate(e.target.value)}
                    className="w-full bg-black/40 border border-white/10 rounded-xl px-2.5 py-1.5 text-white font-mono"
                  />
                </label>
                <label className="space-y-1">
                  <span className="text-[10.5px] text-white/60">ساعت شروع</span>
                  <input
                    type="time"
                    value={evtStartTime}
                    onChange={(e) => setEvtStartTime(e.target.value)}
                    className="w-full bg-black/40 border border-white/10 rounded-xl px-2.5 py-1.5 text-white font-mono"
                  />
                </label>
                <label className="space-y-1">
                  <span className="text-[10.5px] text-white/60">ساعت پایان</span>
                  <input
                    type="time"
                    value={evtEndTime}
                    onChange={(e) => setEvtEndTime(e.target.value)}
                    className="w-full bg-black/40 border border-white/10 rounded-xl px-2.5 py-1.5 text-white font-mono"
                  />
                </label>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <input
                  type="text"
                  value={evtLocation}
                  onChange={(e) => setEvtLocation(e.target.value)}
                  placeholder="مکان یا لینک جلسه (اختیاری)"
                  className="bg-black/40 border border-white/10 rounded-xl px-3 py-2 text-white"
                />
                <select
                  value={evtStatus}
                  onChange={(e) => setEvtStatus(e.target.value)}
                  className="bg-black/40 border border-white/10 rounded-xl px-2.5 py-2 text-white"
                >
                  <option value="UPCOMING" className="bg-[#18191e]">در پیش‌رو (UPCOMING)</option>
                  <option value="COMPLETED" className="bg-[#18191e]">انجام‌شده (COMPLETED)</option>
                  <option value="CANCELLED" className="bg-[#18191e]">لغوشده (CANCELLED)</option>
                </select>
              </div>

              <textarea
                rows={3}
                value={evtNotes}
                onChange={(e) => setEvtNotes(e.target.value)}
                placeholder="دستور جلسه، اهداف یا توضیحات..."
                className="w-full bg-black/40 border border-white/10 rounded-xl px-3 py-2 text-white resize-none"
              />
            </div>

            <div className="flex items-center justify-end gap-2 pt-1">
              <button
                type="button"
                onClick={() => setShowEventForm(false)}
                className="px-3.5 py-1.5 rounded-xl bg-white/[0.07] hover:bg-white/[0.12] text-white/80 text-[12px]"
              >
                انصراف
              </button>
              <button
                type="button"
                disabled={isSavingEvent || !evtTitle.trim() || !evtDate.trim()}
                onClick={() => void handleSaveEvent()}
                className="px-4 py-1.5 rounded-xl bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white text-[12px] font-semibold"
              >
                {isSavingEvent ? "در حال ذخیره..." : editingEventId ? "ذخیره تغییرات" : "ثبت رویداد"}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Multi-Select Existing Tasks -> Calendar Event Linker Modal */}
      {linkingEvent && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="w-full max-w-xl rounded-2xl bg-[#181A22] border border-white/15 p-4 space-y-3 shadow-2xl max-h-[82vh] flex flex-col">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-[13.5px] font-bold text-white">
                  اتصال کارها به رویداد «{linkingEvent.title}»
                </h3>
                <p className="text-[11px] text-white/55">
                  تاریخ رویداد: {linkingEvent.event_date} ({formatPersianDateLabel(linkingEvent.event_date)})
                </p>
              </div>
              <button
                type="button"
                onClick={() => setLinkingEventId(null)}
                className="p-1 rounded-lg hover:bg-white/10 text-white/60"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="flex items-center gap-2 flex-wrap">
              <input
                type="text"
                value={linkSearch}
                onChange={(e) => setLinkSearch(e.target.value)}
                placeholder="جستجوی نام کار، کد TSK یا پروژه..."
                className="flex-1 min-w-[200px] bg-black/40 border border-white/10 rounded-xl px-3 py-1.5 text-[12px] text-white outline-none focus:border-emerald-400"
              />
              <label className="flex items-center gap-1.5 text-[11px] text-white/75 cursor-pointer">
                <input
                  type="checkbox"
                  checked={syncDatesOnLink}
                  onChange={(e) => setSyncDatesOnLink(e.target.checked)}
                />
                <span>همگام‌سازی تاریخ کارهای انتخابی با تاریخ رویداد</span>
              </label>
            </div>

            <div className="flex-1 min-h-0 overflow-y-auto space-y-1.5 pr-1">
              {candidateTasksForLinking.length === 0 ? (
                <div className="text-center py-8 text-[12px] text-white/45">
                  کارِ قابل اتصالی با این فیلتر یافت نشد.
                </div>
              ) : (
                candidateTasksForLinking.map((t) => {
                  const checked = !!selectedTaskIdsToLink[t.id];
                  return (
                    <label
                      key={t.id}
                      className={`px-3 py-2 rounded-xl border flex items-center justify-between gap-2 cursor-pointer transition ${
                        checked
                          ? "bg-emerald-500/15 border-emerald-400/50 text-white"
                          : "bg-white/[0.03] border-white/[0.07] text-white/80 hover:bg-white/[0.06]"
                      }`}
                    >
                      <div className="flex items-center gap-2.5 min-w-0 flex-1">
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={(e) =>
                            setSelectedTaskIdsToLink((prev) => ({
                              ...prev,
                              [t.id]: e.target.checked,
                            }))
                          }
                        />
                        <div className="min-w-0 flex-1">
                          <div className="text-[12px] font-medium truncate">{t.title}</div>
                          <div className="text-[10px] text-white/50 flex items-center gap-2">
                            <span>{projectNameById.get(t.project_id) || t.project_id}</span>
                            <span className="font-mono">{t.id}</span>
                            <span>• {t.duration_minutes}m</span>
                            {t.calendar_event_title && (
                              <span className="text-amber-300">متصل به: {t.calendar_event_title}</span>
                            )}
                          </div>
                        </div>
                      </div>
                    </label>
                  );
                })
              )}
            </div>

            <div className="flex items-center justify-between pt-2 border-t border-white/10">
              <span className="text-[11px] text-emerald-300 font-mono">
                {Object.values(selectedTaskIdsToLink).filter(Boolean).length} کار انتخاب شده
              </span>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => setLinkingEventId(null)}
                  className="px-3.5 py-1.5 rounded-xl bg-white/[0.07] text-white/80 text-[12px]"
                >
                  انصراف
                </button>
                <button
                  type="button"
                  disabled={
                    isLinkingTasks ||
                    Object.values(selectedTaskIdsToLink).filter(Boolean).length === 0
                  }
                  onClick={() => void handleConfirmLinkSelected()}
                  className="px-4 py-1.5 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-white text-[12px] font-semibold"
                >
                  {isLinkingTasks ? "در حال اتصال..." : "اتصال به رویداد"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
