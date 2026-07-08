use std::collections::BTreeMap;
use std::process::Command;

use crate::{
    app::Flake,
    config::AppConfig,
    error::{Error, Result},
    module::Module,
};

impl Flake {
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

        // Order matters: opaque modules need a file before their children
        // can be recovered from it, and cascading needs the final tree.
        self.recover_missing_module_paths();
        self.recover_function_module_children();
        self.cascade_added_modules();

        Ok(())
    }
}
