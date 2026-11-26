import { useEffect, useState } from "react";
import { FolderPlus, X } from "lucide-react";
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

interface NewProjectModalProps {
  open: boolean;
  projects?: Project[];
  defaultParentId?: string | null;
  onClose: () => void;
  onCreate: (
    idCode: string,
    name: string,
    options?: {
      parentId?: string | null;
      realm?: string;
      color?: string;
    }
  ) => void;
}

export function NewProjectModal({
  open,
  projects = [],
  defaultParentId = null,
  onClose,
  onCreate,
}: NewProjectModalProps) {
  const [newProjCode, setNewProjCode] = useState("");
  const [newProjName, setNewProjName] = useState("");
  const [parentId, setParentId] = useState<string>(defaultParentId ?? "");
  const [realm, setRealm] = useState<"WORK" | "LIFE">("WORK");
  const [color, setColor] = useState("#3B82F6");

  useEffect(() => {
    if (open) {
      setParentId(defaultParentId ?? "");
    }
  }, [open, defaultParentId]);

  if (!open) return null;

  const handleSubmit = () => {
    if (!newProjCode.trim() || !newProjName.trim()) return;
    onCreate(newProjCode, newProjName, {
      parentId: parentId.trim() || null,
      realm,
      color,
    });
    setNewProjCode("");
    setNewProjName("");
    setParentId("");
    setRealm("WORK");
    setColor("#3B82F6");
  };

  return (
    <div
      className="fixed inset-0 bg-black/75 backdrop-blur-md flex items-center justify-center p-4 z-50 select-text"
      onClick={onClose}
    >
      <div
        className="bg-mab-surface border border-mab-border w-full max-w-[460px] max-h-[90vh] overflow-y-auto p-5 sm:p-6 space-y-4 rounded-2xl shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-mab-border pb-3">
          <div className="flex items-center gap-2">
            <div className="w-7 h-7 rounded-lg bg-blue-500/15 border border-blue-500/30 flex items-center justify-center text-blue-400">
              <FolderPlus className="w-4 h-4" />
            </div>
            <h3 className="text-[14px] font-bold text-white">
              ساخت پروژه جدید در اکوسیستم یکپارچه
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
          با ساخت پروژه یا زیرپروژه، ساختار درختی آن هم‌زمان در دیتابیس مرکزی MabEcosystem، مب‌کامپس و مب‌فوکس ثبت می‌شود.
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

          {projects.length > 0 && (
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/70">
                پروژه والد (اختیاری - برای ساخت زیرپروژه):
              </label>
              <select
                value={parentId}
                onChange={(e) => setParentId(e.target.value)}
                className="w-full mab-input px-3 py-2 text-[12px] bg-[#14151A]"
              >
                <option value="">پروژه ریشه (بدون والد)</option>
                {projects.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </div>
          )}

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/70">
                قلمرو (Realm):
              </label>
              <div className="flex gap-1.5">
                <button
                  type="button"
                  onClick={() => setRealm("WORK")}
                  className={`flex-1 py-1.5 rounded-lg text-[11.5px] border transition ${
                    realm === "WORK"
                      ? "bg-blue-500/20 border-blue-500/40 text-blue-300 font-semibold"
                      : "bg-white/[0.03] border-white/[0.08] text-white/60"
                  }`}
                >
                  کار (Work)
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setRealm("LIFE");
                    if (color === "#3B82F6") setColor("#10B981");
                  }}
                  className={`flex-1 py-1.5 rounded-lg text-[11.5px] border transition ${
                    realm === "LIFE"
                      ? "bg-emerald-500/20 border-emerald-500/40 text-emerald-300 font-semibold"
                      : "bg-white/[0.03] border-white/[0.08] text-white/60"
                  }`}
                >
                  زندگی (Life)
                </button>
              </div>
            </div>

            <div className="space-y-1">
              <label className="text-[11.5px] font-semibold text-white/70">
                رنگ پروژه:
              </label>
              <div className="flex items-center gap-1.5 pt-1">
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
            onClick={handleSubmit}
            disabled={!newProjCode.trim() || !newProjName.trim()}
            className="px-5 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-[12.5px] transition disabled:opacity-40 shadow-sm"
          >
            ایجاد پروژه و همگام‌سازی
          </button>
        </div>
      </div>
    </div>
  );
}

export default NewProjectModal;
