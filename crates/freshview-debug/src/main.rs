use interprocess::local_socket::{
    traits::tokio::Stream as _,
    tokio::Stream,
    ToFsName,
    GenericFilePath,
};
use serde_json::json;
use std::time::Instant;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let socket_path = ".fresh/data/fresh.sock";
    println!("Connecting to Fresh RPC at {}...", socket_path);
    
    let start = Instant::now();
    
    // Explicitly specify GenericFilePath for name generation
    let name = socket_path.to_fs_name::<GenericFilePath>()?;
    
    let stream = match Stream::connect(name).await {
        Ok(s) => s,
        Err(e) => {
            println!("Connection failed: {}. Start FreshView first!", e);
            return Ok(());
        }
    };
    
    let (_reader, mut writer) = stream.split();
    
    // JSON-RPC to open a file
    let request = json!({
        "jsonrpc": "2.0",
        "method": "editor/open",
        "params": {
            "path": "test_image.png"
        },
        "id": 1
    });
    
    let req_str = request.to_string() + "\n";
    writer.write_all(req_str.as_bytes()).await?;
    
    println!("RPC Request sent in {:?}", start.elapsed());
    println!("Successfully signaled IDE to open test_image.png");
    
    Ok(())
}
