use flamingo::trim::{self, ExportModule, ExportRequest};
use std::fs;
use tempdir::TempDir;

fn write(root: &std::path::Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn fixture() -> TempDir {
    let dir = TempDir::new("flamingo-fixture").unwrap();
    write(
        dir.path(),
        "flake.nix",
        r#"{
  description = "fixture";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };
  outputs = { self, nixpkgs, ... }@inputs: {
    nixosModules.foo = ./modules/foo.nix;
  };
}"#,
    );
    write(
        dir.path(),
        "modules/foo.nix",
        "{ ... }: { imports = [ ../common ]; script = ../scripts/hello.sh; }",
    );
    write(
        dir.path(),
        "common/default.nix",
        "{ ... }: { imports = [ ./base.nix ]; }",
    );
    write(dir.path(), "common/base.nix", "{ }");
    write(dir.path(), "scripts/hello.sh", "echo hi");
    dir
}

#[test]
fn test_export_creates_standalone_config() {
    let src = fixture();
    let out_parent = TempDir::new("flamingo-out").unwrap();
    let output_dir = out_parent.path().join("new-config");

    let report = trim::export(&ExportRequest {
        source: src.path().to_string_lossy().into_owned(),
        hostname: "myhost".into(),
        output_dir: output_dir.clone(),
        modules: vec![ExportModule {
            name: "foo".into(),
            path: Some(src.path().join("modules/foo.nix")),
        }],
    })
    .unwrap();

    for rel in [
        "flake.nix",
        "hosts/myhost/default.nix",
        "hosts/myhost/hardware-configuration.nix",
        "modules/foo.nix",
        "common/default.nix",
        "common/base.nix",
        "scripts/hello.sh",
    ] {
        assert!(output_dir.join(rel).is_file(), "missing {rel}");
    }
    let flake = fs::read_to_string(output_dir.join("flake.nix")).unwrap();
    assert!(flake.contains("nixosConfigurations.myhost"));
    assert!(flake.contains("./modules/foo.nix"));
    assert!(flake.contains("github:NixOS/nixpkgs/nixos-unstable"));
    let host = fs::read_to_string(output_dir.join("hosts/myhost/default.nix")).unwrap();
    assert!(host.contains("networking.hostName = \"myhost\";"));
    assert_eq!(report.output_dir, output_dir);
    assert!(report.files_written.contains(&"modules/foo.nix".into()));
}

#[test]
fn test_export_refuses_non_empty_output_dir() {
    let src = fixture();
    let out = TempDir::new("flamingo-out").unwrap();
    fs::write(out.path().join("existing.txt"), "hi").unwrap();

    let result = trim::export(&ExportRequest {
        source: src.path().to_string_lossy().into_owned(),
        hostname: "myhost".into(),
        output_dir: out.path().to_path_buf(),
        modules: vec![ExportModule {
            name: "foo".into(),
            path: Some(src.path().join("modules/foo.nix")),
        }],
    });
    assert!(result.is_err());
    // Nothing half-written next to the existing file.
    assert_eq!(fs::read_dir(out.path()).unwrap().count(), 1);
}

#[test]
fn test_expand_tilde() {
    let home = std::env::var("HOME").unwrap();
    assert_eq!(
        trim::expand_tilde("~/cfg"),
        std::path::Path::new(&home).join("cfg")
    );
    assert_eq!(
        trim::expand_tilde("/abs/cfg"),
        std::path::Path::new("/abs/cfg")
    );
}
