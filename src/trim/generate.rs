/// Lifts every top-level `inputs …;` declaration from flake.nix source,
/// Known limitation: brace-depth tracking ignores braces inside strings;
/// input URLs virtually never contain braces.
pub fn extract_inputs(src: &str) -> Option<String> {
    let mut found: Vec<String> = Vec::new();
    let mut i = 0;
    while let Some(offset) = src[i..].find("inputs") {
        let start = i + offset;
        let after = start + "inputs".len();
        i = after;
        let boundary_before = !src[..start]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '@');
        let declaration = if src[after..].starts_with('.') {
            // Dotted form: require `=` right after the attribute path
            // (`inputs.foo.url = …`), rejecting usages like
            // `inputs.flake-parts.lib.mkFlake { … }`.
            src[after..]
                .trim_start_matches(|c: char| {
                    c == '.' || c == '-' || c == '_' || c == '"' || c.is_alphanumeric()
                })
                .trim_start()
                .starts_with('=')
        } else {
            src[after..].trim_start().starts_with('=')
        };
        if !boundary_before || !declaration {
            continue;
        }
        let mut depth = 0usize;
        let mut end = None;
        for (j, c) in src[start..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                ';' if depth == 0 => {
                    end = Some(start + j + 1);
                    break;
                }
                _ => {}
            }
        }
        let end = end?;
        found.push(src[start..end].to_string());
        i = end;
    }
    if found.is_empty() {
        None
    } else {
        Some(found.join("\n  "))
    }
}

/// Uppercases the first character, matching the source config's
/// `config<Hostname>` attribute naming.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Generates the new config's flake.nix.
///
/// The output mirrors the source's flake-parts shape: modules (including the
/// generated host) are auto-imported from ./modules, so files copied verbatim
/// keep evaluating at flake level exactly as they did in the source config.
pub fn flake_nix(inputs: &str, hostname: &str, source: &str) -> String {
    let mut inputs = inputs.to_string();
    // The generated outputs need flake-parts and import-tree even when the
    // source declared its inputs some other way.
    if !inputs.contains("flake-parts") {
        inputs.push_str("\n  inputs.flake-parts.url = \"github:hercules-ci/flake-parts\";");
    }
    if !inputs.contains("import-tree") {
        inputs.push_str("\n  inputs.import-tree.url = \"github:vic/import-tree\";");
    }
    format!(
        "{{
  description = \"{hostname} — trimmed from {source} by flamingo\";

  {inputs}

  outputs = inputs: inputs.flake-parts.lib.mkFlake {{ inherit inputs; }} (inputs.import-tree ./modules);
}}
"
    )
}

/// Generates modules/hosts/<hostname>/default.nix: the hardcoded profile
/// wiring the selected modules by name, in the source config's idiom.
pub fn host_default_nix(hostname: &str, state_version: Option<&str>, modules: &[String]) -> String {
    let capitalized = capitalize(hostname);
    let imports = modules
        .iter()
        .map(|name| format!("      self.nixosModules.{name}"))
        .collect::<Vec<_>>()
        .join("\n");
    let state_version = match state_version {
        Some(version) => format!("    system.stateVersion = \"{version}\";"),
        None => {
            "    # system.stateVersion = \"CHANGE-ME\"; # flamingo could not detect the nixpkgs release"
                .to_string()
        }
    };
    format!(
        "{{ inputs, self, ... }}:
{{
  flake.nixosConfigurations.{hostname} = inputs.nixpkgs.lib.nixosSystem {{
    specialArgs = {{
      hostName = \"{hostname}\";
      inherit self;
    }};
    modules = [
      self.nixosModules.config{capitalized}
      self.nixosModules.hardware{capitalized}
    ];
  }};

  flake.nixosModules.config{capitalized} = {{
    imports = [
{imports}
    ];

    networking.hostName = \"{hostname}\";
{state_version}
  }};
}}
"
    )
}

/// Generates modules/hosts/<hostname>/hardware.nix, wrapping the raw
/// hardware file as a named module the way the source config does.
pub fn host_hardware_nix(hostname: &str) -> String {
    let capitalized = capitalize(hostname);
    format!(
        "{{
  flake.nixosModules.hardware{capitalized} = ./_hardware-configuration.nix;
}}
"
    )
}

