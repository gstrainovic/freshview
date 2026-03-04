use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use serde::{Deserialize, Serialize};
use crate::{HardwareMetrics, AppCommand};

#[derive(Serialize, Deserialize, Debug)]
struct RpcRequest {
    method: String,
    params: Option<serde_json::Value>,
    id: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RpcResponse {
    result: Option<serde_json::Value>,
    error: Option<String>,
    id: Option<u64>,
}

pub fn start_server(
    metrics: Arc<Mutex<HardwareMetrics>>,
    command_tx: std::sync::mpsc::Sender<AppCommand>
) {
    thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:9000") {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind RPC server: {}", e);
                return;
            }
        };
        log::info!("RPC Server listening on 127.0.0.1:9000");

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let metrics = metrics.clone();
                    let tx = command_tx.clone();
                    
                    thread::spawn(move || {
                        let mut reader = BufReader::new(stream.try_clone().unwrap());
                        let mut line = String::new();
                        
                        while reader.read_line(&mut line).unwrap_or(0) > 0 {
                            let req: Result<RpcRequest, _> = serde_json::from_str(line.trim());
                            
                            let resp = match req {
                                Ok(req) => {
                                    handle_request(req, &metrics, &tx)
                                }
                                Err(e) => RpcResponse {
                                    result: None,
                                    error: Some(format!("Invalid JSON: {}", e)),
                                    id: None,
                                }
                            };
                            
                            let resp_json = serde_json::to_string(&resp).unwrap();
                            let _ = stream.write_all(resp_json.as_bytes());
                            let _ = stream.write_all(b"
");
                            
                            line.clear();
                        }
                    });
                }
                Err(e) => log::warn!("RPC connection failed: {}", e),
            }
        }
    });
}

fn handle_request(
    req: RpcRequest,
    metrics: &Mutex<HardwareMetrics>,
    tx: &std::sync::mpsc::Sender<AppCommand>
) -> RpcResponse {
    match req.method.as_str() {
        "get_stats" => {
            let m = metrics.lock().unwrap();
            RpcResponse {
                result: Some(serde_json::json!({
                    "cpu": m.cpu_usage,
                    "ram_mb": m.memory_mb,
                    "gpu": m.gpu_usage,
                    "vram_mb": m.vram_mb
                })),
                error: None,
                id: req.id,
            }
        },
        "set_zoom" => {
            if let Some(val) = req.params.and_then(|p| p.as_f64()) {
                let _ = tx.send(AppCommand::SetZoom(val as f32));
                RpcResponse { result: Some(serde_json::json!("OK")), error: None, id: req.id }
            } else {
                RpcResponse { result: None, error: Some("Missing numeric param".into()), id: req.id }
            }
        },
        _ => RpcResponse {
            result: None,
            error: Some("Unknown method".into()),
            id: req.id,
        }
    }
}
