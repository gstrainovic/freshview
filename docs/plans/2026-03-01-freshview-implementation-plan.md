# FreshView IDE — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Cross-platform IDE die Fresh Editor via egui_ratatui rendert, mit mupdf Floating-Windows fuer PDF/Bild-Preview.

**Architecture:** In-Process: Fresh Editor laeuft direkt im egui-Prozess. egui_ratatui rendert Fresh's ratatui-Output als egui Widget. mupdf rendert PDFs/Bilder zu egui Texturen in Floating Windows.

**Tech Stack:** Rust, eframe/egui, egui_ratatui, soft_ratatui, ratatui, fresh-editor (git dep), mupdf, crossterm (Event-Typen)

**Referenz-Dateien im Fresh-Repo (`/home/g/projects/fresh/`):**
- `crates/fresh-gui/src/lib.rs` — GuiApplication Trait, winit→crossterm Input-Translation
- `crates/fresh-editor/src/gui/mod.rs` — EditorApp impl GuiApplication
- `crates/fresh-editor/src/app/mod.rs` — Editor struct, handle_key(), handle_mouse(), render()
- `crates/fresh-editor/src/app/input.rs` — Editor::handle_key(code, modifiers)
- `crates/fresh-editor/src/app/mouse_input.rs` — Editor::handle_mouse(event)
- `crates/fresh-editor/src/app/render.rs` — Editor::render(frame)

**Lizenz-Hinweis:** mupdf ist AGPL-3.0, Fresh ist GPL-2.0. Kompatibilitaet pruefen oder Alternative (pdf.js via WebView, pdfium-render) evaluieren falls noetig.

---

## Task 1: Workspace Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/freshview-app/Cargo.toml`
- Create: `crates/freshview-app/src/main.rs`
- Create: `crates/freshview-editor/Cargo.toml`
- Create: `crates/freshview-editor/src/lib.rs`
- Create: `crates/freshview-viewer/Cargo.toml`
- Create: `crates/freshview-viewer/src/lib.rs`

**Step 1: Workspace Cargo.toml erstellen**

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/freshview-app",
    "crates/freshview-editor",
    "crates/freshview-viewer",
]

[workspace.dependencies]
egui = "0.31"
eframe = "0.31"
egui_ratatui = "2.1"
soft_ratatui = { version = "2.1", features = ["unicodefonts"] }
ratatui = "0.29"
crossterm = "0.28"
anyhow = "1"
log = "0.4"
env_logger = "0.11"
```

**Step 2: freshview-app Crate anlegen**

```toml
# crates/freshview-app/Cargo.toml
[package]
name = "freshview-app"
version = "0.1.0"
edition = "2024"

[dependencies]
eframe = { workspace = true }
egui = { workspace = true }
freshview-editor = { path = "../freshview-editor" }
freshview-viewer = { path = "../freshview-viewer" }
anyhow = { workspace = true }
log = { workspace = true }
env_logger = { workspace = true }
```

```rust
// crates/freshview-app/src/main.rs
fn main() {
    println!("FreshView starting...");
}
```

**Step 3: freshview-editor Crate anlegen**

```toml
# crates/freshview-editor/Cargo.toml
[package]
name = "freshview-editor"
version = "0.1.0"
edition = "2024"

[dependencies]
egui = { workspace = true }
egui_ratatui = { workspace = true }
soft_ratatui = { workspace = true }
ratatui = { workspace = true }
crossterm = { workspace = true }
anyhow = { workspace = true }
```

```rust
// crates/freshview-editor/src/lib.rs
pub mod widget;
```

**Step 4: freshview-viewer Crate anlegen**

```toml
# crates/freshview-viewer/Cargo.toml
[package]
name = "freshview-viewer"
version = "0.1.0"
edition = "2024"

[dependencies]
egui = { workspace = true }
anyhow = { workspace = true }
```

```rust
// crates/freshview-viewer/src/lib.rs
pub mod window;
```

**Step 5: Kompilieren und verifizieren**

Run: `cd /home/g/projects/freshview && cargo check`
Expected: Kompiliert ohne Fehler

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: workspace scaffolding with 3 crates"
```

---

## Task 2: egui_ratatui Hello World

Minimales eframe-Fenster das einen ratatui-Paragraph via egui_ratatui rendert.
Beweist dass die Rendering-Pipeline funktioniert.

**Files:**
- Create: `crates/freshview-editor/src/widget.rs`
- Modify: `crates/freshview-app/src/main.rs`

**Step 1: RatatuiWidget erstellen**

