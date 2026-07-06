use crate::{
    error::Result,
    tui::{state::State, ui::Tab},
};

impl State {
    pub fn top(&mut self) -> Result<()> {
        self.list.first();
        self.selected_option_scroll_index = 0;
        Ok(())
    }
}
