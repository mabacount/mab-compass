# بسته آماده بررسی کد Mab-Compass توسط Gemini (Code Review Pack)

این سند برای بررسی جامع معماری، امنیت و کارایی تغییرات نسخه ۴.۰ اپلیکیشن **Mab-Compass** توسط هوش مصنوعی **Google Gemini** (در Google AI Studio، وب یا ابزارهای کدنویسی) آماده شده است.

---

## ۱. لینک‌های مستقیم گیت‌هاب (GitHub Links)

تمامی تغییرات با موفقیت روی مخزن اصلی منتشر و ادغام (Merge) شده‌اند:

- **مخزن اصلی (شاخه پیش‌فرض `main`):**
  [https://github.com/mabacount/mab-compass](https://github.com/mabacount/mab-compass)
- **لینک مشاهده تغییرات و Diff کامل نسخه ۴ نسبت به نسخه ۳:**
  [https://github.com/mabacount/mab-compass/compare/7dbe5c5...b83a61a](https://github.com/mabacount/mab-compass/compare/7dbe5c5...b83a61a)
- **لینک مستقیم کامیت نسخه ۴:**
  [https://github.com/mabacount/mab-compass/commit/b83a61a25b08162fe3b353320e3ad84b3ea709f4](https://github.com/mabacount/mab-compass/commit/b83a61a25b08162fe3b353320e3ad84b3ea709f4)
- **شاخه اختصاصی فیچر:**
  [https://github.com/mabacount/mab-compass/tree/feature/antigravity-ui-methods-bpmn2-and-legacy-reprocess](https://github.com/mabacount/mab-compass/tree/feature/antigravity-ui-methods-bpmn2-and-legacy-reprocess)

---

## ۲. متن پرامپت آماده برای ارسال به Gemini (Copy-Paste Ready Prompt)

می‌توانید کادر زیر را مستقیماً در **Google AI Studio (Gemini 1.5 Pro / Flash)** یا در **Gemini Chat** کپی و ارسال کنید:

```text
Act as a Principal Software Engineer & Systems Architect specializing in Rust (Tauri v2), SQLite WAL databases, and high-performance React 18/TypeScript/Tailwind frontends.

I need you to perform a thorough, rigorous code review of the latest major release (v4.0) of Mab-Compass.
The repository is publicly accessible here:
https://github.com/mabacount/mab-compass

The specific commit under review is:
Commit: b83a61a (Compare: https://github.com/mabacount/mab-compass/compare/7dbe5c5...b83a61a)

### High-Level Architectural Context:
Mab-Compass is an offline-first desktop personal operating system combining goal execution, knowledge management, and AI copilot workflows.

Key Subsystems Added in v4.0:
1. Method & Playbook Engine (`src-tauri/src/method_engine.rs`):
   - Automated extraction of step-by-step SOPs and playbooks from fragmented chat interactions.
   - BPMN 2.0 XML generator with sub-process hierarchy and cyclic loop detection.
   - Atomic SQLite WAL database backup before bulk migrations.
   - Smart legacy memory and document backfill scanner with chronological queueing.

2. MCP Tool Expansion & Concurrency Guardrails (`src-tauri/src/mcp_tools.rs`, `src-tauri/src/llm/`):
   - New `upsert_method_playbook` tool.
   - Anti-duplication guards for legacy reprocess runs (`LEGACY_REPROCESS` origin tracking).
   - Artifact reference tracking (`written_artifacts_json` in `chat_messages`).
   - Mock TCP server chunk streaming fixes for large LLM tool schemas (>8KB payloads).

3. Antigravity 3-Column Obsidian UI (`src/components/`, `src/App.tsx`):
   - 3-column LTR structural layout with Persian RTL prose rendering.
   - Left navigation rail with active project tree and relative timestamps (`AntigravitySidebar.tsx`).
   - Center chat pane with dark rounded user message cards, clean Markdown assistant rendering, and inline clickable document/artifact cards (`AntigravityChatPane.tsx`).
   - Right multi-tab canvas with draggable splitters and dropdown tab strip (`AntigravityRightCanvas.tsx`).

4. Interactive SVG BPMN 2.0 Visualizer (`src/lib/bpmnEngine.ts`, `src/components/bpmn/BpmnViewer.tsx`):
   - Custom topological layout engine with 3-color DFS cycle detection and arched feedback edges.
   - Breadcrumb navigation for multi-level sub-process drill-down.
   - Zoom/pan viewport and interactive node inspector drawer.

5. Smart Legacy Reprocessing & Vault Archive (`src/components/LegacyReprocessPanel.tsx`, `KnowledgeArchivePanel.tsx`):
   - Filterable candidate table with individual/bulk selection and auto-backup triggers.

### What I Want You to Review:
1. **Rust Safety & Error Handling**: Are there any hidden panics, unwrap() calls on fallible I/O, or SQLite lock contention in `method_engine.rs` and `queries.rs`?
2. **Cycle Detection & Topological Layout Performance**: In `src/lib/bpmnEngine.ts`, does the 3-color DFS back-edge detection properly handle arbitrary complex cyclic and disconnected BPMN 2.0 graphs without UI freezing?
3. **LLM Tool & Queue Concurrency**: Are there any race conditions in `pending_ai_queue` processing or state transitions between `UNPROCESSED` -> `PROCESSING` -> `PROCESSED`?
4. **Memory Leaks & React Re-renders**: Review `BpmnViewer.tsx`, `AntigravityChatPane.tsx`, and `ResizableSplitPane.tsx` for unnecessary re-renders, uncleaned event listeners, or memory leaks during tab switching.
5. **Architectural Improvements**: What critical optimizations, security hardening, or design pattern refinements would you recommend next?

Please structure your review as follows:
- Executive Summary & Grade (A-F)
- Strengths & Well-Engineered Architectural Highlights
- Critical Issues (Security, Deadlocks, Panics, or Data Loss)
- Medium / Low Priority Code Smells & Performance Optimizations
- Actionable Recommendations & Code Snippets
```

---

## ۳. فهرست تفکیکی فایل‌های تغییریافته (25 فایل | +9,907 خط)

| بخش | مسیر فایل | توضیحات کلیدی |
| :--- | :--- | :--- |
| **هسته متد و BPMN** | `src-tauri/src/method_engine.rs` | تولیدکننده XML استاندارد BPMN 2.0، موتور پشتیبان‌گیری اتمیک دیتابیس، اسکنر بازپردازش اسناد قدیمی |
| **اسکیما دیتابیس** | `src-tauri/src/db/schema.rs` | افزودن جدول `method_playbooks` و ستون `written_artifacts_json` |
| **کوئری‌های پایگاه داده** | `src-tauri/src/db/queries.rs` | توابع CRUD پلی‌بوک‌ها، ذخیره آمار اسناد پردازش‌شده و ثبت آرتیفکت‌ها |
| **ابزارهای هوش مصنوعی** | `src-tauri/src/mcp_tools.rs` | ابزار MCP ثبت پلی‌بوک، جلوگیری از تسک‌های تکراری در بازپردازش حافظه |
| **روتر و سرور تستی** | `src-tauri/src/llm/router.rs` | اصلاح خواندن کامل بادی HTTP در پی‌لودهای بالای ۸ کیلوبایت |
| **پرامپت‌های مدل** | `src-tauri/src/llm/prompts.rs` | تزریق قوانین استخراج دستورالعمل‌های گام‌به‌گام و ثبت اسناد |
| **توابع تائوری** | `src-tauri/src/lib.rs` | اکسپورت دستورات Tauri برای متدها، BPMN، پشتیبان‌گیری دیتابیس و آرتیفکت‌ها |
| **تست‌های تجمیعی** | `src-tauri/tests/v4_methods_bpmn_artifacts_and_legacy_test.rs` | تست اند-تو-اند BPMN، استخراج متد، بازپردازش و بکاپ دیتابیس |
| **رابط کاربری اصلی** | `src/App.tsx` | پیاده‌سازی ساختار ۳ ستونه Antigravity با کنترل پنل تب‌ها و اسپلیترها |
| **چت و کارت اسناد** | `src/components/AntigravityChatPane.tsx` | فید چت دارک ابسیدین با کارت‌های کلیک‌پذیر آرتیفکت‌ها در زیر پاسخ‌های اسیستنت |
| **بوم تب‌های سمت راست** | `src/components/AntigravityRightCanvas.tsx` | نوار تب‌های باز شونده با منوی `+` شناور و تغییر تب بدون سربار رندر |
| **سایدبار ناوبری** | `src/components/AntigravitySidebar.tsx` | سایدبار چپ با درخت پروژه‌ها، زمان‌های نسبی و میانبرهای سریع |
| **تایتل‌بار پنجره** | `src/components/AntigravityTitlebar.tsx` | تایتل‌بار تیره frameless با وضعیت اتصال مدل، همگام‌سازی Focus و دکمه‌ها |
| **موتور گراف BPMN** | `src/lib/bpmnEngine.ts` | الگوریتم چیدمان توپولوژیک بر پایه DFS سه‌رنگ برای تشخیص دور و مسیریابی لبه‌ها |
| **کامپوننت ویوور BPMN** | `src/components/bpmn/BpmnViewer.tsx` | رندر وکتور اینتراکتیو با قابلیت دریل‌داون چندسطحی ساب‌پراسس‌ها و زوم/پن |
| **پنل بازپردازش اسناد** | `src/components/LegacyReprocessPanel.tsx` | جدول فیلتردار اسناد، انتخاب دسته‌ای، اجرای بکاپ خودکار و ارسال به صف |
| **رندرکننده Markdown** | `src/components/ui/MarkdownRenderer.tsx` | پشتیبانی بی‌نقص از متون راست‌به‌چپ فارسی، جدول‌ها و بلاک‌های کد |
| **تایپ‌ها و API فرانت‌اند**| `src/lib/tauri_api.ts` | تایپ‌های کامل پلی‌بوک، آرتیفکت، استپ‌ها و توابع فراخوانی Backend |

---

## ۴. روش‌های پیشنهادی برای ارائه به Gemini

1. **روش اول (سریع‌ترین - از طریق لینک مستقیم مخزن در Google AI Studio / Gemini Code Assist):**
   - به [Google AI Studio](https://aistudio.google.com) بروید.
   - مدل **Gemini 1.5 Pro** یا جدیدترین مدل را انتخاب کنید.
   - پرامپت بخش ۲ را به همراه لینک مخزن گیت‌هاب قرار دهید.

2. **روش دوم (از طریق وب چت Gemini Advanced):**
   - پرامپت بخش ۲ را در چت ارسال کنید.
   - در صورت نیاز به بررسی دقیق یک فایل خاص، محتوای فایل‌هایی مانند `src-tauri/src/method_engine.rs` یا `src/lib/bpmnEngine.ts` را کپی کرده و پیوست نمایید.
