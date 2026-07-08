use std::sync::mpsc;

use crate::app::Flake;
use crate::config::keybinding::Keybindings;
use crate::error::Result;
use crate::module::Module;
use crate::trim::ExportReport;
use crate::tui::command::*;
use crate::tui::event::Event;
use crate::tui::ui::prelude::*;
use crate::tui::widgets::list::SelectableList;
use crate::tui::widgets::logo::Logo;
use ratatui::style::Color;
use ratatui_explorer::FileExplorer;
use tui_input::Input;

mod add_module;
mod bottom;
mod exit;
mod export;
mod input;
mod next;
mod open_repo;
mod previous;
mod top;

/// Stage of the export flow.
#[derive(Debug, Default)]
pub enum ExportStage {
    /// No export in progress.
    #[default]
    Idle,
    /// Prompting for the new hostname.
    Hostname,
    /// Prompting for the output directory.
    // TODO use ratatui-explorer
    OutputDir {
        /// Hostname confirmed in the previous stage.
        hostname: String,
    },
    /// Export finished; carries the report or the error message.
    Done(std::result::Result<ExportReport, String>),
}

/// Application state.
#[derive(Debug)]
pub struct State {
    // -- Core --
    /// Is the application running?
    pub running: bool,
    /// Nixos Configuration.
    pub flake: Flake,
    /// Active key bindings (defaults merged with `config.toml`).
    pub keybindings: Keybindings,
    /// Strings call completed.
    pub strings_loaded: bool,

    pub explorer: Option<FileExplorer>,

    // -- Navigation --
    /// Selected tab.
    pub tab: Tab,

    // -- Tab content --
    /// Module rows shown in the Config tab, as `(depth, module index)`.
    pub module_rows: Vec<(usize, usize)>,
    /// List items.
    pub list: SelectableList<Vec<String>>,
    /// Nixos modules options scroll index.
    pub nixos: ModuleTabState,
    /// HomeManager modules options scroll index.
    pub home: ModuleTabState,

    // -- Input & status --
    /// Input.
    pub input: Input,
    /// Enable input.
    pub input_mode: bool,
    /// Transient status message shown in the input slot.
    pub status: Option<String>,

    // -- Export flow --
    /// Export flow stage.
    pub export_stage: ExportStage,

    // -- Appearance --
    /// Terminal accent color.
    pub accent_color: Color,
    /// Logo widget.
    pub logo: Logo,
}

impl State {
    /// Constructs a new instance of [`State`].
    pub fn new(
        accent_color: Option<Color>,
        flake: Flake,
        keybindings: Keybindings,
    ) -> Result<Self> {
        let mut state = Self {
            running: true,
            flake,
            keybindings,
            strings_loaded: false,
            tab: Tab::default(),
            module_rows: Vec::new(),
            list: SelectableList::default(),
            nixos: ModuleTabState::default(),
            home: ModuleTabState::default(),
            input: Input::default(),
            input_mode: false,
            status: None,
            export_stage: ExportStage::default(),
            explorer: None,
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
        if matches!(self.export_stage, ExportStage::Done(_)) {
            self.export_stage = ExportStage::Idle;
            return Ok(());
        }
        self.status = None;
        match command {
            Command::Input(command) => self.input(command, event_sender)?,
            Command::AddModule => self.add_module()?,
            Command::OpenRepo => self.open_repo()?,
            Command::Next(scroll_type, amount) => self.next(scroll_type, amount)?,
            Command::Previous(scroll_type, amount) => self.previous(scroll_type, amount)?,
            Command::Top => self.top()?,
            Command::Bottom => self.bottom()?,
            Command::Exit => self.exit()?,
            Command::Nothing => {}
            Command::Export => self.export()?,
        }
        Ok(())
    }

    pub fn focused_module_tab(&self) -> Option<&ModuleTabState> {
        match self.tab {
            Tab::NixosModules => Some(&self.nixos),
            Tab::HomeManagerModules => Some(&self.home),
            _ => None,
        }
    }

    pub fn focused_module_tab_mut(&mut self) -> Option<&mut ModuleTabState> {
        match self.tab {
            Tab::NixosModules => Some(&mut self.nixos),
            Tab::HomeManagerModules => Some(&mut self.home),
            _ => None,
        }
    }

    pub fn focused_modules(&self) -> Option<&Vec<Module>> {
        match self.tab {
            Tab::NixosModules => Some(&self.flake.nixos_modules),
            Tab::HomeManagerModules => Some(&self.flake.home_modules),
            _ => None,
        }
    }

    pub fn focused_modules_mut(&mut self) -> Option<&mut Vec<Module>> {
        match self.tab {
            Tab::NixosModules => Some(&mut self.flake.nixos_modules),
            Tab::HomeManagerModules => Some(&mut self.flake.home_modules),
            _ => None,
        }
    }

    /// Update the state based on selected tab.
    pub fn handle_tab(&mut self) -> Result<()> {
        match self.tab {
            Tab::General => {
                self.module_rows = Vec::new();
                self.list = SelectableList::default();
            }
            Tab::NixosModules => {
                let modules = &self.flake.nixos_modules;
                self.module_rows = self.flake.module_tree(modules);
                self.list = SelectableList::with_items(
                    self.module_rows
                        .iter()
                        .map(|(_, index)| vec![modules[*index].name.clone()])
                        .collect(),
                );
            }
            Tab::HomeManagerModules => {
                let modules = &self.flake.home_modules;
                self.module_rows = self.flake.module_tree(modules);
                self.list = SelectableList::with_items(
                    self.module_rows
                        .iter()
                        .map(|(_, index)| vec![modules[*index].name.clone()])
                        .collect(),
                );
            }
        }
        Ok(())
    }

    /// Returns the key bindings shown in the help bar.
    pub fn get_key_bindings(&self) -> Vec<(String, &'static str)> {
        let keys = &self.keybindings;
        match self.tab {
            Tab::General => {
                vec![
                    (keys.open_repo.label(), "Open docs"),
                    (keys.tab_next.label(), "Next"),
                    (keys.tab_previous.label(), "Previous"),
                    (keys.export.label(), "Export"),
                    (keys.quit.label(), "Quit"),
                ]
            }
            Tab::NixosModules => {
                vec![
                    (keys.add_module.label(), "Add module"),
                    (keys.tab_next.label(), "Next"),
                    (keys.tab_previous.label(), "Previous"),
                    (keys.quit.label(), "Quit"),
                    (
                        format!(
                            "{}/{}",
                            keys.content_scroll_up.label(),
                            keys.content_scroll_down.label()
                        ),
                        "Focus panel",
                    ),
                    (
                        format!("{}/{}", keys.list_next.label(), keys.list_previous.label()),
                        "Scroll",
                    ),
                ]
            }
            Tab::HomeManagerModules => {
                vec![
                    (keys.add_module.label(), "Add module"),
                    (keys.tab_next.label(), "Next"),
                    (keys.tab_previous.label(), "Previous"),
                    (keys.quit.label(), "Quit"),
                    (
                        format!(
                            "{}/{}",
                            keys.content_scroll_up.label(),
                            keys.content_scroll_down.label()
                        ),
                        "Focus panel",
                    ),
                    (
                        format!("{}/{}", keys.list_next.label(), keys.list_previous.label()),
                        "Scroll",
                    ),
                ]
            }
        }
    }

    /// Changes the tab and rebuilds the tab-dependent state.
    pub fn set_tab(&mut self, tab: Tab) -> Result<()> {
        self.tab = tab;
        self.handle_tab()
    }
}
