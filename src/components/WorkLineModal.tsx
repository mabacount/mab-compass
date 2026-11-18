import { useEffect, useState } from "react";
import { GitBranch, Plus, Trash2, X } from "lucide-react";
import { ChainStepInput, Outcome, WorkLine } from "../lib/tauri_api";
import { MabSelect } from "./ui/MabSelect";

export interface WorkLineCreatePayload {
  outcome_id?: string | null;
  title: string;
  goal_statement: string;
  line_mode: "SEQUENTIAL_CHAIN" | "GOAL_CLUSTER";
  color_hex: string;
  steps: ChainStepInput[];
}

export interface WorkLineUpdatePayload {
  workLineId: string;
  title: string;
  goalStatement: string;
  outcomeId?: string | null;
  colorHex: string;
  status: string;
}

interface WorkLineModalProps {
  open: boolean;
  outcomes: Outcome[];
  initialWorkLine?: WorkLine | null;
  onClose: () => void;
  onCreate: (payload: WorkLineCreatePayload) => Promise<void> | void;
  onUpdate: (payload: WorkLineUpdatePayload) => Promise<void> | void;
}

const COLOR_SWATCHES = [
  "#3B82F6", // Blue
  "#10B981", // Emerald
  "#F59E0B", // Amber
  "#A855F7", // Purple
  "#EC4899", // Pink
  "#06B6D4", // Cyan
  "#EF4444", // Rose
];

const MODE_OPTIONS = [
  {
    value: "SEQUENTIAL_CHAIN",
    label: "زنجیره پشت‌به‌پشت (گام بعدی تا اتمام گام قبلی قفل است)",
  },
  {
    value: "GOAL_CLUSTER",
    label: "خوشه هم‌هدف موازی (همه گام‌ها هم‌زمان باز هستند)",
  },
];

const STATUS_OPTIONS = [
  { value: "ACTIVE", label: "فعال (ACTIVE)" },
  { value: "PAUSED", label: "متوقف (PAUSED)" },
  { value: "COMPLETED", label: "تکمیل‌شده (COMPLETED)" },
];

