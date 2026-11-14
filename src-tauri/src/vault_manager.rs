use chrono::Local;
use std::fs;
use std::path::PathBuf;

pub const SUBFOLDERS: &[&str] = &[
    "00_raw_inputs",
    "01_knowledge_archive",
    "02_path_ahead",
    "03_decisions_adr",
    "04_sops_playbooks",
    "05_meetings_minutes",
    "06_lessons_antipatterns",
    "07_client_reports",
];

/// Sanitize a file or title stem for Windows/macOS/Linux filesystems while preserving Persian text.
pub fn sanitize_filename(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_control()
                || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else if c == ' ' {
                '_'
            } else if c.is_alphanumeric() || c == '-' || c == '_' || c > '\u{0600}' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let collapsed = cleaned
        .trim_matches(|c: char| c == '_' || c == '.' || c.is_whitespace())
        .chars()
        .take(72)
        .collect::<String>()
        .trim_matches(|c: char| c == '_' || c == '.' || c.is_whitespace())
        .to_string();

    if collapsed.is_empty() {
        "document".to_string()
    } else {
        collapsed
    }
}

/// Ensure all 8 standard project vault directories exist on disk for a project.
pub fn ensure_project_directories(project_workspace_path: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(project_workspace_path);
    if !root.exists() {
        fs::create_dir_all(&root).map_err(|e| format!("خطا در ساخت پوشه پروژه: {}", e))?;
    }
    for sub in SUBFOLDERS {
        let p = root.join(sub);
        if !p.exists() {
            fs::create_dir_all(&p)
                .map_err(|e| format!("خطا در ساخت زیرپوشه {}: {}", sub, e))?;
        }
    }
    Ok(root)
}

/// Immediately archive raw user text or Mab-Focus synced notes in `00_raw_inputs/`.
pub fn archive_raw_user_input(
    project_workspace_path: &str,
    source_tag: &str,
    raw_text: &str,
) -> Result<String, String> {
    let root = ensure_project_directories(project_workspace_path)?;
    let now = Local::now();
    let safe_tag = sanitize_filename(source_tag);
    let filename = format!("{}_{}.md", now.format("%Y-%m-%d_%H-%M-%S_%3f"), safe_tag);
    let target_path = root.join("00_raw_inputs").join(&filename);

    let file_content = format!(
        "---\ntimestamp: {}\nsource: {}\n---\n\n{}\n",
        now.to_rfc3339(),
        source_tag,
        raw_text
    );

    fs::write(&target_path, file_content)
        .map_err(|e| format!("خطا در آرشیو ورودی خام: {}", e))?;
    Ok(target_path.to_string_lossy().to_string())
}

/// Archive historical / onboarding project documents in `01_knowledge_archive/`.
pub fn archive_onboarding_document(
    project_workspace_path: &str,
    original_title: &str,
    raw_content: &str,
    ai_summary: &str,
) -> Result<String, String> {
    let root = ensure_project_directories(project_workspace_path)?;
    let safe_name = sanitize_filename(original_title);
    let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = format!("{}_{}.md", timestamp, safe_name);
    let target_path = root.join("01_knowledge_archive").join(&filename);

    let full_doc = format!(
        "# سند آرشیو دانش: {}\n> **تاریخ ورود به حافظه:** `{}`\n\n## خلاصه مدیریتی استخراج‌شده توسط هوش مصنوعی\n{}\n\n---\n\n## متن کامل سند اصلی\n\n{}\n",
        original_title,
        Local::now().format("%Y-%m-%d %H:%M"),
        ai_summary,
        raw_content
    );

    fs::write(&target_path, full_doc).map_err(|e| e.to_string())?;
    Ok(target_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_8_subfolders_and_archives() {
        let dir = std::env::temp_dir().join(format!("mab_vault_test_{}", uuid::Uuid::new_v4()));
        let proj_dir = dir.join("GERDALO");
        let root = ensure_project_directories(&proj_dir.to_string_lossy()).unwrap();

        for sub in SUBFOLDERS {
            assert!(root.join(sub).is_dir(), "Missing subfolder {}", sub);
        }

        let raw_path =
            archive_raw_user_input(&proj_dir.to_string_lossy(), "user_chat", "تست پیام").unwrap();
        assert!(PathBuf::from(&raw_path).exists());

        let doc_path = archive_onboarding_document(
            &proj_dir.to_string_lossy(),
            "صورت‌جلسه ۱: بررسی / رفع باگ؟",
            "محتوای کامل سند",
            "خلاصه مدیریتی",
        )
        .unwrap();
        assert!(PathBuf::from(&doc_path).exists());

        let _ = fs::remove_dir_all(dir);
    }
}
