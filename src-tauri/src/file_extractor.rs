use base64::Engine;
use encoding_rs::{UTF_16BE, UTF_16LE, WINDOWS_1256};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::io::{Cursor, Read};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, Clone)]
pub struct ExtractedFileContent {
    pub text: String,
    pub detected_format: String,
}

pub fn extract_text_from_base64(
    filename: &str,
    content_base64: &str,
) -> Result<ExtractedFileContent, String> {
    let cleaned_b64: String = content_base64
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cleaned_b64)
        .map_err(|e| format!("خطا در دیکد محتوای فایل {}: {}", filename, e))?;
    extract_text_from_bytes(filename, &bytes)
}

pub fn extract_text_from_bytes(
    filename: &str,
    bytes: &[u8],
) -> Result<ExtractedFileContent, String> {
    if bytes.is_empty() {
        return Err(format!("فایل «{}» خالی است.", filename));
    }

    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let is_pdf = ext == "pdf" || bytes.starts_with(b"%PDF-");
    if is_pdf {
        let text = extract_pdf_text(filename, bytes)?;
        return Ok(ExtractedFileContent {
            text,
            detected_format: "PDF".to_string(),
        });
    }

    let is_zip = bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06");
    if is_zip || matches!(ext.as_str(), "docx" | "pptx" | "xlsx" | "odt") {
        if let Some(office) = try_extract_office_zip(filename, &ext, bytes)? {
            return Ok(office);
        }
    }

    let text = decode_text_or_source_code(filename, bytes)?;
    let detected_format = if ext.is_empty() {
        "TEXT".to_string()
    } else {
        ext.to_uppercase()
    };

    Ok(ExtractedFileContent {
        text,
        detected_format,
    })
}

fn extract_pdf_text(filename: &str, bytes: &[u8]) -> Result<String, String> {
    let parsed = catch_unwind(AssertUnwindSafe(|| pdf_extract::extract_text_from_mem(bytes)));
    match parsed {
        Ok(Ok(raw)) => {
            let normalized = normalize_extracted_text(&raw);
            if normalized.is_empty() {
                Err(format!(
                    "فایل PDF «{}» متن قابل استخراجی نداشت (احتمالاً اسکن‌شده/تصویری است یا فونت آن فاقد نقشه کاراکتر است).",
                    filename
                ))
            } else {
                Ok(normalized)
            }
        }
        Ok(Err(err)) => Err(format!("خطا در استخراج متن از PDF «{}»: {}", filename, err)),
        Err(_) => Err(format!(
            "ساختار فایل PDF «{}» قابل پردازش نبود (ممکن است آسیب‌دیده یا رمزنگاری‌شده باشد).",
            filename
        )),
    }
}

fn try_extract_office_zip(
    filename: &str,
    ext: &str,
    bytes: &[u8],
) -> Result<Option<ExtractedFileContent>, String> {
    let cursor = Cursor::new(bytes);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => {
            if matches!(ext, "docx" | "pptx" | "xlsx" | "odt") {
                return Err(format!("فایل «{}» ساختار ZIP/Office معتبری ندارد: {}", filename, e));
            }
            return Ok(None);
        }
    };

    let names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();

    // 1. DOCX
    if ext == "docx" || names.iter().any(|n| n == "word/document.xml") {
        let mut target_files: Vec<String> = Vec::new();
        if names.iter().any(|n| n == "word/document.xml") {
            target_files.push("word/document.xml".to_string());
        }
        let mut extra: Vec<String> = names
            .iter()
            .filter(|n| {
                (n.starts_with("word/header")
                    || n.starts_with("word/footer")
                    || n == &"word/footnotes.xml"
                    || n == &"word/endnotes.xml"
                    || n == &"word/comments.xml")
                    && n.ends_with(".xml")
            })
            .cloned()
            .collect();
        extra.sort();
        target_files.extend(extra);

        if target_files.is_empty() {
            return Err(format!("فایل DOCX «{}» فاقد بخش محتوای word/document.xml است.", filename));
        }

        let mut parts: Vec<String> = Vec::new();
        for name in target_files {
            if let Ok(mut entry) = archive.by_name(&name) {
                let mut xml = String::new();
                if entry.read_to_string(&mut xml).is_ok() {
                    let extracted = extract_wordprocessing_xml_text(&xml);
                    if !extracted.is_empty() {
                        parts.push(extracted);
                    }
                }
            }
        }
        let joined = normalize_extracted_text(&parts.join("\n\n"));
        if joined.is_empty() {
            return Err(format!("متن قابل استخراجی در فایل DOCX «{}» یافت نشد.", filename));
        }
        return Ok(Some(ExtractedFileContent {
            text: joined,
            detected_format: "DOCX".to_string(),
        }));
    }

    // 2. PPTX
    if ext == "pptx" || names.iter().any(|n| n.starts_with("ppt/slides/slide")) {
        let mut slides: Vec<String> = names
            .iter()
            .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
            .cloned()
            .collect();
        slides.sort();
        let mut parts = Vec::new();
        for s in slides {
            if let Ok(mut entry) = archive.by_name(&s) {
                let mut xml = String::new();
                if entry.read_to_string(&mut xml).is_ok() {
                    let t = extract_generic_xml_text_tags(&xml, &["a:t", "t"], &["a:p", "p"]);
                    if !t.is_empty() {
                        parts.push(t);
                    }
                }
            }
        }
        let joined = normalize_extracted_text(&parts.join("\n\n"));
        if !joined.is_empty() {
            return Ok(Some(ExtractedFileContent {
                text: joined,
                detected_format: "PPTX".to_string(),
            }));
        }
    }

    // 3. ODT
    if ext == "odt" || names.iter().any(|n| n == "content.xml") {
        if let Ok(mut entry) = archive.by_name("content.xml") {
            let mut xml = String::new();
            if entry.read_to_string(&mut xml).is_ok() {
                let t = extract_generic_xml_text_tags(&xml, &["text:p", "text:h", "text:span"], &["text:p", "text:h"]);
                let norm = normalize_extracted_text(&t);
                if !norm.is_empty() {
                    return Ok(Some(ExtractedFileContent {
                        text: norm,
                        detected_format: "ODT".to_string(),
                    }));
                }
            }
        }
    }

    // 4. XLSX shared strings
    if ext == "xlsx" || names.iter().any(|n| n == "xl/sharedStrings.xml") {
        if let Ok(mut entry) = archive.by_name("xl/sharedStrings.xml") {
            let mut xml = String::new();
            if entry.read_to_string(&mut xml).is_ok() {
                let t = extract_generic_xml_text_tags(&xml, &["t"], &["si"]);
                let norm = normalize_extracted_text(&t);
                if !norm.is_empty() {
                    return Ok(Some(ExtractedFileContent {
                        text: norm,
                        detected_format: "XLSX".to_string(),
                    }));
                }
            }
        }
    }

    Ok(None)
}

