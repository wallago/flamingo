use crate::{
    error::Result,
    tui::{state::State, ui::Tab},
};

impl State {
    pub fn add_module(&mut self) -> Result<()> {
        if self.tab == Tab::Config {
            if let Some(row) = self.list.state.selected()
                && let Some((depth, index)) = self.module_rows.get(row).copied()
                && let Some(module) = self.flake.nixos_modules.get_mut(index)
            {
                module.added = !module.added;
                let added = module.added;
                // Propagate to the whole subtree below this row.
                for &(child_depth, child_index) in self.module_rows.iter().skip(row + 1) {
                    if child_depth <= depth {
                        break;
                    }
                    if let Some(child) = self.flake.nixos_modules.get_mut(child_index) {
                        child.added = added;
                    }
                }
            }
        } else {
            self.show_details = !self.show_details;
        }
        Ok(())
    }
}
