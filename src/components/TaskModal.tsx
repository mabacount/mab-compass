import { useEffect, useState } from "react";
import { Layers, X } from "lucide-react";
import { Outcome, WorkEntity, WorkLine } from "../lib/tauri_api";
import { MabSelect } from "./ui/MabSelect";

export interface ManualTaskPayload {
  title: string;
  category: string;
  definitionOfDone: string;
  firstPhysicalAction: string;
  workLineId?: string | null;
  outcomeId?: string | null;
  energyTag: string;
  deviceTag: string;
  durationMinutes: number;
  scheduledDate?: string | null;
}

export interface TaskAssignPayload {
  taskId: string;
  workLineId?: string | null;
  outcomeId?: string | null;
  priorityScore?: number | null;
  energyTag?: string | null;
  deviceTag?: string | null;
  durationMinutes?: number | null;
  scheduledDate?: string | null;
}

interface TaskModalProps {
  open: boolean;
  workLines: WorkLine[];
  outcomes: Outcome[];
  editingTask?: WorkEntity | null;
  onClose: () => void;
  onCreate: (payload: ManualTaskPayload) => Promise<void> | void;
  onAssign: (payload: TaskAssignPayload) => Promise<void> | void;
}

const CATEGORY_OPTIONS = [
  { value: "GENERAL", label: "عمومی (GENERAL)" },
  { value: "BACKEND", label: "بک‌اند (BACKEND)" },
  { value: "UI_UX", label: "رابط کاربری (UI_UX)" },
  { value: "ARCHITECTURE", label: "معماری (ARCHITECTURE)" },
  { value: "DEPLOY", label: "انتشار و دیپلوی (DEPLOY)" },
  { value: "VIDEO", label: "محتوا / ویدیو (VIDEO)" },
];

const ENERGY_OPTIONS = [
  { value: "HIGH_DEEP", label: "انرژی بالا / کار عمیق" },
  { value: "MEDIUM_NORMAL", label: "انرژی متوسط / عادی" },
  { value: "LOW_ZOMBIE", label: "انرژی کم / کار سبک" },
];

const DEVICE_OPTIONS = [
  { value: "LAPTOP_ONLY", label: "فقط لپ‌تاپ / سیستم" },
  { value: "PHONE_OK", label: "با موبایل هم ممکن است" },
  { value: "ANY_DEVICE", label: "هر دستگاهی" },
];

