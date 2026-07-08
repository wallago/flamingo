use crate::{error::Result, prelude::prelude::Tab, tui::state::State};

impl State {
    pub fn add_module(&mut self) -> Result<()> {
        if self.tab == Tab::NixosModules || self.tab == Tab::HomeManagerModules {
            let Some(row) = self.list.state.selected() else {
                return Ok(());
            };
            let Some(&(depth, index)) = self.module_rows.get(row) else {
                return Ok(());
            };
            let children: Vec<usize> = self
                .module_rows
                .iter()
                .skip(row + 1)
                .take_while(|&&(child_depth, _)| child_depth > depth)
                .map(|&(_, child_index)| child_index)
                .collect();
            let Some(modules) = self.focused_modules_mut() else {
                return Ok(());
            };
            let Some(module) = modules.get_mut(index) else {
                return Ok(());
            };
            module.added = !module.added;
            let added = module.added;
            for child_index in children {
                if let Some(child) = modules.get_mut(child_index) {
                    child.added = added;
                }
            }
        }
        Ok(())
    }
}
