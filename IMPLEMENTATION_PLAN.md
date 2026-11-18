# سند برنامه اجرایی و معماری بازطراحی سیستم Mab-Compass
## (Mab-Compass 4-Phase System Fix & Architecture Implementation Plan)

---

## ۱. مقدمه و مرور کلی (Overview)

پروژه **Mab-Compass** یک سامانه مدیریت وظایف، هم‌راستاسازی زندگی و خروجی‌های هدف‌محور (Outcome-Driven Execution) بر پایه معماری ترکیبی **Tauri v2 + React 18 (TypeScript) + Rust (SQLite / Rusqlite)** است. در نسخه‌های اخیر سیستم، معماری داده در لایه بک‌اند به اکوسیستم ۵ لایه غنی‌سازی ارتقا یافته است:
1. **Life Domain** (حوزه‌های بنیادین زندگی و رشد)
2. **Outcome** (تغییرات واقعی و دستاوردهای هدف‌گذاری شده به همراه شواهد کلیدی KR)
3. **Work Line** (خطوط کاری منسجم به صورت زنجیره ترتیبی Sequential Chain یا خوشه هدفی Goal Cluster)
4. **Work Entity / Task** (تسک‌های اجرایی، متوالی و وابسته‌به‌هم با برچسب‌های زمینه و انرژی)
5. **Execution & AI Context** (دستیار هوشمند، صف پیام‌های معلق و تاریخچه رویدادها)

با این حال، به دلیل عدم انطباق لایه‌های نرم‌افزار، ۳ ناهماهنگی اساسی مانع کارکرد بهینه سیستم شده بود که در این سند ریشه‌یابی و نقشه راه اجرای ۴ فاز برای حل کامل آن‌ها تدوین شده است.

---

## ۲. ریشه‌یابی ۳ مشکل اصلی سیستم (Root Cause Analysis)

### ۲.۱. مشکل اول: عدم تطابق تایپ‌های فرانت‌اند و بک‌اند (TypeScript / Rust Model Mismatch)
* **علت ریشه‌ای:** در لایه Rust (`src-tauri/src/models.rs` و `src-tauri/src/db/queries.rs`) ساختارهای `LifeDomain`, `Outcome`, `WorkLine` و ۱۵ فیلد زنجیره‌ای/ترتیبی برای `WorkEntity` تعریف و از پایگاه داده SQLite استخراج می‌شدند؛ اما فایل تعاریف رابط کاربری (`src/lib/tauri_api.ts`) شامل این اینترفیس‌ها نبود و `WorkEntity` و `DashboardState` ناقص بودند.
* **پیامد:** کامپوننت‌های فرانت‌اند نمی‌توانستند به فیلدهای ساختار WBS و خطوط کاری دسترسی داشته باشند و هرگونه توسعه فرانت‌اند با فعال بودن حالت `strict: true` در تایپ‌اسکریپت بلاک می‌شد.

### ۲.۲. مشکل دوم: نبود ساختار درختی و سلسله‌مراتبی در نمای تسک‌ها (Flat WBS / Lack of Grouping)
* **علت ریشه‌ای:** کامپوننت `WbsAndDocsView.tsx` در تب اصلی `ALL_TASKS` تسک‌ها را صرفاً بر اساس یک لیست مسطح (Flat Array) فیلتر و رندر می‌کرد.
* **پیامد:** وابستگی منطقی تسک‌ها به خطوط کاری (`work_line_id`) و دستاوردها (`outcome_id`) پنهان بود. کاربر نمی‌توانست میزان پیشرفت یک لاین کاری را ببیند، وضعیت قفل بودن تسک‌های بعدی یک زنجیره ترتیبی (`is_locked_by_chain`) نمایش داده نمی‌شد و امکان سازماندهی ذهنی و اولویت‌بندی وجود نداشت.

### ۲.۳. مشکل سوم: گلوگاه تمرکز تک‌تسکی و شکنندگی چت در زمان خطای مدل (Single Priority Bottleneck & Missing Queue Fallback)
* **علت ریشه‌ای بخش الف (UI/Rust):** در `queries.rs` و نمای `PathAndChatView.tsx` سیستم تنها یک گام بعدی (`next_step_task`) را به کاربر پیشنهاد می‌داد. در شرایطی که پروژه‌ها دارای چندین خط کاری موازی فعال هستند، تمرکز روی تنها یک تسک باعث خوابیدن کار روی سایر خطوط می‌شد.
* **علت ریشه‌ای بخش ب (Rust LLM Error Handling):** در توابع فراخوانی دستیار هوش مصنوعی (`cmd_run_assistant_turn` و `cmd_run_file_assistant_turn` در `lib.rs`)، چنانچه فراخوانی API به هر دلیلی (قطعی اینترنت، محدودیت نرخ مصرف یا خطای سرویس‌دهنده) شکست می‌خورد، پیام کاربر در صف معلق (`pending_ai_queue`) ثبت نمی‌شد و به سادگی از دست می‌رفت.

