# FreshView IDE — Design Document

## Vision

Eine cross-platform IDE die den Fresh Terminal-Editor in einer nativen GUI rendert,
mit Floating-Windows fuer PDF- und Bild-Preview via mupdf.

**Kern-Idee:** Fresh als Editor-Engine (in-process) + egui/egui_ratatui als GUI + mupdf als Viewer.

```
┌─ FreshView (eframe/egui) ──────────────────────────────┐
│                                                          │
│  ┌─ CentralPanel ────────────────────────────────────┐  │
│  │  egui_ratatui::RataguiBackend                     │  │
│  │  └─ Fresh Editor (GuiApplication Trait)           │  │
│  │     LSP, Multi-Cursor, Plugins, Terminal, etc.    │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
│  ┌─ Floating Window ──┐  ┌─ Floating Window ──────┐    │
│  │  bild.png           │  │  paper.pdf              │    │
│  │  mupdf → Texture    │  │  mupdf → Texture        │    │
│  │  Zoom / Pan         │  │  Seiten-Navigation      │    │
│  └─────────────────────┘  └─────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

## Zielplattformen

| Plattform | Windowing | GPU |
|-----------|-----------|-----|
| Fedora 43 (GNOME/Wayland) | eframe via winit (Wayland-native) | wgpu oder Software |
| Windows 11 | eframe via winit (Win32) | wgpu oder Software |

## Architektur

### In-Process Modell

Fresh laeuft direkt im egui-Prozess. Kein IPC, kein Server-Daemon.
Der `GuiApplication` Trait von Fresh abstrahiert den Editor vom Backend:

```rust
// Trait aus fresh-gui (bereits vorhanden)
pub trait GuiApplication {
    fn on_key(&mut self, key: CtKeyEvent) -> Result<()>;
    fn on_mouse(&mut self, mouse: CtMouseEvent) -> Result<bool>;
    fn render(&mut self, frame: &mut ratatui::Frame);
    fn tick(&mut self) -> Result<bool>;
    fn should_quit(&self) -> bool;
}
```

Wir implementieren diesen Trait mit egui_ratatui als Rendering-Backend,
anstatt fresh-gui's winit+wgpu Ansatz.

### Datenfluss

```
Keyboard/Mouse
    │
    ▼
egui Input Events
    │
    ▼
Input-Translation (egui → crossterm KeyEvent/MouseEvent)
    │
    ▼
Fresh Editor (GuiApplication::on_key / on_mouse)
    │
    ▼
Editor State Mutation (Buffers, Cursors, LSP, Plugins)
    │
    ▼
Fresh Editor (GuiApplication::render → ratatui::Frame)
    │
    ▼
egui_ratatui::RataguiBackend (ratatui → egui Widget)
    │
    ▼
egui Rendering → Screen
```

### Datei-Oeffnung: Text vs. Viewer

```
User oeffnet Datei
    │
    ▼
Dateierweiterung pruefen
    │
    ├─ .rs .py .md .txt etc. → Fresh Editor (Text)
    │
    └─ .pdf .png .jpg .svg etc. → mupdf Viewer (Floating Window)
```

Die Erkennung erfolgt ueber Dateierweiterung.
PDF/Bild-Dateien werden NICHT im Editor geoeffnet, sondern als separates
egui::Window mit mupdf-gerenderter Textur.

## Crate-Struktur

```
freshview/
├── Cargo.toml                    # Workspace root
├── docs/
│   └── plans/
│       └── 2026-03-01-freshview-ide-design.md
├── crates/
│   ├── freshview-app/            # Hauptanwendung
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs           # eframe::run(), Event-Loop
│   │
│   ├── freshview-editor/         # Fresh-Integration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app.rs            # impl GuiApplication mit egui_ratatui
│   │       └── input.rs          # egui → crossterm Event-Translation
│   │
│   └── freshview-viewer/         # PDF/Bild Viewer
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── document.rs       # mupdf Document Wrapper
│           ├── renderer.rs       # Pixmap → egui::TextureHandle
│           └── window.rs         # Floating egui::Window pro Datei
```

## Komponenten im Detail

### 1. freshview-app

Hauptanwendung. Startet eframe, verwaltet den globalen State.

```rust
struct FreshViewApp {
    editor: FreshEditorWidget,              // freshview-editor
    viewers: Vec<ViewerWindow>,             // freshview-viewer
    pending_opens: Vec<PathBuf>,            // Dateien die geoeffnet werden sollen
}

