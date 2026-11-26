use crate::db::queries::ensure_active_chat_session;
use crate::models::{
    DatabaseBackupResult, LegacyReprocessCandidate, LegacyReprocessSelection, MethodPlaybook,
    MethodPlaybookInput, MethodStep, WrittenArtifactRef,
};
use crate::vault_manager::{sanitize_filename, write_method_playbook_files};
use chrono::Local;
use rusqlite::{params, Connection, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub fn normalize_persian_for_match(raw: &str) -> String {
    raw.chars()
        .map(|c| match c {
            'ي' | 'ى' => 'ی',
            'ك' => 'ک',
            '\u{200c}' => ' ',
            c if c.is_alphanumeric() || c > '\u{0600}' => c.to_lowercase().next().unwrap_or(c),
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_tokens(raw: &str) -> HashSet<String> {
    normalize_persian_for_match(raw)
        .split_whitespace()
        .filter(|w| w.chars().count() >= 2)
        .map(|w| w.to_string())
        .collect()
}

pub fn title_similarity(a: &str, b: &str) -> f64 {
    let na = normalize_persian_for_match(a);
    let nb = normalize_persian_for_match(b);
    if na.is_empty() || nb.is_empty() {
        return 0.0;
    }
    if na == nb {
        return 1.0;
    }
    if na.chars().count() >= 8 && nb.chars().count() >= 8 && (na.contains(&nb) || nb.contains(&na)) {
        return 0.85;
    }
    let ta = title_tokens(&na);
    let tb = title_tokens(&nb);
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }
    let inter = ta.intersection(&tb).count() as f64;
    let union = ta.union(&tb).count() as f64;
    if union <= 0.0 {
        0.0
    } else {
        inter / union
    }
}

pub fn is_duplicate_task_title(conn: &Connection, project_id: &str, candidate_title: &str) -> bool {
    let norm_cand = normalize_persian_for_match(candidate_title);
    if norm_cand.is_empty() {
        return false;
    }
    let Ok(mut stmt) = conn.prepare(
        "SELECT title FROM work_entities WHERE project_id = ?1 AND entity_type = 'TASK' AND status != 'SUPERSEDED'",
    ) else {
        return false;
    };
    let existing: Vec<String> = stmt
        .query_map(params![project_id], |r| r.get::<_, String>(0))
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    for title in existing {
        if title_similarity(&title, candidate_title) >= 0.78 {
            return true;
        }
    }
    false
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Produces a strictly valid XML NCName identifier (ASCII letters/digits/`_`/`-`, never starting with a digit).
fn sanitize_xml_id(raw: &str, fallback: &str) -> String {
    let mut cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    cleaned = cleaned.trim_matches('_').to_string();
    if cleaned.is_empty() {
        cleaned = fallback.to_string();
    }
    if cleaned.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) {
        cleaned = format!("N_{}", cleaned);
    }
    cleaned.chars().take(48).collect()
}

fn format_step_documentation(step: &MethodStep) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !step.description.trim().is_empty() {
        parts.push(step.description.trim().to_string());
    }
    if !step.role.trim().is_empty() {
        parts.push(format!("نقش/مسئول: {}", step.role.trim()));
    }
    if !step.inputs.is_empty() {
        parts.push(format!("ورودی‌ها: {}", step.inputs.join("، ")));
    }
    if !step.outputs.is_empty() {
        parts.push(format!("خروجی‌ها: {}", step.outputs.join("، ")));
    }
    if !step.checklist.is_empty() {
        parts.push(format!(
            "چک‌لیست:\n{}",
            step.checklist
                .iter()
                .map(|c| format!("- {}", c))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    if !step.notes.trim().is_empty() {
        parts.push(format!("نکات: {}", step.notes.trim()));
    }
    if let Some(ref sub_id) = step.sub_method_id {
        if !sub_id.trim().is_empty() {
            parts.push(format!("SUB_METHOD_ID:{}", sub_id.trim()));
        }
    }
    parts.join("\n")
}

fn normalize_bpmn_tag(element_type: &str, has_sub_steps: bool, has_sub_method: bool) -> &'static str {
    if has_sub_steps {
        return "subProcess";
    }
    if has_sub_method {
        return "callActivity";
    }
    match element_type.trim() {
        "userTask" | "USER_TASK" => "userTask",
        "serviceTask" | "SERVICE_TASK" => "serviceTask",
        "scriptTask" | "SCRIPT_TASK" => "scriptTask",
        "manualTask" | "MANUAL_TASK" => "manualTask",
        "subProcess" | "SUB_PROCESS" | "SUBPROCESS" => "subProcess",
        "callActivity" | "CALL_ACTIVITY" => "callActivity",
        "exclusiveGateway" | "EXCLUSIVE_GATEWAY" | "GATEWAY" => "exclusiveGateway",
        "parallelGateway" | "PARALLEL_GATEWAY" => "parallelGateway",
        _ => "task",
    }
}

fn render_steps_as_bpmn_elements(
    steps: &[MethodStep],
    prefix: &str,
    indent: &str,
) -> (String, String) {
    let mut xml = String::new();
    let mut shapes = String::new();

    let start_id = format!("{}_Start", prefix);
    let end_id = format!("{}_End", prefix);

    xml.push_str(&format!(
        "{indent}<bpmn:startEvent id=\"{}\" name=\"شروع\">\n{indent}  <bpmn:outgoing>{}_Flow_Start</bpmn:outgoing>\n{indent}</bpmn:startEvent>\n",
        start_id, prefix
    ));

    let mut x = 140;
    let y = 160;
    shapes.push_str(&format!(
        "      <bpmndi:BPMNShape id=\"{}_di\" bpmnElement=\"{}\"><dc:Bounds x=\"{}\" y=\"{}\" width=\"36\" height=\"36\" /></bpmndi:BPMNShape>\n",
        start_id, start_id, x, y + 22
    ));
    x += 90;

    let end_incoming_flow = if steps.is_empty() {
        format!("{}_Flow_Start", prefix)
    } else {
        format!("{}_Flow_End", prefix)
    };

    if steps.is_empty() {
        xml.push_str(&format!(
            "{indent}<bpmn:sequenceFlow id=\"{}_Flow_Start\" sourceRef=\"{}\" targetRef=\"{}\" />\n",
            prefix, start_id, end_id
        ));
    } else {
        let mut node_ids: Vec<String> = Vec::with_capacity(steps.len());
        let mut used_ids: HashSet<String> = HashSet::new();
        for (idx, step) in steps.iter().enumerate() {
            let base = if step.id.trim().is_empty() {
                format!("{}_Step_{}", prefix, idx + 1)
            } else {
                format!("{}_{}", prefix, sanitize_xml_id(&step.id, &format!("Step_{}", idx + 1)))
            };
            let mut candidate = base.clone();
            let mut suffix = 2usize;
            while !used_ids.insert(candidate.clone()) {
                candidate = format!("{}_{}", base, suffix);
                suffix += 1;
            }
            node_ids.push(candidate);
        }

        for (idx, step) in steps.iter().enumerate() {
            let nid = &node_ids[idx];
            let incoming_flow = if idx == 0 {
                format!("{}_Flow_Start", prefix)
            } else {
                format!("{}_Flow_{}", prefix, idx)
            };
            let outgoing_flow = if idx + 1 == steps.len() {
                format!("{}_Flow_End", prefix)
            } else {
                format!("{}_Flow_{}", prefix, idx + 1)
            };

            let tag = normalize_bpmn_tag(
                &step.element_type,
                !step.sub_steps.is_empty(),
                step.sub_method_id.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false),
            );
            let doc = format_step_documentation(step);

            if tag == "subProcess" {
                let (inner_xml, _inner_shapes) = render_steps_as_bpmn_elements(
                    &step.sub_steps,
                    &format!("{}_Sub", nid),
                    &format!("{}  ", indent),
                );
                xml.push_str(&format!(
                    "{indent}<bpmn:subProcess id=\"{}\" name=\"{}\">\n",
                    xml_escape(nid),
                    xml_escape(&step.title)
                ));
                if !doc.is_empty() {
                    xml.push_str(&format!(
                        "{indent}  <bpmn:documentation>{}</bpmn:documentation>\n",
                        xml_escape(&doc)
                    ));
                }
                xml.push_str(&format!(
                    "{indent}  <bpmn:incoming>{}</bpmn:incoming>\n{indent}  <bpmn:outgoing>{}</bpmn:outgoing>\n",
                    incoming_flow, outgoing_flow
                ));
                xml.push_str(&inner_xml);
                xml.push_str(&format!("{indent}</bpmn:subProcess>\n"));
                shapes.push_str(&format!(
                    "      <bpmndi:BPMNShape id=\"{}_di\" bpmnElement=\"{}\" isExpanded=\"false\"><dc:Bounds x=\"{}\" y=\"{}\" width=\"180\" height=\"80\" /></bpmndi:BPMNShape>\n",
                    nid, nid, x, y
                ));
                x += 230;
            } else if tag == "exclusiveGateway" || tag == "parallelGateway" {
                xml.push_str(&format!(
                    "{indent}<bpmn:{tag} id=\"{}\" name=\"{}\">\n",
                    xml_escape(nid),
                    xml_escape(&step.title)
                ));
                if !doc.is_empty() {
                    xml.push_str(&format!(
                        "{indent}  <bpmn:documentation>{}</bpmn:documentation>\n",
                        xml_escape(&doc)
                    ));
                }
                xml.push_str(&format!(
                    "{indent}  <bpmn:incoming>{}</bpmn:incoming>\n{indent}  <bpmn:outgoing>{}</bpmn:outgoing>\n{indent}</bpmn:{tag}>\n",
                    incoming_flow, outgoing_flow
                ));
                shapes.push_str(&format!(
                    "      <bpmndi:BPMNShape id=\"{}_di\" bpmnElement=\"{}\"><dc:Bounds x=\"{}\" y=\"{}\" width=\"50\" height=\"50\" /></bpmndi:BPMNShape>\n",
                    nid, nid, x, y + 15
                ));
                x += 110;
            } else {
                let extra_attr = if tag == "callActivity" {
                    if let Some(ref sub_id) = step.sub_method_id {
                        format!(" calledElement=\"{}\"", xml_escape(sub_id))
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                xml.push_str(&format!(
                    "{indent}<bpmn:{tag} id=\"{}\" name=\"{}\"{extra_attr}>\n",
                    xml_escape(nid),
                    xml_escape(&step.title)
                ));
                if !doc.is_empty() {
                    xml.push_str(&format!(
                        "{indent}  <bpmn:documentation>{}</bpmn:documentation>\n",
                        xml_escape(&doc)
                    ));
                }
                xml.push_str(&format!(
                    "{indent}  <bpmn:incoming>{}</bpmn:incoming>\n{indent}  <bpmn:outgoing>{}</bpmn:outgoing>\n{indent}</bpmn:{tag}>\n",
                    incoming_flow, outgoing_flow
                ));
                shapes.push_str(&format!(
                    "      <bpmndi:BPMNShape id=\"{}_di\" bpmnElement=\"{}\"><dc:Bounds x=\"{}\" y=\"{}\" width=\"160\" height=\"80\" /></bpmndi:BPMNShape>\n",
                    nid, nid, x, y
                ));
                x += 210;
            }
        }

        // Sequence flows
        xml.push_str(&format!(
            "{indent}<bpmn:sequenceFlow id=\"{}_Flow_Start\" sourceRef=\"{}\" targetRef=\"{}\" />\n",
            prefix, start_id, node_ids[0]
        ));
        for idx in 0..steps.len() {
            let src = &node_ids[idx];
            let (fid, tgt) = if idx + 1 == steps.len() {
                (format!("{}_Flow_End", prefix), end_id.clone())
            } else {
                (format!("{}_Flow_{}", prefix, idx + 1), node_ids[idx + 1].clone())
            };
            let cond_attr = steps[idx]
                .condition_label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|c| format!(" name=\"{}\"", xml_escape(c)))
                .unwrap_or_default();
            xml.push_str(&format!(
                "{indent}<bpmn:sequenceFlow id=\"{}\" sourceRef=\"{}\" targetRef=\"{}\"{cond_attr} />\n",
                fid, src, tgt
            ));
        }
    }

    xml.push_str(&format!(
        "{indent}<bpmn:endEvent id=\"{}\" name=\"پایان\">\n{indent}  <bpmn:incoming>{}</bpmn:incoming>\n{indent}</bpmn:endEvent>\n",
        end_id, end_incoming_flow
    ));
    shapes.push_str(&format!(
        "      <bpmndi:BPMNShape id=\"{}_di\" bpmnElement=\"{}\"><dc:Bounds x=\"{}\" y=\"{}\" width=\"36\" height=\"36\" /></bpmndi:BPMNShape>\n",
        end_id, end_id, x, y + 22
    ));

    (xml, shapes)
}

pub fn generate_bpmn_2_xml(method: &MethodPlaybook) -> String {
    let safe_id = sanitize_xml_id(&method.id, "Method");
    let proc_id = format!("Process_{}", safe_id);
    let (elements_xml, shapes_xml) = render_steps_as_bpmn_elements(&method.steps, "Elem", "    ");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                  xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL"
                  xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
                  xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
                  xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
                  id="Definitions_{def_id}"
                  targetNamespace="http://mab-compass.local/bpmn">
  <bpmn:process id="{proc_id}" name="{proc_name}" isExecutable="false">
    <bpmn:documentation>{proc_doc}</bpmn:documentation>
{elements_xml}  </bpmn:process>
  <bpmndi:BPMNDiagram id="BPMNDiagram_{def_id}">
    <bpmndi:BPMNPlane id="BPMNPlane_{def_id}" bpmnElement="{proc_id}">
{shapes_xml}    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</bpmn:definitions>
"#,
        def_id = xml_escape(&safe_id),
        proc_id = xml_escape(&proc_id),
        proc_name = xml_escape(&method.title),
        proc_doc = xml_escape(&method.objective),
    )
}

pub fn is_valid_bpmn_xml(xml: &str) -> bool {
    let trimmed = xml.trim();
    if trimmed.is_empty()
        || !trimmed.contains("definitions")
        || !trimmed.contains("process")
    {
        return false;
    }
    let mut reader = quick_xml::Reader::from_str(trimmed);
    let mut buf = Vec::new();
    let mut has_definitions = false;
    let mut has_process = false;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) | Ok(quick_xml::events::Event::Empty(e)) => {
                let name = e.local_name().into_inner();
                if name == "definitions" {
                    has_definitions = true;
                } else if name == "process" {
                    has_process = true;
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => return false,
            _ => {}
        }
        buf.clear();
    }
    has_definitions && has_process
}

pub fn render_method_markdown(method: &MethodPlaybook) -> String {
    let mut md = String::with_capacity(2048);
    md.push_str(&format!("# دستورالعمل و متد انجام کار: {}\n\n", method.title));
    md.push_str(&format!(
        "> **شناسه:** `{}` | **نسخه:** `v{}` | **دسته:** `{}` | **آخرین به‌روزرسانی:** `{}`\n\n",
        method.id,
        method.version,
        method.category,
        method.updated_at.chars().take(16).collect::<String>()
    ));

    if !method.summary.trim().is_empty() {
        md.push_str(&format!("{}\n\n", method.summary.trim()));
    }

    md.push_str("## ۱. هدف (Objective)\n");
    if method.objective.trim().is_empty() {
        md.push_str("تثبیت روش استاندارد، قابل تکرار و بدون خطا برای انجام این فرآیند.\n\n");
    } else {
        md.push_str(&format!("{}\n\n", method.objective.trim()));
    }

    if !method.prerequisites.is_empty() {
        md.push_str("## ۲. پیش‌نیازها (Prerequisites)\n");
        for pre in &method.prerequisites {
            md.push_str(&format!("- {}\n", pre.trim()));
        }
        md.push('\n');
    }

    md.push_str("## ۳. گام‌های اجرایی (Step-by-Step Workflow)\n");
    if method.steps.is_empty() {
        md.push_str("*(هنوز گام تفکیک‌شده‌ای ثبت نشده است)*\n\n");
    } else {
        for (idx, step) in method.steps.iter().enumerate() {
            let role_str = if step.role.trim().is_empty() {
                String::new()
            } else {
                format!(" *(مسئول: {})*", step.role.trim())
            };
            let type_badge = if !step.sub_steps.is_empty() || step.element_type == "subProcess" {
                " `[زیرفرآیند BPMN]`"
            } else if step.element_type == "exclusiveGateway" {
                " `[تصمیم‌گیری]`"
            } else {
                ""
            };
            md.push_str(&format!(
                "### گام {}. {}{}{}\n",
                idx + 1,
                step.title.trim(),
                role_str,
                type_badge
            ));
            if !step.description.trim().is_empty() {
                md.push_str(&format!("{}\n", step.description.trim()));
            }
            if !step.inputs.is_empty() {
                md.push_str(&format!("- **ورودی‌ها:** {}\n", step.inputs.join("، ")));
            }
            if !step.outputs.is_empty() {
                md.push_str(&format!("- **خروجی‌ها:** {}\n", step.outputs.join("، ")));
            }
            if !step.checklist.is_empty() {
                for chk in &step.checklist {
                    md.push_str(&format!("- [ ] {}\n", chk.trim()));
                }
            }
            if !step.sub_steps.is_empty() {
                md.push_str("\n**زیرگام‌های داخلی این مجموعه کار:**\n");
                for (sidx, sub) in step.sub_steps.iter().enumerate() {
                    md.push_str(&format!(
                        "{}. **{}**{}\n",
                        sidx + 1,
                        sub.title.trim(),
                        if sub.description.trim().is_empty() {
                            String::new()
                        } else {
                            format!(": {}", sub.description.trim())
                        }
                    ));
                }
            }
            if !step.notes.trim().is_empty() {
                md.push_str(&format!("> 💡 **نکته:** {}\n", step.notes.trim()));
            }
            md.push('\n');
        }
    }

    if !method.checklist.is_empty() {
        md.push_str("## ۴. چک‌لیست کنترل کیفیت (Checklist)\n");
        for item in &method.checklist {
            md.push_str(&format!("- [ ] {}\n", item.trim()));
        }
        md.push('\n');
    }

    if !method.notes_markdown.trim().is_empty() {
        md.push_str("## ۵. نکات کلیدی و ضدتله‌ها (Notes & Guardrails)\n");
        md.push_str(&format!("{}\n\n", method.notes_markdown.trim()));
    }

    md
}

fn dedup_merge_strings(existing: &[String], incoming: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for s in existing.iter().chain(incoming.iter()) {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            continue;
        }
        let norm = normalize_persian_for_match(trimmed);
        if seen.insert(norm) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn normalize_steps(steps: Vec<MethodStep>) -> Vec<MethodStep> {
    steps
        .into_iter()
        .enumerate()
        .filter(|(_, s)| !s.title.trim().is_empty())
        .map(|(i, mut s)| {
            if s.id.trim().is_empty() {
                s.id = format!("S{}", i + 1);
            }
            s.order = (i + 1) as i32;
            if s.element_type.trim().is_empty() {
                s.element_type = if !s.sub_steps.is_empty() {
                    "subProcess".to_string()
                } else {
                    "task".to_string()
                };
            }
            if !s.sub_steps.is_empty() {
                s.sub_steps = normalize_steps(s.sub_steps);
            }
            s
        })
        .collect()
}

pub fn query_all_methods_playbooks(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<MethodPlaybook>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, slug, title, category, summary, objective,
                prerequisites_json, steps_json, checklist_json, notes_markdown,
                markdown_content, bpmn_xml, parent_method_id, parent_element_id,
                version, archived_path, bpmn_file_path, source_refs_json,
                created_at, updated_at
         FROM methods_playbooks
         WHERE project_id = ?1
         ORDER BY updated_at DESC, id DESC",
    )?;
    let rows = stmt.query_map(params![project_id], |r| {
        let pre_str: String = r.get(7).unwrap_or_else(|_| "[]".into());
        let steps_str: String = r.get(8).unwrap_or_else(|_| "[]".into());
        let chk_str: String = r.get(9).unwrap_or_else(|_| "[]".into());
        let src_str: String = r.get(18).unwrap_or_else(|_| "[]".into());
        Ok(MethodPlaybook {
            id: r.get(0)?,
            project_id: r.get(1)?,
            slug: r.get(2)?,
            title: r.get(3)?,
            category: r.get(4).unwrap_or_else(|_| "GENERAL".into()),
            summary: r.get(5).unwrap_or_default(),
            objective: r.get(6).unwrap_or_default(),
            prerequisites: serde_json::from_str(&pre_str).unwrap_or_default(),
            steps: serde_json::from_str(&steps_str).unwrap_or_default(),
            checklist: serde_json::from_str(&chk_str).unwrap_or_default(),
            notes_markdown: r.get(10).unwrap_or_default(),
            markdown_content: r.get(11).unwrap_or_default(),
            bpmn_xml: r.get(12).unwrap_or_default(),
            parent_method_id: r.get(13).ok(),
            parent_element_id: r.get(14).ok(),
            version: r.get(15).unwrap_or(1),
            archived_path: r.get(16).unwrap_or_default(),
            bpmn_file_path: r.get(17).unwrap_or_default(),
            source_refs: serde_json::from_str(&src_str).unwrap_or_default(),
            created_at: r.get(19).unwrap_or_default(),
            updated_at: r.get(20).unwrap_or_default(),
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn find_similar_method(
    conn: &Connection,
    project_id: &str,
    method_id_hint: Option<&str>,
    title: &str,
) -> Option<MethodPlaybook> {
    let all = query_all_methods_playbooks(conn, project_id).ok()?;
    if let Some(mid) = method_id_hint.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(found) = all.iter().find(|m| m.id.eq_ignore_ascii_case(mid)) {
            return Some(found.clone());
        }
    }
    let slug = sanitize_filename(title);
    if let Some(found) = all.iter().find(|m| m.slug == slug) {
        return Some(found.clone());
    }
    let mut best: Option<(f64, MethodPlaybook)> = None;
    for m in all {
        let sim = title_similarity(&m.title, title);
        if sim >= 0.55 {
            if best.as_ref().map(|(s, _)| sim > *s).unwrap_or(true) {
                best = Some((sim, m));
            }
        }
    }
    best.map(|(_, m)| m)
}

/// Automatically creates a new MethodPlaybook or enriches & version-bumps a similar existing one,
/// persisting to SQLite AND writing `<slug>.md` + `<slug>.bpmn` into `<workspace>/04_methods_playbooks/`.
/// Returns `(MethodPlaybook, is_created, WrittenArtifactRef)`.
pub fn upsert_method_playbook(
    conn: &Connection,
    project_id: &str,
    workspace_path: &str,
    input: MethodPlaybookInput,
    source_ref: Option<&str>,
) -> std::result::Result<(MethodPlaybook, bool, WrittenArtifactRef), String> {
    let clean_title = input.title.trim();
    if clean_title.is_empty() {
        return Err("عنوان متد / دستورالعمل نمی‌تواند خالی باشد.".to_string());
    }

    let now = Local::now().to_rfc3339();
    let existing = find_similar_method(conn, project_id, input.method_id.as_deref(), clean_title);
    let incoming_steps = normalize_steps(input.steps);

    let (mut method, is_created) = if let Some(prev) = existing {
        let merged_prereqs = dedup_merge_strings(&prev.prerequisites, &input.prerequisites);
        let merged_checklist = dedup_merge_strings(&prev.checklist, &input.checklist);

        // If incoming provides steps, merge/upgrade intelligently:
        // If incoming has >= prev steps or replaces workflow cleanly, adopt incoming;
        // if incoming only provides incremental new steps not already present, append them!
        let merged_steps = if incoming_steps.is_empty() {
            prev.steps.clone()
        } else if incoming_steps.len() >= prev.steps.len() {
            incoming_steps
        } else {
            let mut combined = prev.steps.clone();
            for s in incoming_steps {
                let already = combined
                    .iter()
                    .position(|ex| title_similarity(&ex.title, &s.title) >= 0.72);
                if let Some(pos) = already {
                    combined[pos] = s;
                } else {
                    combined.push(s);
                }
            }
            normalize_steps(combined)
        };

        let merged_notes = match input.notes_markdown.as_deref().map(str::trim) {
            Some(n) if !n.is_empty() => {
                if prev.notes_markdown.trim().is_empty()
                    || prev.notes_markdown.contains(n)
                    || n.len() >= prev.notes_markdown.len()
                {
                    n.to_string()
                } else {
                    format!("{}\n\n{}", prev.notes_markdown.trim(), n)
                }
            }
            _ => prev.notes_markdown.clone(),
        };

        let mut source_refs = prev.source_refs.clone();
        if let Some(sr) = source_ref.map(str::trim).filter(|s| !s.is_empty()) {
            if !source_refs.iter().any(|x| x == sr) {
                source_refs.push(sr.to_string());
            }
        }

        let updated = MethodPlaybook {
            id: prev.id,
            project_id: project_id.to_string(),
            slug: prev.slug,
            title: clean_title.to_string(),
            category: input
                .category
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&prev.category)
                .to_string(),
            summary: input
                .summary
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&prev.summary)
                .to_string(),
            objective: input
                .objective
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&prev.objective)
                .to_string(),
            prerequisites: merged_prereqs,
            steps: merged_steps,
            checklist: merged_checklist,
            notes_markdown: merged_notes,
            markdown_content: String::new(),
            bpmn_xml: String::new(),
            parent_method_id: input.parent_method_id.or(prev.parent_method_id),
            parent_element_id: input.parent_element_id.or(prev.parent_element_id),
            version: prev.version + 1,
            archived_path: prev.archived_path,
            bpmn_file_path: prev.bpmn_file_path,
            source_refs,
            created_at: prev.created_at,
            updated_at: now.clone(),
        };
        (updated, false)
    } else {
        let new_id = format!("MTH-{}", &Uuid::new_v4().to_string()[..6].to_uppercase());
        let slug = sanitize_filename(clean_title);
        let category = input
            .category
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("GENERAL")
            .to_string();
        let objective = input.objective.unwrap_or_default().trim().to_string();
        let summary = input
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                if !objective.is_empty() {
                    objective.clone()
                } else {
                    format!(
                        "دستورالعمل گام‌به‌گام و نقشه BPMN 2.0 برای «{}» ({} گام)",
                        clean_title,
                        incoming_steps.len()
                    )
                }
            });
        let source_refs = source_ref
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| vec![s.to_string()])
            .unwrap_or_default();

        let created = MethodPlaybook {
            id: new_id,
            project_id: project_id.to_string(),
            slug,
            title: clean_title.to_string(),
            category,
            summary,
            objective,
            prerequisites: dedup_merge_strings(&[], &input.prerequisites),
            steps: incoming_steps,
            checklist: dedup_merge_strings(&[], &input.checklist),
            notes_markdown: input.notes_markdown.unwrap_or_default().trim().to_string(),
            markdown_content: String::new(),
            bpmn_xml: String::new(),
            parent_method_id: input.parent_method_id,
            parent_element_id: input.parent_element_id,
            version: 1,
            archived_path: String::new(),
            bpmn_file_path: String::new(),
            source_refs,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        (created, true)
    };

    // Build BPMN 2.0 XML (validate user/LLM XML, or auto-generate clean BPMN 2.0 from steps)
    let valid_provided_xml = input
        .bpmn_xml
        .as_deref()
        .map(str::trim)
        .filter(|x| is_valid_bpmn_xml(x))
        .map(str::to_string);
    method.bpmn_xml = valid_provided_xml.unwrap_or_else(|| generate_bpmn_2_xml(&method));

    // Render Markdown Playbook
    method.markdown_content = render_method_markdown(&method);

    // Persist files into 04_methods_playbooks/
    if !workspace_path.trim().is_empty() {
        let (md_path, bpmn_path) = write_method_playbook_files(
            workspace_path,
            &method.slug,
            &method.markdown_content,
            &method.bpmn_xml,
        )?;
        method.archived_path = md_path;
        method.bpmn_file_path = bpmn_path;
    }

    let pre_json = serde_json::to_string(&method.prerequisites).unwrap_or_else(|_| "[]".into());
    let steps_json = serde_json::to_string(&method.steps).unwrap_or_else(|_| "[]".into());
    let chk_json = serde_json::to_string(&method.checklist).unwrap_or_else(|_| "[]".into());
    let src_json = serde_json::to_string(&method.source_refs).unwrap_or_else(|_| "[]".into());

    if is_created {
        conn.execute(
            "INSERT INTO methods_playbooks (
                id, project_id, slug, title, category, summary, objective,
                prerequisites_json, steps_json, checklist_json, notes_markdown,
                markdown_content, bpmn_xml, parent_method_id, parent_element_id,
                version, archived_path, bpmn_file_path, source_refs_json,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                method.id,
                method.project_id,
                method.slug,
                method.title,
                method.category,
                method.summary,
                method.objective,
                pre_json,
                steps_json,
                chk_json,
                method.notes_markdown,
                method.markdown_content,
                method.bpmn_xml,
                method.parent_method_id,
                method.parent_element_id,
                method.version,
                method.archived_path,
                method.bpmn_file_path,
                src_json,
                method.created_at,
                method.updated_at
            ],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE methods_playbooks
             SET title = ?1, category = ?2, summary = ?3, objective = ?4,
                 prerequisites_json = ?5, steps_json = ?6, checklist_json = ?7,
                 notes_markdown = ?8, markdown_content = ?9, bpmn_xml = ?10,
                 parent_method_id = ?11, parent_element_id = ?12, version = ?13,
                 archived_path = ?14, bpmn_file_path = ?15, source_refs_json = ?16,
                 updated_at = ?17
             WHERE id = ?18",
            params![
                method.title,
                method.category,
                method.summary,
                method.objective,
                pre_json,
                steps_json,
                chk_json,
                method.notes_markdown,
                method.markdown_content,
                method.bpmn_xml,
                method.parent_method_id,
                method.parent_element_id,
                method.version,
                method.archived_path,
                method.bpmn_file_path,
                src_json,
                method.updated_at,
                method.id
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    let artifact_ref = WrittenArtifactRef {
        artifact_type: "METHOD_PLAYBOOK".to_string(),
        entity_id: method.id.clone(),
        title: method.title.clone(),
        summary: if method.summary.trim().is_empty() {
            format!("دستورالعمل گام‌به‌گام و نمودار BPMN 2.0 ({} گام)", method.steps.len())
        } else {
            method.summary.clone()
        },
        file_path: if method.archived_path.is_empty() {
            None
        } else {
            Some(method.archived_path.clone())
        },
        action: if is_created {
            "CREATED".to_string()
        } else {
            "UPDATED".to_string()
        },
        version: Some(method.version),
        ..Default::default()
    };

    Ok((method, is_created, artifact_ref))
}

pub fn delete_method_playbook(conn: &Connection, method_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM methods_playbooks WHERE id = ?1",
        params![method_id],
    )?;
    Ok(())
}

/// Creates a WAL-safe, atomic SQLite backup snapshot using `VACUUM INTO`.
pub fn backup_sqlite_database(
    conn: &Connection,
    backup_dir: &Path,
) -> std::result::Result<DatabaseBackupResult, String> {
    if !backup_dir.exists() {
        fs::create_dir_all(backup_dir)
            .map_err(|e| format!("خطا در ساخت پوشه پشتیبان دیتابیس: {}", e))?;
    }
    let now = Local::now();
    let filename = format!(
        "mab_compass_backup_{}_{}.sqlite",
        now.format("%Y%m%d_%H%M%S"),
        &Uuid::new_v4().to_string()[..4]
    );
    let target = backup_dir.join(filename);
    let target_str = target.to_string_lossy().to_string();

    conn.execute("VACUUM INTO ?1", params![target_str])
        .map_err(|e| format!("خطا در پشتیبان‌گیری اتمیک SQLite (VACUUM INTO): {}", e))?;

    let meta = fs::metadata(&target)
        .map_err(|e| format!("پشتیبان ساخته شد اما خواندن اندازه آن ناموفق بود: {}", e))?;

    Ok(DatabaseBackupResult {
        backup_path: target_str,
        backup_size_bytes: meta.len(),
        created_at: now.to_rfc3339(),
    })
}

/// Lists all historical documents & substantial raw memory events chronologically (oldest -> newest)
/// so the user can either upgrade ALL legacy memory or pick specific items with checkboxes.
pub fn list_legacy_reprocess_candidates(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<LegacyReprocessCandidate>> {
    let mut candidates: Vec<LegacyReprocessCandidate> = Vec::new();
    let mut seen_doc_paths: HashSet<String> = HashSet::new();

    // 1. All knowledge_documents for this project
    let mut d_stmt = conn.prepare(
        "SELECT d.id, d.project_id, d.title, d.doc_type, d.executive_summary, d.ingested_at,
                d.processing_status, d.archived_path, h.reprocessed_at
         FROM knowledge_documents d
         LEFT JOIN legacy_reprocess_history h
           ON h.source_kind = 'KNOWLEDGE_DOC' AND h.source_id = d.id
         WHERE d.project_id = ?1
         ORDER BY d.ingested_at ASC, d.id ASC",
    )?;
    let doc_rows = d_stmt.query_map(params![project_id], |r| {
        let arch_path: String = r.get(7).unwrap_or_default();
        let upgraded_at: Option<String> = r.get(8).ok();
        let summary: String = r.get(4).unwrap_or_default();
        let snippet: String = summary.trim().chars().take(180).collect();
        Ok(LegacyReprocessCandidate {
            source_kind: "KNOWLEDGE_DOC".to_string(),
            source_id: r.get(0)?,
            project_id: r.get(1)?,
            title: r.get(2)?,
            doc_type: r.get(3).unwrap_or_else(|_| "TECHNICAL_DOC".into()),
            snippet,
            created_at: r.get(5).unwrap_or_default(),
            processing_status: r.get(6).unwrap_or_else(|_| "UNPROCESSED".into()),
            already_upgraded: upgraded_at.is_some(),
            upgraded_at,
            archived_path: if arch_path.is_empty() {
                None
            } else {
                Some(arch_path)
            },
        })
    })?;

    for row in doc_rows.filter_map(|r| r.ok()) {
        if let Some(ref p) = row.archived_path {
            seen_doc_paths.insert(p.replace('\\', "/").to_lowercase());
        }
        candidates.push(row);
    }

    // 2. Substantial raw_events not already represented by a knowledge_document
    let mut r_stmt = conn.prepare(
        "SELECT e.id, e.project_id, e.source_type, e.raw_content, e.archived_path, e.created_at,
                h.reprocessed_at
         FROM raw_events e
         LEFT JOIN legacy_reprocess_history h
           ON h.source_kind = 'RAW_EVENT' AND h.source_id = e.id
         WHERE e.project_id = ?1
           AND e.source_type NOT IN ('MCP_TOOL', 'QUICK_COMPLETE')
           AND LENGTH(TRIM(e.raw_content)) >= 35
         ORDER BY e.created_at ASC, e.id ASC",
    )?;
    let raw_rows = r_stmt.query_map(params![project_id], |r| {
        let content: String = r.get(3).unwrap_or_default();
        let arch_path: Option<String> = r.get(4).ok();
        let upgraded_at: Option<String> = r.get(6).ok();
        let first_line = content
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("رویداد حافظه")
            .chars()
            .take(64)
            .collect::<String>();
        let snippet: String = content.trim().chars().take(180).collect();
        Ok(LegacyReprocessCandidate {
            source_kind: "RAW_EVENT".to_string(),
            source_id: r.get(0)?,
            project_id: r.get(1)?,
            title: first_line,
            doc_type: r.get(2).unwrap_or_else(|_| "USER_CHAT".into()),
            snippet,
            created_at: r.get(5).unwrap_or_default(),
            processing_status: "HISTORICAL".to_string(),
            already_upgraded: upgraded_at.is_some(),
            upgraded_at,
            archived_path: arch_path,
        })
    })?;

    for row in raw_rows.filter_map(|r| r.ok()) {
        if let Some(ref p) = row.archived_path {
            if seen_doc_paths.contains(&p.replace('\\', "/").to_lowercase()) {
                continue;
            }
        }
        candidates.push(row);
    }

    // Strict chronological order: oldest -> newest
    candidates.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.source_id.cmp(&b.source_id)));
    Ok(candidates)
}

pub fn enqueue_legacy_reprocess(
    conn: &Connection,
    project_id: &str,
    mode: &str,
    selected: &[LegacyReprocessSelection],
) -> std::result::Result<usize, String> {
    let all = list_legacy_reprocess_candidates(conn, project_id).map_err(|e| e.to_string())?;
    let targets: Vec<LegacyReprocessCandidate> = if mode.eq_ignore_ascii_case("ALL_CHRONOLOGICAL") {
        all.into_iter().filter(|c| !c.already_upgraded).collect()
    } else {
        let sel_set: HashSet<(String, i64)> = selected
            .iter()
            .map(|s| (s.source_kind.to_uppercase(), s.source_id))
            .collect();
        all.into_iter()
            .filter(|c| sel_set.contains(&(c.source_kind.to_uppercase(), c.source_id)))
            .collect()
    };

    if targets.is_empty() {
        return Ok(0);
    }

    let sid = ensure_active_chat_session(conn, project_id, "COPILOT").unwrap_or_default();
    let now = Local::now().to_rfc3339();
    let mut enqueued = 0usize;

    for item in targets {
        let historical_ts = if item.created_at.trim().is_empty() {
            now.clone()
        } else {
            item.created_at.clone()
        };

        if item.source_kind == "KNOWLEDGE_DOC" {
            // Skip if already actively queued for legacy reprocessing
            let already_queued: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pending_ai_queue
                     WHERE knowledge_doc_id = ?1 AND source_type = 'LEGACY_REPROCESS_DOC'
                       AND status IN ('PENDING', 'PROCESSING')",
                    params![item.source_id],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap_or(0)
                > 0;
            if already_queued {
                continue;
            }

            // Re-activate existing queue row for this knowledge_doc_id OR insert new,
            // preserving historical `created_at` so docs & raw events run in strict chronological order!
            let preview = format!("🔄 بازپردازش سند: {}", item.title);
            let updated = conn
                .execute(
                    "UPDATE pending_ai_queue
                     SET status = 'PENDING', source_type = 'LEGACY_REPROCESS_DOC',
                         title_preview = ?1, last_error = NULL,
                         created_at = ?2, updated_at = ?3, priority_order = 0
                     WHERE knowledge_doc_id = ?4",
                    params![preview, historical_ts, now, item.source_id],
                )
                .map_err(|e| e.to_string())?;
            if updated == 0 {
                conn.execute(
                    "INSERT INTO pending_ai_queue (
                        project_id, session_id, channel, user_message_id, system_message_id,
                        knowledge_doc_id, raw_event_id, title_preview, source_type, archived_path,
                        status, attempts, last_error, created_at, updated_at
                     ) VALUES (?1, ?2, 'COPILOT', NULL, NULL, ?3, NULL, ?4, 'LEGACY_REPROCESS_DOC', ?5, 'PENDING', 0, NULL, ?6, ?7)",
                    params![
                        project_id,
                        sid,
                        item.source_id,
                        preview,
                        item.archived_path,
                        historical_ts,
                        now
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
            enqueued += 1;
        } else {
            // RAW_EVENT
            let preview = format!("🔄 بازپردازش حافظه: {}", item.title);
            let exists_pending: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pending_ai_queue
                     WHERE raw_event_id = ?1 AND status IN ('PENDING', 'PROCESSING')",
                    params![item.source_id],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap_or(0)
                > 0;
            if exists_pending {
                continue;
            }
            conn.execute(
                "INSERT INTO pending_ai_queue (
                    project_id, session_id, channel, user_message_id, system_message_id,
                    knowledge_doc_id, raw_event_id, title_preview, source_type, archived_path,
                    status, attempts, last_error, created_at, updated_at
                 ) VALUES (?1, ?2, 'COPILOT', NULL, NULL, NULL, ?3, ?4, 'LEGACY_REPROCESS_EVENT', ?5, 'PENDING', 0, NULL, ?6, ?7)",
                params![
                    project_id,
                    sid,
                    item.source_id,
                    preview,
                    item.archived_path,
                    historical_ts,
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
            enqueued += 1;
        }
    }

    Ok(enqueued)
}

pub fn mark_legacy_reprocessed(
    conn: &Connection,
    source_kind: &str,
    source_id: i64,
    project_id: &str,
    summary: &str,
) -> Result<()> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO legacy_reprocess_history (
            source_kind, source_id, project_id, reprocessed_at, extracted_summary
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![source_kind, source_id, project_id, now, summary],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::init_database;

    #[test]
    fn test_method_playbook_create_auto_update_and_bpmn_generation() {
        let dir = std::env::temp_dir().join(format!("mab_method_test_{}", Uuid::new_v4()));
        let vault = dir.join("vault");
        let proj_ws = vault.join("GERDALO");
        fs::create_dir_all(&proj_ws).unwrap();
        let conn = init_database(&dir.join("test.sqlite"), &vault).unwrap();

        let mut input1 = MethodPlaybookInput {
            method_id: None,
            title: "متد استقرار نسخه جدید بک‌اند روی سرور".to_string(),
            ..Default::default()
        };
        input1.category = Some("DEPLOY".to_string());
        input1.objective = Some("استقرار بدون قطعی سرویس روی سرور تولید".to_string());
        input1.prerequisites = vec!["دسترسی SSH".to_string(), "بکاپ دیتابیس".to_string()];
        input1.steps = vec![
            MethodStep {
                id: "S1".into(),
                order: 1,
                title: "اجرای تست‌های واحد".into(),
                description: "اجرای cargo test قبل از بیلد".into(),
                element_type: "serviceTask".into(),
                role: "توسعه‌دهنده".into(),
                checklist: vec!["تمامی تست‌ها سبز باشند".into()],
                ..Default::default()
            },
            MethodStep {
                id: "S2".into(),
                order: 2,
                title: "مجموعه کار انتشار و مانیتورینگ".into(),
                description: "بیلد ریلیز و بررسی لاگ‌ها".into(),
                element_type: "subProcess".into(),
                sub_steps: vec![
                    MethodStep {
                        id: "S2_1".into(),
                        order: 1,
                        title: "ساخت باینری Release".into(),
                        element_type: "serviceTask".into(),
                        ..Default::default()
                    },
                    MethodStep {
                        id: "S2_2".into(),
                        order: 2,
                        title: "چک کردن Health Endpoint".into(),
                        element_type: "userTask".into(),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        ];
        input1.checklist = vec!["تایید پاسخ 200 از سرور".into()];

        let (m1, created1, art1) = upsert_method_playbook(
            &conn,
            "GERDALO",
            &proj_ws.to_string_lossy(),
            input1,
            Some("chat_1"),
        )
        .unwrap();

        assert!(created1);
        assert_eq!(m1.version, 1);
        assert_eq!(art1.action, "CREATED");
        assert!(is_valid_bpmn_xml(&m1.bpmn_xml));
        assert!(m1.bpmn_xml.contains("<bpmn:subProcess"));
        assert!(Path::new(&m1.archived_path).exists());
        assert!(Path::new(&m1.bpmn_file_path).exists());
        assert!(m1.archived_path.contains("04_methods_playbooks"));

        // Second call with similar title should auto-detect and bump version to v2
        let input2 = MethodPlaybookInput {
            method_id: None,
            title: "متد استقرار نسخه جدید بک‌اند روی سرور تولید".to_string(),
            prerequisites: vec!["بررسی فضای دیسک".to_string()],
            steps: vec![MethodStep {
                id: "S3".into(),
                order: 3,
                title: "اعلام انتشار به تیم".into(),
                element_type: "userTask".into(),
                ..Default::default()
            }],
            checklist: vec!["ثبت تگ Git".into()],
            ..Default::default()
        };

        let (m2, created2, art2) = upsert_method_playbook(
            &conn,
            "GERDALO",
            &proj_ws.to_string_lossy(),
            input2,
            Some("chat_2"),
        )
        .unwrap();

        assert!(!created2, "Should auto-update similar existing method instead of duplicating");
        assert_eq!(m2.id, m1.id);
        assert_eq!(m2.version, 2);
        assert_eq!(art2.action, "UPDATED");
        assert_eq!(m2.steps.len(), 3, "Incremental step should be merged into existing steps");
        assert_eq!(m2.prerequisites.len(), 3);
        assert_eq!(m2.checklist.len(), 2);

        drop(conn);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_backup_sqlite_and_legacy_reprocess_candidates_and_anti_duplicate() {
        let dir = std::env::temp_dir().join(format!("mab_legacy_test_{}", Uuid::new_v4()));
        let vault = dir.join("vault");
        fs::create_dir_all(&vault).unwrap();
        let conn = init_database(&dir.join("test.sqlite"), &vault).unwrap();

        // Insert a task to test anti-duplicate guard
        conn.execute(
            "INSERT INTO work_entities (id, project_id, entity_type, category, title, status, updated_at)
             VALUES ('TSK-1', 'GERDALO', 'TASK', 'BACKEND', 'پیاده‌سازی احراز هویت کاربران', 'OPEN', '2026-11-01T10:00:00Z')",
            [],
        )
        .unwrap();
        assert!(is_duplicate_task_title(&conn, "GERDALO", "پیاده سازی احراز هویت کاربران"));
        assert!(!is_duplicate_task_title(&conn, "GERDALO", "طراحی داشبورد گزارش مالی"));

        // Seed 2 legacy docs & 1 substantial raw event with explicit timestamps
        conn.execute(
            "INSERT INTO knowledge_documents (project_id, title, doc_type, original_filename, archived_path, executive_summary, extracted_items_count, ingested_at, processing_status)
             VALUES ('GERDALO', 'سند قدیمی ۱', 'TECHNICAL_DOC', 'old1.md', '/tmp/old1.md', 'خلاصه سند ۱', 2, '2026-10-01T08:00:00Z', 'PROCESSED')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO raw_events (project_id, raw_content, archived_path, source_type, created_at)
             VALUES ('GERDALO', 'گزارش کار کامل امروز: ابتدا تست ها را اجرا کردم و سپس روی سرور دیپلوی نمودم', '/tmp/raw1.md', 'USER_CHAT', '2026-10-05T09:00:00Z')",
            [],
        )
        .unwrap();

        // Test atomic SQLite backup
        let backup = backup_sqlite_database(&conn, &dir.join("backups")).unwrap();
        assert!(Path::new(&backup.backup_path).exists());
        assert!(backup.backup_size_bytes > 0);

        // Test chronological listing
        let cands = list_legacy_reprocess_candidates(&conn, "GERDALO").unwrap();
        assert_eq!(cands.len(), 2);
        assert_eq!(cands[0].source_kind, "KNOWLEDGE_DOC");
        assert_eq!(cands[1].source_kind, "RAW_EVENT");
        assert!(!cands[0].already_upgraded);

        // Test selective enqueue
        let count = enqueue_legacy_reprocess(
            &conn,
            "GERDALO",
            "SELECTED",
            &[LegacyReprocessSelection {
                source_kind: "KNOWLEDGE_DOC".into(),
                source_id: cands[0].source_id,
            }],
        )
        .unwrap();
        assert_eq!(count, 1);

        // Mark upgraded & verify flag updates
        mark_legacy_reprocessed(&conn, "KNOWLEDGE_DOC", cands[0].source_id, "GERDALO", "upgraded").unwrap();
        let cands_after = list_legacy_reprocess_candidates(&conn, "GERDALO").unwrap();
        assert!(cands_after[0].already_upgraded);
        assert!(!cands_after[1].already_upgraded);

        // ALL_CHRONOLOGICAL should now only enqueue the 1 remaining un-upgraded item
        let count_all = enqueue_legacy_reprocess(&conn, "GERDALO", "ALL_CHRONOLOGICAL", &[]).unwrap();
        assert_eq!(count_all, 1);

        drop(conn);
        let _ = fs::remove_dir_all(dir);
    }
}
