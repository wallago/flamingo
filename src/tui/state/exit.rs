use crate::{error::Result, tui::state::State};

impl State {
    pub fn exit(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }
}
