//! Translates egui keyboard/mouse events into crossterm event types.
//!
//! This module bridges the gap between egui's input system and crossterm's
//! event types, which are used by the Fresh terminal editor.

use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

/// Translate egui Key + Modifiers into crossterm KeyCode + KeyModifiers.
///
/// `text_char` is the actual character produced by the key press (if available),
/// which respects keyboard layout and shift state. When provided for character
/// keys (letters, digits, punctuation), it takes precedence over the logical key.
pub fn translate_key(
    key: egui::Key,
    modifiers: egui::Modifiers,
    text_char: Option<char>,
) -> (KeyCode, KeyModifiers) {
    let code = translate_keycode(key, text_char);
    let mods = translate_modifiers(modifiers);
    (code, mods)
}

/// Translate egui Modifiers into crossterm KeyModifiers.
///
/// Maps `ctrl` and `command` (macOS Cmd) to `CONTROL`, `shift` to `SHIFT`,
/// and `alt` to `ALT`. The `mac_cmd` field also maps to `CONTROL` for
/// cross-platform compatibility.
pub fn translate_modifiers(m: egui::Modifiers) -> KeyModifiers {
    let mut mods = KeyModifiers::NONE;
    if m.ctrl || m.command || m.mac_cmd {
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

/// Translate a mouse click into a crossterm MouseEvent.
pub fn translate_mouse_click(button: egui::PointerButton, col: u16, row: u16) -> MouseEvent {
    let mouse_button = match button {
        egui::PointerButton::Primary => MouseButton::Left,
        egui::PointerButton::Secondary => MouseButton::Right,
        egui::PointerButton::Middle => MouseButton::Middle,
        // Extra buttons map to Left as a fallback (crossterm only has 3 buttons)
        _ => MouseButton::Left,
    };
    MouseEvent {
        kind: MouseEventKind::Down(mouse_button),
        column: col,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

/// Translate a scroll event into a crossterm MouseEvent.
///
/// Positive `delta_y` maps to ScrollUp, negative or zero to ScrollDown.
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

/// Translate an egui Key into a crossterm KeyCode, using `text_char` when available.
fn translate_keycode(key: egui::Key, text_char: Option<char>) -> KeyCode {
    match key {
        // Navigation
        egui::Key::ArrowDown => KeyCode::Down,
        egui::Key::ArrowUp => KeyCode::Up,
        egui::Key::ArrowLeft => KeyCode::Left,
        egui::Key::ArrowRight => KeyCode::Right,

        // Command keys
        egui::Key::Escape => KeyCode::Esc,
        egui::Key::Tab => KeyCode::Tab,
        egui::Key::Backspace => KeyCode::Backspace,
        egui::Key::Enter => KeyCode::Enter,
        egui::Key::Insert => KeyCode::Insert,
        egui::Key::Delete => KeyCode::Delete,
        egui::Key::Home => KeyCode::Home,
        egui::Key::End => KeyCode::End,
        egui::Key::PageUp => KeyCode::PageUp,
        egui::Key::PageDown => KeyCode::PageDown,

        // Space
        egui::Key::Space => KeyCode::Char(text_char.unwrap_or(' ')),

        // Punctuation — use text_char if available, otherwise default char
        egui::Key::Colon => KeyCode::Char(text_char.unwrap_or(':')),
        egui::Key::Comma => KeyCode::Char(text_char.unwrap_or(',')),
        egui::Key::Backslash => KeyCode::Char(text_char.unwrap_or('\\')),
        egui::Key::Slash => KeyCode::Char(text_char.unwrap_or('/')),
        egui::Key::Pipe => KeyCode::Char(text_char.unwrap_or('|')),
        egui::Key::Questionmark => KeyCode::Char(text_char.unwrap_or('?')),
        egui::Key::OpenBracket => KeyCode::Char(text_char.unwrap_or('[')),
        egui::Key::CloseBracket => KeyCode::Char(text_char.unwrap_or(']')),
        egui::Key::Backtick => KeyCode::Char(text_char.unwrap_or('`')),
        egui::Key::Minus => KeyCode::Char(text_char.unwrap_or('-')),
        egui::Key::Period => KeyCode::Char(text_char.unwrap_or('.')),
        egui::Key::Plus => KeyCode::Char(text_char.unwrap_or('+')),
        egui::Key::Equals => KeyCode::Char(text_char.unwrap_or('=')),
        egui::Key::Semicolon => KeyCode::Char(text_char.unwrap_or(';')),
        egui::Key::Quote => KeyCode::Char(text_char.unwrap_or('\'')),

        // Digits
        egui::Key::Num0 => KeyCode::Char(text_char.unwrap_or('0')),
        egui::Key::Num1 => KeyCode::Char(text_char.unwrap_or('1')),
        egui::Key::Num2 => KeyCode::Char(text_char.unwrap_or('2')),
        egui::Key::Num3 => KeyCode::Char(text_char.unwrap_or('3')),
        egui::Key::Num4 => KeyCode::Char(text_char.unwrap_or('4')),
        egui::Key::Num5 => KeyCode::Char(text_char.unwrap_or('5')),
        egui::Key::Num6 => KeyCode::Char(text_char.unwrap_or('6')),
        egui::Key::Num7 => KeyCode::Char(text_char.unwrap_or('7')),
        egui::Key::Num8 => KeyCode::Char(text_char.unwrap_or('8')),
        egui::Key::Num9 => KeyCode::Char(text_char.unwrap_or('9')),

        // Letters — use text_char to preserve case, fallback to lowercase
        egui::Key::A => KeyCode::Char(text_char.unwrap_or('a')),
        egui::Key::B => KeyCode::Char(text_char.unwrap_or('b')),
        egui::Key::C => KeyCode::Char(text_char.unwrap_or('c')),
        egui::Key::D => KeyCode::Char(text_char.unwrap_or('d')),
        egui::Key::E => KeyCode::Char(text_char.unwrap_or('e')),
        egui::Key::F => KeyCode::Char(text_char.unwrap_or('f')),
        egui::Key::G => KeyCode::Char(text_char.unwrap_or('g')),
        egui::Key::H => KeyCode::Char(text_char.unwrap_or('h')),
        egui::Key::I => KeyCode::Char(text_char.unwrap_or('i')),
        egui::Key::J => KeyCode::Char(text_char.unwrap_or('j')),
        egui::Key::K => KeyCode::Char(text_char.unwrap_or('k')),
        egui::Key::L => KeyCode::Char(text_char.unwrap_or('l')),
        egui::Key::M => KeyCode::Char(text_char.unwrap_or('m')),
        egui::Key::N => KeyCode::Char(text_char.unwrap_or('n')),
        egui::Key::O => KeyCode::Char(text_char.unwrap_or('o')),
        egui::Key::P => KeyCode::Char(text_char.unwrap_or('p')),
        egui::Key::Q => KeyCode::Char(text_char.unwrap_or('q')),
        egui::Key::R => KeyCode::Char(text_char.unwrap_or('r')),
        egui::Key::S => KeyCode::Char(text_char.unwrap_or('s')),
        egui::Key::T => KeyCode::Char(text_char.unwrap_or('t')),
        egui::Key::U => KeyCode::Char(text_char.unwrap_or('u')),
        egui::Key::V => KeyCode::Char(text_char.unwrap_or('v')),
        egui::Key::W => KeyCode::Char(text_char.unwrap_or('w')),
        egui::Key::X => KeyCode::Char(text_char.unwrap_or('x')),
        egui::Key::Y => KeyCode::Char(text_char.unwrap_or('y')),
        egui::Key::Z => KeyCode::Char(text_char.unwrap_or('z')),

        // Function keys
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
        egui::Key::F13 => KeyCode::F(13),
        egui::Key::F14 => KeyCode::F(14),
        egui::Key::F15 => KeyCode::F(15),
        egui::Key::F16 => KeyCode::F(16),
        egui::Key::F17 => KeyCode::F(17),
        egui::Key::F18 => KeyCode::F(18),
        egui::Key::F19 => KeyCode::F(19),
        egui::Key::F20 => KeyCode::F(20),
        egui::Key::F21 => KeyCode::F(21),
        egui::Key::F22 => KeyCode::F(22),
        egui::Key::F23 => KeyCode::F(23),
        egui::Key::F24 => KeyCode::F(24),
        egui::Key::F25 => KeyCode::F(25),
        egui::Key::F26 => KeyCode::F(26),
        egui::Key::F27 => KeyCode::F(27),
        egui::Key::F28 => KeyCode::F(28),
        egui::Key::F29 => KeyCode::F(29),
        egui::Key::F30 => KeyCode::F(30),
        egui::Key::F31 => KeyCode::F(31),
        egui::Key::F32 => KeyCode::F(32),
        egui::Key::F33 => KeyCode::F(33),
        egui::Key::F34 => KeyCode::F(34),
        egui::Key::F35 => KeyCode::F(35),

        // Clipboard keys — map to crossterm Char equivalents (handled by editor)
        egui::Key::Copy => KeyCode::Char('c'),
        egui::Key::Cut => KeyCode::Char('x'),
        egui::Key::Paste => KeyCode::Char('v'),
    }
}
