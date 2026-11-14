use mab_compass_lib::db::schema::init_database;
use mab_compass_lib::mcp_tools::handle_jsonrpc_request;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

fn resolve_db_and_vault_paths(args: &[String]) -> (PathBuf, PathBuf) {
    let mut cli_db: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--db-path" && i + 1 < args.len() {
            cli_db = Some(PathBuf::from(&args[i + 1]));
            i += 2;
            continue;
        }
        i += 1;
    }

    let app_data_dir = dirs::data_dir()
        .or_else(dirs::config_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.mabos.compass");

    let db_path = cli_db
        .or_else(|| std::env::var_os("MAB_COMPASS_DB").map(PathBuf::from))
        .unwrap_or_else(|| app_data_dir.join("mab_compass_v2.sqlite"));

    if let Some(parent) = db_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let default_vault = dirs::document_dir()
        .unwrap_or_else(|| app_data_dir.clone())
        .join("MabOS_Projects_Vault");
    let _ = fs::create_dir_all(&default_vault);

    (db_path, default_vault)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (db_path, default_vault) = resolve_db_and_vault_paths(&args);

    let mut conn = match init_database(&db_path, &default_vault) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[mab-compass-mcp] Failed to init database at {:?}: {}", db_path, e);
            std::process::exit(1);
        }
    };

    eprintln!(
        "[mab-compass-mcp] Ready on stdio (db={})",
        db_path.to_string_lossy()
    );

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line_res in stdin.lock().lines() {
        let Ok(line) = line_res else {
            break;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {}", e) }
                });
                let _ = writeln!(out, "{}", err_resp);
                let _ = out.flush();
                continue;
            }
        };

        if let Some(resp) = handle_jsonrpc_request(&mut conn, &parsed) {
            let _ = writeln!(out, "{}", resp);
            let _ = out.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_protocol_and_tools_end_to_end() {
        let dir = std::env::temp_dir().join(format!("mab_compass_mcp_{}", uuid::Uuid::new_v4()));
        let vault = dir.join("vault");
        fs::create_dir_all(&vault).unwrap();
        let mut conn = init_database(&dir.join("test.sqlite"), &vault).unwrap();

        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2025-03-26" }
        });
        let init_resp = handle_jsonrpc_request(&mut conn, &init_req).unwrap();
        assert_eq!(init_resp["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(init_resp["result"]["serverInfo"]["name"], "mab-compass-mcp");

        assert!(handle_jsonrpc_request(
            &mut conn,
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})
        )
        .is_none());

        let list_resp =
            handle_jsonrpc_request(&mut conn, &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}))
                .unwrap();
        let tools = list_resp["result"]["tools"].as_array().unwrap();
        assert!(tools.iter().any(|t| t["name"] == "get_path_ahead"));
        assert!(tools.iter().any(|t| t["name"] == "create_work_entity"));

        let create_resp = handle_jsonrpc_request(
            &mut conn,
            &json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": {
                    "name": "create_work_entity",
                    "arguments": {
                        "entity_type": "TASK",
                        "category": "BACKEND",
                        "title": "ساخت سرویس MCP",
                        "definition_of_done": "تمام ابزارها تست شوند",
                        "first_physical_action": "باز کردن mab_compass_mcp.rs",
                        "status": "IN_PROGRESS",
                        "is_on_critical_path": true
                    }
                }
            }),
        )
        .unwrap();
        assert_eq!(create_resp["result"]["isError"], false);
        let text_payload = create_resp["result"]["content"][0]["text"].as_str().unwrap();
        let payload_val: Value = serde_json::from_str(text_payload).unwrap();
        let task_id = payload_val["next_step_task"]["id"].as_str().unwrap().to_string();

        let res_resp = handle_jsonrpc_request(
            &mut conn,
            &json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "resources/read",
                "params": { "uri": "compass://active/path-ahead" }
            }),
        )
        .unwrap();
        let res_text = res_resp["result"]["contents"][0]["text"].as_str().unwrap();
        assert!(res_text.contains("ساخت سرویس MCP"));

        let comp_resp = handle_jsonrpc_request(
            &mut conn,
            &json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": {
                    "name": "complete_task",
                    "arguments": { "task_id": task_id, "pomos_spent": 2 }
                }
            }),
        )
        .unwrap();
        assert_eq!(comp_resp["result"]["isError"], false);

        let undo_resp = handle_jsonrpc_request(
            &mut conn,
            &json!({
                "jsonrpc": "2.0",
                "id": 6,
                "method": "tools/call",
                "params": { "name": "undo_last_change", "arguments": {} }
            }),
        )
        .unwrap();
        assert_eq!(undo_resp["result"]["isError"], false);

        drop(conn);
        let _ = fs::remove_dir_all(&dir);
    }
}