---

## ۳. نقشه راه و جزئیات ۴ فاز اجرایی (Execution Roadmap)

```
┌────────────────────────────────────────────────────────┐
│ Phase 1: TypeScript Types Alignment (tauri_api.ts)     │
│ ├── Add LifeDomain, Outcome, WorkLine                  │
│ ├── Add 15 Sequential/Context fields to WorkEntity     │
│ └── Add domains, outcomes, work_lines, top_priority... │
└──────────────────────────┬─────────────────────────────┘
                           │
         ┌─────────────────┴─────────────────┐
         ▼                                   ▼
┌─────────────────────────────────┐ ┌─────────────────────────────────┐
│ Phase 2: Hierarchical WBS View  │ │ Phase 3: 3 Top Priorities       │
│ (WbsAndDocsView.tsx)            │ │ (Rust Backend & Path UI)        │
│ ├── Outcome -> WorkLine Groups  │ │ ├── Rust 3-task selection query │
│ ├── Badges, Locks & Progress    │ │ └── UI 3 priority cards with    │
│ └── Collapse/Expand state       │ │     line badges & action btn    │
└─────────────────────────────────┘ └────────────────┬────────────────┘
                                                     │
                                                     ▼
                                    ┌─────────────────────────────────┐
                                    │ Phase 4: Queue Fallback Logic   │
                                    │ (src-tauri/src/lib.rs)          │
                                    │ ├── PendingQueueInput insertion │
                                    │ └── Automatic retry enqueue on  │
                                    │     LLM network/runtime error   │
                                    └─────────────────────────────────┘
```

---

### فاز ۱: همگام‌سازی تایپ‌های TypeScript با بک‌اند Rust

**هدف:** رفع ریشه‌ای خطاهای کامپایل تایپ‌اسکریپت و مهیاسازی اینترفیس‌های مورد نیاز برای رندرهای سلسله‌مراتبی.
**فایل هدف:** `src/lib/tauri_api.ts`

#### گام‌های پیاده‌سازی:
1. تعریف اینترفیس‌های سه‌گانه اکوسیستم:
```typescript
export interface LifeDomain {
  code: string;
  title: string;
  description: string;
  maturity_state: number;
  maintenance_floor: string;
  desired_state: string;
  is_growth_focus: boolean;
  updated_at: string;
}

export interface Outcome {
  id: string;
  domain_code: string;
  title: string;
  desired_reality_change: string;
  evidence_kr: string;
  time_window: string;
  status: string;
  progress_percent: number;
  linked_projects_count: number;
  created_at: string;
  updated_at: string;
}

export interface WorkLine {
  id: string;
  project_id: string;
  project_name: string;
  outcome_id?: string | null;
  outcome_title?: string | null;
  title: string;
  goal_statement: string;
  line_mode: string; // "SEQUENTIAL_CHAIN" | "GOAL_CLUSTER"
  status: string;
  color_hex: string;
  total_tasks: number;
  completed_tasks: number;
  active_step_id?: string | null;
  created_at: string;
  updated_at: string;
}
```

2. غنی‌سازی `WorkEntity` با ۱۵ فیلد زنجیره‌ای، اولویت‌بندی و زمان‌بندی:
```typescript
export interface WorkEntity {
  // فیلدهای قبلی...
  id: string;
  project_id: string;
  // ...
  
  // فیلدهای جدید اکوسیستم نسخه ۳:
  project_name?: string;
  work_line_id?: string | null;
  work_line_title?: string | null;
  outcome_id?: string | null;
  sequence_order: number;
  depends_on_task_id?: string | null;
  is_locked_by_chain: boolean;
  energy_tag: string;
  duration_minutes: number;
  device_tag: string;
  location_tags: string[];
  scheduled_date?: string | null;
  scheduled_slot?: string | null;
  due_date?: string | null;
  priority_score: number;
}
```

3. به‌روزرسانی ساختار جامع `DashboardState`:
```typescript
export interface DashboardState {
  // فیلدهای پایه‌ای قبلی...
  projects: Project[];
  active_project: Project;
  // ...
  
  // فیلدهای اکوسیستم و اولویت‌های موازی:
  domains: LifeDomain[];
  outcomes: Outcome[];
  work_lines: WorkLine[];
  top_priority_steps?: WorkEntity[];
}
```

---

### فاز ۲: بازنویسی نمای ساختار شکست کار (Hierarchical WBS View)

