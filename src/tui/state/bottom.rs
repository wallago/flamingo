use crate::{
    error::Result,
    tui::{state::State, ui::Tab},
};

impl State {
    pub fn bottom(&mut self) -> Result<()> {
        self.list.last();
        self.selected_option_scroll_index = 0;
        Ok(())
    }
}
