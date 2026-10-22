//! Maps winit keyboard input onto the SDL 3 scancodes/keycodes the editor's Lua API exposes.
//! Numbers come from SDL3/SDL_scancode.h and SDL3/SDL_keycode.h.

use winit::keyboard::{Key, KeyCode, KeyLocation, NamedKey, PhysicalKey};

/// SDL keycodes for named keys carry `SDL_SCANCODE_MASK` when the key has no character.
const SCANCODE_MASK: u32 = 1 << 30;

/// SDL scancode (USB HID usage) for a physical key, 0 when SDL has no equivalent.
pub fn scancode(code: PhysicalKey) -> i32 {
    let PhysicalKey::Code(code) = code else {
        return 0;
    };
    match code {
        KeyCode::Backquote => 53,
        KeyCode::Backslash => 49,
        KeyCode::BracketLeft => 47,
        KeyCode::BracketRight => 48,
        KeyCode::Comma => 54,
        KeyCode::Digit1 => 30,
        KeyCode::Digit2 => 31,
        KeyCode::Digit3 => 32,
        KeyCode::Digit4 => 33,
        KeyCode::Digit5 => 34,
        KeyCode::Digit6 => 35,
        KeyCode::Digit7 => 36,
        KeyCode::Digit8 => 37,
        KeyCode::Digit9 => 38,
        KeyCode::Digit0 => 39,
        KeyCode::Equal => 46,
        KeyCode::IntlBackslash => 100,
        KeyCode::IntlRo => 135,
        KeyCode::IntlYen => 137,
        KeyCode::KeyA => 4,
        KeyCode::KeyB => 5,
        KeyCode::KeyC => 6,
        KeyCode::KeyD => 7,
        KeyCode::KeyE => 8,
        KeyCode::KeyF => 9,
        KeyCode::KeyG => 10,
        KeyCode::KeyH => 11,
        KeyCode::KeyI => 12,
        KeyCode::KeyJ => 13,
        KeyCode::KeyK => 14,
        KeyCode::KeyL => 15,
        KeyCode::KeyM => 16,
        KeyCode::KeyN => 17,
        KeyCode::KeyO => 18,
        KeyCode::KeyP => 19,
        KeyCode::KeyQ => 20,
        KeyCode::KeyR => 21,
        KeyCode::KeyS => 22,
        KeyCode::KeyT => 23,
        KeyCode::KeyU => 24,
        KeyCode::KeyV => 25,
        KeyCode::KeyW => 26,
        KeyCode::KeyX => 27,
        KeyCode::KeyY => 28,
        KeyCode::KeyZ => 29,
        KeyCode::Minus => 45,
        KeyCode::Period => 55,
        KeyCode::Quote => 52,
        KeyCode::Semicolon => 51,
        KeyCode::Slash => 56,
        KeyCode::AltLeft => 226,
        KeyCode::AltRight => 230,
        KeyCode::Backspace => 42,
        KeyCode::CapsLock => 57,
        KeyCode::ContextMenu => 101,
        KeyCode::ControlLeft => 224,
        KeyCode::ControlRight => 228,
        KeyCode::Enter => 40,
        KeyCode::SuperLeft => 227,
        KeyCode::SuperRight => 231,
        KeyCode::ShiftLeft => 225,
        KeyCode::ShiftRight => 229,
        KeyCode::Space => 44,
        KeyCode::Tab => 43,
        KeyCode::Lang1 => 144,
        KeyCode::Lang2 => 145,
        KeyCode::Lang3 => 146,
        KeyCode::Lang4 => 147,
        KeyCode::Lang5 => 148,
        KeyCode::Delete => 76,
        KeyCode::End => 77,
        KeyCode::Help => 117,
        KeyCode::Home => 74,
        KeyCode::Insert => 73,
        KeyCode::PageDown => 78,
        KeyCode::PageUp => 75,
        KeyCode::ArrowDown => 81,
        KeyCode::ArrowLeft => 80,
        KeyCode::ArrowRight => 79,
        KeyCode::ArrowUp => 82,
        KeyCode::NumLock => 83,
        KeyCode::Numpad0 => 98,
        KeyCode::Numpad1 => 89,
        KeyCode::Numpad2 => 90,
        KeyCode::Numpad3 => 91,
        KeyCode::Numpad4 => 92,
        KeyCode::Numpad5 => 93,
        KeyCode::Numpad6 => 94,
        KeyCode::Numpad7 => 95,
        KeyCode::Numpad8 => 96,
        KeyCode::Numpad9 => 97,
        KeyCode::NumpadAdd => 87,
        KeyCode::NumpadBackspace => 187,
        KeyCode::NumpadClear => 216,
        KeyCode::NumpadClearEntry => 217,
        KeyCode::NumpadComma => 133,
        KeyCode::NumpadDecimal => 99,
        KeyCode::NumpadDivide => 84,
        KeyCode::NumpadEnter => 88,
        KeyCode::NumpadEqual => 103,
        KeyCode::NumpadHash => 204,
        KeyCode::NumpadMemoryAdd => 211,
        KeyCode::NumpadMemoryClear => 210,
        KeyCode::NumpadMemoryRecall => 209,
        KeyCode::NumpadMemoryStore => 208,
        KeyCode::NumpadMemorySubtract => 212,
        KeyCode::NumpadMultiply => 85,
        KeyCode::NumpadParenLeft => 182,
        KeyCode::NumpadParenRight => 183,
        KeyCode::NumpadSubtract => 86,
        KeyCode::Escape => 41,
        KeyCode::PrintScreen => 70,
        KeyCode::ScrollLock => 71,
        KeyCode::Pause => 72,
        KeyCode::BrowserBack => 282,
        KeyCode::BrowserFavorites => 286,
        KeyCode::BrowserForward => 283,
        KeyCode::BrowserHome => 281,
        KeyCode::BrowserRefresh => 285,
        KeyCode::BrowserSearch => 280,
        KeyCode::BrowserStop => 284,
        KeyCode::Eject => 270,
        KeyCode::MediaPlayPause => 271,
        KeyCode::MediaSelect => 272,
        KeyCode::MediaStop => 269,
        KeyCode::MediaTrackNext => 267,
        KeyCode::MediaTrackPrevious => 268,
        KeyCode::Power => 102,
        KeyCode::Sleep => 258,
        KeyCode::AudioVolumeDown => 129,
        KeyCode::AudioVolumeMute => 127,
        KeyCode::AudioVolumeUp => 128,
        KeyCode::WakeUp => 259,
        KeyCode::Again => 121,
        KeyCode::Copy => 124,
        KeyCode::Cut => 123,
        KeyCode::Find => 126,
        KeyCode::Open => 274,
        KeyCode::Paste => 125,
        KeyCode::Props => 279,
        KeyCode::Select => 119,
        KeyCode::Undo => 122,
        KeyCode::Hiragana => 147,
        KeyCode::Katakana => 146,
        KeyCode::F1 => 58,
        KeyCode::F2 => 59,
        KeyCode::F3 => 60,
        KeyCode::F4 => 61,
        KeyCode::F5 => 62,
        KeyCode::F6 => 63,
        KeyCode::F7 => 64,
        KeyCode::F8 => 65,
        KeyCode::F9 => 66,
        KeyCode::F10 => 67,
        KeyCode::F11 => 68,
        KeyCode::F12 => 69,
        KeyCode::F13 => 104,
        KeyCode::F14 => 105,
        KeyCode::F15 => 106,
        KeyCode::F16 => 107,
        KeyCode::F17 => 108,
        KeyCode::F18 => 109,
        KeyCode::F19 => 110,
        KeyCode::F20 => 111,
        KeyCode::F21 => 112,
        KeyCode::F22 => 113,
        KeyCode::F23 => 114,
        KeyCode::F24 => 115,
        _ => 0,
    }
}

