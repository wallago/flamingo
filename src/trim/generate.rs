use std::path::PathBuf;

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
        let declaration =
            src[after..].starts_with('.') || src[after..].trim_start().starts_with('=');
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

/// Generates the new config's flake.nix.
pub fn flake_nix(inputs: &str, hostname: &str, source: &str, module_paths: &[PathBuf]) -> String {
    let modules = module_paths
        .iter()
        .map(|path| format!("        ./{}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{{
  description = \"{hostname} — trimmed from {source} by flamingo\";

  {inputs}

  outputs = {{ self, nixpkgs, ... }}@inputs: {{
    nixosConfigurations.{hostname} = nixpkgs.lib.nixosSystem {{
      system = \"x86_64-linux\";
      specialArgs = {{ inherit inputs; }};
      modules = [
        ./hosts/{hostname}
{modules}
      ];
    }};
  }};
}}
"
    )
}

/// Generates hosts/<hostname>/default.nix.
pub fn host_default_nix(hostname: &str, state_version: Option<&str>) -> String {
    let state_version = match state_version {
        Some(version) => format!("  system.stateVersion = \"{version}\";"),
        None => {
            "  # system.stateVersion = \"CHANGE-ME\"; # flamingo could not detect the nixpkgs release"
                .to_string()
        }
    };
    format!(
        "{{ ... }}:
{{
  imports = [ ./hardware-configuration.nix ];

  networking.hostName = \"{hostname}\";

{state_version}
}}
"
    )
}

/// Placeholder hardware config pointing the user at nixos-generate-config.
pub fn hardware_placeholder() -> String {
    "# Replace this file with the real hardware configuration of the target
# machine, generated on that machine with:
#
#   nixos-generate-config --show-hw-config > hardware-configuration.nix
{ ... }:
{ }
"
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
    fn test_flake_nix() {
        let out = flake_nix(
            "inputs.nixpkgs.url = \"github:NixOS/nixpkgs\";",
            "myhost",
            "github:wallago/nix-config",
            &[
                PathBuf::from("modules/foo.nix"),
                PathBuf::from("modules/ssh"),
            ],
        );
        assert!(out.contains("nixosConfigurations.myhost"));
        assert!(out.contains("./hosts/myhost"));
        assert!(out.contains("./modules/foo.nix"));
        assert!(out.contains("./modules/ssh"));
        assert!(out.contains("trimmed from github:wallago/nix-config"));
    }

    #[test]
    fn test_host_default_nix() {
        let with = host_default_nix("myhost", Some("25.05"));
        assert!(with.contains("networking.hostName = \"myhost\";"));
        assert!(with.contains("system.stateVersion = \"25.05\";"));
        assert!(with.contains("./hardware-configuration.nix"));
        let without = host_default_nix("myhost", None);
        assert!(without.contains("# system.stateVersion"));
    }
}
