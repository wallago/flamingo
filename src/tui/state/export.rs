use tui_input::Input;

use crate::{
    error::Result,
    tui::state::{ExportStage, State},
};

impl State {
    pub fn export(&mut self) -> Result<()> {
        if self.flake.nixos_modules.iter().any(|module| module.added) {
            self.export_stage = ExportStage::Hostname;
            self.input = Input::default();
            self.input_mode = true;
        } else {
            self.status = Some("no modules added".to_string());
        }
        Ok(())
    }
}
