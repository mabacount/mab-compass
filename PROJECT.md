# Project: Mab-Compass 4-Phase System Fix & Enhancement

## Architecture
- **Framework**: Tauri v2 + React 18 + TypeScript + Vite + TailwindCSS + SQLite (via `rusqlite` in Rust).
- **Backend**: Rust (`src-tauri/src/`) handling database queries, migrations, models, LLM routing, and IPC commands.
- **Frontend**: React (`src/`) consuming Tauri IPC commands through `src/lib/tauri_api.ts`.
- **Data Flow**:
  1. SQLite stores `life_domains`, `outcomes`, `work_lines`, `work_entities`, `pending_ai_queue`.
  2. `queries.rs` queries SQLite rows and maps them into Rust structs (`models.rs`).
  3. Tauri commands serialize structs to JSON over IPC.
  4. `src/lib/tauri_api.ts` defines TypeScript interfaces matching Rust JSON payloads.
  5. UI views (`WbsAndDocsView.tsx`, `PathAndChatView.tsx`) consume `DashboardState` to render grouped hierarchy and priority steps.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | TypeScript Types Alignment | Add `LifeDomain`, `Outcome`, `WorkLine`, update `WorkEntity` (15 fields) and `DashboardState` | M1 | ORIGINAL_REQUEST § فاز ۱ |
| 2 | Implementation Plan Document | Write comprehensive plan in `IMPLEMENTATION_PLAN.md` | M1 | ORIGINAL_REQUEST § مهم |
| 3 | Rust Top Priority 3 Steps Logic | Update `get_dashboard_state_for_project` & `models.rs` for 3 active steps across WorkLines | M2 | ORIGINAL_REQUEST § فاز ۳-الف |
| 4 | Rust Queue Fallback for Chat | Add `PendingQueueInput`, `insert_pending_queue_item`, and fallback on LLM errors in `lib.rs` | M2 | ORIGINAL_REQUEST § فاز ۴ |
| 5 | WbsAndDocsView Hierarchical Grouping | 2-level grouping (Outcome -> WorkLine), progress bars, color badges, chain locks, collapse/expand | M3 | ORIGINAL_REQUEST § فاز ۲ |
| 6 | PathAndChatView 3 Priority Cards | 3 cards with WorkLine/Outcome badges and conditional "تکمیل گام" button for IN_PROGRESS | M3 | ORIGINAL_REQUEST § فاز ۳-ب |
| 7 | Full End-to-End Build & Verification | `cargo build`, `npm run build`, `cargo test`, and write `WALKTHROUGH.md` | M4 | ORIGINAL_REQUEST § بعد از هر فاز & مهم |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | M1: TypeScript Layer & Plan | `src/lib/tauri_api.ts`, `IMPLEMENTATION_PLAN.md` | none | PLANNED |
| 2 | M2: Rust Backend Queries & Fallback | `src-tauri/src/models.rs`, `src-tauri/src/db/queries.rs`, `src-tauri/src/lib.rs` | none | PLANNED |
| 3 | M3: Frontend Views Refactoring | `src/components/WbsAndDocsView.tsx`, `src/components/PathAndChatView.tsx` | M1, M2 | PLANNED |
| 4 | M4: Final Verification & Walkthrough | `cargo build`, `npm run build`, `cargo test`, `WALKTHROUGH.md` | M3 | PLANNED |

## Interface Contracts
### Rust IPC ↔ TypeScript (`tauri_api.ts`)
- `LifeDomain`: `{ code: string, title: string, description: string, maturity_state: number, maintenance_floor: string, desired_state: string, is_growth_focus: boolean, updated_at: string }`
- `Outcome`: `{ id: string, domain_code: string, title: string, desired_reality_change: string, evidence_kr: string, time_window: string, status: string, progress_percent: number, linked_projects_count: number, created_at: string, updated_at: string }`
- `WorkLine`: `{ id: string, project_id: string, project_name: string, outcome_id?: string | null, outcome_title?: string | null, title: string, goal_statement: string, line_mode: string, status: string, color_hex: string, total_tasks: number, completed_tasks: number, active_step_id?: string | null, created_at: string, updated_at: string }`
- `WorkEntity` additions: `work_line_id?: string | null`, `work_line_title?: string | null`, `outcome_id?: string | null`, `sequence_order: number`, `depends_on_task_id?: string | null`, `is_locked_by_chain: boolean`, `energy_tag: string`, `duration_minutes: number`, `device_tag: string`, `location_tags: string[]`, `scheduled_date?: string | null`, `scheduled_slot?: string | null`, `due_date?: string | null`, `priority_score: number`, `project_name?: string`
- `DashboardState` additions: `domains: LifeDomain[]`, `outcomes: Outcome[]`, `work_lines: WorkLine[]`, `top_priority_steps?: WorkEntity[]`

### Rust DB queries (`queries.rs`)
- `PendingQueueInput<'a>`: `{ project_id: &'a str, session_id: &'a str, channel: &'a str, user_message_id: Option<i64>, title_preview: &'a str, source_type: &'a str, archived_path: Option<&'a str> }`
- `insert_pending_queue_item(conn: &Connection, input: PendingQueueInput) -> Result<i64>`: Calls `enqueue_pending_ai_item`.

## Code Layout
- `src/lib/tauri_api.ts`: TypeScript definitions & Tauri IPC wrappers.
- `src/components/WbsAndDocsView.tsx`: Tabbed WBS view with Outcome & WorkLine grouping.
- `src/components/PathAndChatView.tsx`: Path Ahead with 3 priority cards & Copilot chat.
- `src-tauri/src/models.rs`: Rust domain models with Serde serialization.
- `src-tauri/src/db/queries.rs`: SQLite database query logic & dashboard state compilation.
- `src-tauri/src/lib.rs`: Tauri IPC commands, assistant turn execution, queue error fallback.
- `IMPLEMENTATION_PLAN.md`: Technical implementation plan.
- `WALKTHROUGH.md`: Final walkthrough documentation.
