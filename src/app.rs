use crate::{
    config::AppConfig,
    error::{Error, Result},
    module::Module,
    trim::ExportModule,
};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempdir::TempDir;

/// Nixos configuration.
#[derive(Debug)]
pub struct Flake {
    /// Path of flake.
    pub source: String,
    /// Tempdir to achieve all operations.
    temp: TempDir,
    /// Nixos modules.
    pub nixos_modules: Vec<Module>,
    /// HomeManager modules.
    pub home_modules: Vec<Module>,
}

impl Flake {
    /// Constructs a new instance.
    pub fn new(source: &str) -> Result<Self> {
        let temp = TempDir::new("flamingo")?;
        Ok(Self {
            source: source.to_string(),
            temp,
            nixos_modules: Vec::new(),
            home_modules: Vec::new(),
        })
    }

    /// Runs `nix eval <source>#<module_type> --apply <apply> --json`.
    /// Util to extract infos from configuration.
    fn nix_eval(&self, module_type: &str, apply: &str) -> Result<Vec<u8>> {
        let out = Command::new("nix")
            .args([
                "eval",
                &format!("{}#{}", self.source, module_type),
                "--apply",
                apply,
                "--json",
            ])
            .output()?;
        if !out.status.success() {
            return Err(Error::ConfigError(std::io::Error::other(
                String::from_utf8_lossy(&out.stderr).to_string(),
            )));
        }
        Ok(out.stdout)
    }

    /// Extracts Nixos and HomeManager modules.
    pub fn extract_modules(&mut self, config: &AppConfig) -> Result<()> {
        // Maps each module in `nixosModules` to the list of files it pulls in,
        // by recursively collecting `_file` (path the module system records for
        // each module) through `imports`. Yields `{ name: [file, ...] }` as JSON.
        //
        // Function modules ({ config, ... }: { ... }) are left opaque on
        // purpose: calling them with stub arguments aborts the whole eval on
        // modules that force an argument (tryEval cannot catch coercion or
        // missing-attribute errors). Their children are recovered textually in
        // `recover_function_module_children` instead.
        const COLLECT_MODULE_FILES: &str = "ms: builtins.mapAttrs (_: m: \
        let collect = m: \
          if !(builtins.isAttrs m) then [] \
          else (if m ? _file then [ m._file ] else []) \
               ++ builtins.concatMap collect (m.imports or []); \
        in collect m) ms";

        // Should get this:
        // vec![("desktop", vec![
        //   "/repo/desktop.nix, via option flake.nixosModules.desktop",
        //   "/repo/gnome.nix",
        //   "/repo/fonts.nix",
        // ])]
        let nixos_modules: BTreeMap<String, Vec<String>> =
            serde_json::from_slice(&self.nix_eval("nixosModules", COLLECT_MODULE_FILES)?)?;

        let home_modules: BTreeMap<String, Vec<String>> =
            serde_json::from_slice(&self.nix_eval("homeModules", COLLECT_MODULE_FILES)?)?;

        self.nixos_modules = nixos_modules
            .into_iter()
            .map(|(name, files)| {
                let added = config.modules.nixos.contains(&name);
                Module::from_files(name, files, added)
            })
            .collect();

        self.home_modules = home_modules
            .into_iter()
            .map(|(name, files)| {
                let added = config.modules.home.contains(&name);
                Module::from_files(name, files, added)
            })
            .collect();

        self.recover_function_module_children();
        Ok(())
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
    fn recover_function_module_children(&mut self) {
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
                std::path::Path::new(path)
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

    /// Returns the module list as `(depth, module index)` rows, nesting each
    /// module under the modules whose transitive files include its entry file.
    ///
    /// Transitive reduction keeps only direct children (a module included via
    /// an intermediate module is shown under the intermediate one). Modules
    /// grouped by several parents appear under each of them.
    pub fn module_tree(&self) -> Vec<(usize, usize)> {
        let includes = |a: usize, b: usize| -> bool {
            a != b
                && self.modules[b].path.as_ref().is_some_and(|entry| {
                    self.modules[a].path.as_deref() != Some(entry.as_str())
                        && self.modules[a].files.iter().any(|file| file == entry)
                })
        };
        let count = self.modules.len();
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); count];
        let mut is_child = vec![false; count];
        for (a, children) in children.iter_mut().enumerate() {
            for (b, is_child) in is_child.iter_mut().enumerate() {
                // Keep only direct children: skip b if it is reachable
                // through another module a also includes.
                if includes(a, b) && !(0..count).any(|c| includes(a, c) && includes(c, b)) {
                    children.push(b);
                    *is_child = true;
                }
            }
        }

        fn walk(
            children: &[Vec<usize>],
            index: usize,
            depth: usize,
            trail: &mut Vec<usize>,
            seen: &mut [bool],
            rows: &mut Vec<(usize, usize)>,
        ) {
            if trail.contains(&index) {
                return;
            }
            seen[index] = true;
            rows.push((depth, index));
            trail.push(index);
            for &child in &children[index] {
                walk(children, child, depth + 1, trail, seen, rows);
            }
            trail.pop();
        }

        let mut rows = Vec::new();
        let mut seen = vec![false; count];
        let mut trail = Vec::new();
        for (index, _) in is_child
            .iter()
            .enumerate()
            .filter(|(_, is_child)| !**is_child)
        {
            walk(&children, index, 0, &mut trail, &mut seen, &mut rows);
        }
        // Import cycles can leave modules unreachable from any root; list
        // them flat rather than dropping them.
        for (index, _) in seen.iter().enumerate().filter(|(_, seen)| !**seen) {
            rows.push((0, index));
        }
        rows
    }

