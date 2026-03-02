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

- **eframe/egui 0.33** — GUI Framework
- **egui_ratatui 2.1** — Ratatui Widget in egui
- **soft_ratatui 0.1** — Software Font-Rendering (mono_8x13)
- **ratatui 0.30** — TUI Framework
- **fresh-editor** — Editor-Engine (path dep: ../fresh/crates/fresh-editor)
- **mupdf 0.6** — PDF/Bild Rendering (AGPL-3.0)
- **crossterm 0.29** — Event-Typen (kein Terminal-IO)

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

## Zell-Dimensionen

mono_8x13 Font: 8px breit, 13px hoch. Wird in freshview-editor/src/app.rs fuer
Pixel→Zell-Koordinaten-Umrechnung verwendet.
