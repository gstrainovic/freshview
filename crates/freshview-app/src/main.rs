use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use eframe::egui::{self, ScrollArea, TextureHandle};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use freshview_editor::app::FreshEditorApp;
use freshview_viewer::{document::ViewerDocument, renderer::rgba_to_texture};

// --- Viewer Message System ---

enum ViewerMessage {
    RenderPage { page_idx: i32, zoom: f32 },
    DocumentOpened { total_pages: i32 },
    Rendered { rgba: Vec<u8>, width: u32, height: u32, page_idx: i32, zoom: f32 },
    Error(String),
}

/// A tab that displays PDF pages or images.
struct ViewerTab {
    title: String,
    path: PathBuf,
    texture: Option<TextureHandle>,
    current_page: i32,
    total_pages: i32,
    zoom: f32,
    is_loading: bool,
    error: Option<String>,
    
    // Communication with background worker
    message_rx: Receiver<ViewerMessage>,
    worker_tx: Sender<ViewerMessage>,
    
    last_rendered_page: i32,
    last_rendered_zoom: f32,
}

impl ViewerTab {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let (tab_tx, tab_rx) = std::sync::mpsc::channel();
        let (worker_tx, worker_rx) = std::sync::mpsc::channel();
        
        let path_clone = path.to_path_buf();
        let tab_tx_clone = tab_tx.clone();
        
        thread::spawn(move || {
            let doc = match ViewerDocument::open(&path_clone) {
                Ok(d) => {
                    let pages = d.page_count();
                    let _ = tab_tx_clone.send(ViewerMessage::DocumentOpened { total_pages: pages });
                    d
                }
                Err(e) => {
                    let _ = tab_tx_clone.send(ViewerMessage::Error(format!("Failed to open document: {e}")));
                    return;
                }
            };
            
            while let Ok(msg) = worker_rx.recv() {
                if let ViewerMessage::RenderPage { page_idx, zoom } = msg {
                    match doc.render_page(page_idx, zoom) {
                        Ok((rgba, width, height)) => {
                            let _ = tab_tx_clone.send(ViewerMessage::Rendered {
                                rgba, width, height, page_idx, zoom
                            });
                        }
                        Err(e) => {
                            let _ = tab_tx_clone.send(ViewerMessage::Error(format!("Failed to render page: {e}")));
                        }
                    }
                }
            }
        });

        Ok(Self {
            title: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Viewer".into()),
            path: path.to_path_buf(),
            texture: None,
            current_page: 0,
            total_pages: 0,
            zoom: 1.0,
            is_loading: true,
            error: None,
            message_rx: tab_rx,
            worker_tx,
            last_rendered_page: -1,
            last_rendered_zoom: -1.0,
        })
    }

    fn poll_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.message_rx.try_recv() {
            match msg {
                ViewerMessage::DocumentOpened { total_pages } => {
                    self.total_pages = total_pages;
                    self.request_render();
                }
                ViewerMessage::Rendered { rgba, width, height, page_idx, zoom } => {
                    let name = format!("{}:page{}", self.path.display(), page_idx);
                    self.texture = Some(rgba_to_texture(ctx, &name, &rgba, width, height));
                    self.last_rendered_page = page_idx;
                    self.last_rendered_zoom = zoom;
                    self.is_loading = false;
                }
                ViewerMessage::Error(e) => {
                    self.error = Some(e);
                    self.is_loading = false;
                }
                _ => {}
            }
        }
    }

    fn request_render(&mut self) {
        self.is_loading = true;
        let _ = self.worker_tx.send(ViewerMessage::RenderPage {
            page_idx: self.current_page,
            zoom: self.zoom,
        });
    }
}

// --- egui_dock Implementation ---

enum Tab {
    Editor(FreshEditorApp),
    Viewer(ViewerTab),
}

struct MyTabViewer<'a> {
    ctx: &'a egui::Context,
    added_tabs: &'a mut Vec<Tab>, // To communicate back to the app if editor opens something
}

impl TabViewer for MyTabViewer<'_> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Tab::Editor(_) => "Fresh".into(),
            Tab::Viewer(v) => v.title.as_str().into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            Tab::Editor(editor) => {
                editor.show(ui);
                // Check if editor opened new files
                for path in editor.drain_opened_image_paths() {
                    if let Ok(vt) = ViewerTab::open(&path) {
                        self.added_tabs.push(Tab::Viewer(vt));
                    }
                }
            }
            Tab::Viewer(v) => {
                v.poll_messages(self.ctx);
                
                if let Some(err) = &v.error {
                    ui.colored_label(egui::Color32::RED, err);
                } else {
                    // Toolbar
                    ui.horizontal(|ui| {
                        if v.total_pages > 1 {
                            if ui.button("<<").clicked() && v.current_page > 0 {
                                v.current_page -= 1;
                                v.request_render();
                            }
                            ui.label(format!("{} / {}", v.current_page + 1, v.total_pages));
                            if ui.button(">>").clicked() && v.current_page < v.total_pages - 1 {
                                v.current_page += 1;
                                v.request_render();
                            }
                        }
                        if ui.button("-").clicked() && v.zoom > 0.25 {
                            v.zoom -= 0.25;
                            v.request_render();
                        }
                        ui.label(format!("{}%", (v.zoom * 100.0) as i32));
                        if ui.button("+").clicked() && v.zoom < 4.0 {
                            v.zoom += 0.25;
                            v.request_render();
                        }
                        if v.is_loading { ui.spinner(); }
                    });
                    
                    ui.separator();

                    ScrollArea::both().show(ui, |ui| {
                        if let Some(tex) = &v.texture {
                            ui.image(egui::load::SizedTexture::from(tex));
                        }
                    });
                }
            }
        }
    }

    fn on_close(&mut self, tab: &mut Self::Tab) -> bool {
        // Prevent closing the editor
        !matches!(tab, Tab::Editor(_))
    }
}