fn extract_wordprocessing_xml_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut out = String::new();
    let mut in_text_elem = false;
    let mut in_instr_text = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name();
                match name.as_ref() {
                    "w:t" | "t" | "w:delText" => in_text_elem = true,
                    "w:instrText" => in_instr_text = true,
                    _ => {}
                }
            }
            Ok(Event::Empty(e)) => {
                let name = e.name();
                match name.as_ref() {
                    "w:tab" | "tab" => out.push('\t'),
                    "w:br" | "w:cr" | "br" | "cr" => out.push('\n'),
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_elem && !in_instr_text {
                    out.push_str(&e.xml10_content());
                }
            }
            Ok(Event::GeneralRef(e)) => {
                if in_text_elem && !in_instr_text {
                    if let Ok(Some(ch)) = e.resolve_char_ref() {
                        out.push(ch);
                    } else {
                        match e.as_ref() {
                            "amp" => out.push('&'),
                            "lt" => out.push('<'),
                            "gt" => out.push('>'),
                            "quot" => out.push('"'),
                            "apos" => out.push('\''),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                match name.as_ref() {
                    "w:t" | "t" | "w:delText" => in_text_elem = false,
                    "w:instrText" => in_instr_text = false,
                    "w:p" | "p" | "w:tr" | "tr" => {
                        if !out.ends_with('\n') {
                            out.push('\n');
                        }
                    }
                    "w:tc" | "tc" => {
                        if !out.ends_with(" | ") && !out.ends_with('\n') {
                            out.push_str(" | ");
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    out.lines()
        .map(|l| l.trim_end_matches(|c| c == '|' || c == ' ').trim())
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_generic_xml_text_tags(xml: &str, text_tags: &[&str], block_tags: &[&str]) -> String {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut out = String::new();
    let mut depth_text = 0usize;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name();
                if text_tags.contains(&name.as_ref()) {
                    depth_text += 1;
                }
            }
            Ok(Event::Text(e)) => {
                if depth_text > 0 {
                    out.push_str(&e.xml10_content());
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                if text_tags.contains(&name.as_ref()) {
                    depth_text = depth_text.saturating_sub(1);
                }
                if block_tags.contains(&name.as_ref()) && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    out
}

fn decode_text_or_source_code(filename: &str, bytes: &[u8]) -> Result<String, String> {
    // 1. UTF-8 BOM
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        if let Ok(s) = std::str::from_utf8(&bytes[3..]) {
            return Ok(normalize_extracted_text(s));
        }
    }

    // 2. UTF-16 LE / BE BOM
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (cow, _, _) = UTF_16LE.decode(&bytes[2..]);
        let s = normalize_extracted_text(&cow);
        if !s.is_empty() {
            return Ok(s);
        }
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (cow, _, _) = UTF_16BE.decode(&bytes[2..]);
        let s = normalize_extracted_text(&cow);
        if !s.is_empty() {
            return Ok(s);
        }
    }

    // 3. Valid UTF-8
    if let Ok(s) = std::str::from_utf8(bytes) {
        if !s.contains('\0') {
            return Ok(normalize_extracted_text(s));
        }
    }

    // 4. Reject obvious binary files (images, executables, raw archives)
    let sample = &bytes[..bytes.len().min(8192)];
    let control_or_nul = sample
        .iter()
        .filter(|&&b| b == 0 || (b < 0x09) || (0x0E..=0x1F).contains(&b))
        .count();
    if control_or_nul * 100 > sample.len() * 12 {
        return Err(format!(
            "فایل «{}» یک فایل باینری غیرمتنی است و متن قابل استخراج ندارد.",
            filename
        ));
    }

    // 5. Legacy Windows-1256 / lossy UTF-8 fallback
    let (cow, _, had_errors) = WINDOWS_1256.decode(bytes);
    if !had_errors {
        return Ok(normalize_extracted_text(&cow));
    }

    Ok(normalize_extracted_text(&String::from_utf8_lossy(bytes)))
}

fn normalize_extracted_text(raw: &str) -> String {
    let unix = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<&str> = Vec::new();
    let mut consecutive_empty = 0;

    for line in unix.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            consecutive_empty += 1;
            if consecutive_empty <= 1 && !lines.is_empty() {
                lines.push("");
            }
        } else {
            consecutive_empty = 0;
            lines.push(trimmed);
        }
    }

    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    #[test]
    fn test_extract_source_code_and_extensionless_files() {
        let rust_code = "fn main() {\n    println!(\"سلام دنیا\");\n}\n";
        let res = extract_text_from_bytes("main.rs", rust_code.as_bytes()).unwrap();
        assert_eq!(res.detected_format, "RS");
        assert!(res.text.contains("سلام دنیا"));

        let dockerfile = "FROM rust:1.85\nRUN cargo build --release\n";
        let res2 = extract_text_from_bytes("Dockerfile", dockerfile.as_bytes()).unwrap();
        assert_eq!(res2.detected_format, "TEXT");
        assert!(res2.text.contains("cargo build"));
    }

    #[test]
    fn test_extract_utf16le_persian_text() {
        let text = "سلام Mab-Compass\nگام اول";
        let mut bytes = vec![0xFF, 0xFE];
        for code_unit in text.encode_utf16() {
            bytes.extend_from_slice(&code_unit.to_le_bytes());
        }
        let res = extract_text_from_bytes("notes.log", &bytes).unwrap();
        assert!(res.text.contains("سلام Mab-Compass"));
        assert!(res.text.contains("گام اول"));
    }

    #[test]
    fn test_extract_docx_with_persian_and_table() {
        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            zip.start_file("word/document.xml", opts).unwrap();
            let doc_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
                <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
                  <w:body>
                    <w:p><w:r><w:t>صورت‌جلسه پروژه گردالو</w:t></w:r></w:p>
                    <w:tbl>
                      <w:tr>
                        <w:tc><w:p><w:r><w:t>TSK-1</w:t></w:r></w:p></w:tc>
                        <w:tc><w:p><w:r><w:t>طراحی API پرداخت</w:t></w:r></w:p></w:tc>
                      </w:tr>
                    </w:tbl>
                  </w:body>
                </w:document>"#;
            zip.write_all(doc_xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }

        let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
        let res = extract_text_from_base64("meeting.docx", &b64).unwrap();
        assert_eq!(res.detected_format, "DOCX");
        assert!(res.text.contains("صورت‌جلسه پروژه گردالو"));
        assert!(res.text.contains("TSK-1"));
        assert!(res.text.contains("طراحی API پرداخت"));
    }

    #[test]
    fn test_extract_minimal_pdf_and_reject_binary() {
        // Construct a minimal valid PDF containing "Hello Mab Compass"
        let stream_body = b"BT /F1 12 Tf 72 720 Td (Hello Mab Compass) Tj ET";
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let off1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        let off2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
        let off3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n");
        let off4 = pdf.len();
        pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", stream_body.len()).as_bytes());
        pdf.extend_from_slice(stream_body);
        pdf.extend_from_slice(b"\nendstream endobj\n");
        let off5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj\n");
        let xref_pos = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 6\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer << /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                off1, off2, off3, off4, off5, xref_pos
            )
            .as_bytes(),
        );

        let res = extract_text_from_bytes("spec.pdf", &pdf).unwrap();
        assert_eq!(res.detected_format, "PDF");
        assert!(res.text.contains("Hello Mab Compass"));

        // Binary image header with NUL bytes should be rejected cleanly
        let binary_png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 1, 2, 0, 0, 0];
        let err = extract_text_from_bytes("logo.png", &binary_png).unwrap_err();
        assert!(err.contains("باینری"));
    }
}