    /// Returns the added modules for export, flagging which ones the
    /// generated host profile imports directly. A module reachable from
    /// another added module arrives transitively and must not be wired
    /// again (see [`ExportModule::top_level`]).
    pub fn export_selection(&self) -> Vec<ExportModule> {
        let added: Vec<&Module> = self.modules.iter().filter(|module| module.added).collect();
        added
            .iter()
            .map(|module| {
                let top_level = !added.iter().any(|parent| {
                    parent.name != module.name
                        && module.path.as_ref().is_some_and(|entry| {
                            parent.path.as_deref() != Some(entry.as_str())
                                && parent.files.iter().any(|file| file == entry)
                        })
                });
                ExportModule {
                    name: module.name.clone(),
                    path: module.path.clone().map(PathBuf::from),
                    top_level,
                }
            })
            .collect()
    }

    /// Marks every module whose name appears in `names` as added, cascading
    /// to its subtree exactly like the manual toggle in the TUI. Unknown
    /// names log a warning: module sets vary per flake source.
    pub fn apply_default_modules(&mut self, names: &[String]) {
        let rows = self.module_tree();
        for name in names {
            let mut matched = false;
            let mut row = 0;
            while row < rows.len() {
                let (depth, index) = rows[row];
                if &self.modules[index].name == name {
                    matched = true;
                    self.modules[index].added = true;
                    let mut child = row + 1;
                    while child < rows.len() && rows[child].0 > depth {
                        self.modules[rows[child].1].added = true;
                        child += 1;
                    }
                    row = child;
                } else {
                    row += 1;
                }
            }
            if !matched {
                log::warn!("default module `{name}` not found in source");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use crate::{app::Flake, module::Module};

    #[fixture]
    fn flake() -> Flake {
        Flake::new("https://github.com/wallago/nix-config")
            .expect("failed to create flake fixture (tempdir creation)")
    }

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
    fn should_process_flake(flake: Flake) {
        assert_eq!(flake.source, "https://github.com/wallago/nix-config");
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

    //     fn module(name: &str, path: &str, files: &[&str]) -> Module {
    //         Module {
    //             name: name.to_string(),
    //             path: Some(path.to_string()),
    //             content: None,
    //             files: files.iter().map(|file| file.to_string()).collect(),
    //             added: false,
    //         }
    //     }
    //
    //     #[test]
    //     fn test_module_tree_nesting() {
    //         let mut config = Config::new(".").expect("tempdir");
    //         config.modules = vec![
    //             module(
    //                 "desktop",
    //                 "/r/desktop.nix",
    //                 &["/r/desktop.nix", "/r/apps.nix", "/r/firefox.nix"],
    //             ),
    //             module("apps", "/r/apps.nix", &["/r/apps.nix", "/r/firefox.nix"]),
    //             module("firefox", "/r/firefox.nix", &["/r/firefox.nix"]),
    //             module("server", "/r/server.nix", &["/r/server.nix"]),
    //         ];
    //         // desktop > apps > firefox, server standalone; firefox is NOT a
    //         // direct child of desktop (reduced through apps).
    //         assert_eq!(config.module_tree(), vec![(0, 0), (1, 1), (2, 2), (0, 3)]);
    //     }
    //
    //     #[test]
    //     fn test_extract_modules_sees_through_function_modules() {
    //         let dir = tempdir::TempDir::new("flamingo-flake").expect("tempdir");
    //         let modules_dir = dir.path().join("modules");
    //         let hosts_dir = modules_dir.join("hosts").join("coral");
    //         std::fs::create_dir_all(&hosts_dir).expect("hosts dir");
    //         let ssh_path = modules_dir.join("ssh.nix");
    //         let networking_path = modules_dir.join("networking.nix");
    //         let host_path = hosts_dir.join("configuration.nix");
    //         std::fs::write(&ssh_path, "{ services.openssh.enable = true; }").expect("write ssh");
    //         // Existing hosts import most of the module set; they must not be
    //         // inspected (exports scaffold a fresh host instead).
    //         std::fs::write(
    //             &host_path,
    //             "{ imports = [ self.nixosModules.ssh self.nixosModules.networking ]; }",
    //         )
    //         .expect("write host");
    //         // Mirrors the real breakage: a function module referencing a sibling
    //         // (static, discoverable) and one through interpolation (dynamic,
    //         // must be skipped without aborting).
    //         std::fs::write(
    //             &networking_path,
    //             r#"{ hostName, ... }:
    // {
    //   imports = [
    //     self.nixosModules.ssh
    //     self.nixosModules."preferencesAttic${self.lib.capitalize hostName}"
    //   ];
    // }"#,
    //         )
    //         .expect("write networking");
    //         std::fs::write(
    //             dir.path().join("flake.nix"),
    //             format!(
    //                 r#"{{
    //   outputs = {{ self, ... }}: {{
    //     nixosModules = {{
    //       ssh = {{
    //         _file = "{ssh}, via option flake.nixosModules.ssh";
    //         imports = [ ];
    //       }};
    //       networking = {{
    //         _file = "{networking}, via option flake.nixosModules.networking";
    //         imports = [
    //           ({{ hostName, ... }}: {{ imports = [ self.nixosModules.ssh ]; }})
    //         ];
    //       }};
    //       configCoral = {{
    //         _file = "{host}, via option flake.nixosModules.configCoral";
    //         imports = [ ];
    //       }};
    //     }};
    //   }};
    // }}"#,
    //                 ssh = ssh_path.display(),
    //                 networking = networking_path.display(),
    //                 host = host_path.display(),
    //             ),
    //         )
    //         .expect("write flake");
    //
    //         let mut config = Config::new(dir.path().to_str().expect("utf8 path")).expect("config");
    //         config.extract_modules().expect("extract");
    //
    //         let networking = config
    //             .modules
    //             .iter()
    //             .find(|module| module.name == "networking")
    //             .expect("networking extracted");
    //         // The import hidden inside the function body must be recovered.
    //         assert!(
    //             networking
    //                 .files
    //                 .iter()
    //                 .any(|file| file == ssh_path.to_str().expect("utf8 path")),
    //             "networking files miss ssh: {:?}",
    //             networking.files
    //         );
    //         // The dynamic interpolated reference must not invent a child.
    //         assert!(
    //             !networking
    //                 .files
    //                 .iter()
    //                 .any(|file| file.contains("preferencesAttic"))
    //         );
    //         // Host modules are not inspected: no recovered children despite the
    //         // references in their file.
    //         let host = config
    //             .modules
    //             .iter()
    //             .find(|module| module.name == "configCoral")
    //             .expect("configCoral extracted");
    //         assert_eq!(host.files, vec![host_path.to_str().expect("utf8 path")]);
    //         // And the tree must nest ssh under networking, with the host flat
    //         // (BTreeMap order: configCoral, networking, ssh).
    //         assert_eq!(config.module_tree(), vec![(0, 0), (0, 1), (1, 2)]);
    //     }
    //
    //     #[test]
    //     fn test_module_tree_cycle_falls_back_flat() {
    //         let mut config = Config::new(".").expect("tempdir");
    //         config.modules = vec![
    //             module("a", "/r/a.nix", &["/r/a.nix", "/r/b.nix"]),
    //             module("b", "/r/b.nix", &["/r/b.nix", "/r/a.nix"]),
    //         ];
    //         let rows = config.module_tree();
    //         // Both modules stay visible despite the cycle.
    //         let mut indices: Vec<usize> = rows.iter().map(|(_, index)| *index).collect();
    //         indices.sort_unstable();
    //         indices.dedup();
    //         assert_eq!(indices, vec![0, 1]);
    //     }
    //
    //     #[test]
    //     fn test_apply_default_modules_marks_matches_and_children() {
    //         let mut config = Config::new(".").expect("tempdir");
    //         config.modules = vec![
    //             module(
    //                 "desktop",
    //                 "/r/desktop.nix",
    //                 &["/r/desktop.nix", "/r/apps.nix", "/r/firefox.nix"],
    //             ),
    //             module("apps", "/r/apps.nix", &["/r/apps.nix", "/r/firefox.nix"]),
    //             module("firefox", "/r/firefox.nix", &["/r/firefox.nix"]),
    //             module("server", "/r/server.nix", &["/r/server.nix"]),
    //         ];
    //         config.apply_default_modules(&["apps".to_string()]);
    //         // `apps` and its child `firefox` are added; parents/siblings are not.
    //         assert!(!config.modules[0].added);
    //         assert!(config.modules[1].added);
    //         assert!(config.modules[2].added);
    //         assert!(!config.modules[3].added);
    //     }
    //
    //     #[test]
    //     fn test_export_selection_wires_only_top_level_modules() {
    //         let mut config = Config::new(".").expect("tempdir");
    //         config.modules = vec![
    //             module(
    //                 "desktop",
    //                 "/r/desktop.nix",
    //                 &["/r/desktop.nix", "/r/apps.nix", "/r/firefox.nix"],
    //             ),
    //             module("apps", "/r/apps.nix", &["/r/apps.nix", "/r/firefox.nix"]),
    //             module("firefox", "/r/firefox.nix", &["/r/firefox.nix"]),
    //             module("server", "/r/server.nix", &["/r/server.nix"]),
    //         ];
    //         for module in &mut config.modules {
    //             module.added = true;
    //         }
    //         let selection = config.export_selection();
    //         let wired: Vec<(&str, bool)> = selection
    //             .iter()
    //             .map(|module| (module.name.as_str(), module.top_level))
    //             .collect();
    //         // Children arrive transitively through their parent: wiring them
    //         // again would import their flake-parts wrapper twice.
    //         assert_eq!(
    //             wired,
    //             vec![
    //                 ("desktop", true),
    //                 ("apps", false),
    //                 ("firefox", false),
    //                 ("server", true),
    //             ]
    //         );
    //     }
    //
    //     #[test]
    //     fn test_export_selection_child_alone_is_top_level() {
    //         let mut config = Config::new(".").expect("tempdir");
    //         config.modules = vec![
    //             module("apps", "/r/apps.nix", &["/r/apps.nix", "/r/firefox.nix"]),
    //             module("firefox", "/r/firefox.nix", &["/r/firefox.nix"]),
    //         ];
    //         config.modules[1].added = true;
    //         let selection = config.export_selection();
    //         assert_eq!(selection.len(), 1);
    //         assert_eq!(selection[0].name, "firefox");
    //         assert!(selection[0].top_level);
    //     }
    //
    //     #[test]
    //     fn test_apply_default_modules_unknown_name_is_ignored() {
    //         let mut config = Config::new(".").expect("tempdir");
    //         config.modules = vec![module("server", "/r/server.nix", &["/r/server.nix"])];
    //         config.apply_default_modules(&["nope".to_string()]);
    //         assert!(!config.modules[0].added);
    //     }
}
