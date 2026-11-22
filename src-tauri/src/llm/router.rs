use crate::mcp_tools::{
    copilot_envelope_json_schema, execute_tool_with_context, gemini_copilot_response_schema,
    gemini_tools_payload, openai_tools_payload,
};
use crate::models::{
    CopilotEnvelope, LiveModelOption, LiveModelsResponse, LlmConfig, LlmConnectionStatus,
    WrittenArtifactRef,
};
use rusqlite::Connection;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex as TokioMutex;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Global sequential concurrency gate for local models (e.g. LM Studio).
/// Guarantees that only ONE request hits the local inference engine at any time,
/// preventing parallel requests from splitting GPU/CPU resources and triggering timeouts.
static LOCAL_OPENAI_GATE: LazyLock<TokioMutex<()>> = LazyLock::new(|| TokioMutex::new(()));

#[derive(Debug, Clone)]
pub struct CopilotTurnResult {
    pub envelope: CopilotEnvelope,
    pub mcp_ops_executed: Vec<String>,
    pub mcp_written_artifacts: Vec<WrittenArtifactRef>,
    pub transport_label: String,
}

fn collect_artifact_from_mcp_output(val: &Value, acc: &mut Vec<WrittenArtifactRef>) {
    if let Some(art_val) = val.get("artifact_ref") {
        if let Ok(art) = serde_json::from_value::<WrittenArtifactRef>(art_val.clone()) {
            if !art.title.trim().is_empty() {
                acc.push(art);
            }
        }
    }
}

pub async fn run_llm_text(
    app_data_dir: PathBuf,
    config: &LlmConfig,
    prompt: &str,
) -> Result<String, String> {
    match config.active_provider.as_str() {
        "GEMINI_REST" => {
            call_gemini_rest_structured(&config.gemini_api_key, &config.gemini_model, prompt, false)
                .await
        }
        "LOCAL_OPENAI" => {
            call_local_openai_structured(&config.local_endpoint, &config.local_model, prompt, false)
                .await
        }
        _ => match call_agy_cli(app_data_dir, &config.agy_model, prompt).await {
            Ok(res) => Ok(res),
            Err(agy_err) => {
                if !config.gemini_api_key.trim().is_empty() {
                    call_gemini_rest_structured(
                        &config.gemini_api_key,
                        &config.gemini_model,
                        prompt,
                        false,
                    )
                    .await
                } else {
                    Err(agy_err)
                }
            }
        },
    }
}

fn looks_like_cli_or_transport_error(text: &str) -> bool {
    let lower = text.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return true;
    }
    lower.starts_with("error:")
        || lower.starts_with("usage:")
        || lower.starts_with("agy :")
        || lower.contains("authentication required")
        || lower.contains("no capacity available")
        || lower.contains("deadline_exceeded")
        || lower.contains("unexpected eof")
        || lower.contains("failed to send message:")
}

fn parse_or_recover_envelope(
    raw_out: &str,
    mcp_ops_executed: &[String],
    default_transport: &str,
) -> Result<(CopilotEnvelope, String), String> {
    let clean_text = strip_reasoning_blocks(raw_out).trim().to_string();
    if let Ok(json_val) = extract_json_object(raw_out) {
        if let Ok(env) = serde_json::from_value::<CopilotEnvelope>(json_val) {
            return Ok((env, default_transport.to_string()));
        }
    }

    if !mcp_ops_executed.is_empty() {
        let reply = if clean_text.is_empty() {
            format!("عملیات با موفقیت از طریق MCP اجرا شد: {}", mcp_ops_executed.join("، "))
        } else {
            clean_text
        };
        return Ok((
            CopilotEnvelope {
                conversational_reply: reply,
                ..Default::default()
            },
            "MCP_TOOLS".to_string(),
        ));
    }

    if !clean_text.is_empty() && !looks_like_cli_or_transport_error(&clean_text) {
        return Ok((
            CopilotEnvelope {
                conversational_reply: clean_text,
                ..Default::default()
            },
            "PROSE_FALLBACK".to_string(),
        ));
    }

    let json_val = extract_json_object(raw_out)?;
    let env: CopilotEnvelope =
        serde_json::from_value(json_val).map_err(|e| format!("خطا در پارس پاسخ JSON مدل: {}", e))?;
    Ok((env, default_transport.to_string()))
}

pub async fn extract_copilot_envelope(
    app_data_dir: PathBuf,
    config: &LlmConfig,
    prompt: &str,
) -> Result<CopilotEnvelope, String> {
    let raw_out = match config.active_provider.as_str() {
        "GEMINI_REST" => {
            call_gemini_rest_structured(&config.gemini_api_key, &config.gemini_model, prompt, true)
                .await?
        }
        "LOCAL_OPENAI" => {
            call_local_openai_structured(&config.local_endpoint, &config.local_model, prompt, true)
                .await?
        }
        _ => match call_agy_cli(app_data_dir, &config.agy_model, prompt).await {
            Ok(res) => res,
            Err(agy_err) => {
                if !config.gemini_api_key.trim().is_empty() {
                    call_gemini_rest_structured(
                        &config.gemini_api_key,
                        &config.gemini_model,
                        prompt,
                        true,
                    )
                    .await?
                } else {
                    return Err(agy_err);
                }
            }
        },
    };
    let (env, _) = parse_or_recover_envelope(&raw_out, &[], "JSON_SCHEMA")?;
    Ok(env)
}

#[allow(clippy::too_many_arguments)]
pub async fn run_copilot_turn(
    db: &Mutex<Connection>,
    app_data_dir: PathBuf,
    config: &LlmConfig,
    project_id: &str,
    existing_event_id: Option<i64>,
    source_origin: &str,
    mcp_system_prompt: &str,
    mcp_user_prompt: &str,
    structured_fallback_prompt: &str,
) -> Result<CopilotTurnResult, String> {
    match config.active_provider.as_str() {
        "LOCAL_OPENAI" => {
            match run_local_openai_mcp_turn(
                db,
                &config.local_endpoint,
                &config.local_model,
                project_id,
                existing_event_id,
                source_origin,
                mcp_system_prompt,
                mcp_user_prompt,
            )
            .await
            {
                Ok(res) => Ok(res),
                Err(tool_err) => {
                    let err_lower = tool_err.to_ascii_lowercase();
                    if err_lower.contains("timed out")
                        || err_lower.contains("timeout")
                        || err_lower.contains("connection refused")
                        || err_lower.contains("10061")
                        || err_lower.contains("error sending request")
                    {
                        return Err(format!("خطای اتصال یا تایم‌اوت مدل لوکال: {}", tool_err));
                    }
                    let raw = call_local_openai_structured(
                        &config.local_endpoint,
                        &config.local_model,
                        structured_fallback_prompt,
                        true,
                    )
                    .await?;
                    let (env, label) = parse_or_recover_envelope(&raw, &[], "JSON_SCHEMA")?;
                    Ok(CopilotTurnResult {
                        envelope: env,
                        mcp_ops_executed: Vec::new(),
                        mcp_written_artifacts: Vec::new(),
                        transport_label: label,
                    })
                }
            }
        }
        "GEMINI_REST" => {
            match run_gemini_mcp_turn(
                db,
                &config.gemini_api_key,
                &config.gemini_model,
                project_id,
                existing_event_id,
                source_origin,
                mcp_system_prompt,
                mcp_user_prompt,
            )
            .await
            {
                Ok(res) => Ok(res),
                Err(_tool_err) => {
                    let raw = call_gemini_rest_structured(
                        &config.gemini_api_key,
                        &config.gemini_model,
                        structured_fallback_prompt,
                        true,
                    )
                    .await?;
                    let (env, label) = parse_or_recover_envelope(&raw, &[], "JSON_SCHEMA")?;
                    Ok(CopilotTurnResult {
                        envelope: env,
                        mcp_ops_executed: Vec::new(),
                        mcp_written_artifacts: Vec::new(),
                        transport_label: label,
                    })
                }
            }
        }
        _ => {
            match call_agy_cli(app_data_dir, &config.agy_model, structured_fallback_prompt).await {
                Ok(raw) => {
                    let (env, label) = parse_or_recover_envelope(&raw, &[], "AGY_JSON_SCHEMA")?;
                    Ok(CopilotTurnResult {
                        envelope: env,
                        mcp_ops_executed: Vec::new(),
                        mcp_written_artifacts: Vec::new(),
                        transport_label: label,
                    })
                }
                Err(agy_err) => {
                    if !config.gemini_api_key.trim().is_empty() {
                        run_gemini_mcp_turn(
                            db,
                            &config.gemini_api_key,
                            &config.gemini_model,
                            project_id,
                            existing_event_id,
                            source_origin,
                            mcp_system_prompt,
                            mcp_user_prompt,
                        )
                        .await
                    } else {
                        Err(agy_err)
                    }
                }
            }
        }
    }
}

