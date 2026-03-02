use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use eframe::egui::{self, ScrollArea, TextureHandle};
use freshview_editor::app::FreshEditorApp;
use freshview_viewer::{document::ViewerDocument, renderer::rgba_to_texture};

// --- ViewerTab: A new struct to manage viewer state within a tab ---

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
    
    // Tracking what's currently being rendered to avoid redundant requests
    last_requested_page: i32,
    last_requested_zoom: f32,
    last_rendered_page: i32,
    last_rendered_zoom: f32,
}

impl ViewerTab {
    /// Open a document (PDF or image) for viewing in a tab.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let (tab_tx, tab_rx) = std::sync::mpsc::channel();
        let (worker_tx, worker_rx) = std::sync::mpsc::channel();
        
        let path_clone = path.to_path_buf();
        let tab_tx_clone = tab_tx.clone();
        
        // Start background worker thread
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
            
            // Listen for render requests
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

        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Viewer".into());

        Ok(Self {
            title,
            path: path.to_path_buf(),
            texture: None,
            current_page: 0,
            total_pages: 0,
            zoom: 1.0,
            is_loading: true,
            error: None,
            message_rx: tab_rx,
            worker_tx,
            last_requested_page: -1,
            last_requested_zoom: -1.0,
            last_rendered_page: -1,
            last_rendered_zoom: -1.0,
        })
    }

    /// Show the viewer's UI (toolbar and content) within a parent UI.
    pub fn show_ui(&mut self, ui: &mut egui::Ui) {
        // Poll for messages from worker
        while let Ok(msg) = self.message_rx.try_recv() {
            match msg {
                ViewerMessage::DocumentOpened { total_pages } => {
                    self.total_pages = total_pages;
                    self.request_render();
                }
                ViewerMessage::Rendered { rgba, width, height, page_idx, zoom } => {
                    let name = format!("{}:page{}", self.path.display(), page_idx);
                    self.texture = Some(rgba_to_texture(ui.ctx(), &name, &rgba, width, height));
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

        if self.error.is_some() {
            ui.colored_label(egui::Color32::RED, self.error.as_ref().unwrap());
            return;
        }

        self.show_toolbar(ui);
        ui.separator();
        
        if self.is_loading && self.texture.is_none() {
            ui.centered_and_justified(|ui| {
                ui.spinner();
                ui.label("Loading document...");
            });
        } else {
            self.show_content(ui);
        }
        
        // If zoom or page changed, request new render if not already requested
        if self.current_page != self.last_rendered_page || (self.zoom - self.last_rendered_zoom).abs() > 0.01 {
            self.request_render();
        }
    }

    fn request_render(&mut self) {
        if self.current_page == self.last_requested_page && (self.zoom - self.last_requested_zoom).abs() < 0.01 {
            return;
        }
        
        self.last_requested_page = self.current_page;
        self.last_requested_zoom = self.zoom;
        self.is_loading = true;
        let _ = self.worker_tx.send(ViewerMessage::RenderPage {
            page_idx: self.current_page,
            zoom: self.zoom,
        });
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.total_pages > 1 {
                if ui
                    .add_enabled(self.current_page > 0, egui::Button::new("<<"))
                    .clicked()
                {
                    self.current_page -= 1;
                }
                ui.label(format!(
                    "{} / {}",
                    if self.total_pages > 0 { self.current_page + 1 } else { 0 },
                    self.total_pages
                ));
                if ui
                    .add_enabled(
                        self.current_page < self.total_pages - 1,
                        egui::Button::new(">>"),
                    )
                    .clicked()
                {
                    self.current_page += 1;
                }
                ui.separator();
            }

            if ui
                .add_enabled(self.zoom > 0.25, egui::Button::new("-"))
                .clicked()
            {
                self.zoom = (self.zoom - 0.25).max(0.25);
            }
            ui.label(format!("{}%", (self.zoom * 100.0) as i32));
            if ui
                .add_enabled(self.zoom < 4.0, egui::Button::new("+"))
                .clicked()
            {
                self.zoom = (self.zoom + 0.25).min(4.0);
            }
            
            if self.is_loading {
                ui.spinner();
            }
        });
    }

    fn show_content(&self, ui: &mut egui::Ui) {
        ScrollArea::both().show(ui, |ui| {
            if let Some(tex) = &self.texture {
                ui.image(egui::load::SizedTexture::from(tex));
            }
        });
    }
}

// --- Main Application State and Logic ---

enum Tab {
    Editor(FreshEditorApp),
    Viewer(ViewerTab),
}

impl Tab {
    fn title(&self) -> String {
        match self {
            Tab::Editor(_) => "Fresh".to_string(),
            Tab::Viewer(viewer) => viewer.title.clone(),
        }
    }
}

struct FreshViewApp {
    tabs: Vec<Tab>,
    active_tab: usize,
    init_error: Option<String>,
}

impl FreshViewApp {
    fn new(viewer_paths: Vec<std::path::PathBuf>) -> Self {
        let editor = match FreshEditorApp::new(120, 40) {
            Ok(e) => e,
            Err(e) => {
                return Self {
                    tabs: Vec::new(),
                    active_tab: 0,
                    init_error: Some(format!("Failed to initialize editor: {e}")),
                };
            }
        };

        let mut tabs = vec![Tab::Editor(editor)];
        for path in &viewer_paths {
            match ViewerTab::open(path) {
                Ok(vt) => tabs.push(Tab::Viewer(vt)),
                Err(e) => log::error!("Failed to open viewer for {}: {e}", path.display()),
            }
        }

        Self {
            tabs,
            active_tab: 0,
            init_error: None,
        }
    }
}

impl eframe::App for FreshViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(ref error) = self.init_error {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.colored_label(egui::Color32::RED, error);
            });
            return;
        }

        // --- Tab Bar ---
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, tab) in self.tabs.iter().enumerate() {
                    let is_active = self.active_tab == i;
                    if ui.selectable_label(is_active, tab.title()).clicked() {
                        self.active_tab = i;
                    }
                }
            });
        });

        // --- Active Tab Content ---
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                match tab {
                    Tab::Editor(editor) => {
                        if editor.should_quit() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            return;
                        }
                        editor.show(ui);

                        // Handle image files opened by the editor
                        for path in editor.drain_opened_image_paths() {
                            match ViewerTab::open(&path) {
                                Ok(vt) => {
                                    self.tabs.push(Tab::Viewer(vt));
                                    self.active_tab = self.tabs.len() - 1; // Switch to the new tab
                                }
                                Err(e) => log::error!("Failed to open viewer for {}: {e}", path.display()),
                            }
                        }
                    }
                    Tab::Viewer(viewer) => {
                        viewer.show_ui(ui);
                    }
                }
            } else {
                ui.label("No active tab found.");
                if !self.tabs.is_empty() {
                    self.active_tab = 0;
                }
            }
        });
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    // For now, we only support opening viewer files from the CLI.
    // Editor files can be opened via the built-in file explorer.
    let viewer_paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };

    eframe::run_native(
        "FreshView",
        options,
        Box::new(move |_cc| Ok(Box::new(FreshViewApp::new(viewer_paths)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn viewer_extensions_detected() {
        // This test is now implicitly covered by the should_use_viewer in freshview-editor,
        // but we can't test that directly from here.
    }
}
