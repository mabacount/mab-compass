import { useState } from "react";
import { FolderPlus, X } from "lucide-react";

interface NewProjectModalProps {
  open: boolean;
  onClose: () => void;
  onCreate: (idCode: string, name: string) => void;
}

export function NewProjectModal({ open, onClose, onCreate }: NewProjectModalProps) {
  const [newProjCode, setNewProjCode] = useState("");
  const [newProjName, setNewProjName] = useState("");

  if (!open) return null;

  const handleSubmit = () => {
    if (!newProjCode.trim() || !newProjName.trim()) return;
    onCreate(newProjCode, newProjName);
    setNewProjCode("");
    setNewProjName("");
  };

  return (
    <div
      className="fixed inset-0 bg-black/75 backdrop-blur-md flex items-center justify-center p-4 z-50 select-text"
      onClick={onClose}
    >
      <div
        className="bg-[#18191E] border border-white/[0.1] w-full max-w-[440px] p-6 space-y-4 rounded-2xl shadow-mab-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-white/[0.06] pb-3">
          <div className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-lg bg-blue-500/15 border border-blue-500/30 flex items-center justify-center text-blue-400">
              <FolderPlus className="w-4 h-4" />
            </div>
            <h3 className="text-[14px] font-bold text-white">
              ساخت پروژه جدید در اکوسیستم
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

        <p className="text-[11.5px] text-mab-text-muted leading-relaxed">
          با ساخت پروژه، ۸ زیرپوشه استاندارد در دایرکتوری ایجاد شده و اطلاعات با پایگاه محلی و Mab-Focus سینک می‌شود.
        </p>

        <div className="space-y-3">
          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/70">
              کد انگلیسی کوتاه پروژه (شناسه یکتا):
            </label>
            <input
              value={newProjCode}
              onChange={(e) => setNewProjCode(e.target.value.toUpperCase())}
              placeholder="مثلاً: ZIGHOOSH"
              className="w-full mab-input px-3 py-2 text-[12.5px] font-mono bidi-ltr"
            />
          </div>

          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/70">
              نام کامل فارسی پروژه:
            </label>
            <input
              value={newProjName}
              onChange={(e) => setNewProjName(e.target.value)}
              placeholder="مثلاً: پلتفرم هوشمند زیگهوش"
              className="w-full mab-input px-3 py-2 text-[12.5px]"
            />
          </div>
        </div>

        <div className="flex justify-end gap-2 pt-3 border-t border-white/[0.06]">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 rounded-xl bg-white/[0.04] hover:bg-white/[0.08] text-white/70 text-[12px] font-medium transition"
          >
            انصراف
          </button>
          <button
            type="button"
            onClick={handleSubmit}
            disabled={!newProjCode.trim() || !newProjName.trim()}
            className="px-5 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-[12.5px] transition disabled:opacity-40 shadow-sm"
          >
            ایجاد پروژه و پوشه‌ها
          </button>
        </div>
      </div>
    </div>
  );
}
