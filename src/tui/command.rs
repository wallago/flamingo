use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};
use tui_input::Input;

use crate::config::keybinding::Keybindings;

/// Possible scroll areas.
#[derive(Debug, PartialEq, Eq)]
pub enum ScrollType {
    /// Main application tabs.
    Tab,
    /// Inner tables.
    Table,
    /// Main list.
    List,
}

/// Application command.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Open repository URL.
    OpenRepo,
    /// Add module.
    AddModule,
    /// Next.
    Next(ScrollType, usize),
    /// Previous.
    Previous(ScrollType, usize),
    /// Go to top.
    Top,
    /// Go to bottom.
    Bottom,
    /// Input command.
    Input(InputCommand),
    /// Exit application.
    Exit,
    /// Do nothing.
    Nothing,
    /// Start the export flow.
    Export,
}

impl Keybindings {
    /// Returns the command bound to a key event.
    ///
    /// `ctrl+c` (exit) and `backspace` (resume input) are hardcoded so no
    /// config can shadow them.
    pub fn command_for(&self, event: &KeyEvent) -> Command {
        if event.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(event.code, KeyCode::Char('c') | KeyCode::Char('C'))
        {
            return Command::Exit;
        }
        if event.code == KeyCode::Backspace {
            return Command::Input(InputCommand::Resume(Event::Key(*event)));
        }
        if self.quit.matches(event) {
            Command::Exit
        } else if self.list_next.matches(event) {
            Command::Next(ScrollType::List, 1)
        } else if self.list_previous.matches(event) {
            Command::Previous(ScrollType::List, 1)
        } else if self.list_page_next.matches(event) {
            Command::Next(ScrollType::List, 5)
        } else if self.list_page_previous.matches(event) {
            Command::Previous(ScrollType::List, 5)
        } else if self.content_scroll_down.matches(event) {
            Command::Next(ScrollType::Table, 1)
        } else if self.content_scroll_up.matches(event) {
            Command::Previous(ScrollType::Table, 1)
        } else if self.tab_next.matches(event) {
            Command::Next(ScrollType::Tab, 1)
        } else if self.tab_previous.matches(event) {
            Command::Previous(ScrollType::Tab, 1)
        } else if self.top.matches(event) {
            Command::Top
        } else if self.bottom.matches(event) {
            Command::Bottom
        } else if self.search.matches(event) {
            Command::Input(InputCommand::Enter)
        } else if self.add_module.matches(event) {
            Command::AddModule
        } else if self.open_repo.matches(event) {
            Command::OpenRepo
        } else if self.export.matches(event) {
            Command::Export
        } else {
            Command::Nothing
        }
    }
}

impl From<MouseEvent> for Command {
    fn from(mouse_event: MouseEvent) -> Self {
        match mouse_event.kind {
            MouseEventKind::ScrollDown => Self::Next(ScrollType::List, 1),
            MouseEventKind::ScrollUp => Self::Previous(ScrollType::List, 1),
            _ => Self::Nothing,
        }
    }
}

/// Input mode command.
#[derive(Debug, PartialEq, Eq)]
pub enum InputCommand {
    /// Handle input.
    Handle(Event),
    /// Enter input mode.
    Enter,
    /// Confirm input.
    Confirm,
    /// Resume input.
    Resume(Event),
    /// Exit input mode
    Exit,
}

impl InputCommand {
    /// Parses the event.
    pub fn parse(key_event: KeyEvent, input: &Input) -> Self {
        if key_event.code == KeyCode::Esc
            || (key_event.code == KeyCode::Backspace && input.value().is_empty())
        {
            Self::Exit
        } else if key_event.code == KeyCode::Enter {
            Self::Confirm
        } else {
            Self::Handle(Event::Key(key_event))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::keybinding::Keybindings;

    use super::*;

    /// Shorthand for a plain key press.
    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::from(code)
    }

    #[test]
    fn test_default_bindings_reproduce_current_behavior() {
        let bindings = Keybindings::default();
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('j'))),
            Command::Next(ScrollType::List, 1)
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Up)),
            Command::Previous(ScrollType::List, 1)
        );
        assert_eq!(
            bindings.command_for(&KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Command::Next(ScrollType::List, 5)
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('l'))),
            Command::Next(ScrollType::Table, 1)
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Tab)),
            Command::Next(ScrollType::Tab, 1)
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::BackTab)),
            Command::Previous(ScrollType::Tab, 1)
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('t'))),
            Command::Top
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('q'))),
            Command::Exit
        );
        assert_eq!(bindings.command_for(&press(KeyCode::Esc)), Command::Exit);
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('/'))),
            Command::Input(InputCommand::Enter)
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Enter)),
            Command::AddModule
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('o'))),
            Command::OpenRepo
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('e'))),
            Command::Export
        );
        // Unbound keys do nothing; plain `d` needs ctrl.
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('d'))),
            Command::Nothing
        );
        assert_eq!(
            bindings.command_for(&press(KeyCode::Char('z'))),
            Command::Nothing
        );
    }

    #[test]
    fn test_hardcoded_escape_hatches() {
        // Rebind quit away from ctrl+c; ctrl+c must still exit.
        let mut bindings = Keybindings::default();
        bindings.quit = crate::config::keybinding::Binding(vec!["x".parse().unwrap()]);
        assert_eq!(
            bindings.command_for(&KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Command::Exit
        );
        assert!(matches!(
            bindings.command_for(&press(KeyCode::Backspace)),
            Command::Input(InputCommand::Resume(_))
        ));
    }
}
