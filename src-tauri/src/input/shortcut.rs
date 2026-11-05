use std::fmt;

/// A physical modifier key. Left and right are distinct so single-key shortcuts
/// like Right ⌥ can be bound without stealing the left one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModKey {
    LeftCtrl,
    RightCtrl,
    LeftAlt,
    RightAlt,
    LeftShift,
    RightShift,
    LeftMeta,
    RightMeta,
    /// The Fn / 🌐 key (macOS only).
    Fn,
}

/// Modifier set, independent of side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl Mods {
    pub fn is_empty(&self) -> bool {
        !(self.ctrl || self.alt || self.shift || self.meta)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Space,
    /// A–Z or 0–9, stored uppercase.
    Char(char),
    /// F1–F24.
    F(u8),
}

/// Either a modifier combination plus a key (`Meta+Shift+Space`) or a single
/// modifier held on its own (`Fn`, `RightAlt`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Shortcut {
    Combo { mods: Mods, key: Key },
    Solo(ModKey),
}

impl Shortcut {
    pub fn parse(text: &str) -> Result<Self, String> {
        let parts: Vec<&str> = text.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            return Err("is empty".into());
        }
        if parts.len() == 1 {
            if let Some(solo) = parse_solo(parts[0]) {
                return Ok(Shortcut::Solo(solo));
            }
        }
        let mut mods = Mods::default();
        let mut key = None;
        for part in &parts {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => mods.ctrl = true,
                "alt" | "option" | "opt" => mods.alt = true,
                "shift" => mods.shift = true,
                "meta" | "cmd" | "command" | "win" | "super" => mods.meta = true,
                other => {
                    if key.is_some() {
                        return Err(format!("has more than one non-modifier key (\"{part}\")"));
                    }
                    if parse_solo(part).is_some() {
                        return Err(format!("\"{part}\" can only be used on its own"));
                    }
                    key = Some(parse_key(other).ok_or_else(|| format!("unknown key \"{part}\""))?);
                }
            }
        }
        let key = key.ok_or("needs a key besides modifiers")?;
        if mods.is_empty() && !matches!(key, Key::F(_)) {
            return Err("needs at least one modifier (only F-keys may be used alone)".into());
        }
        if matches!(key, Key::Char(_)) && mods == (Mods { shift: true, ..Mods::default() }) {
            return Err("Shift plus a letter would block typing capitals".into());
        }
        Ok(Shortcut::Combo { mods, key })
    }

    #[allow(dead_code)]
    pub fn is_solo(&self) -> bool {
        matches!(self, Shortcut::Solo(_))
    }

    /// Whether this machine can produce the shortcut at all.
    pub fn supported_here(&self) -> bool {
        !matches!(self, Shortcut::Solo(ModKey::Fn)) || cfg!(target_os = "macos")
    }
}

fn parse_solo(part: &str) -> Option<ModKey> {
    Some(match part.to_ascii_lowercase().replace([' ', '_', '-'], "").as_str() {
        "fn" | "globe" => ModKey::Fn,
        "rightalt" | "rightoption" | "ropt" | "ralt" | "altgr" => ModKey::RightAlt,
        "rightctrl" | "rightcontrol" | "rctrl" => ModKey::RightCtrl,
        "rightmeta" | "rightcmd" | "rightcommand" | "rcmd" | "rightwin" => ModKey::RightMeta,
        "rightshift" | "rshift" => ModKey::RightShift,
        _ => return None,
    })
}

fn parse_key(part: &str) -> Option<Key> {
    if part == "space" {
        return Some(Key::Space);
    }
    let mut chars = part.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if c.is_ascii_alphanumeric() {
            return Some(Key::Char(c.to_ascii_uppercase()));
        }
    }
    if let Some(n) = part.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
        if (1..=24).contains(&n) {
            return Some(Key::F(n));
        }
    }
    None
}

impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Shortcut::Solo(m) => write!(
                f,
                "{}",
                match m {
                    ModKey::Fn => "Fn",
                    ModKey::RightAlt => "RightAlt",
                    ModKey::RightCtrl => "RightCtrl",
                    ModKey::RightMeta => "RightMeta",
                    ModKey::RightShift => "RightShift",
                    ModKey::LeftCtrl => "LeftCtrl",
                    ModKey::LeftAlt => "LeftAlt",
                    ModKey::LeftShift => "LeftShift",
                    ModKey::LeftMeta => "LeftMeta",
                }
            ),
            Shortcut::Combo { mods, key } => {
                let mut parts = Vec::new();
                if mods.ctrl {
                    parts.push("Ctrl".to_string());
                }
                if mods.alt {
                    parts.push("Alt".to_string());
                }
                if mods.shift {
                    parts.push("Shift".to_string());
                }
                if mods.meta {
                    parts.push("Meta".to_string());
                }
                parts.push(match key {
                    Key::Space => "Space".to_string(),
                    Key::Char(c) => c.to_string(),
                    Key::F(n) => format!("F{n}"),
                });
                write!(f, "{}", parts.join("+"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_combos_in_any_order_and_case() {
        let a = Shortcut::parse("Meta+Shift+Space").unwrap();
        let b = Shortcut::parse("shift + cmd + space").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "Shift+Meta+Space");
        assert_eq!(Shortcut::parse("Ctrl+Alt+Shift+V").unwrap().to_string(), "Ctrl+Alt+Shift+V");
    }

    #[test]
    fn parses_single_modifier_shortcuts() {
        assert_eq!(Shortcut::parse("Fn").unwrap(), Shortcut::Solo(ModKey::Fn));
        assert_eq!(Shortcut::parse("Right Option").unwrap(), Shortcut::Solo(ModKey::RightAlt));
        assert!(Shortcut::parse("Fn+Space").is_err());
    }

    #[test]
    fn rejects_shortcuts_that_would_block_typing() {
        assert!(Shortcut::parse("Space").is_err());
        assert!(Shortcut::parse("A").is_err());
        assert!(Shortcut::parse("Shift+A").is_err());
        assert!(Shortcut::parse("Ctrl+Shift").is_err());
        assert!(Shortcut::parse("Ctrl+A+B").is_err());
        assert!(Shortcut::parse("Ctrl+Banana").is_err());
        assert!(Shortcut::parse("F13").is_ok());
        assert!(Shortcut::parse("F25").is_err());
    }
}