/// Placeholder hardware config pointing the user at nixos-generate-config.
///
/// Underscore-prefixed on disk so import-tree does not auto-import it as a
/// flake-parts module; it only enters the eval through the hardware wrapper.
pub fn hardware_placeholder() -> String {
    "# Replace this file with the real hardware configuration of the target
# machine, generated on that machine with:
#
#   nixos-generate-config --show-hw-config > hardware-configuration.nix
{ ... }:
{
  nixpkgs.hostPlatform = \"x86_64-linux\";
}
"
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_inputs_block() {
        let src = r#"{
  description = "cfg";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = { self, nixpkgs, ... }@inputs: { };
}"#;
        let inputs = extract_inputs(src).unwrap();
        assert!(inputs.starts_with("inputs = {"));
        assert!(inputs.contains("home-manager"));
        assert!(inputs.trim_end().ends_with("};"));
        // The nested `inputs.nixpkgs.follows` must not be extracted twice.
        assert_eq!(inputs.matches("follows").count(), 1);
    }

    #[test]
    fn test_extract_inputs_dotted_and_none() {
        let src = "{\n  inputs.nixpkgs.url = \"github:NixOS/nixpkgs\";\n  outputs = { nixpkgs, ... }@inputs: { };\n}";
        assert_eq!(
            extract_inputs(src).unwrap(),
            "inputs.nixpkgs.url = \"github:NixOS/nixpkgs\";"
        );
        assert_eq!(extract_inputs("{ outputs = _: { }; }"), None);
    }

    #[test]
    fn test_extract_inputs_ignores_usages() {
        let src = r#"{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
  };
  outputs = inputs: inputs.flake-parts.lib.mkFlake { inherit inputs; } (inputs.import-tree ./modules);
}"#;
        let inputs = extract_inputs(src).unwrap();
        assert!(inputs.contains("nixpkgs.url"));
        assert!(!inputs.contains("mkFlake"));
        assert!(!inputs.contains("import-tree"));
    }

    #[test]
    fn test_flake_nix() {
        let out = flake_nix(
            "inputs.nixpkgs.url = \"github:NixOS/nixpkgs\";",
            "myhost",
            "github:wallago/nix-config",
        );
        assert!(out.contains("trimmed from github:wallago/nix-config"));
        // flake-parts shape: modules are auto-imported, never listed.
        assert!(
            out.contains(
                "outputs = inputs: \
                 inputs.flake-parts.lib.mkFlake { inherit inputs; } \
                 (inputs.import-tree ./modules);"
            )
        );
        // The machinery inputs are appended when the source lacks them.
        assert!(out.contains("flake-parts.url"));
        assert!(out.contains("import-tree.url"));
    }

    #[test]
    fn test_flake_nix_keeps_existing_machinery_inputs() {
        let out = flake_nix(
            "inputs = {\n    nixpkgs.url = \"github:NixOS/nixpkgs\";\n    \
             flake-parts.url = \"github:hercules-ci/flake-parts\";\n    \
             import-tree.url = \"github:vic/import-tree\";\n  };",
            "myhost",
            "src",
        );
        assert_eq!(out.matches("flake-parts.url").count(), 1);
        assert_eq!(out.matches("import-tree.url").count(), 1);
    }

    #[test]
    fn test_host_default_nix() {
        let with = host_default_nix(
            "myhost",
            Some("25.05"),
            &["networking".to_string(), "ssh".to_string()],
        );
        // Host wiring mirrors the source config's flake-parts idiom.
        assert!(with.contains("{ inputs, self, ... }:"));
        assert!(with.contains("flake.nixosConfigurations.myhost = inputs.nixpkgs.lib.nixosSystem"));
        assert!(with.contains("hostName = \"myhost\";"));
        assert!(with.contains("self.nixosModules.configMyhost"));
        assert!(with.contains("self.nixosModules.hardwareMyhost"));
        assert!(with.contains("flake.nixosModules.configMyhost"));
        // Selected modules are wired by name, one per line.
        assert!(with.contains("self.nixosModules.networking"));
        assert!(with.contains("self.nixosModules.ssh"));
        assert!(with.contains("networking.hostName = \"myhost\";"));
        assert!(with.contains("system.stateVersion = \"25.05\";"));
        let without = host_default_nix("myhost", None, &[]);
        assert!(without.contains("# system.stateVersion"));
    }

    #[test]
    fn test_host_hardware_nix() {
        let out = host_hardware_nix("myhost");
        assert!(out.contains("flake.nixosModules.hardwareMyhost"));
        // The raw hardware file is underscore-prefixed so import-tree
        // ignores it; only the wrapper is auto-imported.
        assert!(out.contains("./_hardware-configuration.nix"));
    }
}
