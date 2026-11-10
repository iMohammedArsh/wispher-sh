//! Platform-neutral keyboard logic: shortcut parsing and a matcher that turns raw
//! key events into hotkey actions. Each OS only translates its native key codes
//! into [`KeyId`] and decides whether to swallow the event.

mod matcher;
mod shortcut;

pub use matcher::Matcher;
pub use shortcut::{Key, ModKey, Mods, Shortcut};

/// What a hotkey is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Binding {
    Dictate,
    Command,
    PasteLast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    Pressed(Binding),
    Released(Binding),
    /// Escape while recording.
    Escape,
    /// Another key was pressed while a modifier-only shortcut (e.g. Fn) was held:
    /// the user was typing a key combination, not dictating.
    Interrupted(Binding),
}

/// A physical key, as reported by the platform layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyId {
    Mod(ModKey),
    Key(Key),
    Escape,
    /// Any key we don't bind to; still matters for interrupting modifier-only holds.
    Other,
}
