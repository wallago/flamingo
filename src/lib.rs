//! **flamingo** - Trim Nix config like a pruner.

#![warn(missing_docs, clippy::unwrap_used)]

/// Main application.
pub mod app;

/// Terminal user interface.
pub mod tui;

/// Command-line arguments parser.
pub mod args;

/// Error handler implementation.
pub mod error;

/// Nixos module.
pub mod module;

/// Trim configuration behavior.
pub mod trim;

/// App configuration.
pub mod config;

/// Common types that can be glob-imported for convenience.
pub mod prelude;

use args::Args;
use prelude::*;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io;
use tui::{Tui, state::State};

use crate::app::Flake;

/// Runs app.
pub fn run(args: Args) -> Result<()> {
    let config = AppConfig::load(args.config.as_deref())?;
    let flake = Flake::load(&args.source, &config)?;
    start_tui(args, &config, flake)
}

/// Starts the terminal user interface.
pub fn start_tui(args: Args, config: &AppConfig, flake: Flake) -> Result<()> {
    // Create an application.
    let mut state = State::new(args.accent_color, flake, config.keybindings.clone())?;

    // Change tab depending on cli arguments.
    state.set_tab(args.tab)?;

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    let mut tui = Tui::new(terminal, events);
    tui.init()?;

    // Start the main loop.
    while state.running {
        // Render the user interface.
        tui.draw(&mut state)?;
        // Handle events.
        match tui.events.next()? {
            Event::Tick => {}
            Event::Key(key_event) => {
                let command = if state.input_mode {
                    Command::Input(InputCommand::parse(key_event, &state.input))
                } else {
                    state.keybindings.command_for(&key_event)
                };
                state.run_command(command, tui.events.sender.clone())?;
            }
            Event::Mouse(mouse_event) => {
                state.run_command(Command::from(mouse_event), tui.events.sender.clone())?;
            }
            Event::Resize(_, _) => {}
            Event::Restart(path) => {
                let mut args = args.clone();
                // match path {
                //     Some(path) => {
                //         args.files.push(path);
                //     }
                //     None => {
                //         args.files.pop();
                //     }
                // }
                // if !args.files.is_empty() {
                //     tui.exit()?;
                //     state.running = false;
                //     run(args)?;
                // }
            }
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