/// SDL scancode for a named key; 0 when SDL has no equivalent (IME keys, F25-F35, ...).
fn named_scancode(named: NamedKey) -> i32 {
    use NamedKey as N;
    let code = match named {
        N::Alt => KeyCode::AltLeft,
        N::AltGraph => KeyCode::AltRight,
        N::CapsLock => KeyCode::CapsLock,
        N::Control => KeyCode::ControlLeft,
        N::Fn => KeyCode::Fn,
        N::FnLock => KeyCode::FnLock,
        N::NumLock => KeyCode::NumLock,
        N::ScrollLock => KeyCode::ScrollLock,
        N::Shift => KeyCode::ShiftLeft,
        N::Super => KeyCode::SuperLeft,
        N::Meta => KeyCode::Meta,
        N::Hyper => KeyCode::Hyper,
        N::Tab => KeyCode::Tab,
        N::Space => KeyCode::Space,
        N::ArrowDown => KeyCode::ArrowDown,
        N::ArrowLeft => KeyCode::ArrowLeft,
        N::ArrowRight => KeyCode::ArrowRight,
        N::ArrowUp => KeyCode::ArrowUp,
        N::End => KeyCode::End,
        N::Home => KeyCode::Home,
        N::PageDown => KeyCode::PageDown,
        N::PageUp => KeyCode::PageUp,
        N::Backspace => KeyCode::Backspace,
        N::Clear => KeyCode::NumpadClear,
        N::Copy => KeyCode::Copy,
        N::Cut => KeyCode::Cut,
        N::Delete => KeyCode::Delete,
        N::Insert => KeyCode::Insert,
        N::Paste => KeyCode::Paste,
        N::Undo => KeyCode::Undo,
        N::Again => KeyCode::Again,
        N::ContextMenu => KeyCode::ContextMenu,
        N::Escape => KeyCode::Escape,
        N::Find => KeyCode::Find,
        N::Help => KeyCode::Help,
        N::Pause => KeyCode::Pause,
        N::Props => KeyCode::Props,
        N::Select => KeyCode::Select,
        N::Eject => KeyCode::Eject,
        N::Power => KeyCode::Power,
        N::PrintScreen => KeyCode::PrintScreen,
        N::WakeUp => KeyCode::WakeUp,
        N::F1 => KeyCode::F1,
        N::F2 => KeyCode::F2,
        N::F3 => KeyCode::F3,
        N::F4 => KeyCode::F4,
        N::F5 => KeyCode::F5,
        N::F6 => KeyCode::F6,
        N::F7 => KeyCode::F7,
        N::F8 => KeyCode::F8,
        N::F9 => KeyCode::F9,
        N::F10 => KeyCode::F10,
        N::F11 => KeyCode::F11,
        N::F12 => KeyCode::F12,
        N::F13 => KeyCode::F13,
        N::F14 => KeyCode::F14,
        N::F15 => KeyCode::F15,
        N::F16 => KeyCode::F16,
        N::F17 => KeyCode::F17,
        N::F18 => KeyCode::F18,
        N::F19 => KeyCode::F19,
        N::F20 => KeyCode::F20,
        N::F21 => KeyCode::F21,
        N::F22 => KeyCode::F22,
        N::F23 => KeyCode::F23,
        N::F24 => KeyCode::F24,
        N::Hiragana => KeyCode::Hiragana,
        N::Katakana => KeyCode::Katakana,
        N::HangulMode => KeyCode::Lang1,
        N::HanjaMode => KeyCode::Lang2,
        N::KanaMode => KeyCode::KanaMode,
        N::BrowserBack => KeyCode::BrowserBack,
        N::BrowserFavorites => KeyCode::BrowserFavorites,
        N::BrowserForward => KeyCode::BrowserForward,
        N::BrowserHome => KeyCode::BrowserHome,
        N::BrowserRefresh => KeyCode::BrowserRefresh,
        N::BrowserSearch => KeyCode::BrowserSearch,
        N::BrowserStop => KeyCode::BrowserStop,
        // SDL has media scancodes for these, but winit has no matching `KeyCode`.
        N::MediaPlay => return 262,
        N::MediaPause => return 263,
        N::MediaRecord => return 264,
        N::MediaFastForward => return 265,
        N::MediaRewind => return 266,
        N::MediaPlayPause => KeyCode::MediaPlayPause,
        N::MediaStop => KeyCode::MediaStop,
        N::MediaTrackNext => KeyCode::MediaTrackNext,
        N::MediaTrackPrevious => KeyCode::MediaTrackPrevious,
        N::AudioVolumeDown => KeyCode::AudioVolumeDown,
        N::AudioVolumeMute => KeyCode::AudioVolumeMute,
        N::AudioVolumeUp => KeyCode::AudioVolumeUp,
        _ => return 0,
    };
    scancode(PhysicalKey::Code(code))
}

