use crate::{error::Result, tui::state::State};

impl State {
    pub fn top(&mut self) -> Result<()> {
        self.list.first();
        if let Some(tab) = self.focused_module_tab_mut() {
            tab.scroll_index = 0;
        }
        Ok(())
    }
}
