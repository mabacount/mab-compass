import { useEffect, useState } from "react";
import { Target, X } from "lucide-react";
import { LifeDomain, Outcome } from "../lib/tauri_api";
import { MabSelect } from "./ui/MabSelect";

export interface OutcomeFormValues {
  outcomeId?: string | null;
  domainCode: string;
  title: string;
  desiredRealityChange: string;
  evidenceKr: string;
  timeWindow: string;
  status: string;
  progressPercent: number;
}

interface OutcomeModalProps {
  open: boolean;
  domains: LifeDomain[];
  initialOutcome?: Outcome | null;
  onClose: () => void;
  onSubmit: (values: OutcomeFormValues) => Promise<void> | void;
}

const STATUS_OPTIONS = [
  { value: "ACTIVE", label: "فعال (ACTIVE)" },
  { value: "PAUSED", label: "متوقف موقت (PAUSED)" },
  { value: "ACHIEVED", label: "محقق‌شده (ACHIEVED)" },
];

export function OutcomeModal({
  open,
  domains,
  initialOutcome,
  onClose,
  onSubmit,
}: OutcomeModalProps) {
  const [domainCode, setDomainCode] = useState("WEALTH_FINANCE");
  const [title, setTitle] = useState("");
  const [desiredRealityChange, setDesiredRealityChange] = useState("");
  const [evidenceKr, setEvidenceKr] = useState("");
  const [timeWindow, setTimeWindow] = useState("90 روزه");
  const [status, setStatus] = useState("ACTIVE");
  const [progressPercent, setProgressPercent] = useState(0);
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    if (initialOutcome) {
      setDomainCode(initialOutcome.domain_code || "WEALTH_FINANCE");
      setTitle(initialOutcome.title || "");
      setDesiredRealityChange(initialOutcome.desired_reality_change || "");
      setEvidenceKr(initialOutcome.evidence_kr || "");
      setTimeWindow(initialOutcome.time_window || "90 روزه");
      setStatus(initialOutcome.status || "ACTIVE");
      setProgressPercent(initialOutcome.progress_percent ?? 0);
    } else {
      setDomainCode(domains[0]?.code || "WEALTH_FINANCE");
      setTitle("");
      setDesiredRealityChange("");
      setEvidenceKr("");
      setTimeWindow("90 روزه");
      setStatus("ACTIVE");
      setProgressPercent(0);
    }
  }, [open, initialOutcome, domains]);

  if (!open) return null;

  const domainOptions =
    domains.length > 0
      ? domains.map((d) => ({ value: d.code, label: `${d.title} (${d.code})` }))
      : [
          { value: "WEALTH_FINANCE", label: "ثروت و حرفه (WEALTH_FINANCE)" },
          { value: "MIND_LEARNING", label: "ذهن و یادگیری (MIND_LEARNING)" },
          { value: "HEALTH_ENERGY", label: "سلامت و انرژی (HEALTH_ENERGY)" },
          { value: "PURPOSE", label: "معنا و رسالت (PURPOSE)" },
        ];

  const handleSave = async () => {
    if (!title.trim() || isSaving) return;
    setIsSaving(true);
    try {
      await onSubmit({
        outcomeId: initialOutcome?.id ?? null,
        domainCode,
        title: title.trim(),
        desiredRealityChange: desiredRealityChange.trim(),
        evidenceKr: evidenceKr.trim(),
        timeWindow: timeWindow.trim(),
        status,
        progressPercent,
      });
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
        className="bg-mab-surface border border-mab-border w-full max-w-[520px] max-h-[90vh] overflow-y-auto p-5 sm:p-6 space-y-4 rounded-2xl shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-mab-border pb-3">
          <div className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-lg bg-amber-500/15 border border-amber-500/30 flex items-center justify-center text-amber-400">
              <Target className="w-4 h-4" />
            </div>
            <h3 className="text-[14px] font-bold text-white">
              {initialOutcome
                ? `ویرایش Outcome (${initialOutcome.id})`
                : "ساخت Outcome (دستاورد هدف‌مند) جدید"}
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
              حوزه زندگی / کار (CADM Domain):
            </label>
            <MabSelect
              value={domainCode}
              onChange={setDomainCode}
              options={domainOptions}
              className="w-full"
            />
          </div>

          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/75">
              عنوان Outcome (نتیجه نهایی مطلوب):
            </label>
            <input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="مثلاً: انتشار نسخه پایدار محصول و جذب ۵ مشتری اول"
              className="w-full mab-input px-3 py-2 text-[12.5px]"
            />
          </div>

          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/75">
              تغییر واقعی در واقعیت (Desired Reality Change):
            </label>
            <textarea
              value={desiredRealityChange}
              onChange={(e) => setDesiredRealityChange(e.target.value)}
              rows={2}
              placeholder="پس از رسیدن به این Outcome چه چیزی در عمل تغییر کرده است؟"
              className="w-full mab-input px-3 py-2 text-[12.5px] resize-none"
            />
          </div>

          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/75">
              سنجه و شاهد عینی تحقق (Evidence / KR):
            </label>
            <input
              value={evidenceKr}
              onChange={(e) => setEvidenceKr(e.target.value)}
              placeholder="مثلاً: ۵ فاکتور پرداخت‌شده + ۰ باگ بحرانی"
              className="w-full mab-input px-3 py-2 text-[12.5px]"
            />
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                افق زمانی:
              </label>
              <input
                value={timeWindow}
                onChange={(e) => setTimeWindow(e.target.value)}
                placeholder="مثلاً: فصل پاییز / ۶۰ روز"
                className="w-full mab-input px-3 py-2 text-[12.5px]"
              />
            </div>

            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/75">
                وضعیت:
              </label>
              <MabSelect
                value={status}
                onChange={setStatus}
                options={STATUS_OPTIONS}
                className="w-full"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <div className="flex items-center justify-between text-[11.5px]">
              <span className="font-semibold text-white/75">درصد پیشرفت فعلی:</span>
              <span className="font-timer font-bold text-emerald-400">
                {progressPercent}%
              </span>
            </div>
            <input
              type="range"
              min={0}
              max={100}
              step={5}
              value={progressPercent}
              onChange={(e) => setProgressPercent(Number(e.target.value))}
              className="w-full accent-emerald-500 cursor-pointer"
            />
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
            disabled={!title.trim() || isSaving}
            className="px-5 py-2 rounded-xl bg-amber-600 hover:bg-amber-500 text-white font-semibold text-[12.5px] transition disabled:opacity-40 shadow-sm"
          >
            {isSaving
              ? "در حال ذخیره..."
              : initialOutcome
              ? "ذخیره تغییرات"
              : "ایجاد Outcome"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default OutcomeModal;