/// SDL keycode for the unshifted key: text keys are their lowercased codepoint.
pub fn keycode(key_without_modifiers: &Key) -> u32 {
    match key_without_modifiers {
        Key::Character(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => {
                    let mut lower = c.to_lowercase();
                    match (lower.next(), lower.next()) {
                        // SDL keeps the codepoint when lowercasing changes the char count.
                        (Some(c), None) => c as u32,
                        _ => 0,
                    }
                }
                _ => 0,
            }
        }
        Key::Named(NamedKey::Enter) => 0x0D,
        Key::Named(NamedKey::Escape) => 0x1B,
        Key::Named(NamedKey::Backspace) => 0x08,
        Key::Named(NamedKey::Tab) => 0x09,
        Key::Named(NamedKey::Space) => 0x20,
        Key::Named(NamedKey::Delete) => 0x7F,
        Key::Named(named) => match named_scancode(*named) {
            0 => 0,
            sc => sc as u32 | SCANCODE_MASK,
        },
        Key::Dead(_) | Key::Unidentified(_) => 0,
    }
}

/// SDL keypad scancodes: the `SDL_SCANCODE_KP_*` block in SDL_scancode.h.
fn is_keypad_scancode(scancode: i32) -> bool {
    matches!(scancode, 84..=99 | 103 | 133 | 176..=221)
}