pub async fn check_llm_connectivity(config: &LlmConfig) -> LlmConnectionStatus {
    match config.active_provider.as_str() {
        "LOCAL_OPENAI" => {
            let live = fetch_lm_studio_models(&config.local_endpoint).await;
            let chat_models: Vec<&LiveModelOption> = live
                .models
                .iter()
                .filter(|m| !m.id.to_ascii_lowercase().contains("embed"))
                .collect();
            let chosen_model = if !config.local_model.trim().is_empty() {
                config.local_model.clone()
            } else {
                chat_models
                    .first()
                    .copied()
                    .or_else(|| live.models.first())
                    .map(|m| m.id.clone())
                    .unwrap_or_default()
            };
            let connected = live.live_fetched && !chat_models.is_empty();
            let message = if connected {
                format!("متصل به LM Studio ({})", chosen_model)
            } else if live.live_fetched {
                "سرور LM Studio باز است اما مدل غیر-embedding بارگذاری نشده است.".to_string()
            } else {
                live.status_message
            };
            LlmConnectionStatus {
                connected,
                provider: "LOCAL_OPENAI".into(),
                model: chosen_model,
                message,
            }
        }
        "GEMINI_REST" => {
            let key = config.gemini_api_key.trim();
            let model = if config.gemini_model.trim().is_empty() {
                "gemini-2.5-flash".to_string()
            } else {
                config.gemini_model.clone()
            };
            if key.is_empty() {
                return LlmConnectionStatus {
                    connected: false,
                    provider: "GEMINI_REST".into(),
                    model,
                    message: "کلید Gemini API در تنظیمات وارد نشده است.".into(),
                };
            }
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models?key={}&pageSize=5",
                key
            );
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(4))
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    return LlmConnectionStatus {
                        connected: false,
                        provider: "GEMINI_REST".into(),
                        model,
                        message: format!("خطای کلاینت شبکه: {}", e),
                    };
                }
            };
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => LlmConnectionStatus {
                    connected: true,
                    provider: "GEMINI_REST".into(),
                    model: model.clone(),
                    message: format!("اتصال به Gemini API برقرار است ({})", model),
                },
                Ok(resp) => LlmConnectionStatus {
                    connected: false,
                    provider: "GEMINI_REST".into(),
                    model,
                    message: format!("خطای Gemini API ({})", resp.status()),
                },
                Err(e) => LlmConnectionStatus {
                    connected: false,
                    provider: "GEMINI_REST".into(),
                    model,
                    message: format!("عدم دسترسی به Gemini API: {}", e),
                },
            }
        }
        _ => {
            let model = config.agy_model.clone();
            let agy_live = fetch_agy_cli_models().await;
            if agy_live.live_fetched && !agy_live.models.is_empty() {
                LlmConnectionStatus {
                    connected: true,
                    provider: "AGY_CLI".into(),
                    model: model.clone(),
                    message: format!("متصل به Antigravity CLI ({})", model),
                }
            } else {
                LlmConnectionStatus {
                    connected: false,
                    provider: "AGY_CLI".into(),
                    model,
                    message: agy_live.status_message,
                }
            }
        }
    }
}

pub async fn fetch_live_models(
    provider: &str,
    local_endpoint: &str,
    gemini_api_key: &str,
) -> LiveModelsResponse {
    match provider {
        "LOCAL_OPENAI" => fetch_lm_studio_models(local_endpoint).await,
        "GEMINI_REST" => fetch_gemini_models(gemini_api_key).await,
        _ => fetch_agy_cli_models().await,
    }
}

pub fn derive_lm_studio_models_url(endpoint: &str) -> Result<String, String> {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("آدرس سرور لوکال LM Studio خالی است.".to_string());
    }
    if let Some(prefix) = trimmed.strip_suffix("/v1/chat/completions") {
        return Ok(format!("{}/v1/models", prefix));
    }
    if let Some(prefix) = trimmed.strip_suffix("/chat/completions") {
        return Ok(format!("{}/models", prefix));
    }
    if trimmed.ends_with("/v1/models") {
        return Ok(trimmed.to_string());
    }
    if trimmed.ends_with("/v1") {
        return Ok(format!("{}/models", trimmed));
    }
    if let Some(idx) = trimmed.find("/v1/") {
        return Ok(format!("{}/v1/models", &trimmed[..idx]));
    }
    Ok(format!("{}/v1/models", trimmed))
}

pub fn derive_lm_studio_chat_url(endpoint: &str) -> Result<String, String> {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("آدرس سرور لوکال LM Studio خالی است.".to_string());
    }
    if trimmed.ends_with("/chat/completions") {
        return Ok(trimmed.to_string());
    }
    if let Some(prefix) = trimmed.strip_suffix("/v1/models") {
        return Ok(format!("{}/v1/chat/completions", prefix));
    }
    if trimmed.ends_with("/v1") {
        return Ok(format!("{}/chat/completions", trimmed));
    }
    Ok(format!("{}/v1/chat/completions", trimmed))
}

pub fn parse_lm_studio_models_json(val: &Value) -> Vec<LiveModelOption> {
    let items = val["data"]
        .as_array()
        .or_else(|| val["models"].as_array())
        .or_else(|| val.as_array());

    let Some(arr) = items else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut chat_models = Vec::new();
    let mut embed_models = Vec::new();

    for item in arr {
        let id = item["id"]
            .as_str()
            .or_else(|| item["name"].as_str())
            .map(str::trim)
            .unwrap_or("");
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        let is_embed = id.to_ascii_lowercase().contains("embed");
        let owned_by = item["owned_by"].as_str().map(str::trim).filter(|s| !s.is_empty());
        let opt = LiveModelOption {
            id: id.to_string(),
            label: if is_embed {
                format!("{} (embedding)", id)
            } else {
                id.to_string()
            },
            details: owned_by.map(|o| o.to_string()),
        };
        if is_embed {
            embed_models.push(opt);
        } else {
            chat_models.push(opt);
        }
    }

    chat_models.extend(embed_models);
    chat_models
}

