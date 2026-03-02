use std::path::Path;

use eframe::egui;
use freshview_editor::app::FreshEditorApp;
use freshview_viewer::ViewerWindow;

const VIEWER_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "tiff",
];

fn should_use_viewer(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIEWER_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

struct FreshViewApp {
    editor: Option<FreshEditorApp>,
    viewers: Vec<ViewerWindow>,
    init_error: Option<String>,
}

impl FreshViewApp {
    fn new(viewer_paths: Vec<std::path::PathBuf>) -> Self {
        let editor = match FreshEditorApp::new(120, 40) {
            Ok(e) => Some(e),
            Err(e) => {
                return Self {
                    editor: None,
                    viewers: Vec::new(),
                    init_error: Some(format!("Failed to initialize editor: {e}")),
                };
            }
        };

        let mut viewers = Vec::new();
        for path in &viewer_paths {
            match ViewerWindow::open(path) {
                Ok(vw) => viewers.push(vw),
                Err(e) => log::error!("Failed to open viewer for {}: {e}", path.display()),
            }
        }

        Self {
            editor,
            viewers,
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

        if let Some(ref mut editor) = self.editor {
            if editor.should_quit() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }

            editor.tick();

            egui::CentralPanel::default().show(ctx, |ui| {
                editor.show(ui);
            });
        }

        self.viewers.retain_mut(|v| v.show(ctx));
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    let mut viewer_paths = Vec::new();
    for arg in std::env::args().skip(1) {
        let path = std::path::PathBuf::from(&arg);
        if should_use_viewer(&path) {
            viewer_paths.push(path);
        } else {
            log::info!("Editor file (not yet supported via CLI): {arg}");
        }
    }

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
        assert!(should_use_viewer(Path::new("doc.pdf")));
        assert!(should_use_viewer(Path::new("photo.PNG")));
        assert!(should_use_viewer(Path::new("image.jpg")));
        assert!(should_use_viewer(Path::new("pic.jpeg")));
        assert!(should_use_viewer(Path::new("anim.gif")));
        assert!(should_use_viewer(Path::new("icon.bmp")));
        assert!(should_use_viewer(Path::new("logo.svg")));
        assert!(should_use_viewer(Path::new("hero.webp")));
        assert!(should_use_viewer(Path::new("scan.tiff")));
    }

    #[test]
    fn text_files_not_viewer() {
        assert!(!should_use_viewer(Path::new("main.rs")));
        assert!(!should_use_viewer(Path::new("README.md")));
        assert!(!should_use_viewer(Path::new("config.toml")));
        assert!(!should_use_viewer(Path::new("data.json")));
    }

    #[test]
    fn no_extension_not_viewer() {
        assert!(!should_use_viewer(Path::new("Makefile")));
        assert!(!should_use_viewer(Path::new("")));
    }

    #[test]
    fn case_insensitive() {
        assert!(should_use_viewer(Path::new("file.PDF")));
        assert!(should_use_viewer(Path::new("file.Jpg")));
        assert!(should_use_viewer(Path::new("file.WEBP")));
    }
}
