import { useEffect, useState } from "react";
import { GitMerge, Settings2, Trash2, X } from "lucide-react";
import { Project } from "../lib/tauri_api";

const PROJECT_COLORS = [
  "#3B82F6",
  "#10B981",
  "#8B5CF6",
  "#F59E0B",
  "#EF4444",
  "#06B6D4",
  "#EC4899",
];

interface ProjectManageModalProps {
  project: Project | null;
  projects: Project[];
  onClose: () => void;
  onSave: (params: {
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
  onMerge: (sourceId: string, targetId: string) => Promise<void> | void;
  onDelete: (projectId: string) => Promise<void> | void;
}

export function ProjectManageModal({
  project,
  projects,
  onClose,
  onSave,
  onMerge,
  onDelete,
}: ProjectManageModalProps) {
  const [name, setName] = useState("");
  const [parentId, setParentId] = useState("");
  const [realm, setRealm] = useState("WORK");
  const [status, setStatus] = useState("ACTIVE");
  const [color, setColor] = useState("#3B82F6");
  const [dailyQuota, setDailyQuota] = useState(0);
  const [weeklyQuota, setWeeklyQuota] = useState(0);
  const [hourlyRate, setHourlyRate] = useState(0);
  const [mergeTargetId, setMergeTargetId] = useState("");
  const [confirmMerge, setConfirmMerge] = useState(false);

  useEffect(() => {
    if (project) {
      setName(project.name);
      setParentId(project.parent_id ?? "");
      setRealm((project.realm || "WORK").toUpperCase());
      setStatus((project.lifecycle_status || "ACTIVE").toUpperCase());
      setColor(project.color || "#3B82F6");
      setDailyQuota(project.daily_quota_minutes ?? 0);
      setWeeklyQuota(project.weekly_quota_minutes ?? 0);
      setHourlyRate(project.hourly_rate ?? 0);
      const other = projects.find((p) => p.id !== project.id);
      setMergeTargetId(other?.id ?? "");
      setConfirmMerge(false);
    }
  }, [project, projects]);

  if (!project) return null;

  // جلوگیری از انتخاب خود پروژه یا فرزندانش به‌عنوان والد
  const descendantIds = new Set<string>([project.id]);
  let added = true;
  while (added) {
    added = false;
    for (const p of projects) {
      if (p.parent_id && descendantIds.has(p.parent_id) && !descendantIds.has(p.id)) {
        descendantIds.add(p.id);
        added = true;
      }
    }
  }
  const availableParents = projects.filter((p) => !descendantIds.has(p.id));
  const mergeTargets = projects.filter((p) => p.id !== project.id);

  const handleSave = async () => {
    if (!name.trim()) return;
    await onSave({
      projectId: project.id,
      name: name.trim(),
      parentId: parentId.trim() || null,
      clearParent: !parentId.trim(),
      realm,
      lifecycleStatus: status,
      color,
      dailyQuotaMinutes: dailyQuota,
      weeklyQuotaMinutes: weeklyQuota,
      monthlyQuotaMinutes: project.monthly_quota_minutes ?? 0,
      hourlyRate,
    });
    onClose();
  };

  return (
    <div
      className="fixed inset-0 bg-black/75 backdrop-blur-md flex items-center justify-center p-4 z-50 select-text"
      onClick={onClose}
    >
      <div
        className="bg-mab-surface border border-mab-border w-full max-w-[500px] max-h-[90vh] overflow-y-auto p-5 sm:p-6 space-y-4 rounded-2xl shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-mab-border pb-3">
          <div className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-lg bg-purple-500/15 border border-purple-500/30 flex items-center justify-center text-purple-300">
              <Settings2 className="w-4 h-4" />
            </div>
            <div>
              <h3 className="text-[14px] font-bold text-white">
                مدیریت یکپارچه پروژه
              </h3>
              <span className="text-[10.5px] font-mono text-white/45">
                {project.id}
              </span>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] text-white/50 hover:text-white transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="space-y-3">
          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/70">
              نام پروژه:
            </label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full mab-input px-3 py-2 text-[12.5px]"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/70">
                پروژه والد:
              </label>
              <select
                value={parentId}
                onChange={(e) => setParentId(e.target.value)}
                className="w-full mab-input px-2.5 py-2 text-[12px] bg-[#14151A]"
              >
                <option value="">ریشه (بدون والد)</option>
                {availableParents.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </div>

            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/70">
                وضعیت پروژه:
              </label>
              <select
                value={status}
                onChange={(e) => setStatus(e.target.value)}
                className="w-full mab-input px-2.5 py-2 text-[12px] bg-[#14151A]"
              >
                <option value="ACTIVE">فعال (Active)</option>
                <option value="PAUSED">متوقف / معلق (Suspended)</option>
                <option value="ARCHIVED">آرشیو شده (Archived)</option>
              </select>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/70">
                قلمرو:
              </label>
              <select
                value={realm}
                onChange={(e) => setRealm(e.target.value)}
                className="w-full mab-input px-2.5 py-2 text-[12px] bg-[#14151A]"
              >
                <option value="WORK">کار (Work)</option>
                <option value="LIFE">زندگی (Life)</option>
              </select>
            </div>

            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/70">
                رنگ:
              </label>
              <div className="flex items-center gap-1.5 pt-1.5">
                {PROJECT_COLORS.map((hex) => (
                  <button
                    key={hex}
                    type="button"
                    onClick={() => setColor(hex)}
                    style={{ backgroundColor: hex }}
                    className={`w-5 h-5 rounded-full transition ${
                      color === hex ? "ring-2 ring-white ring-offset-2 ring-offset-[#14151A] scale-110" : "opacity-70 hover:opacity-100"
                    }`}
                  />
                ))}
              </div>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-2">
            <div className="space-y-1">
              <label className="text-[11px] text-white/60">سهمیه روزانه (دقیقه)</label>
              <input
                type="number"
                min={0}
                value={dailyQuota}
                onChange={(e) => setDailyQuota(Number(e.target.value) || 0)}
                className="w-full mab-input px-2.5 py-1.5 text-[12px] font-mono bidi-ltr"
              />
            </div>
            <div className="space-y-1">
              <label className="text-[11px] text-white/60">سهمیه هفتگی (دقیقه)</label>
              <input
                type="number"
                min={0}
                value={weeklyQuota}
                onChange={(e) => setWeeklyQuota(Number(e.target.value) || 0)}
                className="w-full mab-input px-2.5 py-1.5 text-[12px] font-mono bidi-ltr"
              />
            </div>
            <div className="space-y-1">
              <label className="text-[11px] text-white/60">نرخ ساعتی</label>
              <input
                type="number"
                min={0}
                value={hourlyRate}
                onChange={(e) => setHourlyRate(Number(e.target.value) || 0)}
                className="w-full mab-input px-2.5 py-1.5 text-[12px] font-mono bidi-ltr"
              />
            </div>
          </div>
        </div>

        {/* ادغام با پروژه دیگر */}
        {mergeTargets.length > 0 && (
          <div className="p-3 rounded-xl bg-amber-500/[0.06] border border-amber-500/20 space-y-2">
            <div className="flex items-center gap-1.5 text-[12px] font-semibold text-amber-300">
              <GitMerge className="w-3.5 h-3.5" />
              <span>ادغام با پروژه دیگر (همگام در کامپس و مب‌فوکس)</span>
            </div>
            <div className="flex items-center gap-2">
              <select
                value={mergeTargetId}
                onChange={(e) => {
                  setMergeTargetId(e.target.value);
                  setConfirmMerge(false);
                }}
                className="flex-1 mab-input px-2.5 py-1.5 text-[12px] bg-[#14151A]"
              >
                {mergeTargets.map((p) => (
                  <option key={p.id} value={p.id}>
                    ادغام در: {p.name}
                  </option>
                ))}
              </select>
              {!confirmMerge ? (
                <button
                  type="button"
                  onClick={() => setConfirmMerge(true)}
                  className="px-3 py-1.5 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-500/30 text-amber-200 text-[11.5px] font-semibold transition"
                >
                  ادغام پروژه
                </button>
              ) : (
                <button
                  type="button"
                  onClick={async () => {
                    if (!mergeTargetId) return;
                    await onMerge(project.id, mergeTargetId);
                    onClose();
                  }}
                  className="px-3 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-[11.5px] font-semibold transition"
                >
                  تایید ادغام نهایی
                </button>
              )}
            </div>
            <p className="text-[10.5px] text-white/55 leading-relaxed">
              تمام تسک‌ها، چت‌ها، متدها، اسناد، لاگ‌های زمانی و زیرپروژه‌ها بدون حذف شدن به پروژه مقصد منتقل و در هر دو اپلیکیشن اعمال می‌شوند.
            </p>
          </div>
        )}

        <div className="flex items-center justify-between pt-3 border-t border-mab-border">
          {projects.length > 1 ? (
            <button
              type="button"
              onClick={async () => {
                await onDelete(project.id);
                onClose();
              }}
              className="px-3 py-1.5 rounded-lg bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/25 text-rose-300 text-[11.5px] inline-flex items-center gap-1.5 transition"
            >
              <Trash2 className="w-3.5 h-3.5" />
              <span>حذف پروژه</span>
            </button>
          ) : (
            <span />
          )}

          <div className="flex gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-xl bg-white/[0.04] hover:bg-white/[0.08] text-white/70 text-[12px] font-medium transition"
            >
              انصراف
            </button>
            <button
              type="button"
              onClick={() => void handleSave()}
              disabled={!name.trim()}
              className="px-5 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-[12.5px] transition disabled:opacity-40"
            >
              ذخیره و همگام‌سازی
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ProjectManageModal;
