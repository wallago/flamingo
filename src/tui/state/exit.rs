use crate::{
    error::Result,
    tui::{state::State, ui::Tab},
};

impl State {
    pub fn exit(&mut self) -> Result<()> {
        if self.show_details {
            self.show_details = false;
        } else {
            self.running = false;
        }
        Ok(())
    }
}
