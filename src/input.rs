use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, NamedKey};

pub fn handle_key_input(
    logical_key: &Key,
    text: Option<&str>,
    state: ElementState,
    modifiers: ModifiersState,
) -> Option<Vec<u8>> {
    if state != ElementState::Pressed {
        return None;
    }

    // 1. AltGr (Windows sends both Ctrl and Alt for AltGr, used in ABNT2 and international layouts)
    if modifiers.control_key() && modifiers.alt_key() {
        if let Some(t) = text {
            return Some(t.as_bytes().to_vec());
        }
        return None;
    }

    // 2. Normal Alt (without Ctrl)
    if modifiers.alt_key() {
        if matches!(logical_key, Key::Named(NamedKey::Space)) {
            return None;
        }
        // Other Alt key combinations sent as ESC + char if text exists
        if let Some(t) = text {
            let mut bytes = vec![0x1b];
            bytes.extend_from_slice(t.as_bytes());
            return Some(bytes);
        }
        return None;
    }

    // 3. Normal Ctrl (without Alt)
    if modifiers.control_key() {
        // Prevent Ctrl + ' / ` / " from ever sending characters to shell
        if let Some(t) = text {
            if t == "'" || t == "`" || t == "´" || t == "\"" {
                return None;
            }
        }
        match logical_key {
            Key::Character(s) if s == "'" || s == "`" || s == "´" || s == "\"" => return None,
            Key::Dead(Some('\'') | Some('`') | Some('´') | Some('"')) => return None,
            Key::Named(NamedKey::Backspace) => return Some(vec![0x08]), // Ctrl+Backspace: delete word backward
            Key::Named(NamedKey::Space) => return Some(vec![0x00]),     // Ctrl+Space: NUL
            Key::Named(NamedKey::PageUp) | Key::Named(NamedKey::PageDown) | Key::Named(NamedKey::Tab) => {
                return None;
            }
            Key::Named(NamedKey::ArrowLeft) => return Some(b"\x1b[1;5D".to_vec()),  // Word backward
            Key::Named(NamedKey::ArrowRight) => return Some(b"\x1b[1;5C".to_vec()), // Word forward
            Key::Named(NamedKey::ArrowUp) => return Some(b"\x1b[1;5A".to_vec()),
            Key::Named(NamedKey::ArrowDown) => return Some(b"\x1b[1;5B".to_vec()),
            Key::Named(NamedKey::Delete) => return Some(b"\x1b[3;5~".to_vec()), // Delete word forward
            Key::Named(NamedKey::Home) => return Some(b"\x1b[1;5H".to_vec()),
            Key::Named(NamedKey::End) => return Some(b"\x1b[1;5F".to_vec()),
            Key::Character(s) => {
                if let Some(c) = s.chars().next() {
                    if c.is_ascii_control() {
                        return Some(vec![c as u8]);
                    }
                    let lower = c.to_ascii_lowercase();
                    if lower.is_ascii_lowercase() {
                        let code = (lower as u8) - b'a' + 1;
                        return Some(vec![code]);
                    }
                }
            }
            _ => {}
        }
        return None;
    }

    // 4. Special & Named Keys (No Ctrl, No Alt)
    match logical_key {
        Key::Named(NamedKey::Enter) => Some(vec![b'\r']),
        Key::Named(NamedKey::Backspace) => Some(vec![0x7f]), // Normal Backspace
        Key::Named(NamedKey::Tab) => {
            if modifiers.shift_key() {
                Some(b"\x1b[Z".to_vec()) // Shift+Tab
            } else {
                Some(vec![b'\t'])
            }
        }
        Key::Named(NamedKey::Space) => {
            // When user types a dead key followed by Space (e.g. ~ + space -> ~),
            // winit delivers logical_key = Space and text = Some("~")
            if let Some(t) = text {
                if t != " " {
                    Some(t.as_bytes().to_vec())
                } else {
                    Some(vec![b' '])
                }
            } else {
                Some(vec![b' '])
            }
        }
        Key::Named(NamedKey::Escape) => Some(vec![0x1b]),
        Key::Named(NamedKey::F1) => None,
        Key::Named(NamedKey::ArrowUp) => {
            if modifiers.shift_key() {
                Some(b"\x1b[1;2A".to_vec())
            } else {
                Some(b"\x1b[A".to_vec())
            }
        }
        Key::Named(NamedKey::ArrowDown) => {
            if modifiers.shift_key() {
                Some(b"\x1b[1;2B".to_vec())
            } else {
                Some(b"\x1b[B".to_vec())
            }
        }
        Key::Named(NamedKey::ArrowRight) => {
            if modifiers.shift_key() {
                Some(b"\x1b[1;2C".to_vec())
            } else {
                Some(b"\x1b[C".to_vec())
            }
        }
        Key::Named(NamedKey::ArrowLeft) => {
            if modifiers.shift_key() {
                Some(b"\x1b[1;2D".to_vec())
            } else {
                Some(b"\x1b[D".to_vec())
            }
        }
        Key::Named(NamedKey::Home) => {
            if modifiers.shift_key() {
                Some(b"\x1b[1;2H".to_vec())
            } else {
                Some(b"\x1b[H".to_vec())
            }
        }
        Key::Named(NamedKey::End) => {
            if modifiers.shift_key() {
                Some(b"\x1b[1;2F".to_vec())
            } else {
                Some(b"\x1b[F".to_vec())
            }
        }
        Key::Named(NamedKey::Delete) => Some(b"\x1b[3~".to_vec()),
        Key::Named(NamedKey::PageUp) => Some(b"\x1b[5~".to_vec()),
        Key::Named(NamedKey::PageDown) => Some(b"\x1b[6~".to_vec()),
        Key::Named(NamedKey::Insert) => Some(b"\x1b[2~".to_vec()),
        Key::Character(s) => {
            if let Some(t) = text {
                Some(t.as_bytes().to_vec())
            } else {
                Some(s.as_bytes().to_vec())
            }
        }
        Key::Dead(_) => {
            // If dead key produced resolved text directly, send it; otherwise None while composing
            if let Some(t) = text {
                Some(t.as_bytes().to_vec())
            } else {
                None
            }
        }
        _ => {
            if let Some(t) = text {
                Some(t.as_bytes().to_vec())
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_altgr_characters() {
        let mut mods = ModifiersState::default();
        mods |= ModifiersState::CONTROL;
        mods |= ModifiersState::ALT;

        let key = Key::Character("/".into());
        let res = handle_key_input(&key, Some("/"), ElementState::Pressed, mods);
        assert_eq!(res, Some(b"/".to_vec()));

        let key_bracket = Key::Character("[".into());
        let res_bracket = handle_key_input(&key_bracket, Some("["), ElementState::Pressed, mods);
        assert_eq!(res_bracket, Some(b"[".to_vec()));
    }

    #[test]
    fn test_dead_key_followed_by_space() {
        let mods = ModifiersState::default();
        let key = Key::Named(NamedKey::Space);
        let res = handle_key_input(&key, Some("~"), ElementState::Pressed, mods);
        assert_eq!(res, Some(b"~".to_vec()));

        let res_acute = handle_key_input(&key, Some("´"), ElementState::Pressed, mods);
        assert_eq!(res_acute, Some("´".as_bytes().to_vec()));
    }

    #[test]
    fn test_accented_characters() {
        let mods = ModifiersState::default();
        let key = Key::Character("ã".into());
        let res = handle_key_input(&key, Some("ã"), ElementState::Pressed, mods);
        assert_eq!(res, Some("ã".as_bytes().to_vec()));

        let key_cedilla = Key::Character("ç".into());
        let res_c = handle_key_input(&key_cedilla, Some("ç"), ElementState::Pressed, mods);
        assert_eq!(res_c, Some("ç".as_bytes().to_vec()));
    }

    #[test]
    fn test_ctrl_shortcuts() {
        let mut mods = ModifiersState::default();
        mods |= ModifiersState::CONTROL;

        let key_c = Key::Character("c".into());
        assert_eq!(handle_key_input(&key_c, None, ElementState::Pressed, mods), Some(vec![0x03]));

        let key_left = Key::Named(NamedKey::ArrowLeft);
        assert_eq!(handle_key_input(&key_left, None, ElementState::Pressed, mods), Some(b"\x1b[1;5D".to_vec()));
    }
}