/// SDL keycode for a side-agnostic modifier named key on the right side of the keyboard.
fn right_modifier_keycode(named: NamedKey) -> Option<u32> {
    let code = match named {
        NamedKey::Control => KeyCode::ControlRight,
        NamedKey::Shift => KeyCode::ShiftRight,
        NamedKey::Alt => KeyCode::AltRight,
        NamedKey::Super => KeyCode::SuperRight,
        _ => return None,
    };
    Some(scancode(PhysicalKey::Code(code)) as u32 | SCANCODE_MASK)
}

/// One winit key event as the SDL codes the Lua API exposes: `(scancode, keycode, numlock)`.
///
/// Keycodes come from the layout (`key_without_modifiers`), except where only the event knows
/// better: a keypad physical key keeps its physical scancode keycode whatever the layout types,
/// and a right-side modifier gets the right-side code. `numlock` is the effective keypad level,
/// not a lock-state query (the user-approved rule for unusual XKB keypad variants).
pub fn key_event_codes(
    physical_key: PhysicalKey,
    location: KeyLocation,
    key_without_modifiers: &Key,
    logical_key: &Key,
) -> (i32, u32, bool) {
    let scancode = scancode(physical_key);
    let keycode = match (location, key_without_modifiers) {
        (KeyLocation::Numpad, _) if is_keypad_scancode(scancode) => scancode as u32 | SCANCODE_MASK,
        // AltGraph is already AltRight, which right_modifier_keycode does not shift again.
        (KeyLocation::Right, Key::Named(named)) => {
            right_modifier_keycode(*named).unwrap_or_else(|| keycode(key_without_modifiers))
        }
        _ => keycode(key_without_modifiers),
    };
    // shortcut: winit omits NumLock state; revisit this inference if it exposes locked modifiers.
    let numlock = location == KeyLocation::Numpad && matches!(logical_key, Key::Character(_));
    (scancode, keycode, numlock)
}

#[cfg(test)]
mod tests {
    use super::*;