```rust
// crates/freshview-editor/src/widget.rs
use egui_ratatui::RataguiBackend;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use soft_ratatui::embedded_graphics_unicodefonts::{
    mono_8x13_atlas, mono_8x13_bold_atlas, mono_8x13_italic_atlas,
};
use soft_ratatui::{EmbeddedGraphics, SoftBackend};

pub struct RatatuiWidget {
    terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
}

impl RatatuiWidget {
    pub fn new() -> Self {
        let font_regular = mono_8x13_atlas();
        let font_italic = mono_8x13_italic_atlas();
        let font_bold = mono_8x13_bold_atlas();
        let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
            120, 40,
            font_regular,
            Some(font_bold),
            Some(font_italic),
        );
        let backend = RataguiBackend::new("freshview", soft_backend);
        let terminal = Terminal::new(backend).unwrap();
        Self { terminal }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.terminal
            .draw(|frame| {
                let area = frame.area();
                frame.render_widget(
                    Paragraph::new("Hello from egui_ratatui! FreshView works.")
                        .block(Block::new().title("FreshView").borders(Borders::ALL))
                        .white()
                        .on_blue()
                        .wrap(Wrap { trim: false }),
                    area,
                );
            })
            .expect("draw failed");

        ui.add(self.terminal.backend_mut());
    }
}
```

**Step 2: main.rs mit eframe-Fenster**

```rust
// crates/freshview-app/src/main.rs
use eframe::egui;
use freshview_editor::widget::RatatuiWidget;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("FreshView"),
        ..Default::default()
    };

    let mut widget = RatatuiWidget::new();

    eframe::run_simple_native("FreshView", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            widget.show(ui);
        });
    })
}
```

**Step 3: Kompilieren und starten**

Run: `cd /home/g/projects/freshview && cargo run -p freshview-app`
Expected: Fenster oeffnet sich mit blauem ratatui-Paragraph "Hello from egui_ratatui!"

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: egui_ratatui hello world rendering"
```

---

## Task 3: egui Input → crossterm Event Translation

Kernkomponente: Uebersetzt egui Keyboard/Mouse Events in crossterm-Typen
die Fresh versteht. Referenz: `fresh/crates/fresh-gui/src/lib.rs` Zeilen 557-860.

**Files:**
- Create: `crates/freshview-editor/src/input.rs`
- Create: `crates/freshview-editor/tests/input_tests.rs`

**Step 1: Test fuer Keyboard-Translation schreiben**

```rust
// crates/freshview-editor/tests/input_tests.rs
use crossterm::event::{KeyCode, KeyModifiers};
use freshview_editor::input::translate_key;

#[test]
fn test_translate_simple_char() {
    let (code, mods) = translate_key(egui::Key::A, egui::Modifiers::NONE, Some('a'));
    assert_eq!(code, KeyCode::Char('a'));
    assert_eq!(mods, KeyModifiers::NONE);
}

#[test]
fn test_translate_ctrl_s() {
    let (code, mods) = translate_key(
        egui::Key::S,
        egui::Modifiers { ctrl: true, ..Default::default() },
        Some('s'),
    );
    assert_eq!(code, KeyCode::Char('s'));
    assert_eq!(mods, KeyModifiers::CONTROL);
}

#[test]
fn test_translate_enter() {
    let (code, mods) = translate_key(egui::Key::Enter, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Enter);
    assert_eq!(mods, KeyModifiers::NONE);
}

#[test]
fn test_translate_arrow_keys() {
    let (code, _) = translate_key(egui::Key::ArrowUp, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Up);
    let (code, _) = translate_key(egui::Key::ArrowDown, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Down);
    let (code, _) = translate_key(egui::Key::ArrowLeft, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Left);
    let (code, _) = translate_key(egui::Key::ArrowRight, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Right);
}

#[test]
fn test_translate_function_keys() {
    let (code, _) = translate_key(egui::Key::F1, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::F(1));
    let (code, _) = translate_key(egui::Key::F12, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::F(12));
}

