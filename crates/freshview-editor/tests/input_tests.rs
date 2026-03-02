use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use freshview_editor::input::{
    translate_key, translate_modifiers, translate_mouse_click, translate_scroll,
};

#[test]
fn test_translate_simple_char() {
    let (code, mods) = translate_key(egui::Key::A, egui::Modifiers::NONE, Some('a'));
    assert_eq!(code, KeyCode::Char('a'));
    assert_eq!(mods, KeyModifiers::NONE);
}

#[test]
fn test_translate_simple_char_uppercase() {
    let (code, mods) = translate_key(egui::Key::A, egui::Modifiers::NONE, Some('A'));
    assert_eq!(code, KeyCode::Char('A'));
    assert_eq!(mods, KeyModifiers::NONE);
}

#[test]
fn test_translate_char_without_text_char() {
    // When no text_char provided, should fall back to lowercase
    let (code, _mods) = translate_key(egui::Key::A, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Char('a'));
}

#[test]
fn test_translate_ctrl_s() {
    let (code, mods) = translate_key(
        egui::Key::S,
        egui::Modifiers {
            alt: false,
            ctrl: true,
            shift: false,
            mac_cmd: false,
            command: true,
        },
        None,
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
fn test_translate_tab() {
    let (code, _) = translate_key(egui::Key::Tab, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Tab);
}

#[test]
fn test_translate_backspace() {
    let (code, _) = translate_key(egui::Key::Backspace, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Backspace);
}

#[test]
fn test_translate_delete() {
    let (code, _) = translate_key(egui::Key::Delete, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Delete);
}

#[test]
fn test_translate_escape() {
    let (code, _) = translate_key(egui::Key::Escape, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Esc);
}

#[test]
fn test_translate_home_end() {
    let (code, _) = translate_key(egui::Key::Home, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Home);
    let (code, _) = translate_key(egui::Key::End, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::End);
}

#[test]
fn test_translate_page_up_down() {
    let (code, _) = translate_key(egui::Key::PageUp, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::PageUp);
    let (code, _) = translate_key(egui::Key::PageDown, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::PageDown);
}

#[test]
fn test_translate_space() {
    let (code, _) = translate_key(egui::Key::Space, egui::Modifiers::NONE, Some(' '));
    assert_eq!(code, KeyCode::Char(' '));
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

    let (code, _) = translate_key(egui::Key::F5, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::F(5));
}

#[test]
fn test_translate_number_keys() {
    let (code, _) = translate_key(egui::Key::Num0, egui::Modifiers::NONE, Some('0'));
    assert_eq!(code, KeyCode::Char('0'));

    let (code, _) = translate_key(egui::Key::Num9, egui::Modifiers::NONE, Some('9'));
    assert_eq!(code, KeyCode::Char('9'));
}

#[test]
fn test_translate_punctuation() {
    let (code, _) = translate_key(egui::Key::Minus, egui::Modifiers::NONE, Some('-'));
    assert_eq!(code, KeyCode::Char('-'));

    let (code, _) = translate_key(egui::Key::Equals, egui::Modifiers::NONE, Some('='));
    assert_eq!(code, KeyCode::Char('='));

    let (code, _) = translate_key(egui::Key::Comma, egui::Modifiers::NONE, Some(','));
    assert_eq!(code, KeyCode::Char(','));

    let (code, _) = translate_key(egui::Key::Period, egui::Modifiers::NONE, Some('.'));
    assert_eq!(code, KeyCode::Char('.'));

    let (code, _) = translate_key(egui::Key::Slash, egui::Modifiers::NONE, Some('/'));
    assert_eq!(code, KeyCode::Char('/'));

    let (code, _) = translate_key(egui::Key::Semicolon, egui::Modifiers::NONE, Some(';'));
    assert_eq!(code, KeyCode::Char(';'));
}

#[test]
fn test_translate_insert() {
    let (code, _) = translate_key(egui::Key::Insert, egui::Modifiers::NONE, None);
    assert_eq!(code, KeyCode::Insert);
}

#[test]
fn test_translate_modifiers_ctrl_shift() {
    let mods = translate_modifiers(egui::Modifiers {
        alt: false,
        ctrl: true,
        shift: true,
        mac_cmd: false,
        command: true,
    });
    assert!(mods.contains(KeyModifiers::CONTROL));
    assert!(mods.contains(KeyModifiers::SHIFT));
    assert!(!mods.contains(KeyModifiers::ALT));
}

#[test]
fn test_translate_modifiers_alt() {
    let mods = translate_modifiers(egui::Modifiers {
        alt: true,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: false,
    });
    assert_eq!(mods, KeyModifiers::ALT);
}

#[test]
fn test_translate_modifiers_command_maps_to_control() {
    // macOS compat: command key maps to CONTROL for crossterm
    let mods = translate_modifiers(egui::Modifiers {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: true,
        command: true,
    });
    assert!(mods.contains(KeyModifiers::CONTROL));
}

#[test]
fn test_translate_modifiers_none() {
    let mods = translate_modifiers(egui::Modifiers::NONE);
    assert_eq!(mods, KeyModifiers::NONE);
}

#[test]
fn test_translate_mouse_click_primary() {
    let event = translate_mouse_click(egui::PointerButton::Primary, 10, 5);
    assert_eq!(event.kind, MouseEventKind::Down(MouseButton::Left));
    assert_eq!(event.column, 10);
    assert_eq!(event.row, 5);
}

#[test]
fn test_translate_mouse_click_secondary() {
    let event = translate_mouse_click(egui::PointerButton::Secondary, 3, 7);
    assert_eq!(event.kind, MouseEventKind::Down(MouseButton::Right));
    assert_eq!(event.column, 3);
    assert_eq!(event.row, 7);
}

#[test]
fn test_translate_mouse_click_middle() {
    let event = translate_mouse_click(egui::PointerButton::Middle, 0, 0);
    assert_eq!(event.kind, MouseEventKind::Down(MouseButton::Middle));
}

#[test]
fn test_translate_scroll_up() {
    let event = translate_scroll(1.0, 5, 10);
    assert_eq!(event.kind, MouseEventKind::ScrollUp);
    assert_eq!(event.column, 5);
    assert_eq!(event.row, 10);
}

#[test]
fn test_translate_scroll_down() {
    let event = translate_scroll(-1.0, 5, 10);
    assert_eq!(event.kind, MouseEventKind::ScrollDown);
    assert_eq!(event.column, 5);
    assert_eq!(event.row, 10);
}

#[test]
fn test_translate_scroll_zero_defaults_to_down() {
    // Edge case: zero delta defaults to ScrollDown
    let event = translate_scroll(0.0, 0, 0);
    assert_eq!(event.kind, MouseEventKind::ScrollDown);
}
