use crate::{
    error::Result,
    tui::{
        command::ScrollType,
        state::{Panel, State},
        ui::{MAIN_TABS, Tab},
    },
};

impl State {
    pub fn previous(&mut self, scroll_type: ScrollType, amount: usize) -> Result<()> {
        match scroll_type {
            ScrollType::Tab => {
                self.tab = (self.tab as usize)
                    .checked_sub(amount)
                    .unwrap_or(MAIN_TABS.len() - 1)
                    .into();
                self.handle_tab()?;
            }
            ScrollType::Table => {
                if self.tab == Tab::Config {
                    self.focused_panel = Panel::Modules;
                }
            }
            ScrollType::List => match self.focused_panel {
                Panel::Modules => {
                    self.list.previous(amount);
                    self.selected_option_scroll_index = 0;
                }
                Panel::Content => {
                    self.selected_option_scroll_index =
                        self.selected_option_scroll_index.saturating_sub(amount);
                }
            },
        }
        Ok(())
    }
}