**هدف:** تبدیل نمای یکنواخت به یک درخت سلسله‌مراتبی سازمان‌یافته با قابلیت ردگیری پیشرفت، آیکون‌های قفل زنجیره، و گروه‌بندی پویا.
**فایل هدف:** `src/components/WbsAndDocsView.tsx`

#### ساختار بصری هدف:
```text
📦 Outcome: تحقق زیرساخت فنی محصول (OUT-101) [تکمیل: ۶۵٪]
  ├── 🔗 WorkLine: زنجیره ترتیبی احراز هویت [SEQUENTIAL_CHAIN] ─── (۳/۵ تسک) [=====---]
  │     ├── ▶️ [T-001] تعریف مدل دیتابیس (IN_PROGRESS) - ۳ پومودورو
  │     ├── 🔒 [T-002] پیاده‌سازی اندپوئینت توکن (قفل: در انتظار T-001)
  │     └── 🔒 [T-003] تست‌های یکپارچگی احراز هویت (قفل)
  └── 🔗 WorkLine: تنظیمات ظاهری داشبورد [GOAL_CLUSTER] ─── (۲/۲ تسک) [========]
        └── 📋 [T-004] بازطراحی پالت رنگی (OPEN)

📦 تسک‌های بدون Outcome (آزاد / Unassigned)
  └── 🔗 تسک‌های مستقل (Standalone Tasks)
        └── 📋 [T-005] پاکسازی کش لاگ‌ها (OPEN)
```

#### گام‌های پیاده‌سازی:
1. **استخراج ساختار خوشه‌بندی:**
   - گروه‌بندی اول: گروه‌بندی تسک‌های فیلترشده (`filteredTasks`) بر اساس `outcome_id`.
   - ایجاد یک دسته‌بندی پیش‌فرض برای تسک‌هایی که فاقد `outcome_id` هستند.
   - نگاشت اطلاعات کامل Outcome از آرایه `dash.outcomes`.
2. **گروه‌بندی دوم درون هر Outcome بر اساس `work_line_id`:**
   - نگاشت متادیتای WorkLine از آرایه `dash.work_lines`.
   - گروه‌بندی تسک‌های بدون WorkLine تحت عنوان «تسک‌های متفرقه/آزاد».
3. **نمایش شاخص‌های عملکردی WorkLine:**
   - نمایش Badge رنگی با استفاده از فیلد `color_hex` مربوط به WorkLine.
   - نمایش نوار درصد پیشرفت (`completed_tasks / total_tasks * 100`).
   - نمایش نوع خط کاری (`SEQUENTIAL_CHAIN` یا `GOAL_CLUSTER`).
4. **نمایش وضعیت و محدودیت‌های زنجیره‌ای تسک:**
   - چنانچه `is_locked_by_chain == true`، آیکون قفل 🔒 و غیرفعال‌سازی دکمه‌های اقدام سریع.
   - نمایش شماره ترتیب اجرای متوالی (`#${task.sequence_order}`) در زنجیره‌های ترتیبی.
   - نمایش شناسه تسک پیش‌نیاز (`depends_on_task_id`).
5. **کنترل تعاملی باز/بسته شدن (Collapse / Expand):**
   - نگهداری مجموعه‌ای از شناسه‌های باز در State کامپوننت (`collapsedWorkLines: Set<string>`).
   - امکان جمع کردن هر WorkLine یا Outcome برای تمرکز بر بخش‌های مشخص پروژه.

---

### فاز ۳: پیاده‌سازی اولویت‌بندی سه‌گانه موازی (Rust & UI)

**هدف:** جایگزینی پیشنهاد تک‌تسکی با ۳ گام با بالاترین اولویت کاری در امتداد خطوط کاری مجزا، تا انسداد در یک مسیر، کل پیشرفت کاربر را متوقف نکند.

#### ۳-الف: لایه بک‌اند (`src-tauri/src/models.rs` و `src-tauri/src/db/queries.rs`)
1. اضافه کردن فیلد به ساختار `DashboardState` در Rust:
```rust
pub struct DashboardState {
    // فیلدهای قبلی...
    pub domains: Vec<LifeDomain>,
    pub outcomes: Vec<Outcome>,
    pub work_lines: Vec<WorkLine>,
    pub top_priority_steps: Vec<WorkEntity>, // فیلد جدید
}
```
2. اصلاح متد `get_dashboard_state_for_project` در `queries.rs`:
```rust
// منطق انتخاب ۳ تسک برتر از خطوط کاری مجزا:
let top_priority_steps: Vec<WorkEntity> = work_lines
    .iter()
    .filter_map(|wl| {
        open_tasks.iter()
            .filter(|t| t.work_line_id.as_deref() == Some(wl.id.as_str()))
            .filter(|t| !t.is_locked_by_chain)
            .find(|t| t.status == "IN_PROGRESS")
            .or_else(|| {
                open_tasks.iter()
                    .filter(|t| t.work_line_id.as_deref() == Some(wl.id.as_str()))
                    .filter(|t| !t.is_locked_by_chain)
                    .min_by_key(|t| t.sequence_order)
            })
            .cloned()
    })
    .take(3)
    .collect();
```