    // SDL_scancode.h: every KeyCode that has a scancode.
    const SCANCODES: &[(KeyCode, i32)] = &[
        (KeyCode::KeyA, 4),
        (KeyCode::KeyB, 5),
        (KeyCode::KeyC, 6),
        (KeyCode::KeyD, 7),
        (KeyCode::KeyE, 8),
        (KeyCode::KeyF, 9),
        (KeyCode::KeyG, 10),
        (KeyCode::KeyH, 11),
        (KeyCode::KeyI, 12),
        (KeyCode::KeyJ, 13),
        (KeyCode::KeyK, 14),
        (KeyCode::KeyL, 15),
        (KeyCode::KeyM, 16),
        (KeyCode::KeyN, 17),
        (KeyCode::KeyO, 18),
        (KeyCode::KeyP, 19),
        (KeyCode::KeyQ, 20),
        (KeyCode::KeyR, 21),
        (KeyCode::KeyS, 22),
        (KeyCode::KeyT, 23),
        (KeyCode::KeyU, 24),
        (KeyCode::KeyV, 25),
        (KeyCode::KeyW, 26),
        (KeyCode::KeyX, 27),
        (KeyCode::KeyY, 28),
        (KeyCode::KeyZ, 29),
        (KeyCode::Digit1, 30),
        (KeyCode::Digit2, 31),
        (KeyCode::Digit3, 32),
        (KeyCode::Digit4, 33),
        (KeyCode::Digit5, 34),
        (KeyCode::Digit6, 35),
        (KeyCode::Digit7, 36),
        (KeyCode::Digit8, 37),
        (KeyCode::Digit9, 38),
        (KeyCode::Digit0, 39),
        (KeyCode::Enter, 40),
        (KeyCode::Escape, 41),
        (KeyCode::Backspace, 42),
        (KeyCode::Tab, 43),
        (KeyCode::Space, 44),
        (KeyCode::Minus, 45),
        (KeyCode::Equal, 46),
        (KeyCode::BracketLeft, 47),
        (KeyCode::BracketRight, 48),
        (KeyCode::Backslash, 49),
        (KeyCode::Semicolon, 51),
        (KeyCode::Quote, 52),
        (KeyCode::Backquote, 53),
        (KeyCode::Comma, 54),
        (KeyCode::Period, 55),
        (KeyCode::Slash, 56),
        (KeyCode::CapsLock, 57),
        (KeyCode::F1, 58),
        (KeyCode::F2, 59),
        (KeyCode::F3, 60),
        (KeyCode::F4, 61),
        (KeyCode::F5, 62),
        (KeyCode::F6, 63),
        (KeyCode::F7, 64),
        (KeyCode::F8, 65),
        (KeyCode::F9, 66),
        (KeyCode::F10, 67),
        (KeyCode::F11, 68),
        (KeyCode::F12, 69),
        (KeyCode::PrintScreen, 70),
        (KeyCode::ScrollLock, 71),
        (KeyCode::Pause, 72),
        (KeyCode::Insert, 73),
        (KeyCode::Home, 74),
        (KeyCode::PageUp, 75),
        (KeyCode::Delete, 76),
        (KeyCode::End, 77),
        (KeyCode::PageDown, 78),
        (KeyCode::ArrowRight, 79),
        (KeyCode::ArrowLeft, 80),
        (KeyCode::ArrowDown, 81),
        (KeyCode::ArrowUp, 82),
        (KeyCode::NumLock, 83),
        (KeyCode::NumpadDivide, 84),
        (KeyCode::NumpadMultiply, 85),
        (KeyCode::NumpadSubtract, 86),
        (KeyCode::NumpadAdd, 87),
        (KeyCode::NumpadEnter, 88),
        (KeyCode::Numpad1, 89),
        (KeyCode::Numpad2, 90),
        (KeyCode::Numpad3, 91),
        (KeyCode::Numpad4, 92),
        (KeyCode::Numpad5, 93),
        (KeyCode::Numpad6, 94),
        (KeyCode::Numpad7, 95),
        (KeyCode::Numpad8, 96),
        (KeyCode::Numpad9, 97),
        (KeyCode::Numpad0, 98),
        (KeyCode::NumpadDecimal, 99),
        (KeyCode::IntlBackslash, 100),
        (KeyCode::ContextMenu, 101),
        (KeyCode::Power, 102),
        (KeyCode::NumpadEqual, 103),
        (KeyCode::F13, 104),
        (KeyCode::F14, 105),
        (KeyCode::F15, 106),
        (KeyCode::F16, 107),
        (KeyCode::F17, 108),
        (KeyCode::F18, 109),
        (KeyCode::F19, 110),
        (KeyCode::F20, 111),
        (KeyCode::F21, 112),
        (KeyCode::F22, 113),
        (KeyCode::F23, 114),
        (KeyCode::F24, 115),
        (KeyCode::Help, 117),
        (KeyCode::Select, 119),
        (KeyCode::Again, 121),
        (KeyCode::Undo, 122),
        (KeyCode::Cut, 123),
        (KeyCode::Copy, 124),
        (KeyCode::Paste, 125),
        (KeyCode::Find, 126),
        (KeyCode::AudioVolumeMute, 127),
        (KeyCode::AudioVolumeUp, 128),
        (KeyCode::AudioVolumeDown, 129),
        (KeyCode::NumpadComma, 133),
        (KeyCode::IntlRo, 135),
        (KeyCode::IntlYen, 137),
        (KeyCode::Lang1, 144),
        (KeyCode::Lang2, 145),
        (KeyCode::Lang3, 146),
        (KeyCode::Lang4, 147),
        (KeyCode::Lang5, 148),
        (KeyCode::NumpadParenLeft, 182),
        (KeyCode::NumpadParenRight, 183),
        (KeyCode::NumpadBackspace, 187),
        (KeyCode::NumpadHash, 204),
        (KeyCode::NumpadMemoryStore, 208),
        (KeyCode::NumpadMemoryRecall, 209),
        (KeyCode::NumpadMemoryClear, 210),
        (KeyCode::NumpadMemoryAdd, 211),
        (KeyCode::NumpadMemorySubtract, 212),
        (KeyCode::NumpadClear, 216),
        (KeyCode::NumpadClearEntry, 217),
        (KeyCode::ControlLeft, 224),
        (KeyCode::ShiftLeft, 225),
        (KeyCode::AltLeft, 226),
        (KeyCode::SuperLeft, 227),
        (KeyCode::ControlRight, 228),
        (KeyCode::ShiftRight, 229),
        (KeyCode::AltRight, 230),
        (KeyCode::SuperRight, 231),
        (KeyCode::Sleep, 258),
        (KeyCode::WakeUp, 259),
        (KeyCode::MediaTrackNext, 267),
        (KeyCode::MediaTrackPrevious, 268),
        (KeyCode::MediaStop, 269),
        (KeyCode::Eject, 270),
        (KeyCode::MediaPlayPause, 271),
        (KeyCode::MediaSelect, 272),
        (KeyCode::Open, 274),
        (KeyCode::Props, 279),
        (KeyCode::BrowserSearch, 280),
        (KeyCode::BrowserHome, 281),
        (KeyCode::BrowserBack, 282),
        (KeyCode::BrowserForward, 283),
        (KeyCode::BrowserStop, 284),
        (KeyCode::BrowserRefresh, 285),
        (KeyCode::BrowserFavorites, 286),
        (KeyCode::Hiragana, 147),
        (KeyCode::Katakana, 146),
    ];

