use std::collections::HashSet;

use super::{Binding, HotkeyAction, Key, KeyId, ModKey, Mods, Shortcut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MatchResult {
    pub action: Option<HotkeyAction>,
    /// The platform layer should stop this event from reaching the focused app.
    pub swallow: bool,
}

impl MatchResult {
    fn pass() -> Self {
        Self::default()
    }
    fn act(action: HotkeyAction, swallow: bool) -> Self {
        Self { action: Some(action), swallow }
    }
    fn swallow() -> Self {
        Self { action: None, swallow: true }
    }
}

/// Turns raw key events into hotkey actions. Pure state machine: no OS calls,
/// no timing. Timing (tap vs hold, double-tap) lives in the controller.
#[derive(Debug, Default)]
pub struct Matcher {
    bindings: Vec<(Binding, Shortcut)>,
    held: HashSet<ModKey>,
    active: Option<(Binding, Shortcut)>,
    swallowing_escape: bool,
}

impl Matcher {
    pub fn new(bindings: Vec<(Binding, Shortcut)>) -> Self {
        Self { bindings, ..Self::default() }
    }

    pub fn set_bindings(&mut self, bindings: Vec<(Binding, Shortcut)>) {
        self.bindings = bindings;
        self.active = None;
    }

    fn current_mods(&self) -> Mods {
        let has = |a: ModKey, b: ModKey| self.held.contains(&a) || self.held.contains(&b);
        Mods {
            ctrl: has(ModKey::LeftCtrl, ModKey::RightCtrl),
            alt: has(ModKey::LeftAlt, ModKey::RightAlt),
            shift: has(ModKey::LeftShift, ModKey::RightShift),
            meta: has(ModKey::LeftMeta, ModKey::RightMeta),
        }
    }

    /// Interrupt a held single-modifier shortcut (the user is typing a combo).
    fn interrupt_solo(&mut self) -> Option<HotkeyAction> {
        match self.active {
            Some((binding, Shortcut::Solo(_))) => {
                self.active = None;
                Some(HotkeyAction::Interrupted(binding))
            }
            _ => None,
        }
    }

    pub fn handle(&mut self, key: KeyId, down: bool, recording: bool) -> MatchResult {
        match key {
            KeyId::Mod(m) => self.handle_modifier(m, down),
            KeyId::Key(k) => self.handle_key(k, down),
            KeyId::Escape => {
                if down {
                    if recording {
                        self.active = None;
                        self.swallowing_escape = true;
                        return MatchResult::act(HotkeyAction::Escape, true);
                    }
                    if let Some(action) = self.interrupt_solo() {
                        return MatchResult::act(action, false);
                    }
                    MatchResult::pass()
                } else if std::mem::take(&mut self.swallowing_escape) {
                    MatchResult::swallow()
                } else {
                    MatchResult::pass()
                }
            }
            KeyId::Other => match (down, self.interrupt_solo()) {
                (true, Some(action)) => MatchResult::act(action, false),
                _ => MatchResult::pass(),
            },
        }
    }

    fn handle_modifier(&mut self, m: ModKey, down: bool) -> MatchResult {
        if down {
            self.held.insert(m);
            match self.active {
                None => {
                    let alone = self.held.len() == 1;
                    let hit = self
                        .bindings
                        .iter()
                        .find(|(_, s)| *s == Shortcut::Solo(m))
                        .copied();
                    if let (true, Some((binding, shortcut))) = (alone, hit) {
                        self.active = Some((binding, shortcut));
                        return MatchResult::act(HotkeyAction::Pressed(binding), false);
                    }
                }
                Some((_, Shortcut::Solo(active))) if active != m => {
                    if let Some(action) = self.interrupt_solo() {
                        return MatchResult::act(action, false);
                    }
                }
                _ => {}
            }
            MatchResult::pass()
        } else {
            self.held.remove(&m);
            if let Some((binding, Shortcut::Solo(active))) = self.active {
                if active == m {
                    self.active = None;
                    return MatchResult::act(HotkeyAction::Released(binding), false);
                }
            }
            MatchResult::pass()
        }
    }

    fn handle_key(&mut self, k: Key, down: bool) -> MatchResult {
        if down {
            if let Some(action) = self.interrupt_solo() {
                return MatchResult::act(action, false);
            }
            if let Some((_, Shortcut::Combo { key, .. })) = self.active {
                // Auto-repeat of the held shortcut key.
                return if key == k { MatchResult::swallow() } else { MatchResult::pass() };
            }
            let mods = self.current_mods();
            let hit = self
                .bindings
                .iter()
                .find(|(_, s)| *s == Shortcut::Combo { mods, key: k })
                .copied();
            match hit {
                Some((binding, shortcut)) => {
                    self.active = Some((binding, shortcut));
                    MatchResult::act(HotkeyAction::Pressed(binding), true)
                }
                None => MatchResult::pass(),
            }
        } else {
            match self.active {
                Some((binding, Shortcut::Combo { key, .. })) if key == k => {
                    self.active = None;
                    MatchResult::act(HotkeyAction::Released(binding), true)
                }
                _ => MatchResult::pass(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use HotkeyAction::*;

    fn matcher() -> Matcher {
        Matcher::new(vec![
            (Binding::Dictate, Shortcut::parse("Meta+Shift+Space").unwrap()),
            (Binding::Command, Shortcut::parse("RightAlt").unwrap()),
            (Binding::PasteLast, Shortcut::parse("F13").unwrap()),
        ])
    }

    fn press(m: &mut Matcher, key: KeyId, down: bool) -> MatchResult {
        m.handle(key, down, false)
    }

    #[test]
    fn combo_press_repeat_release() {
        let mut m = matcher();
        press(&mut m, KeyId::Mod(ModKey::LeftMeta), true);
        press(&mut m, KeyId::Mod(ModKey::RightShift), true);
        assert_eq!(press(&mut m, KeyId::Key(Key::Space), true), MatchResult::act(Pressed(Binding::Dictate), true));
        assert_eq!(press(&mut m, KeyId::Key(Key::Space), true), MatchResult::swallow());
        // Releasing modifiers first does not end the hold; the key release does.
        press(&mut m, KeyId::Mod(ModKey::LeftMeta), false);
        assert_eq!(press(&mut m, KeyId::Key(Key::Space), false), MatchResult::act(Released(Binding::Dictate), true));
    }

    #[test]
    fn wrong_modifiers_do_not_match() {
        let mut m = matcher();
        press(&mut m, KeyId::Mod(ModKey::LeftMeta), true);
        assert_eq!(press(&mut m, KeyId::Key(Key::Space), true), MatchResult::pass());
        press(&mut m, KeyId::Mod(ModKey::LeftShift), true);
        press(&mut m, KeyId::Mod(ModKey::LeftAlt), true);
        assert_eq!(press(&mut m, KeyId::Key(Key::Space), true), MatchResult::pass());
    }

    #[test]
    fn solo_modifier_hold_and_release() {
        let mut m = matcher();
        assert_eq!(press(&mut m, KeyId::Mod(ModKey::RightAlt), true).action, Some(Pressed(Binding::Command)));
        assert_eq!(press(&mut m, KeyId::Mod(ModKey::RightAlt), false).action, Some(Released(Binding::Command)));
        // Left Alt is not Right Alt.
        assert_eq!(press(&mut m, KeyId::Mod(ModKey::LeftAlt), true).action, None);
    }

    #[test]
    fn typing_while_solo_held_interrupts() {
        let mut m = matcher();
        press(&mut m, KeyId::Mod(ModKey::RightAlt), true);
        let result = press(&mut m, KeyId::Other, true);
        assert_eq!(result, MatchResult::act(Interrupted(Binding::Command), false));
        assert_eq!(press(&mut m, KeyId::Mod(ModKey::RightAlt), false).action, None);
    }

    #[test]
    fn solo_does_not_fire_when_other_modifiers_are_held() {
        let mut m = matcher();
        press(&mut m, KeyId::Mod(ModKey::LeftMeta), true);
        assert_eq!(press(&mut m, KeyId::Mod(ModKey::RightAlt), true).action, None);
    }

    #[test]
    fn escape_cancels_only_while_recording() {
        let mut m = matcher();
        assert_eq!(m.handle(KeyId::Escape, true, false), MatchResult::pass());
        assert_eq!(m.handle(KeyId::Escape, true, true), MatchResult::act(Escape, true));
        assert_eq!(m.handle(KeyId::Escape, false, true), MatchResult::swallow());
        assert_eq!(m.handle(KeyId::Escape, false, true), MatchResult::pass());
    }

    #[test]
    fn function_key_alone() {
        let mut m = matcher();
        assert_eq!(press(&mut m, KeyId::Key(Key::F(13)), true).action, Some(Pressed(Binding::PasteLast)));
        assert_eq!(press(&mut m, KeyId::Key(Key::F(13)), false).action, Some(Released(Binding::PasteLast)));
    }
}
