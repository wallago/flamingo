use std::path::PathBuf;
use std::sync::mpsc;

use crate::error::{Error, Result};
use crate::tui::command::*;
use crate::tui::event::Event;
use crate::tui::ui::{MAIN_TABS, Tab};
use crate::tui::widgets::list::SelectableList;
use crate::tui::widgets::logo::Logo;
use ratatui::style::Color;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

/// Application state.
#[derive(Debug)]
pub struct State {
    /// Is the application running?
    pub running: bool,
    /// Selected tab.
    pub tab: Tab,
    /// Elf info.
    pub info_index: usize,
    /// List items.
    pub list: SelectableList<Vec<String>>,
    /// Show details.
    pub show_details: bool,
    /// Input.
    pub input: Input,
    /// Enable input.
    pub input_mode: bool,
    /// Available config options scroll index.
    pub available_option_scroll_index: usize,
    /// Selected config options scroll index.
    pub selected_option_scroll_index: usize,
    /// Strings call completed.
    pub strings_loaded: bool,
    /// Terminal accent color.
    pub accent_color: Color,
    /// Logo widget.
    pub logo: Logo,
}

impl State {
    /// Constructs a new instance of [`State`].
    pub fn new(accent_color: Option<Color>) -> Result<Self> {
        let mut state = Self {
            running: true,
            tab: Tab::default(),
            info_index: 0,
            list: SelectableList::default(),
            show_details: false,
            input: Input::default(),
            input_mode: false,
            available_option_scroll_index: 0,
            selected_option_scroll_index: 0,
            strings_loaded: false,
            accent_color: accent_color.unwrap_or(Color::White),
            logo: Logo::default(),
        };
        state.handle_tab()?;
        Ok(state)
    }

    /// Runs a command and updates the state.
    pub fn run_command(
        &mut self,
        command: Command,
        event_sender: mpsc::Sender<Event>,
    ) -> Result<()> {
        match command {
            Command::Input(command) => {
                match command {
                    InputCommand::Handle(event) => {
                        self.input.handle_event(&event);
                    }
                    InputCommand::Enter => {}
                    InputCommand::Confirm => {
                        self.input_mode = false;
                    }
                    InputCommand::Resume(event) => {
                        if self.tab == Tab::General {
                            event_sender
                                .send(Event::Restart(None))
                                .expect("failed to send restart event");
                            return Ok(());
                        }
                        if !self.input.value().is_empty() {
                            self.input_mode = true;
                            self.input.handle_event(&event);
                        }
                    }
                    InputCommand::Exit => {
                        self.input = Input::default();
                        self.input_mode = false;
                    }
                }
                self.handle_tab()?;
            }
            Command::ShowDetails => {
                if self.tab == Tab::General {
                    if let Some(path) = self.list.selected().map(|v| PathBuf::from(v[1].clone())) {
                        event_sender
                            .send(Event::Restart(Some(path)))
                            .expect("failed to send trace event");
                    }
                    return Ok(());
                } else {
                    self.show_details = !self.show_details;
                }
            }
            Command::OpenRepo => {
                if self.tab == Tab::General {
                    webbrowser::open(env!("CARGO_PKG_HOMEPAGE"))?;
                }
            }
            Command::Next(scroll_type, amount) => match scroll_type {
                ScrollType::Tab => {
                    self.tab = (((self.tab as usize).checked_add(amount).unwrap_or_default())
                        % MAIN_TABS.len())
                    .into();
                    self.handle_tab()?;
                }
                ScrollType::Table => {}
                ScrollType::List => {}
            },
            Command::Previous(scroll_type, amount) => match scroll_type {
                ScrollType::Tab => {
                    self.tab = (self.tab as usize)
                        .checked_sub(amount)
                        .unwrap_or(MAIN_TABS.len() - 1)
                        .into();
                    self.handle_tab()?;
                }
                ScrollType::Table => {
                    if self.tab == Tab::General {
                        self.available_option_scroll_index =
                            self.available_option_scroll_index.saturating_sub(amount);
                    }
                }
                ScrollType::List => self.list.previous(amount),
            },
            Command::Top => {
                self.list.first();
            }
            Command::Bottom => {
                self.list.last();
            }
            Command::Exit => {
                if self.show_details {
                    self.show_details = false;
                } else {
                    self.running = false;
                }
            }
            Command::Nothing => {}
        }
        Ok(())
    }

    /// Update the state based on selected tab.
    pub fn handle_tab(&mut self) -> Result<()> {
        match self.tab {
            Tab::General => {
                self.list = SelectableList::with_items([["aha".to_string()].to_vec()].to_vec());
            }
        }
        Ok(())
    }

    /// Returns the key bindings.
    pub fn get_key_bindings(&self) -> Vec<(&str, &str)> {
        match self.tab {
            Tab::General => {
                vec![
                    ("o", "Open docs"),
                    ("⏎ ", "Analyze lib"),
                    ("h/j/k/l", "Scroll"),
                    ("Tab", "Next"),
                    ("⇧+Tab", "Previous"),
                    ("Bksp", "Back"),
                    ("q", "Quit"),
                ]
            }
        }
    }

    /// Changes the tab
    pub fn set_tab(&mut self, tab: Tab) {
        self.tab = tab;
    }
}
