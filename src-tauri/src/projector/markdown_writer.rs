use crate::models::DashboardState;
use crate::vault_manager::{ensure_project_directories, sanitize_filename};
use chrono::Local;
use std::collections::HashMap;
use std::fs;

fn escape_md_cell(raw: &str) -> String {
    raw.replace('|', "\\|").replace(['\r', '\n'], " ")
}

pub fn sync_all_project_projections(
    state: &DashboardState,
    meeting_title: Option<&str>,
    meeting_md: Option<&str>,
) -> Result<(), String> {
    let root = ensure_project_directories(&state.active_project.workspace_path)?;

    // ۱. پوشه 02_path_ahead: فایل NEXT_STEP.md و نقشه راه WBS_ROADMAP.md
    let mut next_md = format!(
        "# تیکت گام فعال — {}\n> **آخرین کالیبراسیون خودکار:** `{}`\n\n",
        state.active_project.name,
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    if !state.active_guardrails.is_empty() {
        next_md.push_str("## 🛑 سد پیشگیرانه ضدتکرار خطا (Pre-Flight Gate)\n");
        for g in &state.active_guardrails {
            next_md.push_str(&format!(
                "- [ ] **{}** *(الگو: {})*\n",
                g.preventive_gate_question, g.mistake_pattern
            ));
        }
        next_md.push_str("\n---\n\n");
    }

    if let Some(task) = &state.next_step_task {
        next_md.push_str(&format!(
            "## 🎯 گام فعال کنونی: `[{}]` {}\n- **دسته‌بندی:** `{}` | **تخمین کالیبره‌شده PERT:** `{}` پومودورو\n\n### ⚡ اولین اقدام فیزیکی ۲ دقیقه‌ای\n> **{}**\n\n### 🏁 شرط خروج قطعی (Definition of Done)\n- {}\n\n",
            task.id,
            task.title,
            task.category,
            task.calibrated_pomos,
            task.first_physical_action,
            task.definition_of_done
        ));
    } else {
        next_md.push_str("## ✅ هیچ گام بازی در صف نیست.\n");
    }

    if !state.horizon_tasks.is_empty() {
        next_md.push_str("## 🛤️ افق گام‌های بعدی در مسیر بحرانی\n");
        for (idx, ht) in state.horizon_tasks.iter().enumerate() {
            next_md.push_str(&format!(
                "{}. **`[{}]` {}** ({} Pomo) — *شروع با: {}*\n",
                idx + 2,
                ht.id,
                ht.title,
                ht.calibrated_pomos,
                ht.first_physical_action
            ));
        }
    }

    fs::write(root.join("02_path_ahead").join("NEXT_STEP.md"), next_md)
        .map_err(|e| e.to_string())?;

    let mut wbs_md = format!(
        "# نقشه راه و ساختار شکست کار (WBS) — {}\n\n| کد | عنوان گام | دسته | وضعیت | تخمین PERT | شرط خروج (DoD) |\n|---|---|---|---|---|---|\n",
        state.active_project.name
    );
    for t in &state.open_tasks {
        wbs_md.push_str(&format!(
            "| `{}` | {} | `{}` | **{}** | `{} Pomo` | {} |\n",
            t.id,
            escape_md_cell(&t.title),
            t.category,
            t.status,
            t.calibrated_pomos,
            escape_md_cell(&t.definition_of_done)
        ));
    }
    fs::write(root.join("02_path_ahead").join("WBS_ROADMAP.md"), wbs_md)
        .map_err(|e| e.to_string())?;

    // ۲. پوشه 03_decisions_adr: رجیستری تصمیمات
    let mut adr_md = format!(
        "# رجیستری تصمیمات معماری و توافقات (ADR) — {}\n\n",
        state.active_project.name
    );
    for d in &state.active_decisions {
        adr_md.push_str(&format!(
            "## `[{}]` {}\n- **جزئیات و مبنا:** {}\n- **ذی‌نفع:** `{}`\n\n",
            d.id,
            d.title,
            d.definition_of_done,
            d.stakeholder_tag.clone().unwrap_or_else(|| "داخلی".into())
        ));
    }
    fs::write(root.join("03_decisions_adr").join("ADR_REGISTRY.md"), adr_md)
        .map_err(|e| e.to_string())?;

    // ۳. پوشه 04_sops_playbooks: دستورالعمل‌های زنده
    let mut sop_groups: HashMap<String, Vec<String>> = HashMap::new();
    for s in &state.active_sops {
        sop_groups
            .entry(s.category.clone())
            .or_default()
            .push(s.rule_statement.clone());
    }
    for (cat, rules) in sop_groups {
        let mut content = format!("# دستورالعمل اجرایی زنده (SOP): {}\n\n", cat);
        for r in rules {
            content.push_str(&format!("- [ ] {}\n", r));
        }
        let safe_cat = sanitize_filename(&cat.to_lowercase());
        fs::write(
            root.join("04_sops_playbooks")
                .join(format!("{}_SOP.md", safe_cat)),
            content,
        )
        .map_err(|e| e.to_string())?;
    }

    // ۴. پوشه 05_meetings_minutes: صورت‌جلسات رسمی
    if let (Some(title), Some(body)) = (meeting_title, meeting_md) {
        if !body.trim().is_empty() {
            let safe_title = sanitize_filename(title);
            let fname = format!("{}_{}.md", Local::now().format("%Y-%m-%d_%H%M"), safe_title);
            fs::write(root.join("05_meetings_minutes").join(fname), body)
                .map_err(|e| e.to_string())?;
        }
    }

    // ۵. پوشه 06_lessons_antipatterns: بانک درس‌آموخته‌ها
    let mut lessons_md = format!(
        "# بانک تله‌ها و درس‌آموخته‌های اجرایی — {}\n\n",
        state.active_project.name
    );
    for g in &state.active_guardrails {
        lessons_md.push_str(&format!(
            "- **[{}]** خطا: {} → **گیت پیشگیرانه:** {}\n",
            g.trigger_category, g.mistake_pattern, g.preventive_gate_question
        ));
    }
    fs::write(
        root.join("06_lessons_antipatterns")
            .join("LESSONS_LEARNED.md"),
        lessons_md,
    )
    .map_err(|e| e.to_string())?;

    // ۶. پوشه 07_client_reports: گزارش پیشرفت کارفرما
    let mut rep_md = format!(
        "# گزارش رسمی پیشرفت کارفرما — {}\n\n| تاریخ | پومودورو | شرح خدمات و اقلام تحویلی مهندسی |\n|---|---|---|\n",
        state.active_project.name
    );
    for l in &state.recent_logs {
        let date_str: String = l.logged_at.chars().take(10).collect();
        rep_md.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            date_str,
            l.effective_pomos,
            escape_md_cell(&l.client_sanitized_note)
        ));
    }
    fs::write(root.join("07_client_reports").join("CLIENT_REPORT.md"), rep_md)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use std::path::PathBuf;

    #[test]
    fn test_sync_all_v2_folders_projections() {
        let dir = std::env::temp_dir().join(format!("mab_proj_v2_{}", uuid::Uuid::new_v4()));
        let state = DashboardState {
            projects: vec![],
            active_project: Project {
                id: "GERDALO".into(),
                name: "پروژه گردالو".into(),
                workspace_path: dir.to_string_lossy().to_string(),
                external_focus_id: Some("GERDALO".into()),
                is_active: true,
                updated_at: "2026-11-11".into(),
            },
            next_step_task: Some(WorkEntity {
                id: "TSK-101".into(),
                project_id: "GERDALO".into(),
                entity_type: "TASK".into(),
                category: "BACKEND".into(),
                title: "تست سرویس اصلی".into(),
                definition_of_done: "تست پاس شود".into(),
                first_physical_action: "باز کردن lib.rs".into(),
                status: "OPEN".into(),
                is_on_critical_path: true,
                optimistic_pomos: 1.0,
                most_likely_pomos: 2.0,
                pessimistic_pomos: 3.0,
                calibrated_pomos: 2.0,
                superseded_by_id: None,
                stakeholder_tag: None,
                updated_at: "2026-11-11".into(),
            }),
            horizon_tasks: vec![],
            active_guardrails: vec![LearnedAntipattern {
                id: 1,
                project_id: "GERDALO".into(),
                trigger_category: "BACKEND".into(),
                mistake_pattern: "فراموشی ایندکس دیتابیس".into(),
                preventive_gate_question: "آیا ایندکس اضافه شد؟".into(),
                severity_score: 8,
                times_triggered: 1,
            }],
            active_sops: vec![SopRule {
                id: 1,
                project_id: "GERDALO".into(),
                category: "BACKEND".into(),
                rule_statement: "همیشه تراکنش اتمیک بساز".into(),
                updated_at: "2026-11-11".into(),
            }],
            open_tasks: vec![],
            closed_tasks_count: 0,
            active_decisions: vec![WorkEntity {
                id: "ADR-01".into(),
                project_id: "GERDALO".into(),
                entity_type: "DECISION".into(),
                category: "ARCHITECTURE".into(),
                title: "استفاده از SQLite WAL".into(),
                definition_of_done: "همگام‌سازی بدون قفل".into(),
                first_physical_action: "".into(),
                status: "OPEN".into(),
                is_on_critical_path: false,
                optimistic_pomos: 0.0,
                most_likely_pomos: 0.0,
                pessimistic_pomos: 0.0,
                calibrated_pomos: 0.0,
                superseded_by_id: None,
                stakeholder_tag: Some("@Architect".into()),
                updated_at: "2026-11-11".into(),
            }],
            recent_logs: vec![SessionWorkLog {
                id: 1,
                project_id: "GERDALO".into(),
                task_id: Some("TSK-101".into()),
                category: "BACKEND".into(),
                estimated_pomos: 2.0,
                effective_pomos: 2,
                deliverable_summary: "تحویل نسخه اولیه".into(),
                internal_friction_note: "".into(),
                client_sanitized_note: "پیاده‌سازی هسته | فاز ۱".into(),
                source_origin: "USER_CHAT".into(),
                logged_at: "2026-11-11T10:00:00+03:30".into(),
            }],
            chat_history: vec![],
            vault_chat_history: vec![],
            chat_sessions: vec![],
            all_project_chat_sessions: vec![],
            active_copilot_session_id: String::new(),
            active_vault_session_id: String::new(),
            raw_events_count: 0,
            recent_raw_events: vec![],
            knowledge_docs: vec![],
            pending_queue: vec![],
            settings: AppSettings {
                vault_root_path: dir.to_string_lossy().to_string(),
                mab_focus_data_path: dir.to_string_lossy().to_string(),
                auto_sync_mab_focus: true,
                active_llm_provider: "AGY_CLI".into(),
                agy_model: "Gemini 3.1 Pro (High)".into(),
                gemini_model: "gemini-2.5-flash".into(),
                local_model: String::new(),
                gemini_api_key: String::new(),
                local_endpoint: "http://127.0.0.1:1234/v1/chat/completions".into(),
            },
            can_undo: false,
            last_undo_summary: None,
        };

        sync_all_project_projections(
            &state,
            Some("جلسه بررسی: نسخه ۱ / ۲؟"),
            Some("# صورت‌جلسه رسمی\nتوافق انجام شد."),
        )
        .unwrap();

        assert!(dir.join("02_path_ahead").join("NEXT_STEP.md").exists());
        assert!(dir.join("02_path_ahead").join("WBS_ROADMAP.md").exists());
        assert!(dir.join("03_decisions_adr").join("ADR_REGISTRY.md").exists());
        assert!(dir.join("04_sops_playbooks").join("backend_SOP.md").exists());
        assert!(dir.join("06_lessons_antipatterns").join("LESSONS_LEARNED.md").exists());
        assert!(dir.join("07_client_reports").join("CLIENT_REPORT.md").exists());
        let meetings_count = fs::read_dir(dir.join("05_meetings_minutes")).unwrap().count();
        assert_eq!(meetings_count, 1);

        let _ = fs::remove_dir_all(PathBuf::from(dir));
    }
}
