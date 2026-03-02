use std::path::Path;
use std::time::{Duration, Instant};
use sysinfo::{System, Pid};
use freshview_viewer::document::ViewerDocument;

#[test]
fn test_load_performance_cpu_usage() {
    let home = std::env::var("HOME").expect("HOME env var not set");
    let test_dir = Path::new(&home).join("projects/auto-service/tmp");
    
    // Find the first PDF file in the directory
    let test_file = std::fs::read_dir(&test_dir)
        .expect("Failed to read test directory")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|s| s.to_str()) == Some("pdf"))
        .expect("No PDF files found in ~/projects/auto-service/tmp/");

    println!("Using test file: {}", test_file.display());

    let mut sys = System::new_all();
    let pid = Pid::from(std::process::id() as usize);

    // Initial refresh
    sys.refresh_all();
    thread_sleep(200); // Wait for a baseline

    let start_time = Instant::now();
    let mut cpu_samples = Vec::new();

    // Perform loading in a loop to get a sustained measurement
    for _ in 0..10 {
        // We don't care if it fails (not a real PDF), we just want to measure load
        let _ = ViewerDocument::open(&test_file);
        
        sys.refresh_all();
        if let Some(process) = sys.process(pid) {
            cpu_samples.push(process.cpu_usage());
        }
        thread_sleep(500);
    }

    let duration = start_time.elapsed();
    let avg_cpu = if cpu_samples.is_empty() {
        0.0
    } else {
        // Average the samples
        cpu_samples.iter().sum::<f32>() / cpu_samples.len() as f32
    };

    println!("Load Performance Report:");
    println!("File: {}", test_file.display());
    println!("Duration: {:?}", duration);
    println!("Process CPU Usage (avg core-equivalent): {:.2}%", avg_cpu);
    println!("Samples: {:?}", cpu_samples);

    // Assert that the process doesn't hog more than 40% of a core on average during this task
    assert!(avg_cpu <= 40.0, "Process CPU usage too high: {:.2}% > 40% (target: efficient background loading)", avg_cpu);
}

fn thread_sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}
