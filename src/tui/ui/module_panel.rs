/// Focusable module panels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ModulePanel {
    /// Module list panel.
    #[default]
    Modules,
    /// Module content panel.
    // TODO tui-syntax-highlight
    Content,
}