export function TaskModal({
  open,
  workLines,
  outcomes,
  editingTask,
  onClose,
  onCreate,
  onAssign,
}: TaskModalProps) {
  const [title, setTitle] = useState("");
  const [category, setCategory] = useState("GENERAL");
  const [firstAction, setFirstAction] = useState("");
  const [dod, setDod] = useState("");
  const [workLineId, setWorkLineId] = useState("");
  const [outcomeId, setOutcomeId] = useState("");
  const [energyTag, setEnergyTag] = useState("MEDIUM_NORMAL");
  const [deviceTag, setDeviceTag] = useState("LAPTOP_ONLY");
  const [durationMinutes, setDurationMinutes] = useState(25);
  const [priorityScore, setPriorityScore] = useState(50);
  const [scheduledDate, setScheduledDate] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    if (editingTask) {
      setTitle(editingTask.title);
      setCategory(editingTask.category || "GENERAL");
      setFirstAction(editingTask.first_physical_action || "");
      setDod(editingTask.definition_of_done || "");
      setWorkLineId(editingTask.work_line_id || "");
      setOutcomeId(editingTask.outcome_id || "");
      setEnergyTag(editingTask.energy_tag || "MEDIUM_NORMAL");
      setDeviceTag(editingTask.device_tag || "LAPTOP_ONLY");
      setDurationMinutes(editingTask.duration_minutes || 25);
      setPriorityScore(editingTask.priority_score ?? 50);
      setScheduledDate(editingTask.scheduled_date || "");
    } else {
      setTitle("");
      setCategory("GENERAL");
      setFirstAction("");
      setDod("");
      setWorkLineId("");
      setOutcomeId("");
      setEnergyTag("MEDIUM_NORMAL");
      setDeviceTag("LAPTOP_ONLY");
      setDurationMinutes(25);
      setPriorityScore(50);
      setScheduledDate("");
    }
  }, [open, editingTask]);

  if (!open) return null;

  const workLineOptions = [
    { value: "", label: "تسک مستقل (بدون لاین کاری)" },
    ...workLines.map((w) => ({
      value: w.id,
      label: `[${w.id}] ${w.title} (${
        w.line_mode === "SEQUENTIAL_CHAIN" ? "پشت‌به‌پشت" : "موازی"
      })`,
    })),
  ];

  const outcomeOptions = [
    { value: "", label: "ارث‌بری از لاین کاری / بدون Outcome" },
    ...outcomes.map((o) => ({ value: o.id, label: `[${o.id}] ${o.title}` })),
  ];

  const handleSave = async () => {
    if ((!editingTask && !title.trim()) || isSaving) return;
    setIsSaving(true);
    try {
      if (editingTask) {
        await onAssign({
          taskId: editingTask.id,
          workLineId: workLineId || null,
          outcomeId: outcomeId || null,
          priorityScore,
          energyTag,
          deviceTag,
          durationMinutes,
          scheduledDate: scheduledDate.trim(),
        });
      } else {
        await onCreate({
          title: title.trim(),
          category,
          definitionOfDone: dod.trim(),
          firstPhysicalAction: firstAction.trim(),
          workLineId: workLineId || null,
          outcomeId: outcomeId || null,
          energyTag,
          deviceTag,
          durationMinutes,
          scheduledDate: scheduledDate.trim() || null,
        });
      }
      onClose();
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div
      className="fixed inset-0 bg-black/75 backdrop-blur-md flex items-center justify-center p-3 sm:p-4 z-50 select-text"
      onClick={onClose}
    >
      <div
        className="bg-mab-surface border border-mab-border w-full max-w-[540px] max-h-[90vh] overflow-y-auto p-5 sm:p-6 space-y-4 rounded-2xl shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-mab-border pb-3">
          <div className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-lg bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
              <Layers className="w-4 h-4" />
            </div>
            <h3 className="text-[14px] font-bold text-white">
              {editingTask
                ? `تخصیص و زمان‌بندی تسک [${editingTask.id}]`
                : "ساخت تسک جدید در پروژه"}
            </h3>
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
          {editingTask ? (
            <div className="p-3 rounded-xl bg-white/[0.03] border border-mab-border text-[13px] font-bold text-white">
              {editingTask.title}
            </div>
          ) : (
            <>
              <div className="space-y-1">
                <label className="text-[11.5px] font-semibold text-white/75">
                  عنوان تسک:
                </label>
                <input
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="مثلاً: طراحی جدول نرخ‌ها در صفحه قیمت‌گذاری"
                  className="w-full mab-input px-3 py-2 text-[12.5px]"
                />
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div className="space-y-1">
                  <label className="text-[11.5px] font-semibold text-white/75">
                    دسته‌بندی:
                  </label>
                  <MabSelect
                    value={category}
                    onChange={setCategory}
                    options={CATEGORY_OPTIONS}
                    className="w-full"
                  />
                </div>
                <div className="space-y-1">
                  <label className="text-[11.5px] font-semibold text-white/75">
                    اقدام شروع ۲ دقیقه‌ای:
                  </label>
                  <input
                    value={firstAction}
                    onChange={(e) => setFirstAction(e.target.value)}
                    placeholder="اولین قدم کوچک برای شروع..."
                    className="w-full mab-input px-3 py-2 text-[12.5px]"
                  />
                </div>
              </div>

              <div className="space-y-1">
                <label className="text-[11.5px] font-semibold text-white/75">
                  شرط تکمیل قطعی (DoD):
                </label>
                <input
                  value={dod}
                  onChange={(e) => setDod(e.target.value)}
                  placeholder="چه زمانی این تسک ۱۰۰٪ تمام‌شده محسوب می‌شود؟"
                  className="w-full mab-input px-3 py-2 text-[12.5px]"
                />
              </div>
            </>
          )}

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                لاین کاری (WorkLine):
              </label>
              <MabSelect
                value={workLineId}
                onChange={setWorkLineId}
                options={workLineOptions}
                className="w-full"
              />
            </div>

            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                دستاورد هدف (Outcome):
              </label>
              <MabSelect
                value={outcomeId}
                onChange={setOutcomeId}
                options={outcomeOptions}
                className="w-full"
              />
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                سطح انرژی مورد نیاز:
              </label>
              <MabSelect
                value={energyTag}
                onChange={setEnergyTag}
                options={ENERGY_OPTIONS}
                className="w-full"
              />
            </div>

            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                وسیله مورد نیاز:
              </label>
              <MabSelect
                value={deviceTag}
                onChange={setDeviceTag}
                options={DEVICE_OPTIONS}
                className="w-full"
              />
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                زمان تخمینی (دقیقه):
              </label>
              <input
                type="number"
                min={5}
                max={480}
                value={durationMinutes}
                onChange={(e) => setDurationMinutes(Number(e.target.value) || 25)}
                className="w-full mab-input px-3 py-2 text-[12.5px] font-mono"
              />
            </div>

            {editingTask && (
              <div className="space-y-1">
                <label className="text-[11.5px] font-semibold text-white/75">
                  امتیاز اولویت (1-100):
                </label>
                <input
                  type="number"
                  min={1}
                  max={100}
                  value={priorityScore}
                  onChange={(e) =>
                    setPriorityScore(
                      Math.max(1, Math.min(100, Number(e.target.value) || 50))
                    )
                  }
                  className="w-full mab-input px-3 py-2 text-[12.5px] font-mono"
                />
              </div>
            )}

            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                تاریخ برنامه‌ریزی (اختیاری):
              </label>
              <input
                type="date"
                value={scheduledDate}
                onChange={(e) => setScheduledDate(e.target.value)}
                className="w-full mab-input px-3 py-2 text-[12px] font-mono bidi-ltr"
              />
            </div>
          </div>
        </div>

        <div className="flex justify-end gap-2 pt-3 border-t border-mab-border">
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
            disabled={(!editingTask && !title.trim()) || isSaving}
            className="px-5 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-[12.5px] transition disabled:opacity-40 shadow-sm"
          >
            {isSaving
              ? "در حال ذخیره..."
              : editingTask
              ? "ذخیره تخصیص"
              : "ساخت تسک"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default TaskModal;
