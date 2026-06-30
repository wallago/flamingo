//! **flamingo** - Trimmed your config with style 💃

#![warn(missing_docs, clippy::unwrap_used)]

/// Main application.
pub mod app;

/// Terminal user interface.
pub mod tui;

/// Command-line arguments parser.
pub mod args;

/// Error handler implementation.
pub mod error;

/// Common types that can be glob-imported for convenience.
pub mod prelude;

use args::Args;
use prelude::*;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::{env, fs, io, path::PathBuf};
use tui::{Tui, state::State, ui::Tab};

/// Runs app.
pub fn run(args: Args) -> Result<()> {
    start_tui(args)
}

/// Starts the terminal user interface.
pub fn start_tui(args: Args) -> Result<()> {
    // Create an application.
    let mut state = State::new(args.accent_color)?;

    // Change tab depending on cli arguments.
    state.set_tab(args.tab);

    // Initialize the terminal user interface.
    let backend = CrosstermBackend::new(io::stdout());
    let terminal = Terminal::new(backend)?;
    let events = EventHandler::new(250);
    state.analyzer.extract_strings(events.sender.clone());
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
                } else if state.show_heh {
                    Command::Hexdump(HexdumpCommand::parse(
                        key_event,
                        state.analyzer.file.is_read_only,
                    ))
                } else {
                    Command::from(key_event)
                };
                state.run_command(command, tui.events.sender.clone())?;
            }
            Event::Mouse(mouse_event) => {
                state.run_command(Command::from(mouse_event), tui.events.sender.clone())?;
            }
            Event::Resize(_, _) => {}
            Event::FileStrings(strings) => {
                state.strings_loaded = true;
                state.analyzer.strings = Some(strings?.into_iter().map(|(v, l)| (l, v)).collect());
                if state.tab == Tab::Strings {
                    state.handle_tab()?;
                }
            }
            #[cfg(feature = "dynamic-analysis")]
            Event::Trace => {
                state.system_calls_loaded = false;
                tui.toggle_pause()?;
                tracer::trace_syscalls(&state.analyzer.file, tui.events.sender.clone());
            }
            #[cfg(feature = "dynamic-analysis")]
            Event::TraceResult(syscalls) => {
                state.analyzer.tracer = match syscalls {
                    Ok(v) => v,
                    Err(e) => TraceData {
                        syscalls: console::style(e).red().to_string().as_bytes().to_vec(),
                        ..Default::default()
                    },
                };
                state.system_calls_loaded = true;
                state.dynamic_scroll_index = 0;
                tui.toggle_pause()?;
                state.handle_tab()?;
            }
            #[cfg(not(feature = "dynamic-analysis"))]
            Event::Trace | Event::TraceResult(_) => {}
            Event::Restart(path) => {
                let mut args = args.clone();
                match path {
                    Some(path) => {
                        args.files.push(path);
                    }
                    None => {
                        args.files.pop();
                    }
                }
                if !args.files.is_empty() {
                    tui.exit()?;
                    state.running = false;
                    run(args)?;
                }
            }
        }
    }

    // Exit the user interface.
    tui.exit()?;
    Ok(())
}