impl eframe::App for FreshViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Editor tick (async messages, LSP, plugins)
        self.editor.tick();

        // Pruefen ob Fresh eine Datei oeffnen will
        self.process_pending_opens();

        // Editor als CentralPanel
        egui::CentralPanel::default().show(ctx, |ui| {
            self.editor.show(ui);
        });

        // Viewer als Floating Windows
        self.viewers.retain_mut(|viewer| {
            viewer.show(ctx) // false = geschlossen
        });
    }
}
```

**Dependencies:**
- `eframe`
- `freshview-editor`
- `freshview-viewer`

### 2. freshview-editor

Wrapper um Fresh's Editor mit egui_ratatui als Backend.

```rust
pub struct FreshEditorWidget {
    editor: Editor,                         // fresh-editor
    backend: RataguiBackend,                // egui_ratatui
    terminal: Terminal<RataguiBackend>,      // ratatui
}

impl FreshEditorWidget {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        // 1. egui Input → crossterm Events uebersetzen
        let events = translate_egui_input(ui.ctx());
        for event in events {
            match event {
                TranslatedEvent::Key(k) => self.editor.handle_key(k.code, k.modifiers),
                TranslatedEvent::Mouse(m) => self.editor.handle_mouse(m),
            }
        }

        // 2. Fresh rendert in ratatui Frame
        self.terminal.draw(|frame| {
            self.editor.render(frame);
        }).expect("render failed");

        // 3. RataguiBackend als egui Widget anzeigen
        ui.add(self.terminal.backend_mut());
    }

    pub fn tick(&mut self) {
        self.editor.editor_tick().ok();
    }
}
```

**Input-Translation** (egui → crossterm):

```rust
fn translate_egui_input(ctx: &egui::Context) -> Vec<TranslatedEvent> {
    // egui::Key → crossterm::event::KeyCode
    // egui::Modifiers → crossterm::event::KeyModifiers
    // egui::PointerButton → crossterm::event::MouseEvent
}
```

Fresh-gui hat diese Uebersetzung bereits implementiert (winit → crossterm).
Wir machen das gleiche fuer egui → crossterm.

**Dependencies:**
- `fresh-editor` (als Library, Feature `gui` NICHT noetig)
- `egui_ratatui`
- `soft_ratatui`
- `ratatui`
- `egui`
- `crossterm` (nur fuer Event-Typen)

### 3. freshview-viewer

Floating-Window Viewer fuer PDFs und Bilder via mupdf.

```rust
pub struct ViewerWindow {
    id: egui::Id,
    title: String,
    document: MupdfDocument,
    texture: Option<egui::TextureHandle>,
    current_page: usize,
    total_pages: usize,
    zoom: f32,
    open: bool,
}

impl ViewerWindow {
    pub fn open_pdf(path: &Path) -> Result<Self> {
        let doc = mupdf::Document::open(path)?;
        let total_pages = doc.page_count()?;
        let mut viewer = Self { /* ... */ total_pages, zoom: 1.0, open: true };
        viewer.render_page(0)?;
        Ok(viewer)
    }

    pub fn open_image(path: &Path) -> Result<Self> {
        let pixmap = mupdf::Pixmap::from_file(path)?;
        let mut viewer = Self { /* ... */ total_pages: 1, zoom: 1.0, open: true };
        viewer.set_texture_from_pixmap(&pixmap)?;
        Ok(viewer)
    }