#[test]
fn test_translate_modifiers() {
    let mods = egui::Modifiers { ctrl: true, shift: true, alt: false, ..Default::default() };
    let (_, ct_mods) = translate_key(egui::Key::A, mods, Some('A'));
    assert!(ct_mods.contains(KeyModifiers::CONTROL));
    assert!(ct_mods.contains(KeyModifiers::SHIFT));
    assert!(!ct_mods.contains(KeyModifiers::ALT));
}
```

**Step 2: Test ausfuehren, Fehlschlag verifizieren**

Run: `cargo test -p freshview-editor --test input_tests`
Expected: FAIL — `freshview_editor::input` existiert nicht

**Step 3: Input-Translation implementieren**

```rust
// crates/freshview-editor/src/input.rs
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Uebersetzt einen egui Key + Modifiers in crossterm KeyCode + KeyModifiers.
/// `text_char` ist das Zeichen das egui fuer Text-Input liefert (falls vorhanden).
pub fn translate_key(
    key: egui::Key,
    modifiers: egui::Modifiers,
    text_char: Option<char>,
) -> (KeyCode, KeyModifiers) {
    let code = match key {
        egui::Key::Enter => KeyCode::Enter,
        egui::Key::Tab => KeyCode::Tab,
        egui::Key::Backspace => KeyCode::Backspace,
        egui::Key::Delete => KeyCode::Delete,
        egui::Key::Escape => KeyCode::Esc,
        egui::Key::Home => KeyCode::Home,
        egui::Key::End => KeyCode::End,
        egui::Key::PageUp => KeyCode::PageUp,
        egui::Key::PageDown => KeyCode::PageDown,
        egui::Key::ArrowUp => KeyCode::Up,
        egui::Key::ArrowDown => KeyCode::Down,
        egui::Key::ArrowLeft => KeyCode::Left,
        egui::Key::ArrowRight => KeyCode::Right,
        egui::Key::F1 => KeyCode::F(1),
        egui::Key::F2 => KeyCode::F(2),
        egui::Key::F3 => KeyCode::F(3),
        egui::Key::F4 => KeyCode::F(4),
        egui::Key::F5 => KeyCode::F(5),
        egui::Key::F6 => KeyCode::F(6),
        egui::Key::F7 => KeyCode::F(7),
        egui::Key::F8 => KeyCode::F(8),
        egui::Key::F9 => KeyCode::F(9),
        egui::Key::F10 => KeyCode::F(10),
        egui::Key::F11 => KeyCode::F(11),
        egui::Key::F12 => KeyCode::F(12),
        egui::Key::Space => KeyCode::Char(' '),
        egui::Key::Insert => KeyCode::Insert,
        _ => {
            if let Some(c) = text_char {
                KeyCode::Char(c)
            } else {
                // Fallback: versuche Key-Name als Char
                key_to_char(key)
                    .map(KeyCode::Char)
                    .unwrap_or(KeyCode::Null)
            }
        }
    };

    let mods = translate_modifiers(modifiers);
    (code, mods)
}

/// Uebersetzt egui Modifiers in crossterm KeyModifiers
pub fn translate_modifiers(m: egui::Modifiers) -> KeyModifiers {
    let mut mods = KeyModifiers::NONE;
    if m.ctrl || m.command {
        mods |= KeyModifiers::CONTROL;
    }
    if m.shift {
        mods |= KeyModifiers::SHIFT;
    }
    if m.alt {
        mods |= KeyModifiers::ALT;
    }
    mods
}

