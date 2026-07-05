use crate::{
    error::{Error, Result},
    module::Module,
};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::process::Command;
use tempdir::TempDir;

/// Nixos Config.
#[derive(Debug)]
pub struct Config {
    /// Path of config.
    pub source: String,
    /// Tempdir to achieve all operations.
    temp: TempDir,
    /// Config modules.
    pub modules: Vec<Module>,
}

impl Config {
    /// Constructs a new instance.
    pub fn new(source: &str) -> Result<Self> {
        let temp = TempDir::new("flamingo")?;
        Ok(Self {
            source: source.to_string(),
            temp,
            modules: Vec::new(),
        })
    }

    /// Runs `nix eval <source>#<module_type> --apply <apply> --json`.
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

    /// Extracts modules.
    pub fn extract_modules(&mut self) -> Result<()> {
        let modules: BTreeMap<String, Vec<String>> = serde_json::from_slice(&self.nix_eval(
            "nixosModules",
            "ms: builtins.mapAttrs (_: m: \
           let collect = m: \
             if !(builtins.isAttrs m) then [] \
             else (if m ? _file then [ m._file ] else []) \
                  ++ builtins.concatMap collect (m.imports or []); \
           in collect m) ms",
        )?)?;

        self.modules = modules
            .into_iter()
            .map(|(name, files)| {
                let path = files
                    .iter()
                    .find_map(|file| file.split_once(", via option"))
                    .map(|(path, _)| path.to_string());
                let content = path
                    .as_deref()
                    .and_then(|path| fs::read_to_string(path).ok());
                let files = files
                    .iter()
                    .map(|file| {
                        file.split_once(", via option")
                            .map_or_else(|| file.clone(), |(path, _)| path.to_string())
                    })
                    .collect();
                Module {
                    name,
                    path,
                    content,
                    files,
                    added: false,
                }
            })
            .collect();
        Ok(())
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
        for (index, _) in is_child.iter().enumerate().filter(|(_, is_child)| !**is_child) {
            walk(&children, index, 0, &mut trail, &mut seen, &mut rows);
        }
        // Import cycles can leave modules unreachable from any root; list
        // them flat rather than dropping them.
        for (index, _) in seen.iter().enumerate().filter(|(_, seen)| !**seen) {
            rows.push((0, index));
        }
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(name: &str, path: &str, files: &[&str]) -> Module {
        Module {
            name: name.to_string(),
            path: Some(path.to_string()),
            content: None,
            files: files.iter().map(|file| file.to_string()).collect(),
            added: false,
        }
    }

    #[test]
    fn test_module_tree_nesting() {
        let mut config = Config::new(".").expect("tempdir");
        config.modules = vec![
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
            config.module_tree(),
            vec![(0, 0), (1, 1), (2, 2), (0, 3)]
        );
    }

    #[test]
    fn test_module_tree_cycle_falls_back_flat() {
        let mut config = Config::new(".").expect("tempdir");
        config.modules = vec![
            module("a", "/r/a.nix", &["/r/a.nix", "/r/b.nix"]),
            module("b", "/r/b.nix", &["/r/b.nix", "/r/a.nix"]),
        ];
        let rows = config.module_tree();
        // Both modules stay visible despite the cycle.
        let mut indices: Vec<usize> = rows.iter().map(|(_, index)| *index).collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices, vec![0, 1]);
    }
}
