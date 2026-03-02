//! FreshEditorApp — embeds the Fresh terminal editor inside an egui panel
//! via egui_ratatui.

use std::sync::Arc;

use egui_ratatui::RataguiBackend;
use ratatui::Terminal;
use soft_ratatui::embedded_graphics_unicodefonts::{
    mono_8x13_atlas, mono_8x13_bold_atlas, mono_8x13_italic_atlas,
};
use soft_ratatui::{EmbeddedGraphics, SoftBackend};

use fresh::app::{editor_tick, Editor};
use fresh::config::Config;
use fresh::config_io::DirectoryContext;
use fresh::model::filesystem::StdFileSystem;
use fresh::view::color_support::ColorCapability;

use crate::input;

/// Cell width in pixels for the mono_8x13 font.
const CELL_WIDTH: f32 = 8.0;
/// Cell height in pixels for the mono_8x13 font.
const CELL_HEIGHT: f32 = 13.0;

/// Wraps a Fresh `Editor` and an egui_ratatui `Terminal`, rendering the
/// editor's TUI output into an egui widget each frame.
pub struct FreshEditorApp {
    editor: Editor,
    terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
    cols: u16,
    rows: u16,
}

impl FreshEditorApp {
    /// Create a new `FreshEditorApp` with the given initial terminal dimensions
    /// (in character cells).
    pub fn new(cols: u16, rows: u16) -> anyhow::Result<Self> {
        let font_regular = mono_8x13_atlas();
        let font_italic = mono_8x13_italic_atlas();
        let font_bold = mono_8x13_bold_atlas();
        let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
            cols,
            rows,
            font_regular,
            Some(font_bold),
            Some(font_italic),
        );
        let backend = RataguiBackend::new("freshview-editor", soft_backend);
        let terminal = Terminal::new(backend)?;

        let config = Config::default();
        let dir_context = DirectoryContext::from_system()?;
        let filesystem = Arc::new(StdFileSystem);
        let editor = Editor::new(
            config,
            cols,
            rows,
            dir_context,
            ColorCapability::TrueColor,
            filesystem,
        )?;

        Ok(Self {
            editor,
            terminal,
            cols,
            rows,
        })
    }

    /// Run one tick of the editor (processes async messages, timers, etc.).
    pub fn tick(&mut self) {
        // The clear_terminal callback is a no-op since we render to an offscreen buffer.
        let _ = editor_tick(&mut self.editor, || Ok(()));
    }

    /// Handle egui input, render the editor, and display the result in `ui`.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Compute desired terminal size from available space.
        let available = ui.available_size();
        let new_cols = (available.x / CELL_WIDTH).max(1.0) as u16;
        let new_rows = (available.y / CELL_HEIGHT).max(1.0) as u16;
        if new_cols != self.cols || new_rows != self.rows {
            self.cols = new_cols;
            self.rows = new_rows;
            self.editor.resize(new_cols, new_rows);
            let font_regular = mono_8x13_atlas();
            let font_italic = mono_8x13_italic_atlas();
            let font_bold = mono_8x13_bold_atlas();
            let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
                new_cols,
                new_rows,
                font_regular,
                Some(font_bold),
                Some(font_italic),
            );
            let backend = RataguiBackend::new("freshview-editor", soft_backend);
            self.terminal = Terminal::new(backend).expect("failed to recreate terminal");
        }

        // Process keyboard input.
        ui.input(|input_state| {
            for event in &input_state.events {
                match event {
                    egui::Event::Key {
                        key,
                        modifiers,
                        pressed: true,
                        ..
                    } => {
                        // Try to get the character from the event text
                        let text_char = input_state
                            .events
                            .iter()
                            .find_map(|e| {
                                if let egui::Event::Text(t) = e {
                                    t.chars().next()
                                } else {
                                    None
                                }
                            });
                        let (code, mods) = input::translate_key(*key, *modifiers, text_char);
                        let _ = self.editor.handle_key(code, mods);
                    }
                    egui::Event::PointerButton {
                        button,
                        pos,
                        pressed: true,
                        ..
                    } => {
                        let col = (pos.x / CELL_WIDTH) as u16;
                        let row = (pos.y / CELL_HEIGHT) as u16;
                        let mouse_event = input::translate_mouse_click(*button, col, row);
                        let _ = self.editor.handle_mouse(mouse_event);
                    }
                    egui::Event::MouseWheel { delta, .. } => {
                        let mouse_event = input::translate_scroll(delta.y, 0, 0);
                        let _ = self.editor.handle_mouse(mouse_event);
                    }
                    _ => {}
                }
            }
        });

        // Run one tick.
        self.tick();

        // Render editor into the terminal buffer.
        let editor = &mut self.editor;
        self.terminal
            .draw(|frame| {
                editor.render(frame);
            })
            .expect("failed to draw terminal");

        // Display the rendered terminal as an egui widget.
        ui.add(self.terminal.backend_mut());

        // Request continuous repainting so the editor stays responsive.
        ui.ctx().request_repaint();
    }

    /// Returns `true` if the editor wants to quit.
    pub fn should_quit(&self) -> bool {
        self.editor.should_quit()
    }
}