    #[test]
    fn scancodes_match_sdl() {
        for &(code, expected) in SCANCODES {
            assert_eq!(scancode(PhysicalKey::Code(code)), expected, "{code:?}");
        }
    }

    #[test]
    fn keys_without_scancode_map_to_zero() {
        for code in [
            KeyCode::Fn,
            KeyCode::FnLock,
            KeyCode::Convert,
            KeyCode::KanaMode,
            KeyCode::NonConvert,
            KeyCode::NumpadStar,
            KeyCode::LaunchApp1,
            KeyCode::LaunchApp2,
            KeyCode::LaunchMail,
            KeyCode::Meta,
            KeyCode::Hyper,
            KeyCode::Turbo,
            KeyCode::Abort,
            KeyCode::Resume,
            KeyCode::Suspend,
            KeyCode::F25,
            KeyCode::F35,
        ] {
            assert_eq!(scancode(PhysicalKey::Code(code)), 0, "{code:?}");
        }
    }

    #[test]
    fn contract_scancode_key_pairs() {
        // tests/contract/cases/sysevents.lua: (scancode, keycode) per key.
        let named = &[
            (KeyCode::KeyA, 4, "a", 97),
            (KeyCode::KeyZ, 29, "z", 122),
            (KeyCode::Digit0, 39, "0", 48),
            (KeyCode::Space, 44, " ", 32),
            (KeyCode::Enter, 40, "", 0x0D),
            (KeyCode::Escape, 41, "", 0x1B),
            (KeyCode::Tab, 43, "", 9),
            (KeyCode::Backspace, 42, "", 8),
            (KeyCode::Delete, 76, "", 0x7F),
            (KeyCode::BracketLeft, 47, "[", 91),
            (KeyCode::Backquote, 53, "`", 96),
        ];
        for &(code, sc, text, expected_key) in named {
            assert_eq!(scancode(PhysicalKey::Code(code)), sc, "{code:?}");
            let key = if text.is_empty() {
                Key::Named(match code {
                    KeyCode::Enter => NamedKey::Enter,
                    KeyCode::Escape => NamedKey::Escape,
                    KeyCode::Tab => NamedKey::Tab,
                    KeyCode::Backspace => NamedKey::Backspace,
                    KeyCode::Delete => NamedKey::Delete,
                    other => panic!("no named key for {other:?}"),
                })
            } else {
                Key::Character(text.into())
            };
            assert_eq!(keycode(&key), expected_key, "{code:?}");
        }

        let masked = &[
            (NamedKey::F1, 0x4000003A),
            (NamedKey::ArrowLeft, 0x40000050),
            (NamedKey::ArrowRight, 0x4000004F),
            (NamedKey::Control, 0x400000E0),
            (NamedKey::Shift, 0x400000E1),
            (NamedKey::Alt, 0x400000E2),
            (NamedKey::Super, 0x400000E3),
            (NamedKey::Home, 0x4000004A),
            (NamedKey::PageDown, 0x4000004E),
            (NamedKey::CapsLock, 0x40000039),
            (NamedKey::PrintScreen, 0x40000046),
            (NamedKey::MediaPlay, 0x40000106),
        ];
        for &(named_key, expected) in masked {
            assert_eq!(keycode(&Key::Named(named_key)), expected, "{named_key:?}");
        }

        // Numeric keypad keys use their scancode with the mask.
        assert_eq!(keycode(&Key::Character("\u{0}".into())), 0);
        assert_eq!(SCANCODE_MASK | 208, 0x400000D0);
        assert_eq!(SCANCODE_MASK | 215, 0x400000D7);
    }

