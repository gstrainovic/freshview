use egui_ratatui::RataguiBackend;
use ratatui::Terminal;
use ratatui::prelude::Stylize;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use soft_ratatui::embedded_graphics_unicodefonts::{
    mono_8x13_atlas, mono_8x13_bold_atlas, mono_8x13_italic_atlas,
};
use soft_ratatui::{EmbeddedGraphics, SoftBackend};

/// A widget that renders ratatui content inside an egui UI via egui_ratatui.
pub struct RatatuiWidget {
    terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
}

impl RatatuiWidget {
    /// Create a new RatatuiWidget with a 120x40 character terminal using mono_8x13 fonts.
    pub fn new() -> Self {
        let font_regular = mono_8x13_atlas();
        let font_italic = mono_8x13_italic_atlas();
        let font_bold = mono_8x13_bold_atlas();
        let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
            120,
            40,
            font_regular,
            Some(font_bold),
            Some(font_italic),
        );
        let backend = RataguiBackend::new("freshview", soft_backend);
        let terminal = Terminal::new(backend).expect("failed to create ratatui terminal");

        Self { terminal }
    }

    /// Draw the ratatui content and display it in the egui UI.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(
                    Paragraph::new("Hello from egui_ratatui!")
                        .block(Block::new().title("FreshView").borders(Borders::ALL))
                        .white()
                        .on_blue()
                        .wrap(Wrap { trim: false }),
                    area,
                );
            })
            .expect("failed to draw terminal");

        ui.add(self.terminal.backend_mut());
    }
}
