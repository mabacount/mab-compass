// src/components/QuickModelSwitcher.tsx
import { useEffect, useRef, useState } from "react";
import {
  Check,
  Cpu,
  Globe,
  Pin,
  RefreshCw,
  Search,
  Settings,
  Sparkles,
  Zap,
} from "lucide-react";
import { AppSettings, CompassApi, LiveModelOption } from "../lib/tauri_api";

export interface MruModelEntry {
  provider: string; // "AGY_CLI" | "LOCAL_OPENAI" | "GEMINI_REST"
  modelId: string;
  label: string;
  categoryTag?: string;
}

const MRU_STORAGE_KEY = "mab_mru_models_v1";

const DEFAULT_AGY_MODELS: { id: string; label: string; desc: string }[] = [
  { id: "gemini-3.8-flash-high", label: "Gemini 3.8 Flash (High)", desc: "سرعت بسیار بالا با هوش و استدلال عمیق" },
  { id: "gemini-3.8-flash-medium", label: "Gemini 3.8 Flash (Medium)", desc: "بهینه و متعادل برای کارهای عمومی" },
  { id: "gemini-3.8-flash-low", label: "Gemini 3.8 Flash (Low)", desc: "سریع‌ترین حالت کم‌مصرف" },
  { id: "claude-sonnet-4-6", label: "Claude Sonnet 4.6 (Thinking)", desc: "تفکر ساختاریافته و استدلال عالی برای کد و متن" },
  { id: "claude-opus-4-6-thinking", label: "Claude Opus 4.6 (Thinking)", desc: "قدرتمندترین مدل تحلیلی برای مسائل پیچیده" },
  { id: "gpt-oss-120b-medium", label: "GPT-OSS 120B (Medium)", desc: "مدل متن‌باز ۱۲۰ میلیاردی متوازن" },
];

const DEFAULT_GEMINI_MODELS: { id: string; label: string; desc: string }[] = [
  { id: "gemini-2.5-flash", label: "Gemini 2.5 Flash", desc: "مدل پرسرعت و سبک گوگل" },
  { id: "gemini-2.5-pro", label: "Gemini 2.5 Pro", desc: "مدل باکیفیت و جامع گوگل" },
  { id: "gemini-1.5-flash", label: "Gemini 1.5 Flash", desc: "نسخه نسل قبل پرسرعت" },
];

export interface QuickModelSwitcherProps {
  isOpen: boolean;
  onClose: () => void;
  settings: AppSettings;
  onSelectModel: (provider: string, modelId: string, modelLabel: string) => void;
  onOpenSettingsTab: () => void;
}