    #[test]
    fn named_scancode_pairs() {
        // Keys winit names but SDL has no scancode for.
        for named in [
            NamedKey::Convert,
            NamedKey::NonConvert,
            NamedKey::Fn,
            NamedKey::FnLock,
            NamedKey::Meta,
            NamedKey::Hyper,
            NamedKey::PowerOff,
            NamedKey::Standby,
            NamedKey::Hibernate,
        ] {
            assert_eq!(named_scancode(named), 0, "{named:?}");
        }
        assert_eq!(named_scancode(NamedKey::HangulMode), 144);
        assert_eq!(named_scancode(NamedKey::HanjaMode), 145);
        assert_eq!(named_scancode(NamedKey::Katakana), 146);
        assert_eq!(named_scancode(NamedKey::Hiragana), 147);
        assert_eq!(named_scancode(NamedKey::MediaPlay), 262);
        assert_eq!(named_scancode(NamedKey::MediaRewind), 266);
    }

    #[test]
    fn character_keys_lowercase_the_codepoint() {
        for &(text, expected) in &[
            ("a", 97),
            ("A", 97),
            ("Z", 122),
            ("[", 91),
            ("~", 126),
            ("\u{0444}", 0x0444),
            ("\u{03B2}", 0x03B2),
            ("\u{65E5}", 0x65E5),
            ("", 0),
            ("ab", 0),
        ] {
            assert_eq!(keycode(&Key::Character(text.into())), expected, "{text:?}");
        }
        assert_eq!(keycode(&Key::Dead(Some('`'))), 0);
    }

    #[test]
    fn keypad_digits_report_effective_numlock_codes() {
        let keypad = [
            (KeyCode::Numpad1, 89, "1", NamedKey::End, "end"),
            (KeyCode::Numpad2, 90, "2", NamedKey::ArrowDown, "down"),
            (KeyCode::Numpad3, 91, "3", NamedKey::PageDown, "pagedown"),
            (KeyCode::Numpad4, 92, "4", NamedKey::ArrowLeft, "left"),
            (KeyCode::Numpad5, 93, "5", NamedKey::Clear, ""),
            (KeyCode::Numpad6, 94, "6", NamedKey::ArrowRight, "right"),
            (KeyCode::Numpad7, 95, "7", NamedKey::Home, "home"),
            (KeyCode::Numpad8, 96, "8", NamedKey::ArrowUp, "up"),
            (KeyCode::Numpad9, 97, "9", NamedKey::PageUp, "pageup"),
            (KeyCode::Numpad0, 98, "0", NamedKey::Insert, "ins"),
            (KeyCode::NumpadDecimal, 99, ".", NamedKey::Delete, "delete"),
        ];
        for (code, sc, digit, nav, nav_name) in keypad {
            // Linux winit uses XKB level zero even when NumLock is on; KP_Begin is unidentified.
            let unmodified = if code == KeyCode::Numpad5 {
                Key::Unidentified(winit::keyboard::NativeKey::Unidentified)
            } else {
                Key::Named(nav)
            };
            for (logical, expected, numlock) in [
                (Key::Character(digit.into()), format!("keypad {digit}"), true),
                (unmodified.clone(), nav_name.to_owned(), false),
            ] {
                let codes = key_event_codes(PhysicalKey::Code(code), KeyLocation::Numpad, &unmodified, &logical);
                assert_eq!(codes, (sc, SCANCODE_MASK | sc as u32, numlock), "{code:?}");
                assert_eq!(crate::keynames::key_name(codes.0, codes.1, codes.2), expected, "{code:?}");
            }
        }
    }

