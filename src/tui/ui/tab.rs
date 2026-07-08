use crate::tui::ui::module_panel::ModulePanel;

/// Application tab.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, clap::ValueEnum)]
pub enum Tab {
    /// General information.
    #[default]
    General = 0,
    /// Nixos modules.
    NixosModules = 1,
    /// HomeManager modules.
    HomeManagerModules = 2,
}

impl Tab {
    /// Returns the available tabs.
    pub const fn get_headers() -> &'static [&'static str] {
        &["General", "Nixos Modules", "Home Manager Modules"]
    }
}

impl From<usize> for Tab {
    fn from(v: usize) -> Self {
        match v {
            0 => Self::General,
            1 => Self::NixosModules,
            2 => Self::HomeManagerModules,
            _ => Self::default(),
        }
    }
}

/// UI state owned by one module tab.
#[derive(Debug, Default)]
pub struct ModuleTabState {
    /// Focused panel.
    pub panel: ModulePanel,
    /// Content scroll index.
    pub scroll_index: usize,
}