pub fn parse_gemini_models_json(val: &Value) -> Vec<LiveModelOption> {
    let Some(arr) = val["models"].as_array() else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut models = Vec::new();

    for item in arr {
        let raw_name = item["name"].as_str().map(str::trim).unwrap_or("");
        if raw_name.is_empty() {
            continue;
        }

        if let Some(methods) = item["supportedGenerationMethods"].as_array() {
            let supports_gen = methods
                .iter()
                .filter_map(|m| m.as_str())
                .any(|m| m == "generateContent");
            if !supports_gen {
                continue;
            }
        }

        let id = raw_name.strip_prefix("models/").unwrap_or(raw_name).to_string();
        if !seen.insert(id.clone()) {
            continue;
        }

        let display_name = item["displayName"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let label = match display_name {
            Some(dn) if dn != id => format!("{} ({})", dn, id),
            _ => id.clone(),
        };

        let input_limit = item["inputTokenLimit"].as_u64();
        let details = input_limit.map(|lim| format!("{}K context", lim / 1000));

        models.push(LiveModelOption { id, label, details });
    }

    models
}

fn strip_ansi_escapes(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for ch in chars.by_ref() {
                    if ('@'..='~').contains(&ch) {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn parse_agy_models_output(raw: &str) -> Vec<LiveModelOption> {
    let clean = strip_ansi_escapes(raw);
    let trimmed = clean.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    // 1. Try JSON payload if output is JSON
    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            let arr_opt = val
                .as_array()
                .or_else(|| val["models"].as_array())
                .or_else(|| val["data"].as_array());
            if let Some(arr) = arr_opt {
                let mut seen = HashSet::new();
                let mut out = Vec::new();
                for item in arr {
                    if let Some(s) = item.as_str().map(str::trim).filter(|s| !s.is_empty()) {
                        if seen.insert(s.to_string()) {
                            out.push(LiveModelOption {
                                id: s.to_string(),
                                label: s.to_string(),
                                details: None,
                            });
                        }
                    } else if let Some(obj) = item.as_object() {
                        let id = obj
                            .get("id")
                            .or_else(|| obj.get("name"))
                            .or_else(|| obj.get("model"))
                            .and_then(|v| v.as_str())
                            .map(str::trim)
                            .unwrap_or("");
                        if !id.is_empty() && seen.insert(id.to_string()) {
                            let label = obj
                                .get("displayName")
                                .or_else(|| obj.get("label"))
                                .and_then(|v| v.as_str())
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(|dn| {
                                    if dn != id {
                                        format!("{} ({})", dn, id)
                                    } else {
                                        id.to_string()
                                    }
                                })
                                .unwrap_or_else(|| id.to_string());
                            out.push(LiveModelOption {
                                id: id.to_string(),
                                label,
                                details: None,
                            });
                        }
                    }
                }
                if !out.is_empty() {
                    return out;
                }
            }
        }
    }

    // 2. Plain text / bullet / multi-column line-by-line parser
    let mut seen = HashSet::new();
    let mut models = Vec::new();

    for line in trimmed.lines() {
        let mut s = line.trim();
        if s.is_empty() {
            continue;
        }
        // Strip common bullet / list prefixes
        for prefix in ["- ", "* ", "• ", "> "] {
            if let Some(rest) = s.strip_prefix(prefix) {
                s = rest.trim();
            }
        }
        if s.is_empty()
            || s.starts_with("---")
            || s.starts_with("===")
            || s.ends_with(':')
        {
            continue;
        }

        let lower = s.to_ascii_lowercase();
        if lower.starts_with("available models")
            || lower.starts_with("fetching")
            || lower.starts_with("usage:")
            || lower.starts_with("id ")
            || lower.starts_with("model ")
            || lower == "id"
            || lower == "models"
        {
            continue;
        }

        // If separated by tabs or 2+ spaces, first column is ID, second is display/details
        let parts: Vec<&str> = s
            .split('\t')
            .flat_map(|p| p.split("  "))
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .collect();

        let (id, label, details) = if parts.len() >= 2 {
            let id_col = parts[0].trim_matches('`').trim();
            let desc_col = parts[1..].join(" • ");
            (
                id_col.to_string(),
                format!("{} ({})", parts[1], id_col),
                Some(desc_col),
            )
        } else {
            let single = s.trim_matches('`').trim();
            (single.to_string(), single.to_string(), None)
        };

        if !id.is_empty() && seen.insert(id.clone()) {
            models.push(LiveModelOption { id, label, details });
        }
    }

    models
}

async fn fetch_lm_studio_models(local_endpoint: &str) -> LiveModelsResponse {
    let url = match derive_lm_studio_models_url(local_endpoint) {
        Ok(u) => u,
        Err(e) => {
            return LiveModelsResponse {
                provider: "LOCAL_OPENAI".into(),
                models: vec![],
                live_fetched: false,
                status_message: e,
            }
        }
    };

    let client = match reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return LiveModelsResponse {
                provider: "LOCAL_OPENAI".into(),
                models: vec![],
                live_fetched: false,
                status_message: format!("خطا در ساخت کلاینت HTTP: {}", e),
            }
        }
    };

    match client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<Value>().await {
                Ok(val) if status.is_success() => {
                    let models = parse_lm_studio_models_json(&val);
                    let count = models.len();
                    LiveModelsResponse {
                        provider: "LOCAL_OPENAI".into(),
                        models,
                        live_fetched: true,
                        status_message: if count > 0 {
                            format!("{} مدل زنده از LM Studio دریافت شد.", count)
                        } else {
                            "سرور LM Studio متصل است اما هیچ مدلی بارگذاری نشده است.".into()
                        },
                    }
                }
                Ok(val) => LiveModelsResponse {
                    provider: "LOCAL_OPENAI".into(),
                    models: vec![],
                    live_fetched: false,
                    status_message: format!("پاسخ ناموفق از {} ({}): {}", url, status, val),
                },
                Err(e) => LiveModelsResponse {
                    provider: "LOCAL_OPENAI".into(),
                    models: vec![],
                    live_fetched: false,
                    status_message: format!("خطا در خواندن JSON از {}: {}", url, e),
                },
            }
        }
        Err(e) => LiveModelsResponse {
            provider: "LOCAL_OPENAI".into(),
            models: vec![],
            live_fetched: false,
            status_message: format!("اتصال به LM Studio برقرار نشد ({}): {}", url, e),
        },
    }
}

async fn fetch_gemini_models(api_key: &str) -> LiveModelsResponse {
    let key = api_key.trim();
    if key.is_empty() {
        return LiveModelsResponse {
            provider: "GEMINI_REST".into(),
            models: vec![],
            live_fetched: false,
            status_message: "برای دریافت زنده مدل‌ها از /v1beta/models، ابتدا کلید Gemini API را وارد کنید.".into(),
        };
    }

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}&pageSize=100",
        key
    );

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return LiveModelsResponse {
                provider: "GEMINI_REST".into(),
                models: vec![],
                live_fetched: false,
                status_message: format!("خطا در ساخت کلاینت HTTP: {}", e),
            }
        }
    };

    match client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<Value>().await {
                Ok(val) if status.is_success() => {
                    let models = parse_gemini_models_json(&val);
                    let count = models.len();
                    LiveModelsResponse {
                        provider: "GEMINI_REST".into(),
                        models,
                        live_fetched: true,
                        status_message: format!("{} مدل زنده از Gemini API (/v1beta/models) دریافت شد.", count),
                    }
                }
                Ok(val) => {
                    let api_err = val["error"]["message"]
                        .as_str()
                        .unwrap_or("کلید API نامعتبر یا محدودیت دسترسی");
                    LiveModelsResponse {
                        provider: "GEMINI_REST".into(),
                        models: vec![],
                        live_fetched: false,
                        status_message: format!("خطای Gemini API ({}): {}", status, api_err),
                    }
                }
                Err(e) => LiveModelsResponse {
                    provider: "GEMINI_REST".into(),
                    models: vec![],
                    live_fetched: false,
                    status_message: format!("خطا در خواندن پاسخ Gemini API: {}", e),
                },
            }
        }
        Err(e) => LiveModelsResponse {
            provider: "GEMINI_REST".into(),
            models: vec![],
            live_fetched: false,
            status_message: format!("اتصال به Gemini API برقرار نشد: {}", e),
        },
    }
}