export function WorkLineModal({
  open,
  outcomes,
  initialWorkLine,
  onClose,
  onCreate,
  onUpdate,
}: WorkLineModalProps) {
  const [title, setTitle] = useState("");
  const [goalStatement, setGoalStatement] = useState("");
  const [outcomeId, setOutcomeId] = useState("");
  const [lineMode, setLineMode] = useState<"SEQUENTIAL_CHAIN" | "GOAL_CLUSTER">(
    "SEQUENTIAL_CHAIN"
  );
  const [colorHex, setColorHex] = useState("#3B82F6");
  const [status, setStatus] = useState("ACTIVE");
  const [steps, setSteps] = useState<Array<{ title: string; action: string }>>([
    { title: "", action: "" },
  ]);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    if (initialWorkLine) {
      setTitle(initialWorkLine.title || "");
      setGoalStatement(initialWorkLine.goal_statement || "");
      setOutcomeId(initialWorkLine.outcome_id || "");
      setLineMode(
        initialWorkLine.line_mode === "GOAL_CLUSTER"
          ? "GOAL_CLUSTER"
          : "SEQUENTIAL_CHAIN"
      );
      setColorHex(initialWorkLine.color_hex || "#3B82F6");
      setStatus(initialWorkLine.status || "ACTIVE");
      setSteps([]);
    } else {
      setTitle("");
      setGoalStatement("");
      setOutcomeId(outcomes[0]?.id || "");
      setLineMode("SEQUENTIAL_CHAIN");
      setColorHex("#3B82F6");
      setStatus("ACTIVE");
      setSteps([{ title: "", action: "" }]);
    }
  }, [open, initialWorkLine, outcomes]);

  if (!open) return null;

  const outcomeOptions = [
    { value: "", label: "بدون اتصال به Outcome (آزاد)" },
    ...outcomes.map((o) => ({ value: o.id, label: `[${o.id}] ${o.title}` })),
  ];

  const handleSave = async () => {
    if (!title.trim() || isSaving) return;
    setIsSaving(true);
    try {
      if (initialWorkLine) {
        await onUpdate({
          workLineId: initialWorkLine.id,
          title: title.trim(),
          goalStatement: goalStatement.trim(),
          outcomeId: outcomeId || null,
          colorHex,
          status,
        });
      } else {
        const cleanSteps: ChainStepInput[] = steps
          .filter((s) => s.title.trim().length > 0)
          .map((s) => ({
            title: s.title.trim(),
            first_physical_action: s.action.trim(),
            category: "GENERAL",
          }));
        await onCreate({
          outcome_id: outcomeId || null,
          title: title.trim(),
          goal_statement: goalStatement.trim(),
          line_mode: lineMode,
          color_hex: colorHex,
          steps: cleanSteps,
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
        className="bg-mab-surface border border-mab-border w-full max-w-[560px] max-h-[90vh] overflow-y-auto p-5 sm:p-6 space-y-4 rounded-2xl shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-mab-border pb-3">
          <div className="flex items-center gap-2">
            <div
              className="w-7 h-7 rounded-lg flex items-center justify-center border"
              style={{
                color: colorHex,
                borderColor: `${colorHex}55`,
                backgroundColor: `${colorHex}1A`,
              }}
            >
              <GitBranch className="w-4 h-4" />
            </div>
            <h3 className="text-[14px] font-bold text-white">
              {initialWorkLine
                ? `ویرایش لاین کاری (${initialWorkLine.id})`
                : "ساخت لاین کاری جدید (WorkLine)"}
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
          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/75">
              عنوان لاین کاری:
            </label>
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="مثلاً: پیاده‌سازی درگاه پرداخت و فاکتور خودکار"
              className="w-full mab-input px-3 py-2 text-[12.5px]"
            />
          </div>

          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/75">
              متصل به Outcome (خروجی نهایی):
            </label>
            <MabSelect
              value={outcomeId}
              onChange={setOutcomeId}
              options={outcomeOptions}
              className="w-full"
            />
          </div>

          {!initialWorkLine ? (
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                حالت اجرای گام‌ها:
              </label>
              <MabSelect
                value={lineMode}
                onChange={(v) =>
                  setLineMode(
                    v === "GOAL_CLUSTER" ? "GOAL_CLUSTER" : "SEQUENTIAL_CHAIN"
                  )
                }
                options={MODE_OPTIONS}
                className="w-full"
              />
            </div>
          ) : (
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                وضعیت لاین:
              </label>
              <MabSelect
                value={status}
                onChange={setStatus}
                options={STATUS_OPTIONS}
                className="w-full"
              />
            </div>
          )}

          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/75">
              هدف لاین (Goal Statement):
            </label>
            <input
              value={goalStatement}
              onChange={(e) => setGoalStatement(e.target.value)}
              placeholder="چه دستاورد ملموسی در پایان این لاین تحویل داده می‌شود؟"
              className="w-full mab-input px-3 py-2 text-[12.5px]"
            />
          </div>

          {/* Color Swatches */}
          <div className="space-y-1.5">
            <label className="text-[11.5px] font-semibold text-white/75">
              رنگ تشخیصی لاین:
            </label>
            <div className="flex items-center gap-2 flex-wrap">
              {COLOR_SWATCHES.map((hex) => (
                <button
                  key={hex}
                  type="button"
                  onClick={() => setColorHex(hex)}
                  className={`w-7 h-7 rounded-full transition ${
                    colorHex === hex
                      ? "ring-2 ring-white ring-offset-2 ring-offset-mab-surface scale-110"
                      : "opacity-70 hover:opacity-100"
                  }`}
                  style={{ backgroundColor: hex }}
                />
              ))}
            </div>
          </div>

          {/* Initial Steps Builder (Create Mode) */}
          {!initialWorkLine && (
            <div className="space-y-2 pt-2 border-t border-mab-border">
              <div className="flex items-center justify-between">
                <span className="text-[12px] font-bold text-white">
                  گام‌های اولیه لاین (اختیاری):
                </span>
                <button
                  type="button"
                  onClick={() =>
                    setSteps((prev) => [...prev, { title: "", action: "" }])
                  }
                  className="px-2.5 py-1 rounded-lg bg-blue-500/15 hover:bg-blue-500/25 border border-blue-500/30 text-blue-300 text-[11px] font-semibold flex items-center gap-1 transition"
                >
                  <Plus className="w-3 h-3" />
                  <span>افزودن گام</span>
                </button>
              </div>

              <div className="space-y-2 max-h-48 overflow-y-auto pl-1">
                {steps.map((st, idx) => (
                  <div
                    key={idx}
                    className="p-2.5 rounded-xl bg-white/[0.02] border border-mab-border space-y-1.5"
                  >
                    <div className="flex items-center gap-2">
                      <span className="px-1.5 py-0.5 rounded bg-white/[0.06] font-mono text-[10px] text-white/70 shrink-0">
                        #{idx + 1}
                      </span>
                      <input
                        value={st.title}
                        onChange={(e) => {
                          const v = e.target.value;
                          setSteps((prev) =>
                            prev.map((item, i) =>
                              i === idx ? { ...item, title: v } : item
                            )
                          );
                        }}
                        placeholder={`عنوان گام ${idx + 1}...`}
                        className="flex-1 mab-input px-2.5 py-1 text-[12px]"
                      />
                      {steps.length > 1 && (
                        <button
                          type="button"
                          onClick={() =>
                            setSteps((prev) => prev.filter((_, i) => i !== idx))
                          }
                          className="p-1 text-white/40 hover:text-rose-400 transition"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                    <input
                      value={st.action}
                      onChange={(e) => {
                        const v = e.target.value;
                        setSteps((prev) =>
                          prev.map((item, i) =>
                            i === idx ? { ...item, action: v } : item
                          )
                        );
                      }}
                      placeholder="اولین اقدام فیزیکی ۲ دقیقه‌ای (اختیاری)..."
                      className="w-full mab-input px-2.5 py-1 text-[11.5px]"
                    />
                  </div>
                ))}
              </div>
            </div>
          )}
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
            disabled={!title.trim() || isSaving}
            className="px-5 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-[12.5px] transition disabled:opacity-40 shadow-sm"
          >
            {isSaving
              ? "در حال ذخیره..."
              : initialWorkLine
              ? "ذخیره لاین کاری"
              : "ساخت لاین کاری"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default WorkLineModal;
