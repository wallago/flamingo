use std::sync::mpsc;

use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    error::Result,
    trim::{self, ExportRequest},
    tui::{
        command::InputCommand,
        event::Event,
        state::{ExportStage, State},
        ui::Tab,
    },
};

impl State {
    pub fn input(
        &mut self,
        command: InputCommand,
        event_sender: mpsc::Sender<Event>,
    ) -> Result<()> {
        match command {
            InputCommand::Handle(event) => {
                self.input.handle_event(&event);
            }
            InputCommand::Enter => {}
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
                self.export_stage = ExportStage::Idle;
            }
            InputCommand::Confirm => match std::mem::take(&mut self.export_stage) {
                ExportStage::Hostname => {
                    let hostname = self.input.value().trim().to_string();
                    if hostname.is_empty() {
                        self.export_stage = ExportStage::Hostname;
                    } else {
                        self.export_stage = ExportStage::OutputDir { hostname };
                        self.input = Input::default();
                    }
                }
                ExportStage::OutputDir { hostname } => {
                    let value = self.input.value().trim().to_string();
                    if value.is_empty() {
                        self.export_stage = ExportStage::OutputDir { hostname };
                    } else {
                        let request = ExportRequest {
                            source: self.flake.source.clone(),
                            hostname,
                            output_dir: trim::expand_tilde(&value),
                            modules: self.flake.export_selection(),
                        };
                        let result = trim::export(&request).map_err(|error| error.to_string());
                        self.export_stage = ExportStage::Done(result);
                        self.input = Input::default();
                        self.input_mode = false;
                    }
                }
                stage => {
                    self.export_stage = stage;
                    self.input_mode = false;
                }
            },
        }
        self.handle_tab()?;
        Ok(())
    }
}
