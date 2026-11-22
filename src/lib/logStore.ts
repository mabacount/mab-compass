// src/lib/logStore.ts
// Centralized Logging Store for Mab-Compass with one-click Markdown export

export type LogLevel = "INFO" | "SUCCESS" | "WARN" | "ERROR";
export type LogCategory = "LLM" | "QUEUE" | "SYNC" | "APP" | "SYSTEM";

export interface AppLogEntry {
  id: string;
  timestamp: string; // ISO string
  timeFormatted: string; // HH:mm:ss
  level: LogLevel;
  category: LogCategory;
  message: string;
  details?: string;
}

const STORAGE_KEY = "mab_compass_system_logs_v1";
const MAX_LOGS = 250;

let memoryLogs: AppLogEntry[] = [];
const listeners = new Set<(logs: AppLogEntry[]) => void>();

function formatTime(d: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

// Load persisted logs on startup
try {
  const saved = localStorage.getItem(STORAGE_KEY);
  if (saved) {
    const parsed = JSON.parse(saved);
    if (Array.isArray(parsed)) {
      memoryLogs = parsed.slice(-MAX_LOGS);
    }
  }
} catch {
  memoryLogs = [];
}

function persist() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(memoryLogs.slice(-MAX_LOGS)));
  } catch {
    // ignore
  }
  listeners.forEach((fn) => {
    try {
      fn(memoryLogs);
    } catch {
      // ignore
    }
  });
}

export function addAppLog(
  level: LogLevel,
  category: LogCategory,
  message: string,
  details?: string
): AppLogEntry {
  const now = new Date();
  const entry: AppLogEntry = {
    id: `log_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
    timestamp: now.toISOString(),
    timeFormatted: formatTime(now),
    level,
    category,
    message,
    details: details ? String(details).trim() : undefined,
  };

  memoryLogs.push(entry);
  if (memoryLogs.length > MAX_LOGS) {
    memoryLogs.shift();
  }

  persist();
  return entry;
}

export function getAppLogs(): AppLogEntry[] {
  return [...memoryLogs];
}

export function clearAppLogs(): void {
  memoryLogs = [];
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
  listeners.forEach((fn) => fn([]));
}

export function subscribeAppLogs(listener: (logs: AppLogEntry[]) => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function exportLogsAsMarkdown(): string {
  const logs = [...memoryLogs];
  if (logs.length === 0) {
    return "# گزارش لاگ‌های سیستم Mab-Compass\n\nهیچ لاگی در سیستم ثبت نشده است.";
  }

  const lines: string[] = [
    `# گزارش جامع لاگ‌های سیستم Mab-Compass`,
    `> زمان خروجی: ${new Date().toLocaleString("fa-IR")} (${new Date().toISOString()})`,
    `> تعداد کل رخدادها: ${logs.length}`,
    "",
    "| زمان | سطح | بخش | پیام |",
    "| :--- | :--- | :--- | :--- |",
  ];

  for (const log of logs) {
    const icon =
      log.level === "ERROR"
        ? "🔴 ERROR"
        : log.level === "WARN"
        ? "🟡 WARN"
        : log.level === "SUCCESS"
        ? "🟢 SUCCESS"
        : "⚪ INFO";
    lines.push(
      `| ${log.timeFormatted} | ${icon} | **${log.category}** | ${log.message.replace(/\|/g, "\\|")} |`
    );
  }

  // Include detailed trace for errors and warnings
  const detailedLogs = logs.filter((l) => l.details && l.details.trim().length > 0);
  if (detailedLogs.length > 0) {
    lines.push("");
    lines.push("## جزئیات تفصیلی خطاها و رخدادها (Detailed Diagnostics)");
    lines.push("");
    for (const log of detailedLogs) {
      lines.push(`### [${log.timeFormatted}] [${log.level}] [${log.category}] ${log.message}`);
      lines.push("```text");
      lines.push(log.details || "");
      lines.push("```");
      lines.push("");
    }
  }

  return lines.join("\n");
}
