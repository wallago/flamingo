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
                Module {
                    name,
                    path,
                    content,
                    added: false,
                }
            })
            .collect();
        Ok(())
    }
}
