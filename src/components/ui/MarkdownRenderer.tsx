import React, { useState } from "react";
import { Check, Copy } from "lucide-react";

interface MarkdownRendererProps {
  content: string;
  compact?: boolean;
  className?: string;
}

function renderInline(text: string): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];
  // Matches `code`, **bold**, or *italic*
  const regex = /(`[^`]+`|\*\*[^*]+\*\*|\*[^*]+\*)/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  let key = 0;

  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      nodes.push(text.slice(lastIndex, match.index));
    }
    const token = match[0];
    if (token.startsWith("`") && token.endsWith("`")) {
      nodes.push(
        <code
          key={key++}
          dir="ltr"
          className="inline-block px-1.5 py-[1px] mx-0.5 rounded bg-white/[0.08] border border-white/[0.06] font-mono text-[0.86em] leading-snug text-[#E2E8F0] align-baseline"
        >
          {token.slice(1, -1)}
        </code>
      );
    } else if (token.startsWith("**") && token.endsWith("**")) {
      nodes.push(
        <strong key={key++} className="font-bold text-white">
          {token.slice(2, -2)}
        </strong>
      );
    } else if (token.startsWith("*") && token.endsWith("*")) {
      nodes.push(
        <em key={key++} className="italic text-white/85">
          {token.slice(1, -1)}
        </em>
      );
    }
    lastIndex = regex.lastIndex;
  }

  if (lastIndex < text.length) {
    nodes.push(text.slice(lastIndex));
  }
  return nodes;
}

function CodeBlock({ lang, code }: { lang: string; code: string }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch {
      // ignore
    }
  };

  return (
    <div
      dir="ltr"
      className="my-2.5 rounded-xl bg-[#0B0C0F] border border-white/[0.08] overflow-hidden"
    >
      <div className="px-3 py-1.5 bg-[#131418] border-b border-white/[0.06] flex items-center justify-between text-[10.5px] text-white/50 font-mono">
        <span>{lang || "code"}</span>
        <button
          type="button"
          onClick={() => void handleCopy()}
          className="inline-flex items-center gap-1 text-white/60 hover:text-white transition"
        >
          {copied ? <Check className="w-3 h-3 text-emerald-400" /> : <Copy className="w-3 h-3" />}
          <span>{copied ? "Copied" : "Copy"}</span>
        </button>
      </div>
      <pre className="p-3 overflow-x-auto font-mono text-[12px] leading-relaxed text-slate-200">
        {code}
      </pre>
    </div>
  );
}

export function MarkdownRenderer({
  content,
  compact = false,
  className = "",
}: MarkdownRendererProps) {
  const normalized = (content || "").replace(/\r\n/g, "\n");
  const lines = normalized.split("\n");
  const blocks: React.ReactNode[] = [];

  let i = 0;
  let blockKey = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    // Fenced code block
    if (trimmed.startsWith("```")) {
      const lang = trimmed.slice(3).trim();
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith("```")) {
        codeLines.push(lines[i]);
        i++;
      }
      i++; // consume closing fence
      blocks.push(<CodeBlock key={blockKey++} lang={lang} code={codeLines.join("\n")} />);
      continue;
    }

    if (!trimmed) {
      i++;
      continue;
    }

    // Horizontal rule
    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      blocks.push(<hr key={blockKey++} className="my-3.5 border-white/[0.08]" />);
      i++;
      continue;
    }

    // Headings
    const headingMatch = /^(#{1,4})\s+(.+)$/.exec(trimmed);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const text = headingMatch[2];
      if (level === 1) {
        blocks.push(
          <h1
            key={blockKey++}
            className={`${
              compact ? "text-[16px] mt-3 mb-1.5" : "text-[20px] sm:text-[22px] mt-5 mb-2.5 pb-2 border-b border-white/[0.08]"
            } font-bold text-white tracking-tight leading-snug`}
          >
            {renderInline(text)}
          </h1>
        );
      } else if (level === 2) {
        blocks.push(
          <h2
            key={blockKey++}
            className={`${
              compact ? "text-[14.5px] mt-3 mb-1" : "text-[17px] mt-5 mb-2"
            } font-bold text-white leading-snug`}
          >
            {renderInline(text)}
          </h2>
        );
      } else if (level === 3) {
        blocks.push(
          <h3
            key={blockKey++}
            className={`${
              compact ? "text-[13.5px] mt-2.5 mb-1" : "text-[15px] mt-4 mb-1.5"
            } font-semibold text-white/95 leading-snug`}
          >
            {renderInline(text)}
          </h3>
        );
      } else {
        blocks.push(
          <h4 key={blockKey++} className="text-[13.5px] font-semibold text-blue-300 mt-3 mb-1">
            {renderInline(text)}
          </h4>
        );
      }
      i++;
      continue;
    }

    // Blockquote
    if (trimmed.startsWith(">")) {
      const quoteLines: string[] = [];
      while (i < lines.length && lines[i].trim().startsWith(">")) {
        quoteLines.push(lines[i].trim().replace(/^>\s?/, ""));
        i++;
      }
      blocks.push(
        <blockquote
          key={blockKey++}
          className="my-2.5 pr-3.5 py-2 pl-3 border-r-2 border-blue-500/50 bg-white/[0.025] rounded-l-lg text-[12.5px] text-mab-text-secondary leading-relaxed space-y-1"
        >
          {quoteLines.map((ql, idx) => (
            <p key={idx}>{renderInline(ql)}</p>
          ))}
        </blockquote>
      );
      continue;
    }

    // Markdown Table
    if (
      trimmed.startsWith("|") &&
      i + 1 < lines.length &&
      /^\|[\s:|-]+\|$/.test(lines[i + 1].trim())
    ) {
      const headerCells = trimmed
        .slice(1, -1)
        .split("|")
        .map((c) => c.trim());
      i += 2;
      const bodyRows: string[][] = [];
      while (i < lines.length && lines[i].trim().startsWith("|")) {
        const rowCells = lines[i]
          .trim()
          .slice(1, -1)
          .split("|")
          .map((c) => c.trim());
        bodyRows.push(rowCells);
        i++;
      }
      blocks.push(
        <div
          key={blockKey++}
          className="my-3 overflow-x-auto rounded-xl border border-white/[0.08] bg-[#121317]"
        >
          <table className="w-full text-right text-[12.5px] border-collapse">
            <thead>
              <tr className="border-b border-white/[0.08] bg-white/[0.03] text-white/90">
                {headerCells.map((hc, ci) => (
                  <th key={ci} className="px-3 py-2 font-semibold">
                    {renderInline(hc)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-white/[0.05] text-[#D1D5DB]">
              {bodyRows.map((row, ri) => (
                <tr key={ri} className="hover:bg-white/[0.02]">
                  {row.map((cell, ci) => (
                    <td key={ci} className="px-3 py-2">
                      {renderInline(cell)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      );
      continue;
    }

    // Unordered or Checklist list
    if (/^[-*•]\s+/.test(trimmed)) {
      const items: { checked: boolean | null; text: string }[] = [];
      while (i < lines.length && /^[-*•]\s+/.test(lines[i].trim())) {
        const rawItem = lines[i].trim().replace(/^[-*•]\s+/, "");
        if (/^\[x\]\s+/i.test(rawItem)) {
          items.push({ checked: true, text: rawItem.replace(/^\[x\]\s+/i, "") });
        } else if (/^\[\s\]\s+/.test(rawItem)) {
          items.push({ checked: false, text: rawItem.replace(/^\[\s\]\s+/, "") });
        } else {
          items.push({ checked: null, text: rawItem });
        }
        i++;
      }
      blocks.push(
        <ul key={blockKey++} className="my-2 space-y-1.5 pr-4 list-none">
          {items.map((it, idx) => (
            <li key={idx} className="flex items-start gap-2 text-[#D1D5DB] leading-[1.8]">
              {it.checked === true ? (
                <span className="mt-1.5 w-3.5 h-3.5 rounded bg-emerald-500/20 border border-emerald-500/50 text-emerald-400 inline-flex items-center justify-center text-[10px] shrink-0">
                  ✓
                </span>
              ) : it.checked === false ? (
                <span className="mt-1.5 w-3.5 h-3.5 rounded border border-white/25 shrink-0" />
              ) : (
                <span className="mt-2.5 w-1.5 h-1.5 rounded-full bg-white/45 shrink-0" />
              )}
              <span className="flex-1">{renderInline(it.text)}</span>
            </li>
          ))}
        </ul>
      );
      continue;
    }

    // Ordered list
    if (/^\d+[.)]\s+/.test(trimmed)) {
      const items: { num: string; text: string }[] = [];
      while (i < lines.length && /^\d+[.)]\s+/.test(lines[i].trim())) {
        const m = /^(\d+)[.)]\s+(.+)$/.exec(lines[i].trim());
        if (m) {
          items.push({ num: m[1], text: m[2] });
        }
        i++;
      }
      blocks.push(
        <ol key={blockKey++} className="my-2 space-y-1.5 pr-2 list-none">
          {items.map((it, idx) => (
            <li key={idx} className="flex items-start gap-2 text-[#D1D5DB] leading-[1.8]">
              <span className="font-mono text-[12px] text-white/60 font-semibold mt-0.5 shrink-0">
                {it.num}.
              </span>
              <span className="flex-1">{renderInline(it.text)}</span>
            </li>
          ))}
        </ol>
      );
      continue;
    }

    // Regular paragraph (merge consecutive plain lines)
    const paraLines: string[] = [];
    while (
      i < lines.length &&
      lines[i].trim() !== "" &&
      !lines[i].trim().startsWith("#") &&
      !lines[i].trim().startsWith("```") &&
      !lines[i].trim().startsWith(">") &&
      !/^[-*•]\s+/.test(lines[i].trim()) &&
      !/^\d+[.)]\s+/.test(lines[i].trim()) &&
      !/^(-{3,}|\*{3,}|_{3,})$/.test(lines[i].trim())
    ) {
      paraLines.push(lines[i].trim());
      i++;
    }
    blocks.push(
      <p key={blockKey++} className="my-2 text-[#E5E7EB] leading-[1.85] break-words">
        {renderInline(paraLines.join(" "))}
      </p>
    );
  }

  return (
    <div
      dir="rtl"
      className={`${compact ? "text-[13.5px]" : "text-[14px]"} select-text ${className}`}
    >
      {blocks}
    </div>
  );
}

export default MarkdownRenderer;
