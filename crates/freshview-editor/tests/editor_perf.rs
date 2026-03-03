use std::time::{Duration, Instant};
use freshview_editor::app::FreshEditorApp;
use eframe::egui;

#[test]
fn benchmark_editor_rendering() {
    // Create a virtual UI context
    let ctx = egui::Context::default();
    let mut ui = egui::Ui::new(
        ctx.clone(),
        egui::LayerId::background(),
        egui::Id::new("test"),
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 800.0)),
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 800.0)),
        ui_metadata(),
    );

    let mut app = FreshEditorApp::new(120, 40).expect("Failed to init editor");
    
    println!("Benchmarking Editor Rendering (Batch Optimized)...");
    
    let mut durations = Vec::new();
    for i in 0..100 {
        let start = Instant::now();
        
        // Simulate a frame
        app.show(&mut ui);
        
        durations.push(start.elapsed());
    }
    
    let avg_duration: Duration = durations.iter().sum::<Duration>() / durations.len() as u32;
    let min_duration = durations.iter().min().unwrap();
    let max_duration = durations.iter().max().unwrap();
    
    println!("Performance Report (120x40 Terminal):");
    println!("Average Frame Time: {:?}", avg_duration);
    println!("Min Frame Time:     {:?}", min_duration);
    println!("Max Frame Time:     {:?}", max_duration);
    println!("Theoretical FPS:    {:.2}", 1.0 / avg_duration.as_secs_f64());
    
    // Target: A frame should definitely be under 5ms with batching (usually < 1ms)
    assert!(avg_duration.as_millis() < 5, "Rendering is too slow! Avg: {:?}", avg_duration);
}

// Helper for egui 0.30 Ui creation
fn ui_metadata() -> egui::panel::ContextMetadata {
    // This is a bit complex in 0.30, using a dummy for internal test
    unimplemented!("Egui 0.30 internal UI testing is complex, better use integration test style")
}
