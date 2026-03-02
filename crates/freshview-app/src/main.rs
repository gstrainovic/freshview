use eframe::egui;
use freshview_editor::app::FreshEditorApp;

struct FreshViewApp {
    editor: Option<FreshEditorApp>,
    init_error: Option<String>,
}

impl FreshViewApp {
    fn new() -> Self {
        match FreshEditorApp::new(120, 40) {
            Ok(editor) => Self {
                editor: Some(editor),
                init_error: None,
            },
            Err(e) => Self {
                editor: None,
                init_error: Some(format!("Failed to initialize editor: {e}")),
            },
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

            egui::CentralPanel::default().show(ctx, |ui| {
                editor.show(ui);
            });
        }
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };

    eframe::run_native(
        "FreshView",
        options,
        Box::new(|_cc| Ok(Box::new(FreshViewApp::new()))),
    )
}