pub fn format_agy_not_installed_message(raw_detail: &str) -> String {
    let trimmed = raw_detail.trim();
    if trimmed.is_empty() {
        "ابزار CLI آنتی‌گرویتی (`agy`) روی سیستم نصب نیست یا در PATH قرار ندارد (نصب دسکتاپ IDE به‌تنهایی کافی نیست). برای نصب دستور `irm https://antigravity.google/cli/install.ps1 | iex` را در PowerShell اجرا کنید، یا از بخش تنظیمات به LM Studio لوکال / Gemini API سوییچ نمایید.".to_string()
    } else {
        format!(
            "ابزار CLI آنتی‌گرویتی (`agy`) روی سیستم نصب نیست یا در PATH یافت نشد (جزئیات: {}). برای نصب دستور `irm https://antigravity.google/cli/install.ps1 | iex` را در PowerShell بزنید، یا موتور پردازش را روی LM Studio لوکال / Gemini API بگذارید.",
            trimmed
        )
    }
}

pub fn resolve_agy_executable() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(path_var) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path_var) {
                for cand in ["agy.exe", "agy.cmd", "agy.bat", "agy"] {
                    let p = dir.join(cand);
                    if p.is_file() {
                        return p;
                    }
                }
            }
        }
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let l = PathBuf::from(local);
            candidates.push(l.join("agy").join("bin").join("agy.exe"));
            candidates.push(l.join("agy").join("bin").join("agy.cmd"));
            candidates.push(l.join("Programs").join("antigravity").join("bin").join("agy.exe"));
            candidates.push(l.join("Programs").join("antigravity").join("bin").join("agy.cmd"));
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            let h = PathBuf::from(home);
            candidates.push(h.join(".local").join("bin").join("agy.exe"));
            candidates.push(h.join(".local").join("bin").join("agy.cmd"));
        }
        if let Some(appdata) = std::env::var_os("APPDATA") {
            candidates.push(PathBuf::from(appdata).join("npm").join("agy.cmd"));
        }
        for c in candidates {
            if c.is_file() {
                return c;
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let h = PathBuf::from(home);
            let p = h.join(".local").join("bin").join("agy");
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from("agy")
}

fn is_missing_agy_output(detail: &str) -> bool {
    let lower = detail.to_ascii_lowercase();
    lower.contains("is not recognized as an internal or external command")
        || lower.contains("not found")
        || lower.contains("cannot find the path specified")
        || lower.contains("no such file or directory")
}

async fn run_agy_command(args: &[&str]) -> Result<std::process::Output, String> {
    let agy_bin = resolve_agy_executable();
    let is_cmd_script = agy_bin
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("cmd") || s.eq_ignore_ascii_case("bat"))
        .unwrap_or(false);

    let mut cmd = if is_cmd_script {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&agy_bin).args(args);
        c
    } else {
        let mut c = Command::new(&agy_bin);
        c.args(args);
        c
    };

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let fut = cmd.output();
    match tokio::time::timeout(Duration::from_secs(8), fut).await {
        Ok(Ok(out)) => Ok(out),
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            #[cfg(target_os = "windows")]
            {
                let mut cmd_args = vec!["/C", "agy"];
                cmd_args.extend_from_slice(args);
                let mut fallback = Command::new("cmd");
                fallback.args(&cmd_args).creation_flags(CREATE_NO_WINDOW);
                match tokio::time::timeout(Duration::from_secs(8), fallback.output()).await {
                    Ok(Ok(out)) => Ok(out),
                    Ok(Err(err)) => Err(format_agy_not_installed_message(&err.to_string())),
                    Err(_) => Err("اجرای دستور agy بیش از ۸ ثانیه طول کشید (Timeout).".into()),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err(format_agy_not_installed_message(&e.to_string()))
            }
        }
        Ok(Err(e)) => Err(format!("خطا در اجرای agy CLI: {}", e)),
        Err(_) => Err("اجرای دستور agy بیش از ۸ ثانیه طول کشید (Timeout).".into()),
    }
}

