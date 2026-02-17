use rand::RngExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, BufRead, Write};

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
    id: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                    id: None,
                };
                serde_json::to_writer(&mut stdout, &resp)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
                continue;
            }
        };

        match req.method.as_str() {
            "initialize" => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(json!({
                        "name": "dice_roll",
                        "version": "0.1.0",
                        "description": "Roll dice for RPG games",
                        "tools": [
                            {
                                "name": "roll_dice",
                                "description": "Roll some dice using NdS notation (e.g. 2d20)",
                                "parameters": {
                                    "type": "object",
                                    "properties": {
                                        "notation": {
                                            "type": "string",
                                            "description": "The dice notation (e.g. '2d6', '1d20+5')"
                                        }
                                    },
                                    "required": ["notation"]
                                }
                            }
                        ]
                    })),
                    error: None,
                    id: req.id,
                };
                serde_json::to_writer(&mut stdout, &resp)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
            "call_tool" => {
                let params = req.params.unwrap_or(json!({}));
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");

                if tool_name == "roll_dice" {
                    let notation = params
                        .get("arguments")
                        .and_then(|v| v.get("notation"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("1d6");

                    let result = {
                        let parts: Vec<&str> = notation.split('d').collect();
                        let count = parts
                            .first()
                            .and_then(|s| s.parse::<u32>().ok())
                            .unwrap_or(1);
                        let rest = parts.get(1).copied().unwrap_or("6");
                        let (sides, modifier) = if let Some(i) = rest.find('+') {
                            (
                                rest[..i].parse::<u32>().unwrap_or(6),
                                rest[i + 1..].parse::<i32>().unwrap_or(0),
                            )
                        } else if let Some(i) = rest.find('-') {
                            (
                                rest[..i].parse::<u32>().unwrap_or(6),
                                rest[i + 1..].parse::<i32>().unwrap_or(0),
                            )
                        } else {
                            (rest.parse::<u32>().unwrap_or(6), 0)
                        };
                        let mut rng = rand::rng();
                        let rolls: Vec<u32> =
                            (0..count).map(|_| rng.random_range(1..=sides)).collect();
                        let sum: i32 = rolls.iter().map(|&r| r as i32).sum::<i32>() + modifier;
                        json!({
                            "notation": notation,
                            "count": count,
                            "sides": sides,
                            "modifier": modifier,
                            "rolls": rolls,
                            "total": sum
                        })
                    };

                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: Some(result),
                        error: None,
                        id: req.id,
                    };
                    serde_json::to_writer(&mut stdout, &resp)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                } else {
                    let resp = JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32601,
                            message: format!("Method not found: {}", tool_name),
                        }),
                        id: req.id,
                    };
                    serde_json::to_writer(&mut stdout, &resp)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                }
            }
            _ => {
                let resp = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: format!("Method not found: {}", req.method),
                    }),
                    id: req.id,
                };
                serde_json::to_writer(&mut stdout, &resp)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}
