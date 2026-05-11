use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::cli::{self, MetadataPatch};
use crate::PRODUCT_VERSION;

const PROTOCOL_VERSION: &str = "2024-11-05";

pub(crate) async fn run_stdio() -> Result<()> {
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = io::stdout();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(messages)) => {
                let mut responses = Vec::new();
                for message in messages {
                    if let Some(response) = handle_message(message).await {
                        responses.push(response);
                    }
                }
                if responses.is_empty() {
                    None
                } else {
                    Some(Value::Array(responses))
                }
            }
            Ok(message) => handle_message(message).await,
            Err(err) => Some(error(Value::Null, -32700, &format!("parse error: {err}"))),
        };

        if let Some(response) = response {
            stdout
                .write_all(format!("{}\n", serde_json::to_string(&response)?).as_bytes())
                .await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}

async fn handle_message(message: Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str)?;
    let params = message.get("params").cloned().unwrap_or_else(|| json!({}));

    let id = id?;

    let result = match method {
        "initialize" => Ok(initialize_result(&params)),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => tools_call_result(&params).await,
        "resources/list" => Ok(resources_list_result()),
        "resources/read" => resources_read_result(&params).await,
        _ => Err(anyhow!("method not found: {method}")),
    };

    Some(match result {
        Ok(result) => response(id, result),
        Err(err) if err.to_string().starts_with("method not found:") => {
            error(id, -32601, &err.to_string())
        }
        Err(err) => error(id, -32603, &err.to_string()),
    })
}

fn initialize_result(params: &Value) -> Value {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(PROTOCOL_VERSION);

    json!({
        "protocolVersion": protocol_version,
        "capabilities": {
            "tools": {},
            "resources": {}
        },
        "serverInfo": {
            "name": "hostel",
            "version": PRODUCT_VERSION
        }
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            {
                "name": "list_services",
                "description": "List live localhost services known to Hostel.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            },
            {
                "name": "set_service_metadata",
                "description": "Attach a title, memo, tags, URL path, scheme, or source to a live localhost service.",
                "inputSchema": metadata_schema()
            },
            {
                "name": "clear_service_metadata",
                "description": "Clear Hostel metadata for a live localhost service.",
                "inputSchema": port_schema()
            },
            {
                "name": "open_service",
                "description": "Open a live localhost service using its Hostel URL metadata.",
                "inputSchema": port_schema()
            }
        ]
    })
}

async fn tools_call_result(params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("tools/call requires name"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match name {
        "list_services" => {
            let services = cli::list_service_views().await?;
            text_result(serde_json::to_value(services)?)
        }
        "set_service_metadata" => {
            let service = cli::set_service_metadata(metadata_patch_from_value(&arguments)?).await?;
            text_result(serde_json::to_value(service)?)
        }
        "clear_service_metadata" => {
            let (port, pid) = port_pid_from_value(&arguments)?;
            let service = cli::clear_service_metadata(port, pid).await?;
            text_result(serde_json::to_value(service)?)
        }
        "open_service" => {
            let (port, pid) = port_pid_from_value(&arguments)?;
            let service = cli::open_service(port, pid).await?;
            text_result(serde_json::to_value(service)?)
        }
        _ => Err(anyhow!("unknown tool: {name}")),
    }
}

fn resources_list_result() -> Value {
    json!({
        "resources": [
            {
                "uri": "hostel://services",
                "name": "Hostel live localhost services",
                "description": "Live localhost services with Hostel metadata.",
                "mimeType": "application/json"
            }
        ]
    })
}

async fn resources_read_result(params: &Value) -> Result<Value> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("resources/read requires uri"))?;

    if uri != "hostel://services" {
        return Err(anyhow!("unknown resource: {uri}"));
    }

    let services = cli::list_service_views().await?;
    Ok(json!({
        "contents": [
            {
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&services)?
            }
        ]
    }))
}

fn metadata_patch_from_value(value: &Value) -> Result<MetadataPatch> {
    let (port, pid) = port_pid_from_value(value)?;
    let mut patch = MetadataPatch {
        port,
        pid,
        ..MetadataPatch::default()
    };

    if let Some(title) = optional_string(value, "title")? {
        patch.title = Some(title);
    }
    if let Some(memo) = optional_string(value, "memo")? {
        patch.memo = Some(memo);
    }
    if let Some(url_path) = optional_string(value, "url_path")?.or(optional_string(value, "path")?)
    {
        patch.url_path = Some(url_path);
    }
    if let Some(scheme) = optional_string(value, "scheme")? {
        patch.scheme = Some(scheme);
    }
    if let Some(source) = optional_string(value, "source")? {
        patch.source = Some(source);
    }
    if let Some(tags) = value.get("tags") {
        patch.tags = Some(match tags {
            Value::Array(values) => values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_string)
                        .ok_or_else(|| anyhow!("tags must be strings"))
                })
                .collect::<Result<Vec<_>>>()?,
            Value::String(value) => vec![value.clone()],
            _ => return Err(anyhow!("tags must be a string or string array")),
        });
    }

    Ok(patch)
}

fn port_pid_from_value(value: &Value) -> Result<(u16, Option<u32>)> {
    let port = value
        .get("port")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("port is required"))?;
    let port = u16::try_from(port).map_err(|_| anyhow!("port is out of range"))?;
    let pid = value
        .get("pid")
        .and_then(Value::as_u64)
        .map(u32::try_from)
        .transpose()
        .map_err(|_| anyhow!("pid is out of range"))?;
    Ok((port, pid))
}

fn optional_string(value: &Value, key: &str) -> Result<Option<String>> {
    value
        .get(key)
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| anyhow!("{key} must be a string"))
        })
        .transpose()
}

fn metadata_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "port": { "type": "integer", "minimum": 1024, "maximum": 9999 },
            "pid": { "type": "integer", "minimum": 2 },
            "title": { "type": "string" },
            "memo": { "type": "string" },
            "tags": {
                "oneOf": [
                    { "type": "array", "items": { "type": "string" } },
                    { "type": "string" }
                ]
            },
            "url_path": { "type": "string" },
            "path": { "type": "string" },
            "scheme": { "type": "string", "enum": ["http", "https"] },
            "source": { "type": "string" }
        },
        "required": ["port"],
        "additionalProperties": false
    })
}

fn port_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "port": { "type": "integer", "minimum": 1024, "maximum": 9999 },
            "pid": { "type": "integer", "minimum": 2 }
        },
        "required": ["port"],
        "additionalProperties": false
    })
}

fn text_result(value: Value) -> Result<Value> {
    Ok(json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&value)?
            }
        ]
    }))
}

fn response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}
