import { useCallback, useEffect, useRef, useState } from "react";
import {
  AlertTriangle,
  Check,
  CheckCircle2,
  Cpu,
  Eye,
  EyeOff,
  FolderGit2,
  HardDrive,
  RefreshCw,
  Save,
} from "lucide-react";
import {
  AppSettings,
  CompassApi,
  LiveModelsResponse,
} from "../lib/tauri_api";
import { MabSelect } from "./ui/MabSelect";
import { MabCheckbox } from "./ui/MabCheckbox";

interface SettingsViewProps {
  settingsForm: AppSettings;
  onChange: (next: AppSettings) => void;
  onSave: () => void;
  onQuickSaveSettings?: (next: AppSettings) => void;
}

const LLM_PROVIDER_OPTIONS = [
  {
    value: "AGY_CLI",
    label: "Google Antigravity CLI (agy) — اتصال خودکار به حساب فعال",
  },
  {
    value: "GEMINI_REST",
    label: "Google AI Studio API (Gemini REST) — با کلید اختصاصی",
  },
  {
    value: "LOCAL_OPENAI",
    label: "Local LM Studio (آفلاین محلی روی پورت سیستم)",
  },
];

export function SettingsView({
  settingsForm,
  onChange,
  onSave,
  onQuickSaveSettings,
}: SettingsViewProps) {
  const [liveByProvider, setLiveByProvider] = useState<
    Record<string, LiveModelsResponse>
  >({});
  const [isFetching, setIsFetching] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);

  const formRef = useRef(settingsForm);
  formRef.current = settingsForm;

  const activeProvider = settingsForm.active_llm_provider;

  const getCurrentModelForProvider = (s: AppSettings, provider: string): string => {
    if (provider === "GEMINI_REST") return s.gemini_model || "";
    if (provider === "LOCAL_OPENAI") return s.local_model || "";
    return s.agy_model || "";
  };

  const setModelForProvider = useCallback(
    (provider: string, modelId: string) => {
      const cur = formRef.current;
      if (provider === "GEMINI_REST") {
        onChange({ ...cur, gemini_model: modelId });
      } else if (provider === "LOCAL_OPENAI") {
        onChange({ ...cur, local_model: modelId });
      } else {
        onChange({ ...cur, agy_model: modelId });
      }
    },
    [onChange]
  );

  const handleFetchModels = useCallback(
    async (providerOverride?: string) => {
      const cur = formRef.current;
      const targetProvider = providerOverride || cur.active_llm_provider;
      setIsFetching(true);
      try {
        const res = await CompassApi.fetchLiveModels(
          targetProvider,
          cur.local_endpoint,
          cur.gemini_api_key
        );
        setLiveByProvider((prev) => ({ ...prev, [targetProvider]: res }));

        if (res.models.length > 0) {
          const currentSelected = getCurrentModelForProvider(cur, targetProvider);
          const stillValid = res.models.some((m) => m.id === currentSelected);
          if (!currentSelected.trim() || !stillValid) {
            if (!currentSelected.trim()) {
              setModelForProvider(targetProvider, res.models[0].id);
            }
          }
        }
      } catch (err: unknown) {
        setLiveByProvider((prev) => ({
          ...prev,
          [targetProvider]: {
            provider: targetProvider,
            models: [],
            live_fetched: false,
            status_message: `خطا در دریافت لیست مدل‌ها: ${String(err)}`,
          },
        }));
      } finally {
        setIsFetching(false);
      }
    },
    [setModelForProvider]
  );

  useEffect(() => {
    void handleFetchModels(activeProvider);
  }, [activeProvider, handleFetchModels]);

  const handleSaveClick = () => {
    onSave();
    setSaveSuccess(true);
    setTimeout(() => setSaveSuccess(false), 3000);
  };

  const activeLive = liveByProvider[activeProvider];
  const selectedModel = getCurrentModelForProvider(settingsForm, activeProvider);
  const liveModels = activeLive?.models ?? [];
  const hasSavedFallback =
    selectedModel.trim().length > 0 &&
    !liveModels.some((m) => m.id === selectedModel);

  const providerSourceHint =
    activeProvider === "LOCAL_OPENAI"
      ? "/v1/models (LM Studio)"
      : activeProvider === "GEMINI_REST"
      ? "/v1beta/models (Gemini API)"
      : "agy models (Antigravity CLI)";

  // Build model options for MabSelect
  const modelOptions = [
    ...(hasSavedFallback
      ? [{ value: selectedModel, label: `${selectedModel} (ذخیره‌شده فعلی)` }]
      : []),
    ...liveModels.map((m) => ({
      value: m.id,
      label: `${m.label}${m.details ? ` — ${m.details}` : ""}`,
    })),
  ];

  return (
    <div className="max-w-3xl mx-auto space-y-4 pb-8 select-text">
      {/* Header title */}
      <div className="flex items-center justify-between gap-3 px-1">
        <div>
          <h2 className="text-[17px] font-bold text-white tracking-tight">
            تنظیمات و ترجیحات سیستم
          </h2>
          <p className="text-[12px] text-mab-text-muted mt-0.5">
            پیکربندی هوش مصنوعی، دایرکتوری پروژه‌ها و ارتباط با Mab-Focus
          </p>
        </div>

        <button
          type="button"
          onClick={handleSaveClick}
          className="px-4 py-2 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-[12.5px] font-semibold flex items-center gap-1.5 shadow-sm transition"
        >
          {saveSuccess ? (
            <>
              <Check className="w-4 h-4 text-emerald-300" />
              <span>ذخیره شد!</span>
            </>
          ) : (
            <>
              <Save className="w-4 h-4" />
              <span>ذخیره تغییرات</span>
            </>
          )}
        </button>
      </div>

      {/* 1. LLM Model Configuration */}
      <div className="mab-card p-5 space-y-4 border-mab-border">
        <div className="flex items-center gap-2 border-b border-mab-border pb-3">
          <Cpu className="w-4 h-4 text-purple-400" />
          <h3 className="text-[14px] font-bold text-white">
            موتور هوش مصنوعی و مدل فعال
          </h3>
        </div>

        <div className="space-y-1.5">
          <label className="text-[12px] font-semibold text-white/80">
            سرویس‌دهنده هوش مصنوعی:
          </label>
          <MabSelect
            value={settingsForm.active_llm_provider}
            onChange={(val) => {
              const next = {
                ...settingsForm,
                active_llm_provider: val,
              };
              onChange(next);
              handleFetchModels(val);
              onQuickSaveSettings?.(next);
            }}
            options={LLM_PROVIDER_OPTIONS}
          />
        </div>

        {/* Live Model Selection Box */}
        <div className="p-4 rounded-xl bg-white/[0.02] border border-mab-border space-y-3">
          <div className="flex items-center justify-between gap-2 flex-wrap">
            <div className="flex items-center gap-2">
              <span className="text-[12.5px] font-semibold text-white">
                مدل منتخب:
              </span>
              <span className="font-mono text-[11px] px-2 py-0.5 rounded-md bg-white/[0.06] text-white/60">
                {providerSourceHint}
              </span>
            </div>

            <button
              type="button"
              onClick={() => void handleFetchModels()}
              disabled={isFetching}
              className="px-3 py-1 rounded-lg bg-white/[0.04] hover:bg-white/[0.08] border border-mab-border text-white/80 text-[11.5px] font-medium flex items-center gap-1.5 transition disabled:opacity-50"
            >
              <RefreshCw
                className={`w-3.5 h-3.5 text-blue-400 ${
                  isFetching ? "animate-spin" : ""
                }`}
              />
              <span>{isFetching ? "در حال دریافت..." : "بروزرسانی زنده مدل‌ها"}</span>
            </button>
          </div>

          <MabSelect
            value={selectedModel}
            onChange={(val) => {
              setModelForProvider(activeProvider, val);
              const cur = formRef.current;
              const next =
                activeProvider === "GEMINI_REST"
                  ? { ...cur, gemini_model: val }
                  : activeProvider === "LOCAL_OPENAI"
                  ? { ...cur, local_model: val }
                  : { ...cur, agy_model: val };
              onQuickSaveSettings?.(next);
            }}
            options={modelOptions}
            placeholder={
              liveModels.length === 0 && !hasSavedFallback
                ? "مدلی یافت نشد — روی دکمه بروزرسانی کلیک کنید"
                : "انتخاب مدل هوش مصنوعی..."
            }
            dir="ltr"
          />

          {activeLive && (
            <div
              className={`p-3 rounded-xl text-[12px] flex items-start gap-2.5 ${
                activeLive.live_fetched
                  ? "bg-emerald-500/[0.08] border border-emerald-500/25 text-emerald-300"
                  : "bg-amber-500/[0.08] border border-amber-500/25 text-amber-300"
              }`}
            >
              {activeLive.live_fetched ? (
                <CheckCircle2 className="w-4 h-4 shrink-0 text-emerald-400 mt-0.5" />
              ) : (
                <AlertTriangle className="w-4 h-4 shrink-0 text-amber-400 mt-0.5" />
              )}
              <span className="leading-relaxed">
                {activeLive.status_message}
              </span>
            </div>
          )}

          {activeProvider === "AGY_CLI" && !activeLive?.live_fetched && (
            <div className="pt-2 border-t border-mab-border flex items-center gap-2 flex-wrap">
              <span className="text-[11.5px] text-white/50">پیشنهاد سریع:</span>
              <button
                type="button"
                onClick={() => {
                  const next = {
                    ...settingsForm,
                    active_llm_provider: "LOCAL_OPENAI",
                  };
                  onChange(next);
                  onQuickSaveSettings?.(next);
                }}
                className="px-3 py-1 rounded-lg bg-blue-500/15 hover:bg-blue-500/25 border border-blue-500/30 text-blue-300 text-[11px] font-medium transition"
              >
                سوییچ به LM Studio محلی
              </button>
              <button
                type="button"
                onClick={() => {
                  const next = {
                    ...settingsForm,
                    active_llm_provider: "GEMINI_REST",
                  };
                  onChange(next);
                  onQuickSaveSettings?.(next);
                }}
                className="px-3 py-1 rounded-lg bg-purple-500/15 hover:bg-purple-500/25 border border-purple-500/30 text-purple-300 text-[11px] font-medium transition"
              >
                سوییچ به کلید Gemini API
              </button>
            </div>
          )}
        </div>

        {/* Endpoints & Keys */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/70">
              آدرس سرور لوکال (LM Studio):
            </label>
            <input
              value={settingsForm.local_endpoint}
              onChange={(e) =>
                onChange({
                  ...settingsForm,
                  local_endpoint: e.target.value,
                })
              }
              placeholder="http://127.0.0.1:1234/v1/chat/completions"
              className="w-full mab-input px-3 py-2 text-[12px] font-mono bidi-ltr"
            />
          </div>

          <div className="space-y-1">
            <label className="text-[11.5px] font-semibold text-white/70">
              کلید Google AI Studio (Gemini):
            </label>
            <div className="relative">
              <input
                type={showApiKey ? "text" : "password"}
                value={settingsForm.gemini_api_key}
                onChange={(e) =>
                  onChange({
                    ...settingsForm,
                    gemini_api_key: e.target.value,
                  })
                }
                placeholder="AIzaSy..."
                className="w-full mab-input px-3 py-2 text-[12px] font-mono bidi-ltr pl-9"
              />
              <button
                type="button"
                onClick={() => setShowApiKey((v) => !v)}
                className="absolute left-2.5 top-1/2 -translate-y-1/2 text-white/40 hover:text-white transition"
              >
                {showApiKey ? (
                  <EyeOff className="w-3.5 h-3.5" />
                ) : (
                  <Eye className="w-3.5 h-3.5" />
                )}
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* 2. Project Directory & Mab-Focus Integration */}
      <div className="mab-card p-5 space-y-4 border-mab-border">
        <div className="flex items-center gap-2 border-b border-mab-border pb-3">
          <FolderGit2 className="w-4 h-4 text-blue-400" />
          <h3 className="text-[14px] font-bold text-white">
            دایرکتوری پروژه‌ها و اتصال با Mab-Focus
          </h3>
        </div>

        <div className="space-y-1.5">
          <label className="text-[12px] font-semibold text-white/80">
            مسیر ریشه ذخیره‌سازی پروژه‌ها:
          </label>
          <input
            value={settingsForm.vault_root_path}
            onChange={(e) =>
              onChange({
                ...settingsForm,
                vault_root_path: e.target.value,
              })
            }
            className="w-full mab-input px-3 py-2 text-[12px] font-mono bidi-ltr"
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-[12px] font-semibold text-white/80">
            مسیر دیتای پایگاه داده Mab-Focus:
          </label>
          <input
            value={settingsForm.mab_focus_data_path}
            onChange={(e) =>
              onChange({
                ...settingsForm,
                mab_focus_data_path: e.target.value,
              })
            }
            className="w-full mab-input px-3 py-2 text-[12px] font-mono bidi-ltr"
          />
          <p className="text-[10.5px] text-mab-text-muted leading-relaxed">
            فایل دیتابیس در این مسیر بررسی می‌شود:{" "}
            <code className="text-blue-400">mab_focus_v3.db</code> یا{" "}
            <code className="text-blue-400">mab_focus.db</code> (پیش‌فرض ویندوز:{" "}
            <code className="text-white/60">
              %APPDATA%\com.example\mab_focus\MabFocus
            </code>
            )
          </p>
        </div>

        {/* Auto-Sync Toggle Item with MabCheckbox */}
        <div className="p-3.5 rounded-xl bg-white/[0.02] hover:bg-mab-hover/60 border border-mab-border transition">
          <MabCheckbox
            checked={settingsForm.auto_sync_mab_focus}
            onChange={(checked) =>
              onChange({
                ...settingsForm,
                auto_sync_mab_focus: checked,
              })
            }
            label="همگام‌سازی خودکار دوطرفه در زمان اجرای برنامه"
            description="پروژه‌ها به‌صورت دوطرفه و نشست‌های تمرکز به‌صورت یک‌طرفه از Mab-Focus سینک می‌شوند."
            className="w-full"
          />
        </div>
      </div>

      {/* 3. Storage Info */}
      <div className="mab-card p-4 flex items-center justify-between text-[11.5px] text-white/50 border-mab-border">
        <div className="flex items-center gap-2">
          <HardDrive className="w-3.5 h-3.5 text-white/40" />
          <span>پایگاه داده محلی SQLite با استاندارد اکوسیستم فعال است</span>
        </div>
        <span className="font-mono text-[10.5px] text-white/40">Mab-Compass v2.0</span>
      </div>
    </div>
  );
}

export default SettingsView;