async fn fetch_agy_cli_models() -> LiveModelsResponse {
    let output = match run_agy_command(&["models"]).await {
        Ok(o) => o,
        Err(e) => {
            return LiveModelsResponse {
                provider: "AGY_CLI".into(),
                models: vec![],
                live_fetched: false,
                status_message: e,
            }
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        let status_message = if is_missing_agy_output(detail) {
            format_agy_not_installed_message(detail)
        } else {
            format!("دستور `agy models` با خطا متوقف شد: {}", detail)
        };
        return LiveModelsResponse {
            provider: "AGY_CLI".into(),
            models: vec![],
            live_fetched: false,
            status_message,
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let models = parse_agy_models_output(&stdout);
    let count = models.len();
    LiveModelsResponse {
        provider: "AGY_CLI".into(),
        models,
        live_fetched: count > 0,
        status_message: if count > 0 {
            format!("{} مدل زنده از Antigravity CLI دریافت شد.", count)
        } else {
            "خروجی دستور `agy models` خالی بود یا مدلی در آن تشخیص داده نشد.".into()
        },
    }
}

fn format_agy_runtime_error(detail: &str) -> String {
    let trimmed = detail.trim();
    let lower = trimmed.to_ascii_lowercase();
    if is_missing_agy_output(trimmed) {
        return format_agy_not_installed_message(trimmed);
    }
    if lower.contains("authentication required") || lower.contains("accounts.google.com/o/oauth2") {
        return format!(
            "احراز هویت Antigravity CLI (`agy`) نیاز به ورود مجدد دارد. دستور `agy login` را اجرا کنید یا از تب تنظیمات به LM Studio / Gemini API بروید.\nجزئیات: {}",
            trimmed
        );
    }
    if lower.contains("503") || lower.contains("no capacity available") || lower.contains("unavailable") {
        return format!(
            "سرور مدل انتخاب‌شده در سرویس `agy` موقتاً ظرفیت ندارد (503). در تب تنظیمات مدل دیگری (مثلاً Claude یا GPT-OSS) انتخاب کنید یا به LM Studio سوییچ نمایید.\nجزئیات: {}",
            trimmed
        );
    }
    if lower.contains("deadline_exceeded") || lower.contains("print timeout") {
        return format!(
            "پاسخ‌دهی سرویس `agy` بیش از زمان مجاز طول کشید (Timeout). لطفاً دوباره تلاش کنید یا مدل سریع‌تری انتخاب کنید.\nجزئیات: {}",
            trimmed
        );
    }
    format!("خطای سرویس agy CLI: {}", trimmed)
}

pub fn parse_agy_stream_json_output(stdout: &str, stderr: &str) -> Result<String, String> {
    let mut last_result: Option<Value> = None;
    let mut fallback_text_parts: Vec<String> = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        let Ok(val) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if let Some(res) = val.get("result").filter(|v| v.is_object()) {
            last_result = Some(res.clone());
        } else if val.get("status").is_some() && (val.get("response").is_some() || val.get("error").is_some()) {
            last_result = Some(val.clone());
        } else if let Some(txt) = val
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .or_else(|| val.pointer("/step_update/content").and_then(|v| v.as_str()))
            .or_else(|| val.get("text").and_then(|v| v.as_str()))
        {
            if !txt.trim().is_empty() {
                fallback_text_parts.push(txt.to_string());
            }
        }
    }

    if let Some(res) = last_result {
        if let Some(structured) = res.get("structured_output").filter(|v| v.is_object()) {
            return serde_json::to_string(structured).map_err(|e| e.to_string());
        }
        let resp_str = res.get("response").and_then(|v| v.as_str()).unwrap_or("").trim();
        if !resp_str.is_empty() {
            return Ok(resp_str.to_string());
        }
        let status = res.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let err_str = res.get("error").and_then(|v| v.as_str()).unwrap_or("").trim();
        if status.eq_ignore_ascii_case("ERROR") || !err_str.is_empty() {
            let detail = if err_str.is_empty() { stderr.trim() } else { err_str };
            return Err(format_agy_runtime_error(detail));
        }
    }

    let joined_fallback = fallback_text_parts.join("\n").trim().to_string();
    if !joined_fallback.is_empty() {
        return Ok(joined_fallback);
    }

    let clean_stderr = stderr.trim();
    if !clean_stderr.is_empty() {
        return Err(format_agy_runtime_error(clean_stderr));
    }

    let clean_stdout = stdout.trim();
    if !clean_stdout.is_empty() {
        return Ok(clean_stdout.to_string());
    }

    Err("خروجی `agy CLI` خالی بود و هیچ پاسخی از سرور دریافت نشد.".to_string())
}

fn ensure_copilot_schema_file(clean_room: &std::path::Path) -> Result<PathBuf, String> {
    let schema_path = clean_room.join("copilot_envelope_schema.json");
    let content =
        serde_json::to_string_pretty(&copilot_envelope_json_schema()).map_err(|e| e.to_string())?;
    let needs_write = fs::read_to_string(&schema_path)
        .map(|existing| existing != content)
        .unwrap_or(true);
    if needs_write {
        fs::write(&schema_path, content).map_err(|e| e.to_string())?;
    }
    Ok(schema_path)
}

async fn call_agy_cli(app_data_dir: PathBuf, model: &str, prompt: &str) -> Result<String, String> {
    let clean_room = app_data_dir.join("agy_clean_room");
    if !clean_room.exists() {
        fs::create_dir_all(&clean_room).map_err(|e| e.to_string())?;
    }
    let schema_path = ensure_copilot_schema_file(&clean_room)?;
    let schema_arg = schema_path.to_string_lossy().to_string();

    let model_name = if model.trim().is_empty() || model.trim() == "Gemini 3.1 Pro (High)" {
        "gemini-3.8-flash-high"
    } else {
        model.trim()
    };

    let agy_bin = resolve_agy_executable();
    let is_cmd_script = agy_bin
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("cmd") || s.eq_ignore_ascii_case("bat"))
        .unwrap_or(false);

    let stream_args = [
        "-p=".to_string(),
        "--input-format".to_string(),
        "stream-json".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--json-schema".to_string(),
        schema_arg.clone(),
        "--disable-slash-commands".to_string(),
        "--model".to_string(),
        model_name.to_string(),
        "--print-timeout".to_string(),
        "120s".to_string(),
    ];

    let mut cmd = if is_cmd_script {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&agy_bin).args(&stream_args);
        c
    } else {
        let mut c = Command::new(&agy_bin);
        c.args(&stream_args);
        c
    };
    cmd.current_dir(&clean_room)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let child_res = cmd.spawn();
    let mut child = match child_res {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            #[cfg(target_os = "windows")]
            {
                let mut cmd_fallback = Command::new("cmd");
                cmd_fallback
                    .arg("/C")
                    .arg("agy")
                    .args(&stream_args)
                    .current_dir(&clean_room)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .creation_flags(CREATE_NO_WINDOW);
                cmd_fallback
                    .spawn()
                    .map_err(|err| format_agy_not_installed_message(&err.to_string()))?
            }
            #[cfg(not(target_os = "windows"))]
            {
                return Err(format_agy_not_installed_message(&e.to_string()));
            }
        }
        Err(e) => {
            return Err(format_agy_not_installed_message(&e.to_string()));
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let payload = json!({
            "event": "user",
            "message": {
                "role": "user",
                "content": prompt
            }
        });
        let line = format!("{}\n", payload);
        let _ = stdin.write_all(line.as_bytes()).await;
        let _ = stdin.flush().await;
        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("خطا در انتظار پاسخ agy CLI: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    parse_agy_stream_json_output(&stdout, &stderr)
}

async fn call_gemini_rest_structured(
    api_key: &str,
    model: &str,
    prompt: &str,
    structured: bool,
) -> Result<String, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("کلید Gemini REST API تنظیم نشده است. از تب تنظیمات، کلید API را وارد کنید یا موتور را روی LM Studio / agy قرار دهید.".into());
    }
    let clean_model = model.trim().strip_prefix("models/").unwrap_or(model.trim());
    let model_id = if clean_model.is_empty() {
        "gemini-2.5-flash"
    } else {
        clean_model
    };
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_id, key
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;
    let gen_cfg = if structured {
        json!({
            "temperature": 0.1,
            "responseMimeType": "application/json",
            "responseSchema": gemini_copilot_response_schema()
        })
    } else {
        json!({ "temperature": 0.1 })
    };
    let body = json!({
        "contents": [{ "parts": [{ "text": prompt }] }],
        "generationConfig": gen_cfg
    });
    let resp = client.post(&url).json(&body).send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let val: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("خطای Gemini REST API ({}): {}", status, val));
    }
    val["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("پاسخ نامعتبر از Gemini REST API: {}", val))
}

