use crate::{error::Result, prelude::prelude::Tab, tui::state::State};

impl State {
    pub fn open_repo(&mut self) -> Result<()> {
        if self.tab == Tab::General {
            webbrowser::open(env!("CARGO_PKG_HOMEPAGE"))?;
        }
        Ok(())
    }
}