    #[test]
    fn keypad_operators_use_physical_codes() {
        let operators = [
            (KeyCode::NumpadDivide, 84, Key::Character("/".into()), "keypad /"),
            (KeyCode::NumpadMultiply, 85, Key::Character("*".into()), "keypad *"),
            (KeyCode::NumpadSubtract, 86, Key::Character("-".into()), "keypad -"),
            (KeyCode::NumpadAdd, 87, Key::Character("+".into()), "keypad +"),
            (KeyCode::NumpadEnter, 88, Key::Named(NamedKey::Enter), "keypad enter"),
            (KeyCode::NumpadEqual, 103, Key::Character("=".into()), "keypad ="),
            (KeyCode::NumpadComma, 133, Key::Character(",".into()), "keypad ,"),
        ];
        for (code, sc, key, name) in operators {
            let codes = key_event_codes(PhysicalKey::Code(code), KeyLocation::Numpad, &key, &key);
            assert_eq!((codes.0, codes.1), (sc, SCANCODE_MASK | sc as u32), "{code:?}");
            assert_eq!(crate::keynames::key_name(codes.0, codes.1, codes.2), name, "{code:?}");
        }
    }

    #[test]
    fn right_side_modifiers_use_the_right_code() {
        let modifiers = [
            (KeyCode::ControlLeft, KeyLocation::Left, NamedKey::Control, 224, "left ctrl"),
            (KeyCode::ControlRight, KeyLocation::Right, NamedKey::Control, 228, "right ctrl"),
            (KeyCode::ShiftLeft, KeyLocation::Left, NamedKey::Shift, 225, "left shift"),
            (KeyCode::ShiftRight, KeyLocation::Right, NamedKey::Shift, 229, "right shift"),
            (KeyCode::AltLeft, KeyLocation::Left, NamedKey::Alt, 226, "left alt"),
            (KeyCode::AltRight, KeyLocation::Right, NamedKey::Alt, 230, "right alt"),
            (KeyCode::AltRight, KeyLocation::Right, NamedKey::AltGraph, 230, "right alt"),
            (KeyCode::SuperLeft, KeyLocation::Left, NamedKey::Super, 227, "left gui"),
            (KeyCode::SuperRight, KeyLocation::Right, NamedKey::Super, 231, "right gui"),
        ];
        for (code, location, named, sc, name) in modifiers {
            let key = Key::Named(named);
            let codes = key_event_codes(PhysicalKey::Code(code), location, &key, &key);
            assert_eq!(codes, (sc, SCANCODE_MASK | sc as u32, false), "{code:?}");
            assert_eq!(crate::keynames::key_name(codes.0, codes.1, codes.2), name, "{code:?}");
        }
    }

    #[test]
    fn layout_decides_non_keypad_keycodes() {
        for (code, plain, shifted, sc) in [
            (KeyCode::KeyA, "a", "A", 4), // Shift/Caps do not change the key's plain name.
            (KeyCode::Digit1, "1", "!", 30),
            (KeyCode::KeyY, "z", "Z", 28), // Layout remap, not QWERTY physical identity.
        ] {
            let codes = key_event_codes(
                PhysicalKey::Code(code), KeyLocation::Standard,
                &Key::Character(plain.into()), &Key::Character(shifted.into()),
            );
            assert_eq!(codes, (sc, plain.as_bytes()[0] as u32, false));
            assert_eq!(crate::keynames::key_name(codes.0, codes.1, codes.2), plain);
        }
    }

    #[test]
    fn unknown_physical_keys_fall_back_to_the_layout() {
        let unknown = PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Xkb(9));
        let key = Key::Character("x".into());
        assert_eq!(key_event_codes(unknown, KeyLocation::Standard, &key, &key), (0, 'x' as u32, false));

        let key = Key::Named(NamedKey::F25);
        assert_eq!(
            key_event_codes(PhysicalKey::Code(KeyCode::F25), KeyLocation::Standard, &key, &key),
            (0, 0, false),
        );
    }

    #[test]
    fn standard_navigation_keys_stay_named_keys() {
        for (code, sc, named, key, name) in [
            (KeyCode::Home, 74, NamedKey::Home, SCANCODE_MASK | 74, "home"),
            (KeyCode::End, 77, NamedKey::End, SCANCODE_MASK | 77, "end"),
            (KeyCode::PageUp, 75, NamedKey::PageUp, SCANCODE_MASK | 75, "pageup"),
            (KeyCode::ArrowDown, 81, NamedKey::ArrowDown, SCANCODE_MASK | 81, "down"),
            (KeyCode::Delete, 76, NamedKey::Delete, 0x7F, "delete"),
        ] {
            let named = Key::Named(named);
            let codes = key_event_codes(PhysicalKey::Code(code), KeyLocation::Standard, &named, &named);
            assert_eq!(codes, (sc, key, false), "{code:?}");
            assert_eq!(crate::keynames::key_name(codes.0, codes.1, codes.2), name, "{code:?}");
        }
    }
}
