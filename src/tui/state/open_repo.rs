use crate::{
    error::Result,
    tui::{state::State, ui::Tab},
};

impl State {
    pub fn open_repo(&mut self) -> Result<()> {
        if self.tab == Tab::General {
            webbrowser::open(env!("CARGO_PKG_HOMEPAGE"))?;
        }
        Ok(())
    }
}
