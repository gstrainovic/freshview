//! FreshEditorApp — embeds the Fresh terminal editor inside an egui panel
//! via egui_ratatui.

use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use fresh::app::{editor_tick, Editor};
use fresh::config::Config;
use fresh::config_io::DirectoryContext;
use fresh::model::filesystem::StdFileSystem;
use fresh::view::color_support::ColorCapability;

use crate::input;

/// How often to run editor_tick for background work (LSP, file watching, etc.)
const TICK_INTERVAL: Duration = Duration::from_millis(50);

pub struct FreshEditorApp {
    editor: Editor,
    terminal: Terminal<TestBackend>,
    cols: u16,
    rows: u16,
    /// Set to true when the editor buffer has actually changed.
    dirty: Arc<AtomicBool>,
    /// Last time we ran editor_tick.
    last_tick: Instant,
    /// Stores the path of the active buffer in the previous frame to detect new file opens.
    last_active_buffer_path: Option<PathBuf>,
    /// Paths of image files that were opened by the editor and should be handled by the viewer.
    opened_image_paths: Vec<PathBuf>,
}

impl FreshEditorApp {
    pub fn new(cols: u16, rows: u16) -> anyhow::Result<Self> {
        let backend = TestBackend::new(cols, rows);
        let terminal = Terminal::new(backend)?;

        // FreshView-specific: Store all configs/data in ./.fresh/
        let base_dir = std::env::current_dir()?.join(".fresh");
        std::fs::create_dir_all(&base_dir)?;
        
        let dir_context = DirectoryContext {
            data_dir: base_dir.join("data"),
            config_dir: base_dir.join("config"),
            home_dir: dirs::home_dir(),
            documents_dir: dirs::document_dir(),
            downloads_dir: dirs::download_dir(),
        };

        let config = Config::default();
        let filesystem = Arc::new(StdFileSystem);
        let editor = Editor::new(
            config,
            cols,
            rows,
            dir_context,
            ColorCapability::TrueColor,
            filesystem,
        )?;

        let dirty = Arc::new(AtomicBool::new(true));

        Ok(Self {
            editor,
            terminal,
            cols,
            rows,
            dirty,
            last_tick: Instant::now(),
            last_active_buffer_path: None,
            opened_image_paths: Vec::new(),
        })
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Resize if needed.
        let available = ui.available_size();
        
        // Calculate cell size based on current egui font.
        let font_id = egui::TextStyle::Monospace.resolve(ui.style());
        let (char_width, char_height): (f32, f32) = ui.fonts(|f| {
            (f.glyph_width(&font_id, ' '), f.row_height(&font_id))
        });

        let new_cols = (available.x / char_width).max(1.0) as u16;
        let new_rows = (available.y / char_height).max(1.0) as u16;
        
        if new_cols != self.cols || new_rows != self.rows {
            self.cols = new_cols;
            self.rows = new_rows;
            self.editor.resize(new_cols, new_rows);
            self.terminal.backend_mut().resize(new_cols, new_rows);
            self.dirty.store(true, Ordering::Relaxed);
        }

        // Detect newly active image files.
        let current_active_buffer_id = self.editor.active_buffer();
        let display_name = self.editor.get_buffer_display_name(current_active_buffer_id);
        let current_active_path = if display_name.starts_with('/') {
            Some(PathBuf::from(display_name))
        } else {
            None
        };

        if let Some(ref path_buf) = current_active_path {
            if FreshEditorApp::should_use_viewer(path_buf.as_path()) {
                if self.last_active_buffer_path.as_ref() != Some(path_buf) {
                    self.opened_image_paths.push(path_buf.clone());
                    let _ = self.editor.close_buffer(current_active_buffer_id);
                    self.editor.prev_buffer();
                    self.dirty.store(true, Ordering::Relaxed);
                }
            }
        }
        self.last_active_buffer_path = current_active_path;

        // Process input.
        ui.input(|input_state| {
            for event in &input_state.events {
                match event {
                    egui::Event::Key {
                        key,
                        modifiers,
                        pressed: true,
                        ..
                    } => {
                        let text_char = input_state.events.iter().find_map(|e| {
                            if let egui::Event::Text(t) = e {
                                t.chars().next()
                            } else {
                                None
                            }
                        });
                        let (code, mods) = input::translate_key(*key, *modifiers, text_char);
                        let _ = self.editor.handle_key(code, mods);
                        self.dirty.store(true, Ordering::Relaxed);
                    }
                    egui::Event::PointerButton {
                        button,
                        pos,
                        pressed: true,
                        ..
                    } => {
                        let rel_pos = *pos - ui.min_rect().min;
                        let col = (rel_pos.x / char_width) as u16;
                        let row = (rel_pos.y / char_height) as u16;
                        let _ = self
                            .editor
                            .handle_mouse(input::translate_mouse_click(*button, col, row));
                        self.dirty.store(true, Ordering::Relaxed);
                    }
                    egui::Event::MouseWheel { delta, .. } => {
                        let _ = self
                            .editor
                            .handle_mouse(input::translate_scroll(delta.y, 0, 0));
                        self.dirty.store(true, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }
        });

        // Run editor_tick on a timer.
        if self.last_tick.elapsed() >= TICK_INTERVAL {
            self.last_tick = Instant::now();
            let changed = editor_tick(&mut self.editor, || Ok(())).unwrap_or(false);
            if changed {
                self.dirty.store(true, Ordering::Relaxed);
            }
            ui.ctx().request_repaint_after(TICK_INTERVAL);
        }

        // Draw Ratatui to our internal buffer.
        let editor = &mut self.editor;
        self.terminal.draw(|frame| editor.render(frame)).expect("failed to draw");

        // --- Optimized Batch Rendering ---
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let buffer = self.terminal.backend().buffer();

        for y in 0..self.rows {
            let mut x = 0;
            while x < self.cols {
                let cell = &buffer[(x, y)];
                let fg = translate_color(cell.fg);
                let bg = match cell.bg {
                    ratatui::style::Color::Reset => egui::Color32::TRANSPARENT,
                    c => translate_color(c),
                };

                // Find how many characters to the right have the same style
                let mut run_width = 1;
                while x + run_width < self.cols {
                    let next_cell = &buffer[(x + run_width, y)];
                    if next_cell.fg == cell.fg && next_cell.bg == cell.bg {
                        run_width += 1;
                    } else {
                        break;
                    }
                }

                let pos = rect.min + egui::vec2(x as f32 * char_width, y as f32 * char_height);
                let run_rect = egui::Rect::from_min_size(pos, egui::vec2(char_width * run_width as f32, char_height));

                // Draw background for the whole run
                if bg != egui::Color32::TRANSPARENT {
                    painter.rect_filled(run_rect, 0.0, bg);
                }

                // Draw text for the whole run (if not just spaces)
                let mut text = String::with_capacity(run_width as usize);
                for i in 0..run_width {
                    text.push_str(buffer[(x + i, y)].symbol());
                }
                
                if !text.trim().is_empty() {
                    painter.text(
                        pos,
                        egui::Align2::LEFT_TOP,
                        &text,
                        font_id.clone(),
                        fg,
                    );
                }

                x += run_width;
            }
        }
        
        // Ensure we repaint if something happened
        if self.dirty.load(Ordering::Relaxed) {
            ui.ctx().request_repaint();
            self.dirty.store(false, Ordering::Relaxed);
        }
    }

    pub fn should_quit(&self) -> bool {
        self.editor.should_quit()
    }

    fn should_use_viewer(path: &Path) -> bool {
        const VIEWER_EXTENSIONS: &[&str] = &[
            "pdf", "png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "tiff",
        ];
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| VIEWER_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
            .unwrap_or(false)
    }

    pub fn drain_opened_image_paths(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.opened_image_paths)
    }
}

fn translate_color(c: ratatui::style::Color) -> egui::Color32 {
    match c {
        ratatui::style::Color::Reset => egui::Color32::WHITE,
        ratatui::style::Color::Black => egui::Color32::BLACK,
        ratatui::style::Color::Red => egui::Color32::from_rgb(205, 0, 0),
        ratatui::style::Color::Green => egui::Color32::from_rgb(0, 205, 0),
        ratatui::style::Color::Yellow => egui::Color32::from_rgb(205, 205, 0),
        ratatui::style::Color::Blue => egui::Color32::from_rgb(0, 0, 238),
        ratatui::style::Color::Magenta => egui::Color32::from_rgb(205, 0, 205),
        ratatui::style::Color::Cyan => egui::Color32::from_rgb(0, 205, 205),
        ratatui::style::Color::Gray => egui::Color32::from_rgb(229, 229, 229),
        ratatui::style::Color::DarkGray => egui::Color32::from_rgb(127, 127, 127),
        ratatui::style::Color::LightRed => egui::Color32::from_rgb(255, 0, 0),
        ratatui::style::Color::LightGreen => egui::Color32::from_rgb(0, 255, 0),
        ratatui::style::Color::LightYellow => egui::Color32::from_rgb(255, 255, 0),
        ratatui::style::Color::LightBlue => egui::Color32::from_rgb(92, 92, 255),
        ratatui::style::Color::LightMagenta => egui::Color32::from_rgb(255, 0, 255),
        ratatui::style::Color::LightCyan => egui::Color32::from_rgb(0, 255, 255),
        ratatui::style::Color::White => egui::Color32::WHITE,
        ratatui::style::Color::Rgb(r, g, b) => egui::Color32::from_rgb(r, g, b),
        ratatui::style::Color::Indexed(i) => {
            match i {
                0 => egui::Color32::BLACK,
                1 => egui::Color32::from_rgb(205, 0, 0),
                2 => egui::Color32::from_rgb(0, 205, 0),
                3 => egui::Color32::from_rgb(205, 205, 0),
                4 => egui::Color32::from_rgb(0, 0, 238),
                5 => egui::Color32::from_rgb(205, 0, 205),
                6 => egui::Color32::from_rgb(0, 205, 205),
                7 => egui::Color32::from_rgb(229, 229, 229),
                8 => egui::Color32::from_rgb(127, 127, 127),
                9 => egui::Color32::from_rgb(255, 0, 0),
                10 => egui::Color32::from_rgb(0, 255, 0),
                11 => egui::Color32::from_rgb(255, 255, 0),
                12 => egui::Color32::from_rgb(92, 92, 255),
                13 => egui::Color32::from_rgb(255, 0, 255),
                14 => egui::Color32::from_rgb(0, 255, 255),
                15 => egui::Color32::WHITE,
                _ => egui::Color32::GRAY,
            }
        }
    }
}

