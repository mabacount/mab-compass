# سند وضعیت و انتقال شیفت کاری (HANDOVER / Status Summary)
**پروژه:** Mab-Compass v3.0 (اکوسیستم ماژولار و چت‌محور فوق‌متمرکز)  
**تاریخ و زمان توقف:** 2026-11-14 ساعت 23:05  
**وضعیت اجرا:** کلیه فرآیندها، پردازش‌های پس‌زمینه (Cron Tasks) و ایجنت‌های زیرمجموعه متوقف (Killed/Halted) شدند. هیچ دستوری در حال اجرا نیست و سیستم آماده خاموش شدن است.

---

## ۱. دقیقاً کدام تسک‌ها کامل شده است؟

### الف. فاز کاوش و معماری (Phase 0 - Completed)
- استخراج کامل نیازمندی‌های سند تحول ۳.۰ از `mab_compass_v3_modular_ecosystem_blueprint (1).md`.
- تحلیل شکاف‌های فنی در لایه‌های بک‌اند (Rust / SQLite WAL) و فرانت‌اند (React 18 / Tailwind / Tauri v2).
- تعریف ۳۳ قابلیت تفکیک‌شده (Features F1 تا F33) در ۵ مایلستون اصلی در سند پروژه.

### ب. مایلستون ۱ (M1: Database Schema Migration & Rust Domain Models) — ۱۰۰٪ کامل و تایید رسمی (Passed Gate)
- **۶ جدول جدید SQLite در `src-tauri/src/db/schema.rs`:**
  1. `life_domains`: جداول ۸ بُعد پایدار زندگی/کار بر اساس متدولوژی CADM v2.0 با سطوح بلوغ (0 تا 5)، کف نگهداری (`maintenance_floor`) و نشانگر تمرکز رشد (`is_growth_focus`).
  2. `outcomes`: نتایج مطلوب دگرگون‌کننده واقعیت (`OUT-XXXX`) متصل به حوزه‌ها با سنجه‌های شواهد (`evidence_kr`) و درصد پیشرفت.
  3. `work_lines`: لاین‌های کاری هدف‌مند و زنجیره‌های کارها (`WLN-XXXX`) با دو حالت `SEQUENTIAL_CHAIN` (پشت‌به‌پشت) و `GOAL_CLUSTER` (هم‌هدف موازی).
  4. `calendar_days`: برنامه‌ریزی تقویم روزانه با ثبت دقایق کل ظرفیت و دقایق کار عمیق (`deep_capacity_minutes`).
  5. `pending_proposals`: گیت تصمیم‌گیری چرخه حیات پروژه‌ها برای تایید انسان (`PRP-XXXX` جهت ساخت، تعلیق، آرشیو یا تغییر مسیر).
  6. `personal_vault_items`: خزانه‌یادگیری شخصی، علاقه‌مندی‌ها و درس‌آموخته‌ها (`PVI-XXXX`).
- **۶ ایندکس اختصاصی SQLite** روی روابط کلیدی و کلیدهای خارجی برای سرعت بالای کوئری‌ها.
- **۲۳ ستون جدید با مایگریشن امن (`ensure_column_exists`)**:
  - ارتقای `projects`: `domain_code`, `primary_outcome_id`, `lifecycle_status`, `scope_mode`, `realm`, `priority_weight`, `pause_resume_note`.
  - ارتقای `work_entities`: `work_line_id`, `outcome_id`, `sequence_order`, `depends_on_task_id`, `energy_tag`, `duration_minutes`, `device_tag`, `location_tags_json`, `scheduled_date`, `scheduled_slot`, `due_date`, `priority_score`.
  - ارتقای `knowledge_documents`: `binary_original_path`, `tags_json`, `realm`, `domain_code`.
- **Seed اولیه ۸ بُعد اصلی زندگی بر مبنای CADM:**
  - `MIND_LEARNING`, `HEALTH_ENERGY`, `WEALTH_FINANCE`, `RELATIONS`, `RESILIENCE`, `PURPOSE`, `SPIRITUALITY`, `SANCTUARY`.
- **۷ استراکت مدل جدید در `src-tauri/src/models.rs`:**
  - `LifeDomain`, `Outcome`, `WorkLine`, `CalendarDay`, `PendingProposal`, `PersonalVaultItem`, `ChainStepInput` همراه با فیلدهای جدید در `Project`, `WorkEntity`, `DashboardState` و سازگاری کامل با `serde`.
