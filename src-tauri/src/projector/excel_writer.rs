use crate::models::SessionWorkLog;
use rust_xlsxwriter::{Format, Workbook};
use std::fs;
use std::path::Path;

fn sanitize_sheet_name(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '\\' | '/' | '?' | '*' | '[' | ']' | ':' => '-',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('\'');
    let truncated: String = trimmed.chars().take(31).collect();
    let final_name = truncated.trim().trim_matches('\'').to_string();
    if final_name.is_empty() {
        "ClientReport".to_string()
    } else {
        final_name
    }
}

pub fn export_client_xlsx(
    output_path: &Path,
    project_name: &str,
    logs: &[SessionWorkLog],
) -> Result<String, String> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_right_to_left(true);
    let safe_name = sanitize_sheet_name(project_name);
    worksheet.set_name(&safe_name).map_err(|e| e.to_string())?;

    let header_fmt = Format::new().set_bold();

    let headers = [
        "تاریخ ثبت",
        "کد تسک",
        "بخش فنی",
        "پومودوروی مؤثر",
        "خروجی تحویل‌شده",
        "توضیحات فنی کارفرما (Sanitized)",
        "منبع ثبت",
    ];

    for (col, text) in headers.iter().enumerate() {
        worksheet
            .write_with_format(0, col as u16, *text, &header_fmt)
            .map_err(|e| e.to_string())?;
        worksheet
            .set_column_width(col as u16, 26)
            .map_err(|e| e.to_string())?;
    }

    for (idx, log) in logs.iter().enumerate() {
        let row = (idx + 1) as u32;
        let date_str: String = log.logged_at.chars().take(10).collect();
        worksheet
            .write_string(row, 0, &date_str)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(row, 1, log.task_id.as_deref().unwrap_or("-"))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(row, 2, &log.category)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(row, 3, log.effective_pomos as f64)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(row, 4, &log.deliverable_summary)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(row, 5, &log.client_sanitized_note)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(row, 6, &log.source_origin)
            .map_err(|e| e.to_string())?;
    }

    workbook.save(output_path).map_err(|e| e.to_string())?;
    Ok(output_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_xlsx_with_slash_in_project_name() {
        let dir = std::env::temp_dir().join(format!("mab_xlsx_test_{}", uuid::Uuid::new_v4()));
        let out = dir.join("07_client_reports").join("IRANOIL_Timesheet_Report.xlsx");
        let logs = vec![SessionWorkLog {
            id: 1,
            project_id: "IRANOIL".into(),
            task_id: Some("TSK-101".into()),
            category: "BACKEND".into(),
            estimated_pomos: 2.0,
            effective_pomos: 2,
            deliverable_summary: "تحویل سرویس".into(),
            internal_friction_note: "حواس‌پرتی".into(),
            client_sanitized_note: "توسعه و آزمون سرویس".into(),
            source_origin: "MAB_FOCUS_SYNC".into(),
            logged_at: "2026-11-11T00:00:00+03:30".into(),
        }];
        let res = export_client_xlsx(&out, "پروژه ایران‌اویل‌جاب / زیگهوش", &logs);
        assert!(res.is_ok());
        assert!(out.exists());
        let _ = fs::remove_dir_all(dir);
    }
}