// --- Main Application ---

mod rpc_server;

use sysinfo::{System, Pid};
use std::sync::{Arc, Mutex};

pub enum AppCommand {
    SetZoom(f32),
}

struct FreshViewApp {
    dock_state: DockState<Tab>,
    shared_metrics: Arc<Mutex<HardwareMetrics>>,
    command_rx: std::sync::mpsc::Receiver<AppCommand>,
    
    // Honest frame timing
    last_frame_instant: std::time::Instant,
    frame_time_ms: f32,
    zoom_factor: f32,
}

#[derive(Default, Clone)]
pub struct HardwareMetrics {
    pub cpu_usage: f32,
    pub memory_mb: u64,
    pub gpu_usage: u32,
    pub vram_mb: u64,
}

impl FreshViewApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let editor = FreshEditorApp::new(120, 40).expect("Failed to init editor");
        let dock_state = DockState::new(vec![Tab::Editor(editor)]);
        let metrics = Arc::new(Mutex::new(HardwareMetrics::default()));
        let (tx, rx) = std::sync::mpsc::channel();
        
        // Start RPC Server
        rpc_server::start_server(metrics.clone(), tx);
        
        // --- Creative Background Monitor ---
        let metrics_clone = Arc::clone(&metrics);
        std::thread::spawn(move || {
            let mut sys = System::new_all();
            let pid = Pid::from(std::process::id() as usize);
            
            loop {
                sys.refresh_all();
                let mut new_metrics = HardwareMetrics::default();
                
                if let Some(process) = sys.process(pid) {
                    new_metrics.cpu_usage = process.cpu_usage();
                    new_metrics.memory_mb = process.memory() / 1024 / 1024;
                }

                // Linux GPU/VRAM fallback check (NVIDIA)
                let gpu_info = std::process::Command::new("nvidia-smi")
                    .args(["--query-gpu=utilization.gpu,memory.used", "--format=csv,noheader,nounits"])
                    .output();

                if let Ok(output) = gpu_info {
                    let s = String::from_utf8_lossy(&output.stdout);
                    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
                    if parts.len() >= 2 {
                        new_metrics.gpu_usage = parts[0].parse().unwrap_or(0);
                        new_metrics.vram_mb = parts[1].parse().unwrap_or(0);
                    }
                }

                {
                    let mut m = metrics_clone.lock().unwrap();
                    *m = new_metrics.clone();
                }

                let log_line = format!(
                    "STATS | CPU: {:>5.1}% | RAM: {:>7} MB | GPU: {:>3}% | VRAM: {:>7} MB\n",
                    new_metrics.cpu_usage,
                    new_metrics.memory_mb,
                    new_metrics.gpu_usage,
                    new_metrics.vram_mb
                );
                print!("{}", log_line); // Console
                let _ = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open("freshview.log")
                    .and_then(|mut f| {
                        use std::io::Write;
                        f.write_all(log_line.as_bytes())
                    });
                
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        Self { 
            dock_state, 
            shared_metrics: metrics,
            command_rx: rx,
            last_frame_instant: std::time::Instant::now(),
            frame_time_ms: 0.0,
            zoom_factor: 1.0,
        }
    }
}

impl eframe::App for FreshViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle remote commands
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                AppCommand::SetZoom(z) => {
                    self.zoom_factor = z;
                    ctx.set_pixels_per_point(z);
                }
            }
        }

        // Calculate honest frame time
        let now = std::time::Instant::now();
        self.frame_time_ms = now.duration_since(self.last_frame_instant).as_secs_f32() * 1000.0;
        self.last_frame_instant = now;

        let mut added_tabs = Vec::new();
        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut MyTabViewer { ctx, added_tabs: &mut added_tabs });

        for new_tab in added_tabs {
            self.dock_state.main_surface_mut().push_to_focused_leaf(new_tab);
        }

        // --- Honest Performance HUD ---
        let metrics = self.shared_metrics.lock().unwrap().clone();
        
        egui::Window::new("Perf")
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_black_alpha(180)))
            .show(ctx, |ui| {
                ui.small(format!("Frame: {:.1} ms", self.frame_time_ms));
                ui.small(format!("CPU:   {:.1}%", metrics.cpu_usage));
                ui.small(format!("GPU:   {}%", metrics.gpu_usage));
                ui.small(format!("RAM:   {} MB", metrics.memory_mb));
                ui.small(format!("VRAM:  {} MB", metrics.vram_mb));
            });
    }
}

fn main() -> eframe::Result {
    let _log_file = std::fs::File::create("freshview.log").expect("failed to create log file");
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "FreshView IDE",
        options,
        Box::new(|cc| {
            // Confirm which GPU we are using
            if let Some(render_state) = &cc.wgpu_render_state {
                let info = render_state.adapter.get_info();
                let log_msg = format!(
                    "\n--- GRAPHICS ADAPTER INFO ---\nSelected: {}\nBackend:  {:?}\nDriver:   {}\nType:     {:?}\n-----------------------------\n",
                    info.name, info.backend, info.driver_info, info.device_type
                );
                println!("{}", log_msg);
                let _ = std::fs::OpenOptions::new().append(true).open("freshview.log").and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(log_msg.as_bytes())
                });
            }
            Ok(Box::new(FreshViewApp::new(cc)))
        }),
    )
}
