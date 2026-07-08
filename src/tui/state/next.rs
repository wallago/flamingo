use crate::{
    error::Result,
    prelude::prelude::ModulePanel,
    tui::{command::ScrollType, state::State, ui::MAIN_TABS},
};

impl State {
    pub fn next(&mut self, scroll_type: ScrollType, amount: usize) -> Result<()> {
        match scroll_type {
            ScrollType::Tab => {
                self.tab = (((self.tab as usize).checked_add(amount).unwrap_or_default())
                    % MAIN_TABS.len())
                .into();
                self.handle_tab()?;
            }
            ScrollType::Table => {
                if let Some(tab) = self.focused_module_tab_mut() {
                    tab.panel = ModulePanel::Content;
                }
            }
            ScrollType::List => {
                let Some(panel) = self.focused_module_tab().map(|tab| tab.panel) else {
                    return Ok(());
                };
                match panel {
                    ModulePanel::Modules => {
                        self.list.next(amount);
                        if let Some(tab) = self.focused_module_tab_mut() {
                            tab.scroll_index = 0;
                        }
                    }
                    ModulePanel::Content => {
                        if let Some(tab) = self.focused_module_tab_mut() {
                            tab.scroll_index = tab.scroll_index.saturating_add(amount);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
