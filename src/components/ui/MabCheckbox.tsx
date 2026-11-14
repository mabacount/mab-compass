import type { ReactNode, FC, MouseEvent, KeyboardEvent } from "react";
import { Check } from "lucide-react";

export interface MabCheckboxProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: ReactNode;
  description?: ReactNode;
  disabled?: boolean;
  className?: string;
  id?: string;
}

export const MabCheckbox: FC<MabCheckboxProps> = ({
  checked,
  onChange,
  label,
  description,
  disabled = false,
  className = "",
  id,
}) => {
  const handleClick = (e: MouseEvent) => {
    e.stopPropagation();
    if (!disabled) {
      onChange(!checked);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.key === " " || e.key === "Enter") && !disabled) {
      e.preventDefault();
      e.stopPropagation();
      onChange(!checked);
    }
  };

  return (
    <label
      id={id}
      onClick={handleClick}
      className={`inline-flex items-start gap-2.5 select-none transition-opacity ${
        disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer group"
      } ${className}`}
    >
      <div
        role="checkbox"
        aria-checked={checked}
        tabIndex={disabled ? -1 : 0}
        onKeyDown={handleKeyDown}
        className={`w-4 h-4 rounded-[5px] flex items-center justify-center shrink-0 mt-0.5 border transition-all duration-150 outline-none focus-visible:ring-2 focus-visible:ring-emerald-500/40 ${
          checked
            ? "bg-emerald-600 border-emerald-500 text-white shadow-sm shadow-emerald-950/40"
            : "bg-white/[0.03] border-white/20 group-hover:border-white/35 group-hover:bg-white/[0.06]"
        }`}
      >
        {checked && (
          <Check className="w-3 h-3 stroke-[3] text-white animate-in zoom-in-75 duration-100" />
        )}
      </div>

      {(label || description) && (
        <div className="flex flex-col">
          {label && (
            <span
              className={`text-[13px] leading-snug font-sans transition-colors ${
                checked
                  ? "text-mab-text-primary"
                  : "text-mab-text-secondary group-hover:text-mab-text-primary"
              }`}
            >
              {label}
            </span>
          )}
          {description && (
            <span className="text-[11.5px] text-mab-text-muted mt-0.5 leading-normal">
              {description}
            </span>
          )}
        </div>
      )}
    </label>
  );
};

export default MabCheckbox;
