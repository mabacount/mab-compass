import { useState, useRef, useEffect, type ReactNode, type FC } from "react";
import { ChevronDown, Check } from "lucide-react";

export interface MabSelectOption {
  value: string;
  label: string;
  icon?: ReactNode;
  description?: string;
}

export interface MabSelectProps {
  value: string;
  onChange: (val: string) => void;
  options: MabSelectOption[];
  placeholder?: string;
  className?: string;
  disabled?: boolean;
  id?: string;
  dir?: "rtl" | "ltr";
}

export const MabSelect: FC<MabSelectProps> = ({
  value,
  onChange,
  options,
  placeholder = "انتخاب کنید...",
  className = "",
  disabled = false,
  id,
  dir,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    const handleOutsideClick = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    if (isOpen) {
      document.addEventListener("mousedown", handleOutsideClick);
    }
    return () => {
      document.removeEventListener("mousedown", handleOutsideClick);
    };
  }, [isOpen]);

  // Close on Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && isOpen) {
        setIsOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen]);

  const selectedOption = options.find((opt) => opt.value === value);

  return (
    <div
      ref={containerRef}
      id={id}
      dir={dir}
      className={`relative inline-block w-full select-none text-[13px] ${className}`}
    >
      <button
        type="button"
        disabled={disabled}
        onClick={() => !disabled && setIsOpen(!isOpen)}
        className={`w-full flex items-center justify-between gap-2 px-3 py-2 rounded-mab-input bg-mab-surface border transition-all duration-150 text-right ${
          disabled
            ? "opacity-50 cursor-not-allowed border-mab-border"
            : isOpen
            ? "border-blue-500/50 ring-1 ring-blue-500/20 bg-mab-hover"
            : "border-mab-border-input hover:border-mab-border-input-hover hover:bg-mab-hover/60"
        }`}
      >
        <div className="flex items-center gap-2 truncate">
          {selectedOption?.icon && <span className="shrink-0">{selectedOption.icon}</span>}
          <span
            className={`truncate font-sans ${
              selectedOption ? "text-mab-text-primary" : "text-mab-text-muted"
            }`}
          >
            {selectedOption ? selectedOption.label : placeholder}
          </span>
        </div>
        <ChevronDown
          className={`w-4 h-4 text-mab-text-secondary shrink-0 transition-transform duration-200 ${
            isOpen ? "rotate-180 text-blue-400" : ""
          }`}
        />
      </button>

      {isOpen && (
        <div className="absolute z-50 mt-1 w-full min-w-[160px] rounded-mab-card bg-mab-elevated border border-mab-border shadow-2xl backdrop-blur-md py-1.5 max-h-60 overflow-y-auto animate-in fade-in zoom-in-95 duration-100">
          {options.length === 0 ? (
            <div className="px-3 py-2 text-center text-xs text-mab-text-muted">
              گزینه‌ای یافت نشد
            </div>
          ) : (
            options.map((opt) => {
              const isSelected = opt.value === value;
              return (
                <button
                  key={opt.value}
                  type="button"
                  onClick={() => {
                    onChange(opt.value);
                    setIsOpen(false);
                  }}
                  className={`w-full flex items-center justify-between px-3 py-2 text-right transition-colors text-[12.5px] ${
                    isSelected
                      ? "bg-blue-500/15 text-blue-400 font-medium"
                      : "text-mab-text-primary hover:bg-mab-hover hover:text-white"
                  }`}
                >
                  <div className="flex items-center gap-2 truncate">
                    {opt.icon && <span className="shrink-0">{opt.icon}</span>}
                    <div className="flex flex-col truncate">
                      <span className="truncate">{opt.label}</span>
                      {opt.description && (
                        <span className="text-[11px] text-mab-text-muted truncate">
                          {opt.description}
                        </span>
                      )}
                    </div>
                  </div>
                  {isSelected && (
                    <Check className="w-3.5 h-3.5 text-blue-400 shrink-0 ml-1 stroke-[2.5]" />
                  )}
                </button>
              );
            })
          )}
        </div>
      )}
    </div>
  );
};

export default MabSelect;
