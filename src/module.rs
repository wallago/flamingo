/// A `nixosModules` entry of the source config.
#[derive(Debug)]
pub struct Module {
    /// Module name (attribute in `nixosModules`).
    pub name: String,
    /// Path of the module's entry file.
    pub path: Option<String>,
    /// Source content of the entry file.
    pub content: Option<String>,
    /// Transitive source files collected through the module's imports.
    pub files: Vec<String>,
    /// Whether the module is selected for export.
    pub added: bool,
}
