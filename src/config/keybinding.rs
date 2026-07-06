use std::fmt;
use std::str::FromStr;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

/// Configurable key bindings, one field per action.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Keybindings {
    /// Move down in the focused list.
    pub list_next: Binding,
    /// Move up in the focused list.
    pub list_previous: Binding,
    /// Move down a page in the focused list.
    pub list_page_next: Binding,
    /// Move up a page in the focused list.
    pub list_page_previous: Binding,
    /// Scroll the content panel down / focus it.
    pub content_scroll_down: Binding,
    /// Scroll the content panel up / focus the list.
    pub content_scroll_up: Binding,
    /// Switch to the next tab.
    pub tab_next: Binding,
    /// Switch to the previous tab.
    pub tab_previous: Binding,
    /// Jump to the top of the list.
    pub top: Binding,
    /// Jump to the bottom of the list.
    pub bottom: Binding,
    /// Enter search/input mode.
    pub search: Binding,
    /// Toggle the selected module for export.
    pub add_module: Binding,
    /// Open the repository in the browser.
    pub open_repo: Binding,
    /// Start the export flow.
    pub export: Binding,
    /// Quit the application.
    pub quit: Binding,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            list_next: keys(&["j", "down"]),
            list_previous: keys(&["k", "up"]),
            list_page_next: keys(&["pagedown", "ctrl+d"]),
            list_page_previous: keys(&["pageup", "ctrl+u"]),
            content_scroll_down: keys(&["l", "right"]),
            content_scroll_up: keys(&["h", "left"]),
            tab_next: keys(&["tab"]),
            tab_previous: keys(&["backtab"]),
            top: keys(&["t", "home"]),
            bottom: keys(&["b", "end"]),
            search: keys(&["/", "ctrl+f"]),
            add_module: keys(&["enter", "space"]),
            open_repo: keys(&["o"]),
            export: keys(&["e"]),
            quit: keys(&["q", "esc"]),
        }
    }
}

/// A single key chord, e.g. `q`, `esc` or `ctrl+d`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPattern {
    /// Key code.
    pub code: KeyCode,
    /// Modifier keys that must be held.
    pub modifiers: KeyModifiers,
}

/// Parses a bare key name (no modifiers) into a [`KeyCode`].
fn parse_key_code(s: &str) -> std::result::Result<KeyCode, String> {
    Ok(match s {
        "esc" | "escape" => KeyCode::Esc,
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "space" => KeyCode::Char(' '),
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => KeyCode::Char(c),
                _ => return Err(format!("unknown key `{other}`")),
            }
        }
    })
}

impl fmt::Display for KeyPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            write!(f, "Ctrl+")?;
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            write!(f, "Alt+")?;
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            write!(f, "⇧+")?;
        }
        match self.code {
            KeyCode::Esc => write!(f, "Esc"),
            KeyCode::Enter => write!(f, "⏎"),
            KeyCode::Tab => write!(f, "Tab"),
            KeyCode::BackTab => write!(f, "⇧+Tab"),
            KeyCode::Up => write!(f, "↑"),
            KeyCode::Down => write!(f, "↓"),
            KeyCode::Left => write!(f, "←"),
            KeyCode::Right => write!(f, "→"),
            KeyCode::Home => write!(f, "Home"),
            KeyCode::End => write!(f, "End"),
            KeyCode::PageUp => write!(f, "PgUp"),
            KeyCode::PageDown => write!(f, "PgDn"),
            KeyCode::Backspace => write!(f, "⌫"),
            KeyCode::Delete => write!(f, "Del"),
            KeyCode::Char(' ') => write!(f, "Space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            other => write!(f, "{other:?}"),
        }
    }
}

impl FromStr for KeyPattern {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut modifiers = KeyModifiers::NONE;
        let mut code = None;
        for part in s.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
                "alt" => modifiers |= KeyModifiers::ALT,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                key => {
                    if code.is_some() {
                        return Err(format!("`{s}`: more than one key"));
                    }
                    code = Some(parse_key_code(key)?);
                }
            }
        }
        Ok(Self {
            code: code.ok_or_else(|| format!("`{s}`: missing key"))?,
            modifiers,
        })
    }
}

/// One or more key chords bound to a single action.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Binding(pub Vec<KeyPattern>);

/// Normalizes a key event for comparison: SHIFT is dropped for character
/// keys and `BackTab` (crossterm reports it), and characters are lowercased
/// so bindings are case-insensitive.
fn normalize(code: KeyCode, mut modifiers: KeyModifiers) -> (KeyCode, KeyModifiers) {
    match code {
        KeyCode::Char(c) => {
            modifiers.remove(KeyModifiers::SHIFT);
            (KeyCode::Char(c.to_ascii_lowercase()), modifiers)
        }
        KeyCode::BackTab => {
            modifiers.remove(KeyModifiers::SHIFT);
            (code, modifiers)
        }
        _ => (code, modifiers),
    }
}

/// Builds a [`Binding`] from literal key names; only used for defaults.
fn keys(list: &[&str]) -> Binding {
    Binding(
        list.iter()
            .map(|key| key.parse().expect("invalid default key"))
            .collect(),
    )
}

