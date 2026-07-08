use std::collections::HashMap;
use std::path::Path;

use crate::{app::Flake, module::Module};

impl Flake {
    /// Finds definition sites for modules the eval left opaque (top-level
    /// function modules have no `_file`): scans the source tree for
    /// `<attr>.<name> =` and adopts that file as the module's entry.
    /// Only possible for local sources.
    pub(super) fn recover_missing_module_paths(&mut self) {
        let root = Path::new(&self.source);
        if !root.is_dir() {
            return;
        }
        let mut sources = Vec::new();
        Self::collect_nix_sources(root, &mut sources);
        Self::recover_paths(&mut self.nixos_modules, "nixosModules", &sources);
        Self::recover_paths(&mut self.home_modules, "homeModules", &sources);
    }

    /// Reads every `.nix` file under `dir`, skipping `.git` and host configs.
    fn collect_nix_sources(dir: &Path, sources: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if path.is_dir() {
                // Host configs reference most modules; never adopt them.
                if name != ".git" && name != "hosts" {
                    Self::collect_nix_sources(&path, sources);
                }
            } else if path.extension().is_some_and(|extension| extension == "nix")
                && let Some(path) = path.to_str()
                && let Ok(content) = std::fs::read_to_string(path)
            {
                sources.push((path.to_string(), content));
            }
        }
    }

    /// Adopts the first file defining `<attr>.<name> =` as the entry file of
    /// each module that has none.
    fn recover_paths(modules: &mut [Module], attr: &str, sources: &[(String, String)]) {
        for module in modules.iter_mut().filter(|module| module.path.is_none()) {
            // Matches `homeModules.general =` and `homeModules."general" =`,
            // but not bare references inside `imports` (no `=` there).
            let definition = regex::Regex::new(&format!(
                r#"{attr}\.(?:{name}|"{name}")\s*="#,
                name = regex::escape(&module.name)
            ))
            .expect("definition regex should compile");
            if let Some((path, content)) = sources
                .iter()
                .find(|(_, content)| definition.is_match(content))
            {
                module.path = Some(path.clone());
                module.content = Some(content.clone());
                module.files = vec![path.clone()];
            }
        }
    }

    /// Recovers children hidden inside function modules.
    ///
    /// Function modules can't be inspected without calling them, and calling
    /// aborts on modules that force their arguments. So references are found
    /// textually instead: a `nixosModules.<name>` mention in a module's entry
    /// file adds `<name>`'s entry file to its files.
    ///
    /// Limits: dynamic references (`nixosModules.${x}`) are skipped, and
    /// mentions inside Nix comments still count.
    pub(super) fn recover_function_module_children(&mut self) {
        Self::recover_children(&mut self.nixos_modules, "nixosModules");
        Self::recover_children(&mut self.home_modules, "homeModules");
    }

    /// Textually recovers `<attr>.<name>` references from each module's entry
    /// file and records the referenced module's entry file (see doc above).
    fn recover_children(modules: &mut Vec<Module>, attr: &str) {
        // Matches `nixosModules.name` and `nixosModules."name"`.
        let reference = regex::Regex::new(&format!(
            r#"{attr}\.(?:([A-Za-z_][A-Za-z0-9_'-]*)|"([A-Za-z_][A-Za-z0-9_'-]*)")"#
        ))
        .expect("reference regex should compile");
        // Module name → entry file path.
        // { "desktop" → "/repo/desktop.nix",
        //   "gnome"   → "/repo/gnome.nix",
        //   "fonts"   → "/repo/fonts.nix" }
        let entries: HashMap<String, String> = modules
            .iter()
            .filter_map(|module| module.path.clone().map(|path| (module.name.clone(), path)))
            .collect();

        for module in modules {
            let Some(content) = &module.content else {
                continue;
            };
            // Host modules import most of the set; never treat them as parents.
            if module.path.as_deref().is_some_and(|path| {
                Path::new(path)
                    .components()
                    .any(|component| component.as_os_str() == "hosts")
            }) {
                continue;
            }
            // Scans module content and finds X matches
            for capture in reference.captures_iter(content) {
                // Bare or quoted name, whichever side matched.
                // e.g. nixosModules.gnome -> gnome
                let name = capture
                    .get(1)
                    .or_else(|| capture.get(2))
                    .map(|name| name.as_str())
                    .unwrap_or_default();
                // Not a self-mention
                if name != module.name
                    // Entry is valid
                    && let Some(entry) = entries.get(name)
                    // Path is already inside module
                    && !module.files.contains(entry)
                {
                    module.files.push(entry.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use crate::{app::Flake, module::Module};

    #[fixture]
    fn modules() -> Vec<Module> {
        let module = |name: &str, path: &str, content: Option<&str>| Module {
            name: name.to_string(),
            path: Some(path.to_string()),
            content: content.map(str::to_string),
            files: vec![path.to_string()],
            added: false,
        };
        vec![
            module(
                "desktop",
                "/repo/desktop.nix",
                Some(r#"{ imports = [ self.nixosModules.gnome self.nixosModules."fonts" ]; }"#),
            ),
            module("gnome", "/repo/gnome.nix", None),
            module("fonts", "/repo/fonts.nix", None),
            module(
                "laptop",
                "/repo/hosts/laptop.nix",
                Some("{ imports = [ self.nixosModules.desktop self.nixosModules.gnome ]; }"),
            ),
        ]
    }

    #[rstest]
    #[case::parent_gains_children(0, vec!["/repo/desktop.nix", "/repo/gnome.nix", "/repo/fonts.nix"])]
    #[case::no_content_untouched(1, vec!["/repo/gnome.nix"])]
    #[case::host_never_parent(3, vec!["/repo/hosts/laptop.nix"])]
    fn recovers_referenced_entry_files(
        mut modules: Vec<Module>,
        #[case] index: usize,
        #[case] expected: Vec<&str>,
    ) {
        Flake::recover_children(&mut modules, "nixosModules");
        assert_eq!(modules[index].files, expected);
    }

    #[rstest]
    fn adopts_definition_file_for_opaque_module() {
        let dir = tempdir::TempDir::new("flamingo-recover").expect("tempdir");
        let path = dir.path().join("general.nix");
        std::fs::write(
            &path,
            "{ flake.homeModules.general = { config, ... }: { }; }",
        )
        .expect("write general.nix");
        let mut modules = vec![Module {
            name: "general".to_string(),
            path: None,
            content: None,
            files: vec![],
            added: false,
        }];
        let mut sources = Vec::new();
        Flake::collect_nix_sources(dir.path(), &mut sources);
        Flake::recover_paths(&mut modules, "homeModules", &sources);
        assert_eq!(modules[0].path.as_deref(), path.to_str());
        assert!(modules[0].content.is_some());
    }
}
