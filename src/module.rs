use std::fs;

/// A `nixosModules` entry of the source config.
#[derive(Debug, PartialEq, Eq)]
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

impl Module {
    /// Builds a module from the file list collected by the Nix eval.
    pub fn from_files(name: String, files: Vec<String>, added: bool) -> Self {
        let (path, content, files) = Self::module_parts(files);
        Self {
            name,
            path,
            content,
            files,
            added,
        }
    }

    /// Parse module parts
    fn module_parts(files: Vec<String>) -> (Option<String>, Option<String>, Vec<String>) {
        let path = files
            .iter()
            // Find the first item where this transformation succeeds.
            .find_map(|file| {
                // Achieve this operation:
                //  "/repo/desktop.nix, via option imports" => Some(("/repo/desktop.nix", " imports"))
                //  "/repo/desktop.nix => None
                file.split_once(", via option")
            })
            .map(|(path, _)| path.to_string());
        let content = path
            .as_deref()
            .and_then(|path| fs::read_to_string(path).ok());
        let files = files
            .iter()
            // Get all normalized path
            // "/repo/desktop.nix, via option ..." → "/repo/desktop.nix"
            // "/repo/gnome.nix"                   → "/repo/gnome.nix"
            .map(|file| {
                file.split_once(", via option")
                    .map_or_else(|| file.clone(), |(path, _)| path.to_string())
            })
            .collect();
        (path, content, files)
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use crate::module::Module;

    #[fixture]
    fn raw_files() -> Vec<String> {
        vec![
            "/repo/desktop.nix, via option flake.nixosModules.desktop".to_string(),
            "/repo/gnome.nix".to_string(),
            "/repo/fonts.nix".to_string(),
        ]
    }

    #[rstest]
    fn strips_suffix_from_all_files(raw_files: Vec<String>) {
        let module = Module::from_files("desktop".to_string(), raw_files.clone(), false);
        assert_eq!(
            module,
            Module {
                name: "desktop".to_string(),
                path: Some("/repo/desktop.nix".to_string()),
                files: vec![
                    "/repo/desktop.nix".to_string(),
                    "/repo/gnome.nix".to_string(),
                    "/repo/fonts.nix".to_string()
                ],
                added: false,
                content: None,
            }
        );
    }

    #[rstest]
    #[case::empty(vec![])]
    #[case::no_suffix_anywhere(vec!["/repo/a.nix".to_string(), "/repo/b.nix".to_string()])]
    fn no_entry_file_yields_no_path(#[case] files: Vec<String>) {
        let module = Module::from_files("orphan".to_string(), files, false);
        assert_eq!(module.path, None);
        assert_eq!(module.content, None);
    }
}
