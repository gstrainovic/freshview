use eframe::egui;
use freshview_editor::widget::RatatuiWidget;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };

    let mut widget = RatatuiWidget::new();

    eframe::run_simple_native("FreshView", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            widget.show(ui);
        });
    })
}