#### ۳-ب: لایه فرانت‌اند (`src/components/PathAndChatView.tsx`)
1. جایگزینی بخش منفرد `Path Ahead` با یک گرید ۳ ستونه یا کارت‌های اولویت‌بندی‌شده:
   - دریافت `dash.top_priority_steps` (با fallback به `[dash.next_step_task]` در صورت خالی بودن).
2. هر کارت شامل اطلاعات زیر خواهد بود:
   - برچسب رنگی خط کاری با `color_hex` و نام `work_line_title`.
   - نام دستاورد مرتبط (`outcome_id` / عنوان Outcome).
   - نشانگر وضعیت (`IN_PROGRESS` یا `OPEN`).
   - دکمه فعال «تکمیل گام» صرفاً برای تسک‌هایی که در وضعیت `IN_PROGRESS` هستند.
   - دکمه «شروع گام» برای تسک‌های `OPEN` جهت تغییر وضعیت به `IN_PROGRESS`.

---

### فاز ۴: مکانیزم صف پشتیبان چت در زمان بروز خطای هوش مصنوعی (Queue Fallback)

**هدف:** تضمین عدم گم شدن پیام‌های کاربر و فایل‌های ورودی در صورت خطای اتصال شبکه یا بروز مشکل در سرویس‌دهنده LLM.
**فایل هدف:** `src-tauri/src/lib.rs` و `src-tauri/src/db/queries.rs`

#### جزئیات معماری:
1. در `queries.rs`، بررسی و تضمین وجود ساختار کمکی و تابع درج در صف:
```rust
pub struct PendingQueueInput<'a> {
    pub project_id: &'a str,
    pub session_id: &'a str,
    pub channel: &'a str,
    pub user_message_id: Option<i64>,
    pub title_preview: &'a str,
    pub source_type: &'a str,
    pub archived_path: Option<&'a str>,
}

pub fn insert_pending_queue_item(conn: &rusqlite::Connection, input: PendingQueueInput) -> Result<i64> {
    enqueue_pending_ai_item(
        conn,
        input.project_id,
        input.session_id,
        input.channel,
        input.user_message_id,
        input.title_preview,
        input.source_type,
        input.archived_path,
    )
}
```
2. در `lib.rs`، در توابع `cmd_run_assistant_turn` و `cmd_run_file_assistant_turn`:
   - هنگامی که نتیجه فراخوانی موتور LLM با خطا مواجه می‌شود (`Err(e)`):
```rust
if let Err(e) = &llm_result {
    log::warn!("LLM execution failed: {}. Enqueueing message into pending_ai_queue", e);
    let preview = if prompt.len() > 100 { &prompt[..100] } else { &prompt };
    let _ = queries::insert_pending_queue_item(&conn, queries::PendingQueueInput {
        project_id: &project_id,
        session_id: &session_id,
        channel: &channel,
        user_message_id: Some(user_msg_id),
        title_preview: preview,
        source_type: "CHAT_MESSAGE",
        archived_path: Some(&archived_path),
    });
}
```
3. این پیام‌ها سپس در وضعیت `PENDING` در جدول دیتابیس باقی می‌مانند و فرانت‌اند از طریق کامپوننت `PendingQueueBanner.tsx` امکان تلاش مجدد (Retry) را به کاربر نمایش می‌دهد.

---

## ۴. ماتریس راستی‌آزمایی و استراتژی تست (Verification & Integrity)

| بررسی | ابزار / دستور | خروجی مورد انتظار |
|---|---|---|
| بررسی سینتکس و تایپ فرانت‌اند | `npm run build` | بدون خطا؛ ایجاد باندل تولیدی در `dist/` |
| بررسی کامپایل بک‌اند Rust | `cargo check --manifest-path src-tauri/Cargo.toml` | بدون اخطار یا خطای نوع‌داده (Exit Code 0) |
| تست‌های پایگاه داده و منطق Rust | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | موفقیت ۱۰۰٪ تست‌های منطق داخلی و تبدیل مدل‌ها |
| بررسی یکپارچگی و اصالت کد | ممیزی فایل‌ها و تست دستی | عدم وجود کدهای ساختگی یا فیک (Zero Fake / Zero Facade) |

---
این سند به عنوان مرجع قطعی معماری و نقشه راه کلیه فازهای توسعه سیستم Mab-Compass ملاک عمل است.