- **پاس شدن کامل ۳۵ تست از ۳۵ تست بک‌اند (`cargo test`) و بیلد موفق فرانت‌اند (`npm run build`) بدون کوچک‌ترین خطا.**
- **ممیزی ۵ ایجنتی M1:** تایید متفق‌القول داوران، چالش‌گران و ممیز مستقل با رتبه CLEAN/APPROVE.

### ج. زیرساخت تست مستقل E2E (E2E Test Infrastructure - Completed)
- طراحی سلسله‌مراتب تست ۴ سطحی در `TEST_INFRA.md`.
- پیاده‌سازی فیکسچر مستقل در حافظه (in-memory SQLite) در `src-tauri/tests/common/mod.rs` (۴۵ کیلوبایت).
- اعتبارسنجی موفق ۶ تست پایه در `src-tauri/tests/smoke_harness_test.rs`.

---

## ۲. الان وسط کدام فایل یا کدام مرحله است؟

- **مرحله جاری:** **مایلستون ۲ (M2: Sequential Chain Engine & Context Execution)** به موازات تکمیل تست‌های F1–F33 در تِرَک تست.
- **فایل‌های مشخص در دست اقدام:**
  1. `src-tauri/src/db/queries.rs`:
     - تکمیل تابع `create_work_line_with_chain` (ثبت اتمیک لاین کاری، قرار دادن گام ۱ در وضعیت فعال و گام‌های بعدی در وضعیت `QUEUED_IN_CHAIN` با تنظیم `depends_on_task_id`).
     - تکمیل تابع `advance_sequential_chain_on_complete` (پیشروی خودکار گام‌های زنجیره با اتمام گام قبلی و تکمیل خودکار WorkLine).
     - پیاده‌سازی `is_locked_by_chain` و `query_tasks_by_context` (فیلتر بر اساس تگ‌های ۴ گانه: مکان، وسیله، انرژی، زمان).
  2. `src-tauri/tests/e2e_tier1_feature_coverage.rs`:
     - فایل ۱۶۰۰+ خطی تست‌های یکپارچه که برای سنجش تک‌تک ویژگی‌های F1 تا F33 در حال آماده‌سازی بود.
- **وضعیت فرآیندها:** کلیه زیرفرآیندها و جاب‌های پس‌زمینه با موفقیت متوقف شدند؛ مخزن کاملاً پایدار و بدون پروسه معلق است.

---

## ۳. فردا اولین قدمی که باید بردارد چیست؟

هنگام ازسرگیری کار، مراحل زیر به ترتیب باید انجام شوند:

1. **اعتبارسنجی اولیه وضعیت مخزن:**
   - اجرای `cargo test` در پوشه `src-tauri` برای بررسی پاس شدن تست‌های ۳۵‌گانه پایه.
2. **ادامه و اتمام پیاده‌سازی مایلستون ۲ (M2):**
   - بررسی و تکمیل توابع زنجیره و فیلتر کانتکست در `src-tauri/src/db/queries.rs`.
   - تست توابع زنجیره با کامپایل و اجرای `tests/e2e_tier1_feature_coverage.rs`.
3. **ورود به مایلستون ۳ (M3: MCP Tools & Live State Synchronization):**
   - اضافه کردن ۸ ابزار جدید MCP در `src-tauri/src/mcp_tools.rs` (`upsert_domain_or_outcome`, `create_work_line_chain`, `append_or_reorder_work_line`, `query_tasks_by_context`, `schedule_task_on_calendar`, `propose_project_lifecycle`, `save_personal_or_lesson_item`, `open_in_right_panel`).
   - ساخت اسنپ‌شات زنده ۵ لایه اکوسیستم در پرامپت مدل LLM.
4. **ورود به مایلستون ۴ (M4: Chat-First 3-Column UI & Dynamic Inspector):**
   - پیاده‌سازی کامپوننت‌های فرانت‌اند React:
     - `LeftWorkspaceSidebar.tsx` (سایدبار درختی حوزه‌ها و پروژه‌ها)
     - `ChatWorkspacePane.tsx` + `SlashCommandMenu.tsx` (مرکز فرماندهی چت با کارت‌های پیشنهاد تغییر)
     - `TopRightDirectoryBar.tsx` (نوار دایرکتوری در بالا سمت راست با شمارنده‌های زنده)
     - `RightInspectorDrawer.tsx` (دراور کشویی سمت راست شامل ۵ نمای تخصصی).