/// Uebersetzt egui Pointer-Events in crossterm MouseEvent.
/// `col` und `row` sind Zell-Koordinaten (nicht Pixel).
pub fn translate_mouse_click(
    button: egui::PointerButton,
    col: u16,
    row: u16,
) -> MouseEvent {
    let btn = match button {
        egui::PointerButton::Primary => MouseButton::Left,
        egui::PointerButton::Secondary => MouseButton::Right,
        egui::PointerButton::Middle => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    MouseEvent {
        kind: MouseEventKind::Down(btn),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

/// Uebersetzt egui Scroll-Events in crossterm MouseEvent
pub fn translate_scroll(delta_y: f32, col: u16, row: u16) -> MouseEvent {
    let kind = if delta_y > 0.0 {
        MouseEventKind::ScrollUp
    } else {
        MouseEventKind::ScrollDown
    };
    MouseEvent {
        kind,
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn key_to_char(key: egui::Key) -> Option<char> {
    match key {
        egui::Key::A => Some('a'),
        egui::Key::B => Some('b'),
        egui::Key::C => Some('c'),
        egui::Key::D => Some('d'),
        egui::Key::E => Some('e'),
        egui::Key::F => Some('f'),
        egui::Key::G => Some('g'),
        egui::Key::H => Some('h'),
        egui::Key::I => Some('i'),
        egui::Key::J => Some('j'),
        egui::Key::K => Some('k'),
        egui::Key::L => Some('l'),
        egui::Key::M => Some('m'),
        egui::Key::N => Some('n'),
        egui::Key::O => Some('o'),
        egui::Key::P => Some('p'),
        egui::Key::Q => Some('q'),
        egui::Key::R => Some('r'),
        egui::Key::S => Some('s'),
        egui::Key::T => Some('t'),
        egui::Key::U => Some('u'),
        egui::Key::V => Some('v'),
        egui::Key::W => Some('w'),
        egui::Key::X => Some('x'),
        egui::Key::Y => Some('y'),
        egui::Key::Z => Some('z'),
        egui::Key::Num0 => Some('0'),
        egui::Key::Num1 => Some('1'),
        egui::Key::Num2 => Some('2'),
        egui::Key::Num3 => Some('3'),
        egui::Key::Num4 => Some('4'),
        egui::Key::Num5 => Some('5'),
        egui::Key::Num6 => Some('6'),
        egui::Key::Num7 => Some('7'),
        egui::Key::Num8 => Some('8'),
        egui::Key::Num9 => Some('9'),
        egui::Key::Minus => Some('-'),
        egui::Key::Equals => Some('='),
        egui::Key::OpenBracket => Some('['),
        egui::Key::CloseBracket => Some(']'),
        egui::Key::Backslash => Some('\\'),
        egui::Key::Semicolon => Some(';'),
        egui::Key::Comma => Some(','),
        egui::Key::Period => Some('.'),
        egui::Key::Slash => Some('/'),
        egui::Key::Backtick => Some('`'),
        _ => None,
    }
}
```

**Step 4: lib.rs aktualisieren**

```rust
// crates/freshview-editor/src/lib.rs
pub mod input;
pub mod widget;
```

**Step 5: Tests ausfuehren, Erfolg verifizieren**

Run: `cargo test -p freshview-editor --test input_tests`
Expected: Alle 6 Tests PASS

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: egui to crossterm input translation with tests"
```

---

## Task 4: Fresh Editor Integration

Fresh Editor in-process starten und via egui_ratatui rendern.
Dies ist der kritischste Task — beweist dass die Architektur funktioniert.

**Files:**
- Modify: `crates/freshview-editor/Cargo.toml` (fresh-editor Dependency)
- Create: `crates/freshview-editor/src/app.rs`
- Modify: `crates/freshview-editor/src/lib.rs`
- Modify: `crates/freshview-app/src/main.rs`

**Vorbedingung:** Fresh muss als Library nutzbar sein. Pruefen:

```bash
# Im Fresh-Repo pruefen ob fresh-editor als lib.rs exportiert
ls /home/g/projects/fresh/crates/fresh-editor/src/lib.rs
```

Falls Fresh kein `lib.rs` hat oder nicht als Dependency nutzbar ist,
muss ein Wrapper erstellt oder Fresh geforkt werden.

**Step 1: fresh-editor als Git-Dependency hinzufuegen**

```toml
# Workspace Cargo.toml — unter [workspace.dependencies] ergaenzen:
fresh-editor = { git = "https://github.com/sinelaw/fresh.git", package = "fresh-editor" }
fresh-core = { git = "https://github.com/sinelaw/fresh.git", package = "fresh-core" }
```

Alternativ, falls lokaler Pfad bevorzugt:
```toml
fresh-editor = { path = "../fresh/crates/fresh-editor" }
fresh-core = { path = "../fresh/crates/fresh-core" }
```

```toml
# crates/freshview-editor/Cargo.toml — unter [dependencies] ergaenzen:
fresh-editor = { workspace = true }
```

**Step 2: Pruefen ob fresh-editor kompiliert**

Run: `cargo check -p freshview-editor`
Expected: Kompiliert (evtl. Warnungen). Falls Fehler: Fresh-Dependency anpassen.

**ACHTUNG:** Dieser Schritt kann scheitern falls fresh-editor nicht als Library
exportiert ist oder inkompatible Dependencies hat. In dem Fall:
- Pruefen was `fresh-editor/src/lib.rs` exportiert
- Eventuell Features anpassen
- Falls noetig: lokalen Fork erstellen

**Step 3: FreshEditorApp erstellen**

```rust
// crates/freshview-editor/src/app.rs
use crate::input;
use egui_ratatui::RataguiBackend;
use ratatui::Terminal;
use soft_ratatui::embedded_graphics_unicodefonts::{
    mono_8x13_atlas, mono_8x13_bold_atlas, mono_8x13_italic_atlas,
};
use soft_ratatui::{EmbeddedGraphics, SoftBackend};

/// FreshView Editor Widget — rendert Fresh Editor in egui via egui_ratatui.
pub struct FreshEditorApp {
    editor: fresh::Editor,
    terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
    cols: u16,
    rows: u16,
}

impl FreshEditorApp {
    pub fn new(cols: u16, rows: u16) -> anyhow::Result<Self> {
        let font_regular = mono_8x13_atlas();
        let font_italic = mono_8x13_italic_atlas();
        let font_bold = mono_8x13_bold_atlas();
        let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
            cols as u32, rows as u32,
            font_regular,
            Some(font_bold),
            Some(font_italic),
        );
        let backend = RataguiBackend::new("freshview", soft_backend);
        let terminal = Terminal::new(backend)?;

        // Fresh Editor mit aktuellem Working Directory starten
        let editor = fresh::Editor::with_working_dir(
            std::env::current_dir()?,
            cols,
            rows,
        )?;

        Ok(Self { editor, terminal, cols, rows })
    }

    /// Muss jeden Frame aufgerufen werden — verarbeitet async Messages (LSP, Plugins)
    pub fn tick(&mut self) {
        // fresh::editor_tick() verarbeitet Timers, LSP Responses, Plugin Commands
        let _ = fresh::editor_tick(&mut self.editor);
    }

    /// Rendert den Editor in egui
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // Input verarbeiten
        self.handle_input(ui.ctx());

        // Resize pruefen
        let available = ui.available_size();
        let new_cols = (available.x / 8.0) as u16;  // 8px Zellbreite (mono_8x13)
        let new_rows = (available.y / 13.0) as u16;  // 13px Zellhoehe
        if new_cols != self.cols || new_rows != self.rows {
            self.cols = new_cols;
            self.rows = new_rows;
            self.editor.resize(new_cols, new_rows);
        }

        // Render
        self.terminal
            .draw(|frame| {
                self.editor.render(frame);
            })
            .expect("render failed");

        ui.add(self.terminal.backend_mut());
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            // Keyboard Events
            for event in &input.events {
                match event {
                    egui::Event::Key { key, modifiers, pressed: true, .. } => {
                        let text_char = input.events.iter().find_map(|e| {
                            if let egui::Event::Text(t) = e {
                                t.chars().next()
                            } else {
                                None
                            }
                        });
                        let (code, mods) = input::translate_key(*key, *modifiers, text_char);
                        let _ = self.editor.handle_key(code, mods);
                    }
                    _ => {}
                }
            }

            // Mouse Events
            if let Some(pos) = input.pointer.hover_pos() {
                let col = (pos.x / 8.0) as u16;
                let row = (pos.y / 13.0) as u16;

                // Scroll
                let scroll = input.smooth_scroll_delta;
                if scroll.y != 0.0 {
                    let mouse_event = input::translate_scroll(scroll.y, col, row);
                    let _ = self.editor.handle_mouse(mouse_event);
                }

                // Click
                if input.pointer.any_pressed() {
                    let button = if input.pointer.button_pressed(egui::PointerButton::Primary) {
                        egui::PointerButton::Primary
                    } else if input.pointer.button_pressed(egui::PointerButton::Secondary) {
                        egui::PointerButton::Secondary
                    } else {
                        egui::PointerButton::Middle
                    };
                    let mouse_event = input::translate_mouse_click(button, col, row);
                    let _ = self.editor.handle_mouse(mouse_event);
                }
            }
        });
    }

    /// Gibt true zurueck wenn der Editor beendet werden soll
    pub fn should_quit(&self) -> bool {
        self.editor.should_quit()
    }
}
```

**Step 4: lib.rs aktualisieren**

```rust
// crates/freshview-editor/src/lib.rs
pub mod app;
pub mod input;
pub mod widget;
```

**Step 5: main.rs mit Fresh Editor**

```rust
// crates/freshview-app/src/main.rs
use eframe::egui;
use freshview_editor::app::FreshEditorApp;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("FreshView"),
        ..Default::default()
    };

    eframe::run_native(
        "FreshView",
        options,
        Box::new(|_cc| {
            let editor = FreshEditorApp::new(160, 50)
                .expect("Failed to create editor");
            Ok(Box::new(FreshViewApp { editor }))
        }),
    )
}

struct FreshViewApp {
    editor: FreshEditorApp,
}

impl eframe::App for FreshViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.editor.tick();

        egui::CentralPanel::default().show(ctx, |ui| {
            self.editor.show(ui);
        });

        if self.editor.should_quit() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // 60 FPS
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
```

**Step 6: Kompilieren und testen**

Run: `cargo run -p freshview-app`
Expected: Fenster oeffnet sich mit Fresh Editor. Tippen funktioniert.

**WICHTIG:** Dieser Schritt wird wahrscheinlich Anpassungen brauchen weil:
- Fresh's Editor-Konstruktor andere Parameter haben koennte
- Die exakten Imports/Module-Pfade anders sein koennten
- Event-Handling Feintuning braucht

Iteriere bis Tippen und Cursor-Bewegung funktionieren.

**Step 7: Commit**

```bash
git add -A
git commit -m "feat: fresh editor integration via egui_ratatui"
```

---

## Task 5: mupdf Viewer — PDF Rendering

Floating Window das eine PDF-Seite via mupdf rendert.

**Files:**
- Modify: `crates/freshview-viewer/Cargo.toml`
- Create: `crates/freshview-viewer/src/document.rs`
- Create: `crates/freshview-viewer/src/renderer.rs`
- Create: `crates/freshview-viewer/tests/viewer_tests.rs`

**Step 1: mupdf Dependency hinzufuegen**

```toml
# Workspace Cargo.toml — unter [workspace.dependencies]:
mupdf = "0.6"

# crates/freshview-viewer/Cargo.toml
[dependencies]
egui = { workspace = true }
mupdf = { workspace = true }
anyhow = { workspace = true }
log = { workspace = true }
```

**Step 2: Test fuer PDF-Rendering schreiben**

```rust
// crates/freshview-viewer/tests/viewer_tests.rs
use freshview_viewer::document::ViewerDocument;
use std::path::Path;

#[test]
fn test_open_pdf_page_count() {
    // Erstelle eine minimale Test-PDF oder nutze eine vorhandene
    let test_pdf = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test.pdf");
    if !test_pdf.exists() {
        // Skip wenn keine Test-PDF vorhanden
        eprintln!("Skipping: no test PDF at {:?}", test_pdf);
        return;
    }
    let doc = ViewerDocument::open(&test_pdf).unwrap();
    assert!(doc.page_count() > 0);
}

#[test]
fn test_render_page_to_rgba() {
    let test_pdf = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test.pdf");
    if !test_pdf.exists() {
        eprintln!("Skipping: no test PDF at {:?}", test_pdf);
        return;
    }
    let doc = ViewerDocument::open(&test_pdf).unwrap();
    let (rgba, width, height) = doc.render_page(0, 1.0).unwrap();
    assert!(width > 0);
    assert!(height > 0);
    assert_eq!(rgba.len(), (width * height * 4) as usize);
}
```

**Step 3: Test ausfuehren, Fehlschlag verifizieren**

Run: `cargo test -p freshview-viewer --test viewer_tests`
Expected: FAIL — Module existiert nicht

**Step 4: Document-Wrapper implementieren**

```rust
// crates/freshview-viewer/src/document.rs
use anyhow::Result;
use mupdf::{Colorspace, Document, Matrix};
use std::path::Path;

/// Wrapper um mupdf::Document fuer PDF und Bild-Dateien.
pub struct ViewerDocument {
    doc: Document,
    pages: i32,
}

impl ViewerDocument {
    pub fn open(path: &Path) -> Result<Self> {
        let path_str = path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
        let doc = Document::open(path_str)?;
        let pages = doc.page_count()?;
        Ok(Self { doc, pages })
    }

    pub fn page_count(&self) -> i32 {
        self.pages
    }

    /// Rendert eine Seite zu RGBA-Bytes.
    /// Returns (rgba_bytes, width, height).
    pub fn render_page(&self, page_idx: i32, zoom: f32) -> Result<(Vec<u8>, u32, u32)> {
        let page = self.doc.load_page(page_idx)?;
        let matrix = Matrix::new_scale(zoom * 2.0, zoom * 2.0);
        let pixmap = page.to_pixmap(&matrix, &Colorspace::device_rgb(), false)?;

        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;
        let samples = pixmap.samples();

        // mupdf RGB → RGBA (egui braucht RGBA)
        let n = pixmap.n() as usize;
        let rgba = if n == 4 {
            samples.to_vec()
        } else {
            // RGB → RGBA
            let pixel_count = (width * height) as usize;
            let mut rgba = Vec::with_capacity(pixel_count * 4);
            for i in 0..pixel_count {
                rgba.push(samples[i * 3]);
                rgba.push(samples[i * 3 + 1]);
                rgba.push(samples[i * 3 + 2]);
                rgba.push(255); // Alpha
            }
            rgba
        };

        Ok((rgba, width, height))
    }
}
```

**Step 5: Test-Fixture erstellen und Tests ausfuehren**

```bash
mkdir -p crates/freshview-viewer/tests/fixtures
# Minimale Test-PDF erstellen (falls keine vorhanden)
# Alternativ: Test ueberspringt wenn Datei fehlt
```

Run: `cargo test -p freshview-viewer --test viewer_tests`
Expected: Tests PASS (oder skip wenn keine fixture)

**Step 6: Commit**

```bash
git add -A
git commit -m "feat: mupdf document wrapper for PDF/image rendering"
```

---

## Task 6: mupdf Viewer — egui Floating Window

Floating egui::Window das eine PDF/Bild-Datei anzeigt.

**Files:**
- Modify: `crates/freshview-viewer/src/window.rs`
- Create: `crates/freshview-viewer/src/renderer.rs`
- Modify: `crates/freshview-viewer/src/lib.rs`

**Step 1: Renderer implementieren (Pixmap → egui Texture)**

```rust
// crates/freshview-viewer/src/renderer.rs
use egui::{ColorImage, Context, TextureHandle, TextureOptions};

/// Erstellt ein egui TextureHandle aus RGBA-Daten.
pub fn rgba_to_texture(
    ctx: &Context,
    name: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> TextureHandle {
    let image = ColorImage::from_rgba_unmultiplied(
        [width as usize, height as usize],
        rgba,
    );
    ctx.load_texture(name, image, TextureOptions::LINEAR)
}
```

**Step 2: ViewerWindow implementieren**

```rust
// crates/freshview-viewer/src/window.rs
use crate::document::ViewerDocument;
use crate::renderer;
use anyhow::Result;
use egui::{Context, TextureHandle};
use std::path::{Path, PathBuf};

pub struct ViewerWindow {
    id: egui::Id,
    title: String,
    path: PathBuf,
    document: ViewerDocument,
    texture: Option<TextureHandle>,
    current_page: i32,
    total_pages: i32,
    zoom: f32,
    pub open: bool,
    needs_render: bool,
}

impl ViewerWindow {
    pub fn open(path: &Path) -> Result<Self> {
        let document = ViewerDocument::open(path)?;
        let total_pages = document.page_count();
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Viewer")
            .to_string();
        let id = egui::Id::new(format!("viewer_{}", path.display()));

        Ok(Self {
            id,
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

    /// Zeigt das Floating Window. Returns false wenn geschlossen.
    pub fn show(&mut self, ctx: &Context) -> bool {
        if self.needs_render {
            self.render_current_page(ctx);
            self.needs_render = false;
        }

        egui::Window::new(&self.title)
            .id(self.id)
            .open(&mut self.open)
            .resizable(true)
            .default_size([600.0, 800.0])
            .show(ctx, |ui| {
                // Toolbar
                ui.horizontal(|ui| {
                    // Seitennavigation (nur bei PDFs mit mehreren Seiten)
                    if self.total_pages > 1 {
                        if ui.button("<<").clicked() && self.current_page > 0 {
                            self.current_page -= 1;
                            self.needs_render = true;
                        }
                        ui.label(format!(
                            "Seite {}/{}",
                            self.current_page + 1,
                            self.total_pages
                        ));
                        if ui.button(">>").clicked() && self.current_page < self.total_pages - 1 {
                            self.current_page += 1;
                            self.needs_render = true;
                        }
                        ui.separator();
                    }

                    // Zoom
                    if ui.button("-").clicked() && self.zoom > 0.25 {
                        self.zoom -= 0.25;
                        self.needs_render = true;
                    }
                    ui.label(format!("{:.0}%", self.zoom * 100.0));
                    if ui.button("+").clicked() && self.zoom < 4.0 {
                        self.zoom += 0.25;
                        self.needs_render = true;
                    }
                });

                ui.separator();

                // Bild anzeigen
                egui::ScrollArea::both().show(ui, |ui| {
                    if let Some(tex) = &self.texture {
                        let size = tex.size_vec2() * self.zoom;
                        ui.image(egui::load::SizedTexture::new(tex.id(), size));
                    }
                });
            });

        self.open
    }

    fn render_current_page(&mut self, ctx: &Context) {
        match self.document.render_page(self.current_page, self.zoom) {
            Ok((rgba, w, h)) => {
                self.texture = Some(renderer::rgba_to_texture(
                    ctx,
                    &format!("{}_{}", self.path.display(), self.current_page),
                    &rgba,
                    w,
                    h,
                ));
            }
            Err(e) => {
                log::error!("Failed to render page: {}", e);
            }
        }
    }
}
```

**Step 3: lib.rs aktualisieren**

```rust
// crates/freshview-viewer/src/lib.rs
pub mod document;
pub mod renderer;
pub mod window;

pub use window::ViewerWindow;
```

**Step 4: Kompilieren**

Run: `cargo check -p freshview-viewer`
Expected: Kompiliert ohne Fehler

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: mupdf floating viewer window with page navigation and zoom"
```

---

## Task 7: Integration — Editor + Viewer zusammenfuehren

Verbindet Editor und Viewer in der Hauptanwendung.
Dateierweiterungs-Check bestimmt ob Editor oder Viewer oeffnet.

**Files:**
- Modify: `crates/freshview-app/src/main.rs`

**Step 1: main.rs mit vollstaendiger Integration**

```rust
// crates/freshview-app/src/main.rs
use eframe::egui;
use freshview_editor::app::FreshEditorApp;
use freshview_viewer::ViewerWindow;
use std::path::{Path, PathBuf};

const VIEWER_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "tiff",
];

fn should_use_viewer(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIEWER_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn main() -> eframe::Result {
    env_logger::init();

    let args: Vec<String> = std::env::args().skip(1).collect();

    // Dateien in Editor-Dateien und Viewer-Dateien aufteilen
    let mut editor_files = Vec::new();
    let mut viewer_files = Vec::new();

    for arg in &args {
        let path = PathBuf::from(arg);
        if should_use_viewer(&path) {
            viewer_files.push(path);
        } else {
            editor_files.push(path);
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("FreshView"),
        ..Default::default()
    };

    eframe::run_native(
        "FreshView",
        options,
        Box::new(move |_cc| {
            let editor = FreshEditorApp::new(160, 50)
                .expect("Failed to create editor");

            let mut viewers = Vec::new();
            for path in &viewer_files {
                match ViewerWindow::open(path) {
                    Ok(v) => viewers.push(v),
                    Err(e) => log::error!("Failed to open viewer for {:?}: {}", path, e),
                }
            }

            Ok(Box::new(FreshViewApp { editor, viewers }))
        }),
    )
}

struct FreshViewApp {
    editor: FreshEditorApp,
    viewers: Vec<ViewerWindow>,
}

impl eframe::App for FreshViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.editor.tick();

        // Editor als CentralPanel
        egui::CentralPanel::default().show(ctx, |ui| {
            self.editor.show(ui);
        });

        // Viewer als Floating Windows
        self.viewers.retain_mut(|viewer| viewer.show(ctx));

        // Quit
        if self.editor.should_quit() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
```

**Step 2: Testen mit Textdatei**

Run: `cargo run -p freshview-app -- /home/g/projects/freshview/Cargo.toml`
Expected: Fenster oeffnet sich, Fresh Editor zeigt Cargo.toml

**Step 3: Testen mit PDF**

Run: `cargo run -p freshview-app -- /home/g/projects/freshview/Cargo.toml /tmp/test.pdf`
Expected: Editor zeigt Cargo.toml, Floating Window zeigt PDF

**Step 4: Testen mit Bild**

Run: `cargo run -p freshview-app -- /home/g/projects/freshview/Cargo.toml /tmp/test.png`
Expected: Editor zeigt Cargo.toml, Floating Window zeigt Bild

**Step 5: Commit**

```bash
git add -A
git commit -m "feat: integrate editor and viewer with file-type detection"
```

---

## Task 8: CLAUDE.md und Projekt-Konfiguration

**Files:**
- Create: `CLAUDE.md`
- Create: `.gitignore`

**Step 1: CLAUDE.md erstellen**

```markdown
# FreshView — Projekt-Richtlinien

## Architektur

Cross-platform IDE: Fresh Editor (in-process) + egui/egui_ratatui + mupdf Viewer.

### Crate-Struktur

| Crate | Zweck |
|-------|-------|
| `freshview-app` | Hauptanwendung, eframe, main() |
| `freshview-editor` | Fresh Editor Integration via egui_ratatui |
| `freshview-viewer` | PDF/Bild Viewer via mupdf |

### Dependencies

- **eframe/egui** — GUI Framework
- **egui_ratatui** — Ratatui Widget in egui
- **fresh-editor** — Editor-Engine (Git Dependency)
- **mupdf** — PDF/Bild Rendering (AGPL-3.0)

## Konventionen

- TDD: RED → GREEN → REFACTOR
- Rust Edition 2024
- `cargo clippy` muss warnungsfrei sein
- Fehler via `anyhow::Result`

## Build & Run

```bash
cargo run -p freshview-app                           # Leer starten
cargo run -p freshview-app -- datei.rs               # Mit Datei
cargo run -p freshview-app -- datei.rs bild.pdf      # Editor + Viewer
cargo test --workspace                                # Alle Tests
```

## Plattformen

- Fedora 43 (GNOME/Wayland)
- Windows 11
```

**Step 2: .gitignore erstellen**

```
/target
*.swp
*.swo
.DS_Store
```

**Step 3: Commit**

```bash
git add CLAUDE.md .gitignore
git commit -m "feat: add CLAUDE.md and .gitignore"
```

---

## Zusammenfassung der Tasks

| Task | Beschreibung | Abhaengigkeiten |
|------|--------------|-----------------|
| 1 | Workspace Scaffolding | — |
| 2 | egui_ratatui Hello World | Task 1 |
| 3 | Input Translation + Tests | Task 1 |
| 4 | Fresh Editor Integration | Task 2, 3 |
| 5 | mupdf Viewer — Document Wrapper | Task 1 |
| 6 | mupdf Viewer — Floating Window | Task 5 |
| 7 | Integration Editor + Viewer | Task 4, 6 |
| 8 | CLAUDE.md und Projekt-Config | Task 1 |

Tasks 2+3 und 5+6 koennen parallel ausgefuehrt werden.
Task 4 ist der kritischste — hier wird sich zeigen ob Fresh als Library funktioniert.
