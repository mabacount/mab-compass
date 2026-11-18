import React, { useCallback, useEffect, useRef, useState } from "react";

interface ResizableSplitPaneProps {
  storageKey: string;
  defaultPrimaryPercent?: number;
  minPrimaryPercent?: number;
  maxPrimaryPercent?: number;
  primary: React.ReactNode;
  secondary: React.ReactNode;
}

/**
 * VS Code–style RTL draggable vertical split pane.
 * - On desktop (>= 1024px): horizontal side-by-side split with a draggable col-resize sash.
 *   Primary sits on the right (RTL start), Secondary sits on the left (RTL end).
 * - Double-click sash resets to defaultPrimaryPercent.
 * - Persists width percentage in localStorage.
 * - On narrower viewports (< 1024px): stacks vertically cleanly.
 */
export function ResizableSplitPane({
  storageKey,
  defaultPrimaryPercent = 56,
  minPrimaryPercent = 26,
  maxPrimaryPercent = 74,
  primary,
  secondary,
}: ResizableSplitPaneProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [primaryPct, setPrimaryPct] = useState<number>(() => {
    if (typeof window === "undefined") return defaultPrimaryPercent;
    const saved = window.localStorage.getItem(storageKey);
    const parsed = saved ? Number(saved) : NaN;
    if (
      Number.isFinite(parsed) &&
      parsed >= minPrimaryPercent &&
      parsed <= maxPrimaryPercent
    ) {
      return parsed;
    }
    return defaultPrimaryPercent;
  });
  const [isDragging, setIsDragging] = useState(false);

  const persistPct = useCallback(
    (pct: number) => {
      try {
        window.localStorage.setItem(storageKey, String(Math.round(pct * 10) / 10));
      } catch {
        // Ignore storage quota errors
      }
    },
    [storageKey]
  );

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const el = containerRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0) return;
      // RTL: primary is on the right side
      const rawPct = ((rect.right - e.clientX) / rect.width) * 100;
      const clamped = Math.max(
        minPrimaryPercent,
        Math.min(maxPrimaryPercent, rawPct)
      );
      setPrimaryPct(clamped);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isDragging, minPrimaryPercent, maxPrimaryPercent]);

  useEffect(() => {
    if (!isDragging) {
      persistPct(primaryPct);
    }
  }, [isDragging, primaryPct, persistPct]);

  const handleReset = () => {
    setPrimaryPct(defaultPrimaryPercent);
    persistPct(defaultPrimaryPercent);
  };

  return (
    <div
      ref={containerRef}
      className="flex flex-col lg:flex-row h-full min-h-0 min-w-0 gap-3.5 lg:gap-0"
    >
      {/* Primary Pane (RTL Right / Start) */}
      <div
        className="flex flex-col min-h-0 min-w-0 lg:h-full lg:shrink-0"
        style={{
          flexBasis: `${primaryPct}%`,
        }}
      >
        {primary}
      </div>

      {/* VS Code–style Draggable Sash (Desktop Only) */}
      <div
        role="separator"
        aria-orientation="vertical"
        title="بکشید برای تغییر اندازه پنل‌ها (دوبار کلیک برای بازنشانی)"
        onMouseDown={(e) => {
          e.preventDefault();
          setIsDragging(true);
        }}
        onDoubleClick={handleReset}
        className="hidden lg:flex w-3 shrink-0 cursor-col-resize items-center justify-center group select-none"
      >
        <div
          className={`w-[3px] h-full rounded-full transition-colors duration-150 ${
            isDragging
              ? "bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.65)]"
              : "bg-white/[0.06] group-hover:bg-blue-500/75"
          }`}
        />
      </div>

      {/* Secondary Pane (RTL Left / End) */}
      <div className="flex flex-col min-h-[460px] lg:min-h-0 min-w-0 flex-1 lg:h-full">
        {secondary}
      </div>
    </div>
  );
}

export default ResizableSplitPane;