export function QuickModelSwitcher({
  isOpen,
  onClose,
  settings,
  onSelectModel,
  onOpenSettingsTab,
}: QuickModelSwitcherProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [search, setSearch] = useState("");
  const [localLiveModels, setLocalLiveModels] = useState<LiveModelOption[]>([]);
  const [isLoadingLocal, setIsLoadingLocal] = useState(false);

  // MRU list state (Top 4 pinned)
  const [mruModels, setMruModels] = useState<MruModelEntry[]>(() => {
    try {
      const saved = localStorage.getItem(MRU_STORAGE_KEY);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) {
          return parsed.slice(0, 4);
        }
      }
    } catch {
      // ignore
    }
    return [
      { provider: "AGY_CLI", modelId: "gemini-3.8-flash-high", label: "Gemini 3.8 Flash (High)", categoryTag: "Antigravity" },
      { provider: "AGY_CLI", modelId: "claude-sonnet-4-6", label: "Claude Sonnet 4.6 (Thinking)", categoryTag: "Antigravity" },
      { provider: "LOCAL_OPENAI", modelId: settings.local_model || "LM Studio Local", label: settings.local_model || "LM Studio Local", categoryTag: "Local" },
      { provider: "AGY_CLI", modelId: "gemini-3.8-flash-medium", label: "Gemini 3.8 Flash (Medium)", categoryTag: "Antigravity" },
    ];
  });

  // Fetch live local models on open
  useEffect(() => {
    if (!isOpen) return;
    let active = true;
    setIsLoadingLocal(true);
    CompassApi.fetchLiveModels(
      "LOCAL_OPENAI",
      settings.local_endpoint || "http://127.0.0.1:1234/v1/chat/completions",
      settings.gemini_api_key || ""
    )
      .then((res) => {
        if (!active) return;
        if (res.models && res.models.length > 0) {
          setLocalLiveModels(res.models);
        }
      })
      .catch(() => {
        // Local LM Studio might not be running, that's okay
      })
      .finally(() => {
        if (active) setIsLoadingLocal(false);
      });

    return () => {
      active = false;
    };
  }, [isOpen, settings.local_endpoint]);

  // Handle outside click
  useEffect(() => {
    if (!isOpen) return;
    const handleOutsideClick = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    document.addEventListener("mousedown", handleOutsideClick);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handleOutsideClick);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const currentProvider = settings.active_llm_provider;
  const currentModelId =
    currentProvider === "AGY_CLI"
      ? settings.agy_model
      : currentProvider === "LOCAL_OPENAI"
      ? settings.local_model
      : settings.gemini_model;

  const handleChoose = (provider: string, modelId: string, label: string) => {
    // Update MRU
    const newEntry: MruModelEntry = {
      provider,
      modelId,
      label,
      categoryTag: provider === "AGY_CLI" ? "Antigravity" : provider === "LOCAL_OPENAI" ? "Local" : "Cloud",
    };
    const nextMru = [
      newEntry,
      ...mruModels.filter((m) => !(m.provider === provider && m.modelId === modelId)),
    ].slice(0, 4);

    setMruModels(nextMru);
    try {
      localStorage.setItem(MRU_STORAGE_KEY, JSON.stringify(nextMru));
    } catch {
      // ignore
    }

    onSelectModel(provider, modelId, label);
    onClose();
  };

  const q = search.trim().toLowerCase();

  const filteredAgy = DEFAULT_AGY_MODELS.filter(
    (m) => !q || m.id.toLowerCase().includes(q) || m.label.toLowerCase().includes(q)
  );

  const filteredLocal = (
    localLiveModels.length > 0
      ? localLiveModels
      : [{ id: settings.local_model || "default", label: settings.local_model || "LM Studio (پیش‌فرض)" }]
  ).filter((m) => !q || m.id.toLowerCase().includes(q) || m.label.toLowerCase().includes(q));

  const filteredGemini = DEFAULT_GEMINI_MODELS.filter(
    (m) => !q || m.id.toLowerCase().includes(q) || m.label.toLowerCase().includes(q)
  );

  return (
    <>
      {/* Click-outside backdrop to catch any click outside and prevent accidental trigger */}
      <div
        className="fixed inset-0 z-40 bg-transparent"
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
      />

      <div
        ref={containerRef}
        dir="rtl"
        className="absolute bottom-full mb-2.5 left-0 w-[350px] max-w-[calc(100vw-32px)] max-h-[min(480px,calc(100vh-160px))] bg-[#14151B] border border-white/[0.14] rounded-xl shadow-2xl flex flex-col z-50 overflow-hidden text-right select-none animate-in fade-in slide-in-from-bottom-2 duration-150 backdrop-blur-md"
      >
      {/* Header with Search */}
      <div className="p-2.5 border-b border-white/[0.08] bg-[#181A22] space-y-2 shrink-0">
        <div className="flex items-center justify-between">
          <span className="text-[12px] font-bold text-white/90 flex items-center gap-1.5">
            <Sparkles className="w-3.5 h-3.5 text-blue-400" />
            <span>انتخاب سریع مدل هوش مصنوعی</span>
          </span>
          <span className="text-[10px] text-white/40 font-mono">
            {currentProvider === "AGY_CLI" ? "Antigravity CLI" : currentProvider === "LOCAL_OPENAI" ? "LM Studio" : "Gemini API"}
          </span>
        </div>

        <div className="relative">
          <Search className="w-3.5 h-3.5 absolute right-2.5 top-2.5 text-white/40 pointer-events-none" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="جستجوی مدل..."
            className="w-full pl-3 pr-8 py-1.5 rounded-lg bg-black/40 border border-white/[0.08] text-[11.5px] text-white placeholder:text-white/30 focus:outline-none focus:border-blue-500/50"
            autoFocus
          />
        </div>
      </div>

      {/* Scrollable Model Lists */}
      <div className="flex-1 min-h-0 overflow-y-auto p-1.5 space-y-3">
        {/* SECTION 1: Pinned 4 MRU Models */}
        {!q && mruModels.length > 0 && (
          <div className="space-y-1">
            <div className="px-2 py-1 flex items-center gap-1 text-[10.5px] font-semibold text-amber-300/90 tracking-wide">
              <Pin className="w-3 h-3 fill-amber-300/30 text-amber-300" />
              <span>۴ مدل اخیر استفاده‌شده (پین‌شده)</span>
            </div>
            <div className="space-y-0.5">
              {mruModels.map((m, idx) => {
                const isSelected = currentProvider === m.provider && currentModelId === m.modelId;
                return (
                  <button
                    key={`mru_${m.provider}_${m.modelId}_${idx}`}
                    type="button"
                    onClick={() => handleChoose(m.provider, m.modelId, m.label)}
                    className={`w-full text-right px-2.5 py-1.5 rounded-lg text-[12px] flex items-center justify-between transition ${
                      isSelected
                        ? "bg-blue-600/20 text-blue-300 border border-blue-500/30 font-medium"
                        : "text-white/85 hover:bg-white/[0.06] hover:text-white"
                    }`}
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0" />
                      <div className="truncate">
                        <span className="font-mono text-[11px] block truncate">{m.label}</span>
                        <span className="text-[9.5px] text-white/40">{m.categoryTag || m.provider}</span>
                      </div>
                    </div>
                    {isSelected && <Check className="w-3.5 h-3.5 text-blue-400 shrink-0" />}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* SECTION 2: Antigravity CLI Models */}
        {filteredAgy.length > 0 && (
          <div className="space-y-1">
            <div className="px-2 py-1 flex items-center justify-between text-[10.5px] font-semibold text-blue-300/90 tracking-wide">
              <span className="flex items-center gap-1">
                <Zap className="w-3 h-3 text-blue-400" />
                <span>آنتی‌گرویتی (Antigravity CLI)</span>
              </span>
              <span className="text-[9.5px] text-emerald-400 bg-emerald-500/10 px-1 rounded">سریع و رایگان</span>
            </div>
            <div className="space-y-0.5">
              {filteredAgy.map((m) => {
                const isSelected = currentProvider === "AGY_CLI" && currentModelId === m.id;
                return (
                  <button
                    key={`agy_${m.id}`}
                    type="button"
                    onClick={() => handleChoose("AGY_CLI", m.id, m.label)}
                    className={`w-full text-right px-2.5 py-1.5 rounded-lg text-[12px] flex items-center justify-between transition ${
                      isSelected
                        ? "bg-blue-600/20 text-blue-300 border border-blue-500/30 font-medium"
                        : "text-white/85 hover:bg-white/[0.06] hover:text-white"
                    }`}
                  >
                    <div className="min-w-0 flex-1 pr-1">
                      <div className="flex items-center gap-1.5">
                        <span className="font-mono text-[11px] font-medium text-white truncate">{m.id}</span>
                        {m.id === "gemini-3.8-flash-high" && (
                          <span className="text-[9px] bg-blue-500/20 text-blue-300 px-1 rounded">پیش‌فرض</span>
                        )}
                      </div>
                      <p className="text-[10px] text-white/45 truncate mt-0.5">{m.desc}</p>
                    </div>
                    {isSelected && <Check className="w-3.5 h-3.5 text-blue-400 shrink-0" />}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* SECTION 3: LM Studio Local Models */}
        {filteredLocal.length > 0 && (
          <div className="space-y-1">
            <div className="px-2 py-1 flex items-center justify-between text-[10.5px] font-semibold text-purple-300/90 tracking-wide">
              <span className="flex items-center gap-1">
                <Cpu className="w-3 h-3 text-purple-400" />
                <span>مدل محلی (LM Studio)</span>
              </span>
              {isLoadingLocal ? (
                <RefreshCw className="w-2.5 h-2.5 animate-spin text-purple-400" />
              ) : (
                <span className="text-[9.5px] text-purple-300 bg-purple-500/10 px-1 rounded">آفلاین / امن</span>
              )}
            </div>
            <div className="space-y-0.5">
              {filteredLocal.map((m) => {
                const isSelected = currentProvider === "LOCAL_OPENAI" && currentModelId === m.id;
                return (
                  <button
                    key={`local_${m.id}`}
                    type="button"
                    onClick={() => handleChoose("LOCAL_OPENAI", m.id, m.label)}
                    className={`w-full text-right px-2.5 py-1.5 rounded-lg text-[12px] flex items-center justify-between transition ${
                      isSelected
                        ? "bg-purple-600/20 text-purple-300 border border-purple-500/30 font-medium"
                        : "text-white/85 hover:bg-white/[0.06] hover:text-white"
                    }`}
                  >
                    <div className="min-w-0 flex-1 pr-1">
                      <span className="font-mono text-[11px] font-medium text-white truncate block">{m.label}</span>
                      <p className="text-[10px] text-white/45 truncate mt-0.5 font-mono">{m.id}</p>
                    </div>
                    {isSelected && <Check className="w-3.5 h-3.5 text-purple-400 shrink-0" />}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* SECTION 4: Gemini REST API */}
        {filteredGemini.length > 0 && (
          <div className="space-y-1">
            <div className="px-2 py-1 flex items-center justify-between text-[10.5px] font-semibold text-emerald-300/90 tracking-wide">
              <span className="flex items-center gap-1">
                <Globe className="w-3 h-3 text-emerald-400" />
                <span>گوگل جمینای (Gemini REST API)</span>
              </span>
              <span className="text-[9.5px] text-white/40 font-mono">
                {settings.gemini_api_key ? "کلید فعال" : "نیازمند کلید"}
              </span>
            </div>
            <div className="space-y-0.5">
              {filteredGemini.map((m) => {
                const isSelected = currentProvider === "GEMINI_REST" && currentModelId === m.id;
                return (
                  <button
                    key={`gemini_${m.id}`}
                    type="button"
                    onClick={() => handleChoose("GEMINI_REST", m.id, m.label)}
                    className={`w-full text-right px-2.5 py-1.5 rounded-lg text-[12px] flex items-center justify-between transition ${
                      isSelected
                        ? "bg-emerald-600/20 text-emerald-300 border border-emerald-500/30 font-medium"
                        : "text-white/85 hover:bg-white/[0.06] hover:text-white"
                    }`}
                  >
                    <div className="min-w-0 flex-1 pr-1">
                      <span className="font-mono text-[11px] font-medium text-white truncate block">{m.label}</span>
                      <p className="text-[10px] text-white/45 truncate mt-0.5">{m.desc}</p>
                    </div>
                    {isSelected && <Check className="w-3.5 h-3.5 text-emerald-400 shrink-0" />}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {/* Footer Link to Advanced Settings */}
      <div className="p-2 border-t border-white/[0.08] bg-[#16171E] shrink-0">
        <button
          type="button"
          onClick={() => {
            onClose();
            onOpenSettingsTab();
          }}
          className="w-full px-2.5 py-1.5 rounded-lg text-[11.5px] text-white/70 hover:text-white hover:bg-white/[0.06] flex items-center justify-center gap-1.5 transition"
        >
          <Settings className="w-3 h-3 text-white/50" />
          <span>تنظیمات پیشرفته هوش مصنوعی و کلیدها...</span>
        </button>
      </div>
    </div>
    </>
  );
}

export default QuickModelSwitcher;