#[allow(clippy::too_many_arguments)]
async fn run_gemini_mcp_turn(
    db: &Mutex<Connection>,
    api_key: &str,
    model: &str,
    project_id: &str,
    existing_event_id: Option<i64>,
    source_origin: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<CopilotTurnResult, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("کلید Gemini REST API تنظیم نشده است. از تب تنظیمات، کلید API را وارد کنید یا موتور را روی LM Studio / agy قرار دهید.".into());
    }
    let clean_model = model.trim().strip_prefix("models/").unwrap_or(model.trim());
    let model_id = if clean_model.is_empty() {
        "gemini-2.5-flash"
    } else {
        clean_model
    };
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_id, key
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;

    let tools = gemini_tools_payload();
    let mut contents: Vec<Value> = vec![json!({
        "role": "user",
        "parts": [{ "text": user_prompt }]
    })];
    let mut mcp_ops_executed: Vec<String> = Vec::new();
    let mut mcp_written_artifacts: Vec<WrittenArtifactRef> = Vec::new();

    for _round in 0..6 {
        let body = json!({
            "systemInstruction": { "parts": [{ "text": system_prompt }] },
            "contents": contents,
            "tools": tools,
            "generationConfig": { "temperature": 0.1 }
        });
        let resp = client.post(&url).json(&body).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let val: Value = resp.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("خطای Gemini REST API ({}): {}", status, val));
        }

        let parts = val["candidates"][0]["content"]["parts"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if parts.is_empty() {
            return Err(format!("پاسخ نامعتبر از Gemini REST API: {}", val));
        }

        let fn_calls: Vec<(String, Value)> = parts
            .iter()
            .filter_map(|p| {
                let fc = p.get("functionCall")?;
                let name = fc.get("name")?.as_str()?.to_string();
                let args = fc.get("args").cloned().unwrap_or_else(|| json!({}));
                Some((name, args))
            })
            .collect();

        if !fn_calls.is_empty() {
            contents.push(json!({
                "role": "model",
                "parts": parts
            }));

            let mut fn_responses: Vec<Value> = Vec::new();
            for (fn_name, fn_args) in fn_calls {
                let exec_res = {
                    let mut conn = db.lock().map_err(|e| e.to_string())?;
                    execute_tool_with_context(
                        &mut conn,
                        &fn_name,
                        &fn_args,
                        Some(project_id),
                        existing_event_id,
                        source_origin,
                    )
                };
                let response_obj = match exec_res {
                    Ok(ok_val) => {
                        mcp_ops_executed.push(fn_name.clone());
                        collect_artifact_from_mcp_output(&ok_val, &mut mcp_written_artifacts);
                        ok_val
                    }
                    Err(err_msg) => json!({ "error": err_msg }),
                };
                fn_responses.push(json!({
                    "functionResponse": {
                        "name": fn_name,
                        "response": response_obj
                    }
                }));
            }
            contents.push(json!({
                "role": "user",
                "parts": fn_responses
            }));
            continue;
        }

        let text_out = parts
            .iter()
            .filter_map(|p| p.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n");
        let (env, label) = parse_or_recover_envelope(&text_out, &mcp_ops_executed, "GEMINI_MCP")?;
        return Ok(CopilotTurnResult {
            envelope: env,
            mcp_ops_executed,
            mcp_written_artifacts,
            transport_label: label,
        });
    }

    Ok(CopilotTurnResult {
        envelope: CopilotEnvelope {
            conversational_reply: format!(
                "تغییرات درخواستی از طریق ابزارهای MCP ثبت شد ({}).",
                mcp_ops_executed.join("، ")
            ),
            ..Default::default()
        },
        mcp_ops_executed,
        mcp_written_artifacts,
        transport_label: "MCP_TOOLS".to_string(),
    })
}

async fn call_local_openai_structured(
    endpoint: &str,
    model: &str,
    prompt: &str,
    structured: bool,
) -> Result<String, String> {
    let _gate = LOCAL_OPENAI_GATE.lock().await;
    let ep = derive_lm_studio_chat_url(endpoint)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(360))
        .build()
        .map_err(|e| e.to_string())?;
    let clean_model = model.trim();

    let modes: Vec<Option<Value>> = if structured {
        vec![
            Some(json!({
                "type": "json_schema",
                "json_schema": {
                    "name": "copilot_envelope",
                    "strict": false,
                    "schema": copilot_envelope_json_schema()
                }
            })),
            Some(json!({ "type": "json_object" })),
            None,
        ]
    } else {
        vec![None]
    };

    let mut last_err = String::new();
    for rf in modes {
        let mut body = json!({
            "messages": [{ "role": "user", "content": prompt }],
            "temperature": 0.1,
            "max_tokens": 4096
        });
        if !clean_model.is_empty() {
            body["model"] = json!(clean_model);
        }
        if let Some(format_val) = &rf {
            body["response_format"] = format_val.clone();
        }
        let resp = client.post(&ep).json(&body).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let val: Value = resp.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            last_err = format!("خطای سرور لوکال LM Studio ({}): {}", status, val);
            if status.as_u16() == 400 || status.as_u16() == 422 {
                continue;
            }
            return Err(last_err);
        }
        if let Some(content) = val["choices"][0]["message"]["content"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
        {
            return Ok(content.to_string());
        }
        if let Some(reasoning) = val["choices"][0]["message"]["reasoning_content"]
            .as_str()
            .or_else(|| val["choices"][0]["message"]["reasoning"].as_str())
            .filter(|s| !s.trim().is_empty())
        {
            return Ok(reasoning.to_string());
        }
        last_err = format!("پاسخ نامعتبر از سرور لوکال LM Studio: {}", val);
    }
    Err(last_err)
}

#[allow(clippy::too_many_arguments)]
async fn run_local_openai_mcp_turn(
    db: &Mutex<Connection>,
    endpoint: &str,
    model: &str,
    project_id: &str,
    existing_event_id: Option<i64>,
    source_origin: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<CopilotTurnResult, String> {
    let _gate = LOCAL_OPENAI_GATE.lock().await;
    let ep = derive_lm_studio_chat_url(endpoint)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(360))
        .build()
        .map_err(|e| e.to_string())?;
    let clean_model = model.trim();
    let tools = openai_tools_payload();

    let mut messages: Vec<Value> = vec![
        json!({ "role": "system", "content": system_prompt }),
        json!({ "role": "user", "content": user_prompt }),
    ];
    let mut mcp_ops_executed: Vec<String> = Vec::new();
    let mut mcp_written_artifacts: Vec<WrittenArtifactRef> = Vec::new();

    for _round in 0..6 {
        let mut body = json!({
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "temperature": 0.1,
            "max_tokens": 4096
        });
        if !clean_model.is_empty() {
            body["model"] = json!(clean_model);
        }

        let resp = client.post(&ep).json(&body).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let val: Value = resp.json().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("خطای سرور لوکال LM Studio ({}): {}", status, val));
        }

        let msg = val["choices"][0]["message"].clone();
        if msg.is_null() {
            return Err(format!("پاسخ نامعتبر از سرور لوکال LM Studio: {}", val));
        }

        let tool_calls = msg["tool_calls"].as_array().cloned().unwrap_or_default();
        if !tool_calls.is_empty() {
            messages.push(msg.clone());
            for tc in tool_calls {
                let tc_id = tc["id"].as_str().unwrap_or("call_0").to_string();
                let fn_name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                let raw_args = &tc["function"]["arguments"];
                let fn_args: Value = match raw_args {
                    Value::String(s) => serde_json::from_str(s).unwrap_or_else(|_| json!({})),
                    Value::Object(_) => raw_args.clone(),
                    _ => json!({}),
                };

                let exec_res = {
                    let mut conn = db.lock().map_err(|e| e.to_string())?;
                    execute_tool_with_context(
                        &mut conn,
                        &fn_name,
                        &fn_args,
                        Some(project_id),
                        existing_event_id,
                        source_origin,
                    )
                };
                let tool_text = match exec_res {
                    Ok(ok_val) => {
                        mcp_ops_executed.push(fn_name.clone());
                        collect_artifact_from_mcp_output(&ok_val, &mut mcp_written_artifacts);
                        serde_json::to_string(&ok_val).unwrap_or_else(|_| ok_val.to_string())
                    }
                    Err(err_msg) => json!({ "error": err_msg }).to_string(),
                };
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tc_id,
                    "content": tool_text
                }));
            }
            continue;
        }

        let text_out = msg["content"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| msg["reasoning_content"].as_str().filter(|s| !s.trim().is_empty()))
            .or_else(|| msg["reasoning"].as_str().filter(|s| !s.trim().is_empty()))
            .unwrap_or("")
            .to_string();

        if text_out.trim().is_empty() && mcp_ops_executed.is_empty() {
            return Err("پاسخ مدل خالی بود.".to_string());
        }

        let (env, label) = parse_or_recover_envelope(&text_out, &mcp_ops_executed, "MCP_TOOLS")?;
        return Ok(CopilotTurnResult {
            envelope: env,
            mcp_ops_executed,
            mcp_written_artifacts,
            transport_label: label,
        });
    }

    Ok(CopilotTurnResult {
        envelope: CopilotEnvelope {
            conversational_reply: format!(
                "تغییرات درخواستی از طریق ابزارهای MCP ثبت شد ({}).",
                mcp_ops_executed.join("، ")
            ),
            ..Default::default()
        },
        mcp_ops_executed,
        mcp_written_artifacts,
        transport_label: "MCP_TOOLS".to_string(),
    })
}

