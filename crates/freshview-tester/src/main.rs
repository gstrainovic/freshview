use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use serde_json::json;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    println!("Connecting to FreshView RPC (127.0.0.1:9000)...");
    let mut stream = match TcpStream::connect("127.0.0.1:9000") {
        Ok(s) => s,
        Err(_) => {
            println!("FreshView is not running. Please start it first.");
            return Ok(());
        }
    };
    
    println!("--- Starting Automated Performance Test ---");
    
    // 1. Get baseline stats
    let stats = call_rpc(&mut stream, "get_stats", None)?;
    println!("Baseline: CPU: {}%, RAM: {}MB, GPU: {}%", 
        stats["cpu"], stats["ram_mb"], stats["gpu"]);

    // 2. Stress Test: Zoom In/Out 10 times
    println!("Testing Zoom Performance (10 iterations)...");
    for i in 0..10 {
        let zoom = 1.0 + (i as f32 * 0.2);
        let _ = call_rpc(&mut stream, "set_zoom", Some(json!(zoom)))?;
        std::thread::sleep(Duration::from_millis(100));
    }
    
    let stats_zooming = call_rpc(&mut stream, "get_stats", None)?;
    println!("Zooming Stats: CPU: {}%, GPU: {}%", stats_zooming["cpu"], stats_zooming["gpu"]);

    // 3. Reset Zoom
    let _ = call_rpc(&mut stream, "set_zoom", Some(json!(1.0)))?;
    println!("Test Complete.");
    
    Ok(())
}

fn call_rpc(stream: &mut TcpStream, method: &str, params: Option<serde_json::Value>) -> anyhow::Result<serde_json::Value> {
    let req = json!({
        "method": method,
        "params": params,
        "id": 1
    });
    
    let req_str = req.to_string() + "
";
    stream.write_all(req_str.as_bytes())?;
    
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    
    let resp: serde_json::Value = serde_json::from_str(&line)?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("RPC Error: {}", err);
    }
    
    Ok(resp["result"].clone())
}
