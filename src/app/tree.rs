use std::path::PathBuf;

use crate::{app::Flake, module::Module, trim::ExportModule};

impl Flake {
    /// Returns the module list nesting each module under the modules.
    /// Transitive reduction keeps only direct children.
    pub fn module_tree(&self, modules: &[Module]) -> Vec<(usize, usize)> {
        let includes = |a: usize, b: usize| -> bool {
            a != b
                // Check if module b have a path
                && modules[b].path.as_ref().is_some_and(|entry| {
                    // Check if module a have a path
                    modules[a].path.as_deref() != Some(entry.as_str())
                        // Check if module a have module b inside its files
                        && modules[a].files.iter().any(|file| file == entry)
                })
        };
        let count = modules.len();
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

    /// Cascades `added` from config-marked modules to their subtrees,
    /// mirroring the manual toggle in the TUI.
    pub(super) fn cascade_added_modules(&mut self) {
        let rows = self.module_tree(&self.nixos_modules);
        Self::cascade_rows(&rows, &mut self.nixos_modules);
        let rows = self.module_tree(&self.home_modules);
        Self::cascade_rows(&rows, &mut self.home_modules);
    }

    /// Marks everything below an added row as added.
    fn cascade_rows(rows: &[(usize, usize)], modules: &mut [Module]) {
        let mut row = 0;
        while row < rows.len() {
            let (depth, index) = rows[row];
            if modules[index].added {
                let mut child = row + 1;
                while child < rows.len() && rows[child].0 > depth {
                    modules[rows[child].1].added = true;
                    child += 1;
                }
                row = child;
            } else {
                row += 1;
            }
        }
    }

    /// Returns the added modules for export, flagging which ones the
    /// generated host profile imports directly.
    pub fn export_selection(&self, modules: &[Module]) -> Vec<ExportModule> {
        let added: Vec<&Module> = modules.iter().filter(|module| module.added).collect();
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
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};

    use crate::{app::Flake, module::Module};

    fn module(name: &str, path: &str, files: &[&str]) -> Module {
        Module {
            name: name.to_string(),
            path: Some(path.to_string()),
            content: None,
            files: files.iter().map(|file| file.to_string()).collect(),
            added: false,
        }
    }

    #[fixture]
    fn flake() -> Flake {
        Flake::new(".").expect("flake")
    }

    #[rstest]
    fn module_tree_nests_and_reduces(flake: Flake) {
        let modules = vec![
            module(
                "desktop",
                "/r/desktop.nix",
                &["/r/desktop.nix", "/r/apps.nix", "/r/firefox.nix"],
            ),
            module("apps", "/r/apps.nix", &["/r/apps.nix", "/r/firefox.nix"]),
            module("firefox", "/r/firefox.nix", &["/r/firefox.nix"]),
            module("server", "/r/server.nix", &["/r/server.nix"]),
        ];
        // desktop > apps > firefox, server standalone; firefox is NOT a
        // direct child of desktop (reduced through apps).
        assert_eq!(
            flake.module_tree(&modules),
            vec![(0, 0), (1, 1), (2, 2), (0, 3)]
        );
    }

    #[rstest]
    fn module_tree_cycle_falls_back_flat(flake: Flake) {
        let modules = vec![
            module("a", "/r/a.nix", &["/r/a.nix", "/r/b.nix"]),
            module("b", "/r/b.nix", &["/r/b.nix", "/r/a.nix"]),
        ];
        let rows = flake.module_tree(&modules);
        // Both modules stay visible despite the cycle.
        let mut indices: Vec<usize> = rows.iter().map(|(_, index)| *index).collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices, vec![0, 1]);
    }

    #[rstest]
    fn cascade_marks_added_subtrees(mut flake: Flake) {
        flake.nixos_modules = vec![
            module(
                "desktop",
                "/r/desktop.nix",
                &["/r/desktop.nix", "/r/apps.nix", "/r/firefox.nix"],
            ),
            module("apps", "/r/apps.nix", &["/r/apps.nix", "/r/firefox.nix"]),
            module("firefox", "/r/firefox.nix", &["/r/firefox.nix"]),
            module("server", "/r/server.nix", &["/r/server.nix"]),
        ];
        flake.nixos_modules[1].added = true;
        flake.cascade_added_modules();
        // `apps` and its child `firefox` are added; parents/siblings are not.
        assert!(!flake.nixos_modules[0].added);
        assert!(flake.nixos_modules[1].added);
        assert!(flake.nixos_modules[2].added);
        assert!(!flake.nixos_modules[3].added);
    }

    #[rstest]
    fn export_selection_wires_only_top_level_modules(mut flake: Flake) {
        flake.nixos_modules = vec![
            module(
                "desktop",
                "/r/desktop.nix",
                &["/r/desktop.nix", "/r/apps.nix", "/r/firefox.nix"],
            ),
            module("apps", "/r/apps.nix", &["/r/apps.nix", "/r/firefox.nix"]),
            module("firefox", "/r/firefox.nix", &["/r/firefox.nix"]),
            module("server", "/r/server.nix", &["/r/server.nix"]),
        ];
        for module in &mut flake.nixos_modules {
            module.added = true;
        }
        let wired: Vec<(String, bool)> = flake
            .export_selection()
            .into_iter()
            .map(|module| (module.name, module.top_level))
            .collect();
        // Children arrive transitively through their parent: wiring them
        // again would import their flake-parts wrapper twice.
        assert_eq!(
            wired,
            vec![
                ("desktop".to_string(), true),
                ("apps".to_string(), false),
                ("firefox".to_string(), false),
                ("server".to_string(), true),
            ]
        );
    }

    #[rstest]
    fn export_selection_child_alone_is_top_level(mut flake: Flake) {
        flake.nixos_modules = vec![
            module("apps", "/r/apps.nix", &["/r/apps.nix", "/r/firefox.nix"]),
            module("firefox", "/r/firefox.nix", &["/r/firefox.nix"]),
        ];
        flake.nixos_modules[1].added = true;
        let selection = flake.export_selection();
        assert_eq!(selection.len(), 1);
        assert_eq!(selection[0].name, "firefox");
        assert!(selection[0].top_level);
    }
}