impl Binding {
    /// Help-bar label: the first configured chord, prettified.
    pub fn label(&self) -> String {
        self.0.first().map(ToString::to_string).unwrap_or_default()
    }

    /// Whether the given key event triggers this binding.
    pub fn matches(&self, event: &KeyEvent) -> bool {
        let event = normalize(event.code, event.modifiers);
        self.0
            .iter()
            .any(|pattern| normalize(pattern.code, pattern.modifiers) == event)
    }
}

impl<'de> Deserialize<'de> for Binding {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        /// A TOML value that is either one string or a list of strings.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OneOrMany {
            /// Single key.
            One(String),
            /// Several keys.
            Many(Vec<String>),
        }
        let keys = match OneOrMany::deserialize(deserializer)? {
            OneOrMany::One(key) => vec![key],
            OneOrMany::Many(keys) => keys,
        };
        keys.iter()
            .map(|key| key.parse().map_err(serde::de::Error::custom))
            .collect::<std::result::Result<Vec<KeyPattern>, _>>()
            .map(Binding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::{fixture, rstest};

    #[rstest]
    #[case("e", KeyCode::Char('e'), KeyModifiers::NONE)]
    #[case("ctrl+d", KeyCode::Char('d'), KeyModifiers::CONTROL)]
    #[case("pagedown", KeyCode::PageDown, KeyModifiers::NONE)]
    fn parses_valid_key_pattern(
        #[case] input: &str,
        #[case] code: KeyCode,
        #[case] modifiers: KeyModifiers,
    ) {
        assert_eq!(
            input.parse::<KeyPattern>().unwrap(),
            KeyPattern { code, modifiers }
        );
    }

    #[rstest]
    #[case("bogus_key")]
    #[case("ctrl+")]
    fn rejects_invalid_key_pattern(#[case] input: &str) {
        assert!(input.parse::<KeyPattern>().is_err());
    }

    #[fixture]
    fn binding() -> Binding {
        Binding(vec![
            "q".parse().unwrap(),
            "esc".parse().unwrap(),
            "ctrl+d".parse().unwrap(),
        ])
    }

    #[rstest]
    #[case::plain_q(KeyEvent::from(KeyCode::Char('q')), true)]
    // Uppercase (shift) still matches: bindings are case-insensitive.
    #[case::uppercase_q(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::SHIFT), true)]
    #[case::esc(KeyEvent::from(KeyCode::Esc), true)]
    #[case::ctrl_d(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL), true)]
    #[case::plain_d_without_ctrl(KeyEvent::from(KeyCode::Char('d')), false)]
    #[case::unbound_key(KeyEvent::from(KeyCode::Char('x')), false)]
    fn binding_matching(binding: Binding, #[case] event: KeyEvent, #[case] expected: bool) {
        assert_eq!(binding.matches(&event), expected);
    }

    #[test]
    fn should_deserializes_toml_into_string_or_list() {
        #[derive(Deserialize)]
        struct Wrapper {
            key: Binding,
        }
        let single: Wrapper = toml::from_str("key = \"e\"").unwrap();
        assert_eq!(single.key.0.len(), 1);
        let many: Wrapper = toml::from_str("key = [\"q\", \"esc\"]").unwrap();
        assert_eq!(many.key.0.len(), 2);
        assert!(toml::from_str::<Wrapper>("key = \"not+a+key\"").is_err());
    }

    #[rstest]
    #[case("q", "q")]
    #[case("enter", "⏎")]
    #[case("backtab", "⇧+Tab")]
    #[case("ctrl+d", "Ctrl+d")]
    #[case("space", "Space")]
    #[case("up", "↑")]
    fn key_pattern_labels(#[case] input: &str, #[case] label: &str) {
        assert_eq!(input.parse::<KeyPattern>().unwrap().to_string(), label);
    }

    #[test]
    fn binding_label_uses_first_chord() {
        let binding = Binding(vec!["n".parse().unwrap(), "down".parse().unwrap()]);
        assert_eq!(binding.label(), "n");
        assert_eq!(Binding::default().label(), "");
    }

    #[rstest]
    #[case::quit_q(KeyEvent::from(KeyCode::Char('q')), |b: Keybindings| b.quit)]
    #[case::quit_esc(KeyEvent::from(KeyCode::Esc), |b: Keybindings| b.quit)]
    #[case::list_next_j(KeyEvent::from(KeyCode::Char('j')), |b: Keybindings| b.list_next)]
    #[case::list_next_down(KeyEvent::from(KeyCode::Down), |b: Keybindings| b.list_next)]
    #[case::page_next_ctrl_d(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL), |b: Keybindings| b.list_page_next)]
    #[case::tab_previous_backtab(KeyEvent::from(KeyCode::BackTab), |b: Keybindings| b.tab_previous)]
    #[case::search_slash(KeyEvent::from(KeyCode::Char('/')), |b: Keybindings| b.search)]
    #[case::export_e(KeyEvent::from(KeyCode::Char('e')), |b: Keybindings| b.export)]
    fn default_keybindings_cover_current_behavior(
        #[case] event: KeyEvent,
        #[case] binding: fn(Keybindings) -> Binding,
    ) {
        let bindings = Keybindings::default();
        assert!(binding(bindings).matches(&event));
    }
}