    /// Returns false wenn das Fenster geschlossen wurde
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        egui::Window::new(&self.title)
            .id(self.id)
            .open(&mut self.open)
            .resizable(true)
            .show(ctx, |ui| {
                // Navigation (nur PDF)
                if self.total_pages > 1 {
                    ui.horizontal(|ui| {
                        if ui.button("<").clicked() { self.prev_page(); }
                        ui.label(format!("{}/{}", self.current_page + 1, self.total_pages));
                        if ui.button(">").clicked() { self.next_page(); }
                    });
                }

                // Bild/PDF-Seite anzeigen
                if let Some(tex) = &self.texture {
                    let size = tex.size_vec2() * self.zoom;
                    ui.image(egui::load::SizedTexture::new(tex.id(), size));
                }
            });
        self.open
    }

    fn render_page(&mut self, page_idx: usize) -> Result<()> {
        let page = self.document.load_page(page_idx)?;
        let matrix = mupdf::Matrix::new_scale(self.zoom * 2.0, self.zoom * 2.0);
        let pixmap = page.to_pixmap(&matrix, /* ... */)?;
        self.set_texture_from_pixmap(&pixmap)
    }
}
```

**Dependencies:**
- `mupdf` (PDF + Bild Rendering)
- `egui` (Floating Windows, Texturen)

## Dependencies (Workspace)

| Crate | Version | Zweck | Plattform |
|-------|---------|-------|-----------|
| `eframe` | 0.31+ | egui Windowing + OpenGL/wgpu | Alle |
| `egui` | 0.31+ | Immediate-Mode GUI | Alle |
| `egui_ratatui` | latest | Ratatui Widget in egui | Alle |
| `soft_ratatui` | latest | Software Font-Rendering | Alle |
| `ratatui` | 0.29+ | TUI Framework | Alle |
| `mupdf` | 0.4+ | PDF/Bild Rendering | Alle (C-Library) |
| `fresh-editor` | git | Editor-Engine | Alle |
| `fresh-core` | git | API-Typen | Alle |
| `crossterm` | 0.28+ | Event-Typen (kein Terminal-IO) | Alle |

### mupdf Cross-Platform Hinweise

- Linux: `libmupdf-dev` oder bundled via `mupdf` Crate (Feature `bundled`)
- Windows: Bundled Build (mupdf Crate kompiliert C-Quellen)
- Der `mupdf` Crate handled Cross-Compilation selbst

## Bruecke: Fresh Editor → Viewer

Wenn Fresh eine Datei oeffnet die ein Bild/PDF ist, muss FreshView das abfangen.

### Ansatz: Dateierweiterungs-Check im App-Layer

```rust
const VIEWER_EXTENSIONS: &[&str] = &[
    "pdf", "png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "tiff",
];

fn should_use_viewer(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIEWER_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
```

### Integration mit Fresh

Zwei Optionen:

**Option A — Fresh Plugin (bevorzugt):**
Ein TypeScript-Plugin in Fresh registriert einen `beforeFileOpen` Hook.
Wenn die Datei ein PDF/Bild ist, sendet es einen PluginCommand der von
FreshView abgefangen wird.

**Option B — Kommando-Interception:**
FreshView ueberwacht Fresh's Zustandsaenderungen (neue Buffer).
Wenn ein Buffer mit einer Viewer-Extension geoeffnet wird,
wird er sofort geschlossen und stattdessen ein Viewer-Window geoeffnet.

Option A ist sauberer, Option B erfordert keine Fresh-Modifikation.
Fuer den MVP starten wir mit Option B.

## Nicht im MVP-Scope

Folgende Features sind bewusst ausgeschlossen und koennen spaeter ergaenzt werden:

- File Explorer Panel
- Integriertes Terminal Panel
- Git Integration
- Session Persistence (Detach/Reattach)
- Server-Modus / IPC
- Tabs / Split Views im GUI-Layer (Fresh hat eigene Splits)
- Theming (nutzt Fresh's eingebautes Theming)

## Risiken

| Risiko | Wahrscheinlichkeit | Mitigation |
|--------|---------------------|------------|
| Fresh als Library nutzen ist nicht vorgesehen | Mittel | Fresh hat `GuiApplication` Trait, fresh-gui beweist dass es funktioniert |
| egui_ratatui Performance bei grossen Dateien | Niedrig | egui_ratatui wirbt mit "hundreds of FPS" |
| mupdf C-Library Cross-Compilation | Niedrig | `mupdf` Crate hat `bundled` Feature |
| egui Input-Translation unvollstaendig | Mittel | fresh-gui's winit→crossterm Translation als Referenz nutzen |
| Fresh API-Aenderungen brechen Integration | Mittel | Fresh als Git-Submodule pinnen |

## Erfolgskriterien MVP

1. FreshView startet als natives Fenster auf Fedora und Windows
2. Fresh Editor ist vollstaendig bedienbar (Tippen, Navigation, LSP, Suche)
3. PDF-Dateien oeffnen sich als Floating Window mit Seitennavigation
4. Bilder oeffnen sich als Floating Window mit Zoom
5. Alle Fresh-Keybindings funktionieren korrekt
