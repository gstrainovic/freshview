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

use sysinfo::{System, Pid};

struct FreshViewApp {
    dock_state: DockState<Tab>,
    sys: System,
    pid: Pid,
    last_hw_update: std::time::Instant,
    metrics: HardwareMetrics,
    
    // FPS tracking
    frame_count: u32,
    last_fps_time: std::time::Instant,
    actual_fps: u32,
}

#[derive(Default, Clone)]
struct HardwareMetrics {
    cpu_usage: f32,
    memory_kb: u64,
    gpu_usage: Option<u32>,
    vram_kb: Option<u64>,
}

impl FreshViewApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let editor = FreshEditorApp::new(120, 40).expect("Failed to init editor");
        let dock_state = DockState::new(vec![Tab::Editor(editor)]);
        let mut sys = System::new_all();
        sys.refresh_all();
        let pid = Pid::from(std::process::id() as usize);
        
        Self { 
            dock_state, 
            sys, 
            pid,
            last_hw_update: std::time::Instant::now(),
            metrics: HardwareMetrics::default(),
            frame_count: 0,
            last_fps_time: std::time::Instant::now(),
            actual_fps: 0,
        }
    }

    fn update_hardware_metrics(&mut self) {
        // Update FPS every second
        self.frame_count += 1;
        if self.last_fps_time.elapsed().as_secs() >= 1 {
            self.actual_fps = self.frame_count;
            self.frame_count = 0;
            self.last_fps_time = std::time::Instant::now();
        }

        // Update Hardware stats every second
        if self.last_hw_update.elapsed().as_secs() >= 1 {
            self.sys.refresh_all();
            if let Some(process) = self.sys.process(self.pid) {
                self.metrics.cpu_usage = process.cpu_usage();
                self.metrics.memory_kb = process.memory();
                
                // 1. Try AMD/Intel (DRM)
                self.metrics.gpu_usage = std::fs::read_to_string("/sys/class/drm/card0/device/gpu_busy_percent")
                    .ok().and_then(|s| s.trim().parse().ok());
                
                self.metrics.vram_kb = std::fs::read_to_string("/sys/class/drm/card0/device/mem_info_vram_used")
                    .ok().and_then(|s| s.trim().parse::<u64>().ok().map(|v| v / 1024));

                // 2. Fallback to NVIDIA (nvidia-smi)
                if self.metrics.gpu_usage.is_none() {
                    use std::process::Command;
                    if let Ok(output) = Command::new("nvidia-smi")
                        .args(["--query-gpu=utilization.gpu,memory.used", "--format=csv,noheader,nounits"])
                        .output() 
                    {
                        let s = String::from_utf8_lossy(&output.stdout);
                        let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
                        if parts.len() >= 2 {
                            self.metrics.gpu_usage = parts[0].parse().ok();
                            self.metrics.vram_kb = parts[1].parse::<u64>().ok().map(|v| v * 1024); // MiB to KB
                        }
                    }
                }

                log::info!(
                    "STATS | FPS: {:>3} | CPU: {:>5.1}% | RAM: {:>7} MB | GPU: {:>3}% | VRAM: {:>7} MB",
                    self.actual_fps,
                    self.metrics.cpu_usage,
                    self.metrics.memory_kb / 1024,
                    self.metrics.gpu_usage.unwrap_or(0),
                    self.metrics.vram_kb.unwrap_or(0) / 1024
                );
            }
            self.last_hw_update = std::time::Instant::now();
        }
    }
}

impl eframe::App for FreshViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_hardware_metrics();
        let mut added_tabs = Vec::new();
        
        DockArea::new(&mut self.dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut MyTabViewer { ctx, added_tabs: &mut added_tabs });

        for new_tab in added_tabs {
            self.dock_state.main_surface_mut().push_to_focused_leaf(new_tab);
        }

        // --- Performance HUD ---
        egui::Window::new("🚀 Performance HUD")
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .show(ctx, |ui| {
                ui.label(format!("FPS: {}", self.actual_fps));
                ui.label(format!("CPU: {:.1}%", self.metrics.cpu_usage));
                ui.label(format!("RAM: {} MB", self.metrics.memory_kb / 1024));
                
                if let Some(gpu) = self.metrics.gpu_usage {
                    ui.label(format!("GPU: {}%", gpu));
                } else {
                    ui.label("GPU: N/A");
                }
                
                if let Some(vram) = self.metrics.vram_kb {
                    ui.label(format!("VRAM: {} MB", vram / 1024));
                }
            });
    }
}

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "FreshView IDE",
        options,
        Box::new(|cc| Ok(Box::new(FreshViewApp::new(cc)))),
    )
}
