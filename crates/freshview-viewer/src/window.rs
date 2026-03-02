use std::path::{Path, PathBuf};

use anyhow::Result;
use egui::{ScrollArea, TextureHandle};

use crate::document::ViewerDocument;
use crate::renderer::rgba_to_texture;

/// A floating egui window that displays PDF pages or images via mupdf.
pub struct ViewerWindow {
    id: egui::Id,
    title: String,
    path: PathBuf,
    document: ViewerDocument,
    texture: Option<TextureHandle>,
    current_page: i32,
    total_pages: i32,
    zoom: f32,
    /// Set to `false` to close the window.
    pub open: bool,
    needs_render: bool,
}

impl ViewerWindow {
    /// Open a document (PDF or image) for viewing.
    pub fn open(path: &Path) -> Result<Self> {
        let document = ViewerDocument::open(path)?;
        let total_pages = document.page_count();
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Viewer".into());

        Ok(Self {
            id: egui::Id::new(("viewer_window", path)),
            title,
            path: path.to_path_buf(),
            document,
            texture: None,
            current_page: 0,
            total_pages,
            zoom: 1.0,
            open: true,
            needs_render: true,
        })
    }

    /// Show the floating viewer window. Returns `false` when the window has been closed.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        if !self.open {
            return false;
        }

        if self.needs_render {
            self.render_current_page(ctx);
        }

        let mut open = self.open;
        egui::Window::new(&self.title)
            .id(self.id)
            .open(&mut open)
            .resizable(true)
            .default_size([600.0, 800.0])
            .show(ctx, |ui| {
                self.show_toolbar(ui);
                ui.separator();
                self.show_content(ui);
            });
        self.open = open;

        self.open
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Page navigation (only when multi-page)
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

            // Zoom controls
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
