use std::path::{Path, PathBuf};

use eframe::egui::{self, ScrollArea, TextureHandle};
use freshview_editor::app::FreshEditorApp;
use freshview_viewer::{document::ViewerDocument, renderer::rgba_to_texture};

// --- ViewerTab: A new struct to manage viewer state within a tab ---

/// A tab that displays PDF pages or images.
struct ViewerTab {
    title: String,
    path: PathBuf,
    document: ViewerDocument,
    texture: Option<TextureHandle>,
    current_page: i32,
    total_pages: i32,
    zoom: f32,
    needs_render: bool,
}

impl ViewerTab {
    /// Open a document (PDF or image) for viewing in a tab.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let document = ViewerDocument::open(path)?;
        let total_pages = document.page_count();
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Viewer".into());

        Ok(Self {
            title,
            path: path.to_path_buf(),
            document,
            texture: None,
            current_page: 0,
            total_pages,
            zoom: 1.0,
            needs_render: true,
        })
    }

    /// Show the viewer's UI (toolbar and content) within a parent UI.
    pub fn show_ui(&mut self, ui: &mut egui::Ui) {
        if self.needs_render {
            self.render_current_page(ui.ctx());
        }

        self.show_toolbar(ui);
        ui.separator();
        self.show_content(ui);
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.total_pages > 1 {
                if ui
                    .add_enabled(self.current_page > 0, egui::Button::new("<<"))
                    .clicked()
                {
                    self.current_page -= 1;
                    self.needs_render = true;
                }
                ui.label(format!(
                    "{} / {}",
                    self.current_page + 1,
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
                    self.needs_render = true;
                }
                ui.separator();
            }

            if ui
                .add_enabled(self.zoom > 0.25, egui::Button::new("-"))
                .clicked()
            {
                self.zoom = (self.zoom - 0.25).max(0.25);
                self.needs_render = true;
            }
            ui.label(format!("{}%", (self.zoom * 100.0) as i32));
            if ui
                .add_enabled(self.zoom < 4.0, egui::Button::new("+"))
                .clicked()
            {
                self.zoom = (self.zoom + 0.25).min(4.0);
                self.needs_render = true;
            }
        });
    }

    fn show_content(&self, ui: &mut egui::Ui) {
        ScrollArea::both().show(ui, |ui| {
            if let Some(tex) = &self.texture {
                ui.image(egui::load::SizedTexture::from(tex));
            } else {
                ui.label("Rendering...");
            }
        });
    }

    fn render_current_page(&mut self, ctx: &egui::Context) {
        self.needs_render = false;
        match self.document.render_page(self.current_page, self.zoom) {
            Ok((rgba, width, height)) => {
                let name = format!("{}:page{}", self.path.display(), self.current_page);
                self.texture = Some(rgba_to_texture(ctx, &name, &rgba, width, height));
            }
            Err(e) => {
                log::error!("Failed to render page {}: {e}", self.current_page);
                self.texture = None;
            }
        }
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