fn strip_reasoning_blocks(raw: &str) -> String {
    let mut text = raw.to_string();
    while let (Some(start), Some(end)) = (text.find("<think>"), text.find("</think>")) {
        if end >= start {
            text.replace_range(start..end + "</think>".len(), "");
        } else {
            break;
        }
    }
    text
}

pub fn extract_json_object(raw: &str) -> Result<Value, String> {
    let without_think = strip_reasoning_blocks(raw);
    let cleaned = without_think
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if cleaned.is_empty() {
        return Err("پاسخ مدل خالی بود (هیچ متنی دریافت نشد).".to_string());
    }
    let preview: String = cleaned.chars().take(180).collect();
    let start = cleaned.find('{').ok_or_else(|| {
        format!(
            "پاسخ JSON معتبر از مدل دریافت نشد (علامت شروع '{{' یافت نشد). خروجی مدل: «{}»",
            preview.replace('\n', " ")
        )
    })?;
    let end = cleaned.rfind('}').ok_or_else(|| {
        format!(
            "پایان شیء JSON '}}' در خروجی مدل یافت نشد. خروجی مدل: «{}»",
            preview.replace('\n', " ")
        )
    })?;
    if let Ok(v) = serde_json::from_str::<Value>(&cleaned[start..=end]) {
        return Ok(v);
    }

    // Balanced brace scan fallback in case prose around the JSON includes braces
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            let mut depth = 0usize;
            let mut in_str = false;
            let mut escape = false;
            for j in i..bytes.len() {
                let b = bytes[j];
                if in_str {
                    if escape {
                        escape = false;
                    } else if b == b'\\' {
                        escape = true;
                    } else if b == b'"' {
                        in_str = false;
                    }
                    continue;
                }
                match b {
                    b'"' => in_str = true,
                    b'{' => depth += 1,
                    b'}' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            if let Some(slice) = cleaned.get(i..=j) {
                                if let Ok(v) = serde_json::from_str::<Value>(slice) {
                                    return Ok(v);
                                }
                            }
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
        i += 1;
    }

    serde_json::from_str(&cleaned[start..=end]).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CopilotEnvelope;

    #[test]
    fn test_extract_copilot_envelope_with_fences_and_defaults() {
        let raw = "```json\n{\"conversational_reply\": \"ثبت شد\", \"mutations\": []}\n```";
        let val = extract_json_object(raw).unwrap();
        let env: CopilotEnvelope = serde_json::from_value(val).unwrap();
        assert_eq!(env.conversational_reply, "ثبت شد");
        assert!(!env.meeting_detected);
        assert!(env.clarifying_questions.is_empty());
        assert!(env.mutations.is_empty());
    }

    #[test]
    fn test_extract_envelope_with_think_block_and_clarifying_questions() {
        let raw = "<think>Thinking {internal}...</think>\n```json\n{\"conversational_reply\": \"نیاز به سوال\", \"clarifying_questions\": [\"سوال اول؟\"], \"needs_user_clarification\": true, \"mutations\": [{\"op\": \"UPDATE_ENTITY\", \"target_id\": \"TSK-1\", \"status\": \"IN_PROGRESS\"}]}\n```";
        let val = extract_json_object(raw).unwrap();
        let env: CopilotEnvelope = serde_json::from_value(val).unwrap();
        assert!(env.needs_user_clarification);
        assert_eq!(env.clarifying_questions, vec!["سوال اول؟"]);
        assert_eq!(env.mutations[0].op, "UPDATE_ENTITY");
        assert_eq!(env.mutations[0].status.as_deref(), Some("IN_PROGRESS"));
    }

    #[test]
    fn test_extract_envelope_with_float_zero_integers_and_nulls() {
        let raw = r#"{
            "conversational_reply": "انجام شد",
            "clarifying_questions": null,
            "needs_user_clarification": null,
            "meeting_detected": 0,
            "mutations": [
                {
                    "op": "CREATE_ENTITY",
                    "entity_type": "TASK",
                    "title": "تست پارس انعطاف‌پذیر",
                    "is_on_critical_path": 1,
                    "optimistic_pomos": "1.0",
                    "most_likely_pomos": 2,
                    "pessimistic_pomos": 3.0,
                    "severity_score": 0.0,
                    "effective_pomos": 0.0
                }
            ]
        }"#;
        let val = extract_json_object(raw).unwrap();
        let env: CopilotEnvelope = serde_json::from_value(val).unwrap();
        assert_eq!(env.conversational_reply, "انجام شد");
        assert!(env.clarifying_questions.is_empty());
        assert!(!env.needs_user_clarification);
        assert!(!env.meeting_detected);
        assert_eq!(env.mutations.len(), 1);
        assert_eq!(env.mutations[0].severity_score, Some(0));
        assert_eq!(env.mutations[0].effective_pomos, Some(0));
        assert_eq!(env.mutations[0].optimistic_pomos, Some(1.0));
        assert_eq!(env.mutations[0].is_on_critical_path, Some(true));
    }

    #[test]
    fn test_derive_lm_studio_urls() {
        assert_eq!(
            derive_lm_studio_models_url("http://127.0.0.1:1234/v1/chat/completions").unwrap(),
            "http://127.0.0.1:1234/v1/models"
        );
        assert_eq!(
            derive_lm_studio_models_url("http://127.0.0.1:1234/v1/").unwrap(),
            "http://127.0.0.1:1234/v1/models"
        );
        assert_eq!(
            derive_lm_studio_models_url("http://127.0.0.1:1234").unwrap(),
            "http://127.0.0.1:1234/v1/models"
        );
        assert_eq!(
            derive_lm_studio_chat_url("http://127.0.0.1:1234").unwrap(),
            "http://127.0.0.1:1234/v1/chat/completions"
        );
    }

    #[test]
    fn test_parse_lm_studio_gemini_and_agy_models() {
        let lm_json = serde_json::json!({
            "object": "list",
            "data": [
                { "id": "text-embedding-nomic", "owned_by": "org" },
                { "id": "qwen3.5-4b", "owned_by": "org" },
                { "id": "gemma-4-26b", "owned_by": "org" }
            ]
        });
        let lm_models = parse_lm_studio_models_json(&lm_json);
        assert_eq!(lm_models.len(), 3);
        assert_eq!(lm_models[0].id, "qwen3.5-4b"); // non-embedding prioritized first
        assert_eq!(lm_models[2].id, "text-embedding-nomic");

        let gem_json = serde_json::json!({
            "models": [
                {
                    "name": "models/gemini-2.5-flash",
                    "displayName": "Gemini 2.5 Flash",
                    "inputTokenLimit": 1048576,
                    "supportedGenerationMethods": ["generateContent", "countTokens"]
                },
                {
                    "name": "models/text-embedding-004",
                    "displayName": "Embedding 004",
                    "supportedGenerationMethods": ["embedContent"]
                }
            ]
        });
        let gem_models = parse_gemini_models_json(&gem_json);
        assert_eq!(gem_models.len(), 1);
        assert_eq!(gem_models[0].id, "gemini-2.5-flash");
        assert!(gem_models[0].label.contains("Gemini 2.5 Flash"));

        let agy_text = "\x1b[32mAvailable models:\x1b[0m\n- gemini-3.8-flash-high   Gemini 3.8 Flash (High)\n- Gemini 3.1 Pro (High)\n";
        let agy_models = parse_agy_models_output(agy_text);
        assert_eq!(agy_models.len(), 2);
        assert_eq!(agy_models[0].id, "gemini-3.8-flash-high");
        assert_eq!(agy_models[1].id, "Gemini 3.1 Pro (High)");

        let agy_json = r#"["gemini-3.8-flash-high", "claude-sonnet-4-6"]"#;
        let agy_models2 = parse_agy_models_output(agy_json);
        assert_eq!(agy_models2.len(), 2);
        assert_eq!(agy_models2[1].id, "claude-sonnet-4-6");
    }

    #[tokio::test]
    async fn test_fetch_live_models_lm_studio_http_and_missing_gemini_key() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let body = r#"{"object":"list","data":[{"id":"qwen3.5-4b","owned_by":"local"},{"id":"text-embedding-nomic","owned_by":"local"}]}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            }
        });

        let endpoint = format!("http://{}/v1/chat/completions", addr);
        let live = fetch_live_models("LOCAL_OPENAI", &endpoint, "").await;
        assert!(live.live_fetched);
        assert_eq!(live.models.len(), 2);
        assert_eq!(live.models[0].id, "qwen3.5-4b");

        let _ = server.await;

        let gem_empty = fetch_live_models("GEMINI_REST", "", "").await;
        assert!(!gem_empty.live_fetched);
        assert!(gem_empty.status_message.contains("/v1beta/models"));
    }

    #[test]
    fn test_missing_agy_detection_and_friendly_message() {
        assert!(is_missing_agy_output(
            "'agy' is not recognized as an internal or external command, operable program or batch file."
        ));
        assert!(!is_missing_agy_output("Model timeout exceeded"));
        let msg = format_agy_not_installed_message("'agy' is not recognized");
        assert!(msg.contains("LM Studio"));
        assert!(msg.contains("antigravity.google/cli/install.ps1"));
    }

    #[test]
    fn test_parse_agy_stream_json_and_prose_recovery() {
        let ok_ndjson = r#"{"event":"init","conversation_id":"1"}
{"event":"result","result":{"status":"OK","structured_output":{"conversational_reply":"سلام! ثبت شد","mutations":[]}}}"#;
        let out = parse_agy_stream_json_output(ok_ndjson, "").unwrap();
        let (env, label) = parse_or_recover_envelope(&out, &[], "AGY_JSON_SCHEMA").unwrap();
        assert_eq!(env.conversational_reply, "سلام! ثبت شد");
        assert_eq!(label, "AGY_JSON_SCHEMA");

        let err_ndjson = r#"{"event":"init","conversation_id":"1"}
{"event":"result","result":{"status":"ERROR","response":"","error":"API error: UNAVAILABLE (code 503): No capacity available for model gemini-3.8-flash-medium"}}"#;
        let err = parse_agy_stream_json_output(err_ndjson, "").unwrap_err();
        assert!(err.contains("503"));
        assert!(err.contains("ظرفیت"));

        let (prose_env, prose_label) =
            parse_or_recover_envelope("این یک پاسخ متنی مستقیم به فارسی است.", &[], "JSON_SCHEMA")
                .unwrap();
        assert_eq!(prose_label, "PROSE_FALLBACK");
        assert!(prose_env.conversational_reply.contains("پاسخ متنی مستقیم"));
    }

    #[tokio::test]
    async fn test_run_local_openai_mcp_tool_loop_end_to_end() {
        use crate::db::schema::init_database;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let dir = std::env::temp_dir().join(format!("mab_mcp_loop_{}", uuid::Uuid::new_v4()));
        let vault = dir.join("vault");
        fs::create_dir_all(&vault).unwrap();
        let conn = init_database(&dir.join("test.sqlite"), &vault).unwrap();
        let db = Mutex::new(conn);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            for round in 0..2 {
                if let Ok((mut socket, _)) = listener.accept().await {
                    // Drain full HTTP request (headers + Content-Length body) so Windows doesn't TCP RST on large tool payloads
                    let mut req_buf = Vec::with_capacity(16384);
                    let mut tmp = [0u8; 4096];
                    let mut expected_total: Option<usize> = None;
                    loop {
                        match socket.read(&mut tmp).await {
                            Ok(0) => break,
                            Ok(n) => {
                                req_buf.extend_from_slice(&tmp[..n]);
                                if expected_total.is_none() {
                                    if let Some(pos) = req_buf
                                        .windows(4)
                                        .position(|w| w == b"\r\n\r\n")
                                    {
                                        let headers_end = pos + 4;
                                        let headers_str =
                                            String::from_utf8_lossy(&req_buf[..headers_end]);
                                        let content_len = headers_str
                                            .lines()
                                            .find_map(|l| {
                                                let lower = l.to_ascii_lowercase();
                                                lower
                                                    .strip_prefix("content-length:")
                                                    .and_then(|v| v.trim().parse::<usize>().ok())
                                            })
                                            .unwrap_or(0);
                                        expected_total = Some(headers_end + content_len);
                                    }
                                }
                                if let Some(total) = expected_total {
                                    if req_buf.len() >= total {
                                        break;
                                    }
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let body = if round == 0 {
                        serde_json::json!({
                            "choices": [{
                                "message": {
                                    "role": "assistant",
                                    "content": null,
                                    "tool_calls": [{
                                        "id": "call_1",
                                        "type": "function",
                                        "function": {
                                            "name": "create_work_entity",
                                            "arguments": "{\"entity_type\":\"TASK\",\"category\":\"VIDEO\",\"title\":\"تست صدای دکل در خروجی\",\"status\":\"IN_PROGRESS\",\"is_on_critical_path\":true}"
                                        }
                                    }]
                                }
                            }]
                        })
                        .to_string()
                    } else {
                        serde_json::json!({
                            "choices": [{
                                "message": {
                                    "role": "assistant",
                                    "content": "تسک جدید برای تست صدای دکل ثبت شد و در مسیر پیش‌رو قرار گرفت."
                                }
                            }]
                        })
                        .to_string()
                    };
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                }
            }
        });

        let endpoint = format!("http://{}/v1/chat/completions", addr);
        let res = run_local_openai_mcp_turn(
            &db,
            &endpoint,
            "qwen3.5-4b",
            "IRANOIL",
            None,
            "USER_CHAT",
            "system",
            "user",
        )
        .await
        .unwrap();

        let _ = server.await;

        assert_eq!(res.mcp_ops_executed, vec!["create_work_entity"]);
        assert_eq!(res.transport_label, "MCP_TOOLS");
        assert!(res.envelope.conversational_reply.contains("تسک جدید"));

        drop(db);
        let _ = fs::remove_dir_all(&dir);
    }
}
