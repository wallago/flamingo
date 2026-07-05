# Trim & Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export the TUI-selected (`added`) modules of a source NixOS flake into a fresh, self-contained config repo for a new host.

**Architecture:** A pure `src/trim/` pipeline (no ratatui types): `deps.rs` statically scans copied `.nix` files for relative path literals to fixpoint, `generate.rs` produces `flake.nix`/host scaffold text, `mod.rs` orchestrates (locate flake root → scan → stage files in a temp dir → generate → atomic rename) and returns an `ExportReport`. The TUI adds an `e`-triggered two-prompt flow (hostname, output dir) and a result popup.

**Tech Stack:** Rust edition 2024, ratatui 0.30, tui-input, tempdir 0.3, serde_json, regex (new dep), `nix` CLI (eval/metadata), thiserror.

**Spec:** `docs/superpowers/specs/2026-07-05-trim-export-design.md`

## Global Constraints

- Crate has `#![warn(missing_docs, clippy::unwrap_used)]` — every new pub item needs a `///` doc comment; no `unwrap()` outside `#[cfg(test)]` code.
- Repo is jj-colocated on a detached git HEAD. Commit with `jj split <files> -m "<msg>"` (commits only those paths, leaves the rest in the working copy). **Never push.**
- The working copy contains the user's unrelated WIP — never commit files a task didn't touch; never revert files a task doesn't own.
- The working copy currently does NOT compile: `src/app.rs` calls `trim::deps::scan(modules)` which doesn't exist yet, and drops the `self.modules` assignment. Task 1 fixes this first.
- All `nix` subprocess calls must pass `--no-write-lock-file` and degrade gracefully (warning or `None`) when `nix` fails — except `flake_root` for non-local sources, where failure is a hard error.
- Copied files must land at their **source-repo-relative paths** in the output so relative `imports` keep working. No Nix rewriting anywhere.

---

### Task 1: Restore compilation + path-literal scanner (`deps.rs` part 1)

**Files:**
- Modify: `Cargo.toml` (add `regex = "1.11"`)
- Modify: `src/app.rs:63-83` (restore `self.modules` assignment, drop the sketch `scan` call)
- Create: `src/trim/deps.rs` (types + `strip_comments` + `path_literals`; `scan` comes in Task 2)
- Modify: `src/prelude.rs` (user already added `pub use super::trim::deps::*;` — keep it)

**Interfaces:**
- Produces: `Warning` enum (all variants below — later tasks add none), `PathLiteral { text: String, dynamic: bool }`, `fn path_literals(src: &str) -> Vec<PathLiteral>`.

- [ ] **Step 1: Fix `src/app.rs` so the crate compiles**

In `extract_modules`, restore the field assignment and remove the sketch call (dependency scanning happens at export time, not extraction time — see spec):

```rust
        let modules: Vec<Module> = modules
            .into_iter()
            .map(|(name, files)| {
                // ... existing body unchanged ...
            })
            .collect();

        self.modules = modules;
        Ok(())
```

Also remove `trim,` from the `use crate::{...}` list at the top of `app.rs` (nothing in this file uses it anymore).

- [ ] **Step 2: Add the regex dependency**

In `Cargo.toml` under `[dependencies]` (keep alphabetical order):

```toml
regex = "1.11"
```

- [ ] **Step 3: Verify compilation is restored**

Run: `cargo check 2>&1 | tail -3`
Expected: `Finished` line, no errors (warnings are OK).

- [ ] **Step 4: Write failing tests for `path_literals`**

Replace the empty `src/trim/deps.rs` with the types plus tests (implementation stubs come next step — write the full file in Step 5; here, write tests first inside the same file and confirm they fail to compile, which counts as the failing state for a new module):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_literals_basic() {
        let found = path_literals("{ imports = [ ./a.nix ../common ]; }");
        assert_eq!(
            found,
            vec![
                PathLiteral { text: "./a.nix".into(), dynamic: false },
                PathLiteral { text: "../common".into(), dynamic: false },
            ]
        );
    }

    #[test]
    fn test_path_literals_dynamic_and_comments() {
        let src = "# see ./ignored.nix\n/* ./also.nix */\nsource = ./hosts/${name}/x.nix;";
        let found = path_literals(src);
        assert_eq!(found.len(), 1);
        assert!(found[0].dynamic);
        assert_eq!(found[0].text, "./hosts/");
    }

    #[test]
    fn test_path_literals_ignores_ellipsis_and_word_glued_dots() {
        assert!(path_literals("{ pkgs, ... }: {}").is_empty());
        // `.../x` : the leading dot glues to a previous dot — not a path.
        assert!(path_literals("a.../x").is_empty());
    }
}
```

- [ ] **Step 5: Implement `deps.rs` types + scanner primitives**

Full file content (tests from Step 4 stay at the bottom):

```rust
use regex::Regex;
use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Non-fatal problem discovered while trimming a config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// A path literal contains `${...}` interpolation and cannot be followed.
    Dynamic {
        /// File containing the literal.
        file: PathBuf,
        /// The literal text.
        literal: String,
    },
    /// A path literal points outside the source repo.
    OutsideRepo {
        /// File containing the literal.
        file: PathBuf,
        /// The literal text.
        literal: String,
    },
    /// A path literal does not resolve to an existing file.
    Missing {
        /// File containing the literal.
        file: PathBuf,
        /// The literal text.
        literal: String,
    },
    /// A `.nix` file could not be read.
    Unreadable {
        /// The unreadable file.
        file: PathBuf,
    },
    /// An added module has no resolvable source path and was skipped.
    UnresolvedModule {
        /// The module name.
        name: String,
    },
    /// The chosen hostname already exists in the source config.
    HostnameExists {
        /// The colliding hostname.
        hostname: String,
    },
    /// The nixpkgs release could not be determined; stateVersion left commented.
    NoStateVersion,
    /// No inputs were found in the source flake.nix; a default was generated.
    NoInputs,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dynamic { file, literal } => {
                write!(f, "dynamic path `{literal}` in {} skipped", file.display())
            }
            Self::OutsideRepo { file, literal } => {
                write!(f, "`{literal}` in {} points outside the repo", file.display())
            }
            Self::Missing { file, literal } => {
                write!(f, "`{literal}` in {} does not exist", file.display())
            }
            Self::Unreadable { file } => write!(f, "could not read {}", file.display()),
            Self::UnresolvedModule { name } => {
                write!(f, "module `{name}` has no resolvable source, skipped")
            }
            Self::HostnameExists { hostname } => {
                write!(f, "host `{hostname}` already exists in the source config")
            }
            Self::NoStateVersion => {
                write!(f, "could not detect nixpkgs release; stateVersion left commented")
            }
            Self::NoInputs => {
                write!(f, "no inputs found in source flake.nix; default nixpkgs input generated")
            }
        }
    }
}

/// A relative path literal found in Nix source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathLiteral {
    /// The literal text, e.g. `../common/base.nix`.
    pub text: String,
    /// Whether the literal is immediately followed by `${` interpolation.
    pub dynamic: bool,
}

/// Removes `#` line comments and `/* */` block comments from Nix source.
///
/// Deliberately does not track string literals: a `#` inside a string eats
/// the rest of that line, which at worst hides a dependency (warned about
/// later as missing at eval time) and never invents one.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '#' => {
                for (_, c) in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if src[i..].starts_with("/*") => {
                chars.next(); // consume '*'
                let mut prev = ' ';
                for (_, c) in chars.by_ref() {
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Extracts relative path literals (`./…`, `../…`) from Nix source text.
pub fn path_literals(src: &str) -> Vec<PathLiteral> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"\.\.?/[A-Za-z0-9_\-./+@]*").expect("hardcoded regex is valid")
    });
    let stripped = strip_comments(src);
    re.find_iter(&stripped)
        .filter(|m| {
            // Reject matches glued to a preceding word or dot (`a./x`, `.../x`).
            !stripped[..m.start()]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '.' || c == '_')
        })
        .map(|m| PathLiteral {
            text: m.as_str().to_string(),
            dynamic: stripped[m.end()..].starts_with("${"),
        })
        .collect()
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test --lib trim::deps -- --nocapture`
Expected: 3 tests PASS.

- [ ] **Step 7: Commit**

```bash
jj split Cargo.toml Cargo.lock src/app.rs src/trim/deps.rs -m "feat(trim): path-literal scanner and export warnings"
```

---

### Task 2: Transitive dependency scan (`deps.rs` part 2)

**Files:**
- Modify: `src/trim/deps.rs` (append `ScanResult` + `scan`)

**Interfaces:**
- Consumes: `Warning`, `path_literals` from Task 1.
- Produces: `pub struct ScanResult { pub files: BTreeSet<PathBuf>, pub warnings: Vec<Warning> }`, `pub fn scan(root: &Path, seeds: &[PathBuf]) -> ScanResult`. `root` and `seeds` must be canonicalized absolute paths; `files` holds absolute paths; `.nix` files are read and recursed, other files are copied verbatim (no recursion).

- [ ] **Step 1: Write failing tests**

Append to the `tests` module in `src/trim/deps.rs`:

```rust
    use std::fs;
    use tempdir::TempDir;

    fn write(root: &std::path::Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_scan_collects_transitive_deps() {
        let dir = TempDir::new("flamingo-scan").unwrap();
        write(dir.path(), "modules/foo.nix",
            "{ ... }: { imports = [ ../common ]; home.file.\"x\".source = ../scripts/hello.sh; }");
        write(dir.path(), "common/default.nix", "{ ... }: { imports = [ ./base.nix ]; }");
        write(dir.path(), "common/base.nix", "{ }");
        write(dir.path(), "scripts/hello.sh", "echo hi");
        let root = fs::canonicalize(dir.path()).unwrap();

        let result = scan(&root, &[root.join("modules/foo.nix")]);

        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
        let rel: Vec<PathBuf> = result.files.iter()
            .map(|f| f.strip_prefix(&root).unwrap().to_path_buf())
            .collect();
        assert_eq!(rel, vec![
            PathBuf::from("common/base.nix"),
            PathBuf::from("common/default.nix"),
            PathBuf::from("modules/foo.nix"),
            PathBuf::from("scripts/hello.sh"),
        ]);
    }

    #[test]
    fn test_scan_warns_on_missing_dynamic_and_escape() {
        let dir = TempDir::new("flamingo-scan-warn").unwrap();
        write(dir.path(), "repo/mod.nix",
            "{ imports = [ ./gone.nix ]; a = ./cfg/${x}.nix; b = ../outside.nix; }");
        write(dir.path(), "outside.nix", "{ }");
        let root = fs::canonicalize(dir.path().join("repo")).unwrap();

        let result = scan(&root, &[root.join("mod.nix")]);

        assert_eq!(result.files.len(), 1); // only the seed itself
        assert_eq!(result.warnings.len(), 3);
        assert!(result.warnings.iter().any(|w| matches!(w, Warning::Missing { .. })));
        assert!(result.warnings.iter().any(|w| matches!(w, Warning::Dynamic { .. })));
        assert!(result.warnings.iter().any(|w| matches!(w, Warning::OutsideRepo { .. })));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib trim::deps 2>&1 | tail -5`
Expected: compile error — `scan` and `ScanResult` not found.

- [ ] **Step 3: Implement `ScanResult` and `scan`**

Append above the `tests` module (add `use std::fs;` and `use std::path::Path;` to the imports):

```rust
/// Result of a transitive dependency scan.
#[derive(Debug, Default)]
pub struct ScanResult {
    /// Absolute paths of every file to copy.
    pub files: BTreeSet<PathBuf>,
    /// Non-fatal problems encountered while scanning.
    pub warnings: Vec<Warning>,
}

/// Transitively collects the local files referenced by the seed `.nix` files.
///
/// `root` and `seeds` must be canonicalized absolute paths. `.nix` targets
/// are recursed into; directories contribute their `default.nix`; any other
/// existing file is collected as an asset without recursion.
pub fn scan(root: &Path, seeds: &[PathBuf]) -> ScanResult {
    let mut result = ScanResult::default();
    let mut todo: Vec<PathBuf> = seeds.to_vec();
    while let Some(file) = todo.pop() {
        if !result.files.insert(file.clone()) {
            continue;
        }
        let src = match fs::read_to_string(&file) {
            Ok(src) => src,
            Err(_) => {
                result.files.remove(&file);
                result.warnings.push(Warning::Unreadable { file });
                continue;
            }
        };
        let dir = file.parent().unwrap_or(root).to_path_buf();
        for literal in path_literals(&src) {
            if literal.dynamic {
                result.warnings.push(Warning::Dynamic {
                    file: file.clone(),
                    literal: literal.text,
                });
                continue;
            }
            let target = match fs::canonicalize(dir.join(&literal.text)) {
                Ok(target) => target,
                Err(_) => {
                    result.warnings.push(Warning::Missing {
                        file: file.clone(),
                        literal: literal.text,
                    });
                    continue;
                }
            };
            if !target.starts_with(root) {
                result.warnings.push(Warning::OutsideRepo {
                    file: file.clone(),
                    literal: literal.text,
                });
                continue;
            }
            if target.is_dir() {
                let entry = target.join("default.nix");
                if entry.is_file() {
                    todo.push(entry);
                } else {
                    result.warnings.push(Warning::Missing {
                        file: file.clone(),
                        literal: format!("{}/default.nix", literal.text),
                    });
                }
            } else if target.extension().is_some_and(|ext| ext == "nix") {
                todo.push(target);
            } else {
                result.files.insert(target);
            }
        }
    }
    result
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib trim::deps`
Expected: 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
jj split src/trim/deps.rs -m "feat(trim): transitive local-dependency scan"
```

---

### Task 3: Generators (`generate.rs`)

**Files:**
- Modify: `src/trim/generate.rs` (currently empty; module is already declared privately in `src/trim/mod.rs`)

**Interfaces:**
- Consumes: nothing from other tasks (pure string functions).
- Produces (all `pub`, but the module stays private — used only by `trim::mod`):
  - `pub fn extract_inputs(src: &str) -> Option<String>`
  - `pub fn flake_nix(inputs: &str, hostname: &str, source: &str, module_paths: &[PathBuf]) -> String` (`module_paths` are repo-relative)
  - `pub fn host_default_nix(hostname: &str, state_version: Option<&str>) -> String`
  - `pub fn hardware_placeholder() -> String`

- [ ] **Step 1: Write failing tests**

Put this at the bottom of `src/trim/generate.rs`:

```rust
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
            &[PathBuf::from("modules/foo.nix"), PathBuf::from("modules/ssh")],
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib trim::generate 2>&1 | tail -5`
Expected: compile error — functions not found.

- [ ] **Step 3: Implement the generators**

File content above the tests:

```rust
use std::path::PathBuf;

/// Lifts every top-level `inputs …;` declaration from flake.nix source,
/// verbatim — both `inputs = { … };` blocks and `inputs.foo.url = "…";`
/// dotted forms. Returns `None` when nothing was found.
///
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
        let declaration = src[after..].starts_with('.')
            || src[after..].trim_start().starts_with('=');
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
    if found.is_empty() { None } else { Some(found.join("\n  ")) }
}

/// Generates the new config's flake.nix.
pub fn flake_nix(
    inputs: &str,
    hostname: &str,
    source: &str,
    module_paths: &[PathBuf],
) -> String {
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib trim::generate`
Expected: 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
jj split src/trim/generate.rs -m "feat(trim): flake.nix and host scaffold generators"
```

---

### Task 4: Export orchestrator (`mod.rs`) + integration test

**Files:**
- Modify: `src/trim/mod.rs` (currently only module declarations)
- Modify: `src/error.rs` (add `ExportError` variant)
- Test: `tests/export.rs` (new integration test)

**Interfaces:**
- Consumes: `deps::{scan, ScanResult, Warning}` (Task 2), `generate::*` (Task 3), `Error`/`Result` from `crate::error`.
- Produces (used by Task 5's TUI):
  - `pub struct ExportModule { pub name: String, pub path: Option<PathBuf> }`
  - `pub struct ExportRequest { pub source: String, pub hostname: String, pub output_dir: PathBuf, pub modules: Vec<ExportModule> }`
  - `pub struct ExportReport { pub output_dir: PathBuf, pub files_written: Vec<PathBuf>, pub warnings: Vec<Warning> }` (derives `Debug, Clone`)
  - `pub fn export(request: &ExportRequest) -> Result<ExportReport>`
  - `pub fn expand_tilde(input: &str) -> PathBuf`

- [ ] **Step 1: Add the error variant**

In `src/error.rs`, add to the `Error` enum:

```rust
    /// Error that may occur during config export.
    #[error("Export error: `{0}`")]
    ExportError(String),
```

- [ ] **Step 2: Write the failing integration test**

Create `tests/export.rs`:

```rust
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
    write(dir.path(), "common/default.nix", "{ ... }: { imports = [ ./base.nix ]; }");
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
    assert_eq!(trim::expand_tilde("/abs/cfg"), std::path::Path::new("/abs/cfg"));
}
```

Note: the fixture's `nix eval …#inputs.nixpkgs.lib.release` / `#nixosConfigurations` calls may succeed or fail depending on network/cache — both paths only affect warnings, so the test asserts neither.

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --test export 2>&1 | tail -5`
Expected: compile error — `trim::export`, `ExportRequest` not found.

- [ ] **Step 4: Implement `src/trim/mod.rs`**

Full file content (keeping the existing module declarations):

```rust
//! Trimming a source config into a fresh standalone host config.

pub mod deps;

mod generate;

use crate::error::{Error, Result};
use deps::Warning;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempdir::TempDir;

/// A module chosen for export.
#[derive(Debug, Clone)]
pub struct ExportModule {
    /// Module name (attribute in `nixosModules`).
    pub name: String,
    /// Absolute path of the module's entry file, if resolved.
    pub path: Option<PathBuf>,
}

/// Everything needed to generate a new host config.
#[derive(Debug, Clone)]
pub struct ExportRequest {
    /// Source flake reference (path or URL form).
    pub source: String,
    /// Name of the new host.
    pub hostname: String,
    /// Directory to create the new config in.
    pub output_dir: PathBuf,
    /// Modules to export.
    pub modules: Vec<ExportModule>,
}

/// Outcome of a successful export.
#[derive(Debug, Clone)]
pub struct ExportReport {
    /// Where the new config was written.
    pub output_dir: PathBuf,
    /// Repo-relative files that were written, sorted.
    pub files_written: Vec<PathBuf>,
    /// Non-fatal problems encountered.
    pub warnings: Vec<Warning>,
}

/// Expands a leading `~/` using `$HOME`.
pub fn expand_tilde(input: &str) -> PathBuf {
    if let Some(rest) = input.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(input)
}

/// Runs `nix <args>` and returns trimmed stdout on success.
fn nix_raw(args: &[&str]) -> Option<String> {
    let out = Command::new("nix").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Resolves the flake reference to a local source tree root.
fn flake_root(source: &str) -> Result<PathBuf> {
    let local = Path::new(source);
    if local.join("flake.nix").is_file() {
        return Ok(fs::canonicalize(local)?);
    }
    let out = Command::new("nix")
        .args(["flake", "metadata", "--json", "--no-write-lock-file", source])
        .output()?;
    if !out.status.success() {
        return Err(Error::ExportError(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&out.stdout)?;
    let path = metadata
        .get("path")
        .and_then(|path| path.as_str())
        .ok_or_else(|| Error::ExportError("flake metadata has no path".into()))?;
    Ok(PathBuf::from(path))
}

/// Trims the source config down to the selected modules and writes a fresh,
/// self-contained flake for `hostname` into `output_dir`.
///
/// Stages everything in a temp dir next to the target and renames it into
/// place, so a failed export never leaves a half-written config.
pub fn export(request: &ExportRequest) -> Result<ExportReport> {
    let output_dir = &request.output_dir;
    if output_dir.exists() && fs::read_dir(output_dir)?.next().is_some() {
        return Err(Error::ExportError(format!(
            "output directory {} is not empty",
            output_dir.display()
        )));
    }
    let root = flake_root(&request.source)?;
    let mut warnings = Vec::new();

    // Seed the scanner with the added modules' resolved entry files.
    let mut seeds = Vec::new();
    let mut wired = Vec::new();
    for module in &request.modules {
        match module.path.as_ref().and_then(|path| fs::canonicalize(path).ok()) {
            Some(path) if path.starts_with(&root) => {
                if let Ok(rel) = path.strip_prefix(&root) {
                    wired.push(rel.to_path_buf());
                }
                seeds.push(path);
            }
            _ => warnings.push(Warning::UnresolvedModule {
                name: module.name.clone(),
            }),
        }
    }
    if seeds.is_empty() {
        return Err(Error::ExportError("no exportable modules selected".into()));
    }

    let mut scanned = deps::scan(&root, &seeds);
    warnings.append(&mut scanned.warnings);

    // Stage into a sibling temp dir; rename into place at the very end.
    let parent = output_dir
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    fs::create_dir_all(&parent)?;
    let staging = TempDir::new_in(&parent, ".flamingo-export")?;
    let mut files_written = Vec::new();

    // Copy scanned files, mirroring the source layout.
    for file in &scanned.files {
        let Ok(rel) = file.strip_prefix(&root) else {
            continue;
        };
        let dest = staging.path().join(rel);
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::copy(file, &dest)?;
        files_written.push(rel.to_path_buf());
    }

    // flake.nix with the source's inputs lifted verbatim.
    let flake_src = fs::read_to_string(root.join("flake.nix")).unwrap_or_default();
    let inputs = match generate::extract_inputs(&flake_src) {
        Some(inputs) => inputs,
        None => {
            warnings.push(Warning::NoInputs);
            "inputs.nixpkgs.url = \"github:NixOS/nixpkgs/nixos-unstable\";".into()
        }
    };
    fs::write(
        staging.path().join("flake.nix"),
        generate::flake_nix(&inputs, &request.hostname, &request.source, &wired),
    )?;
    files_written.push(PathBuf::from("flake.nix"));

    // flake.lock copied whole to preserve every pin.
    if root.join("flake.lock").is_file() {
        fs::copy(root.join("flake.lock"), staging.path().join("flake.lock"))?;
        files_written.push(PathBuf::from("flake.lock"));
    }

    // Host scaffold.
    let state_version = nix_raw(&[
        "eval",
        &format!("{}#inputs.nixpkgs.lib.release", request.source),
        "--raw",
        "--no-write-lock-file",
    ]);
    if state_version.is_none() {
        warnings.push(Warning::NoStateVersion);
    }
    let host_rel = PathBuf::from("hosts").join(&request.hostname);
    let host_dir = staging.path().join(&host_rel);
    fs::create_dir_all(&host_dir)?;
    fs::write(
        host_dir.join("default.nix"),
        generate::host_default_nix(&request.hostname, state_version.as_deref()),
    )?;
    fs::write(
        host_dir.join("hardware-configuration.nix"),
        generate::hardware_placeholder(),
    )?;
    files_written.push(host_rel.join("default.nix"));
    files_written.push(host_rel.join("hardware-configuration.nix"));

    // Hostname collision check (best effort — new host should be new).
    if let Some(hosts) = nix_raw(&[
        "eval",
        &format!("{}#nixosConfigurations", request.source),
        "--apply",
        "builtins.attrNames",
        "--json",
        "--no-write-lock-file",
    ])
    .and_then(|out| serde_json::from_str::<Vec<String>>(&out).ok())
        && hosts.contains(&request.hostname)
    {
        warnings.push(Warning::HostnameExists {
            hostname: request.hostname.clone(),
        });
    }

    // Move into place.
    let staged = staging.into_path();
    if output_dir.exists() {
        fs::remove_dir(output_dir)?;
    }
    if let Err(error) = fs::rename(&staged, output_dir) {
        let _ = fs::remove_dir_all(&staged);
        return Err(error.into());
    }

    files_written.sort();
    Ok(ExportReport {
        output_dir: output_dir.clone(),
        files_written,
        warnings,
    })
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test export`
Expected: 3 tests PASS.

Run: `cargo test`
Expected: all tests PASS (lib + integration).

- [ ] **Step 6: Commit**

```bash
jj split src/trim/mod.rs src/error.rs tests/export.rs -m "feat(trim): export orchestrator with atomic staging"
```

---

### Task 5: TUI export flow

**Files:**
- Modify: `src/tui/command.rs` (add `Command::Export`, map `e`)
- Modify: `src/tui/state.rs` (add `ExportStage`, `status`, wire `run_command`)
- Modify: `src/tui/ui.rs` (prompt label, status line, result popup)

**Interfaces:**
- Consumes: `trim::{export, expand_tilde, ExportModule, ExportRequest, ExportReport}` (Task 4).
- Produces: `pub enum ExportStage { Idle, Hostname, OutputDir { hostname: String }, Done(std::result::Result<ExportReport, String>) }` in `state.rs`; `State.export_stage: ExportStage`; `State.status: Option<String>`.

- [ ] **Step 1: Add the command**

In `src/tui/command.rs`, add a variant to `Command`:

```rust
    /// Start the export flow.
    Export,
```

and in `impl From<KeyEvent> for Command`, add before the catch-all arm:

```rust
            KeyCode::Char('e') => Self::Export,
```

- [ ] **Step 2: Add export state**

In `src/tui/state.rs`, add imports:

```rust
use crate::trim::{self, ExportModule, ExportRequest, ExportReport};
```

Add above `pub struct State`:

```rust
/// Stage of the export flow.
#[derive(Debug, Default)]
pub enum ExportStage {
    /// No export in progress.
    #[default]
    Idle,
    /// Prompting for the new hostname.
    Hostname,
    /// Prompting for the output directory.
    OutputDir {
        /// Hostname confirmed in the previous stage.
        hostname: String,
    },
    /// Export finished; carries the report or the error message.
    Done(std::result::Result<ExportReport, String>),
}
```

Add two fields to `State` (with doc comments) and initialize them in `State::new`:

```rust
    /// Export flow stage.
    pub export_stage: ExportStage,
    /// Transient status message shown in the input slot.
    pub status: Option<String>,
```

```rust
            export_stage: ExportStage::default(),
            status: None,
```

- [ ] **Step 3: Wire `run_command`**

At the very top of `run_command` (before `match command`):

```rust
        if matches!(self.export_stage, ExportStage::Done(_)) {
            self.export_stage = ExportStage::Idle;
            return Ok(());
        }
        self.status = None;
```

Add a `Command::Export` arm:

```rust
            Command::Export => {
                if self.config.modules.iter().any(|module| module.added) {
                    self.export_stage = ExportStage::Hostname;
                    self.input = Input::default();
                    self.input_mode = true;
                } else {
                    self.status = Some("no modules added".to_string());
                }
            }
```

Replace the `InputCommand::Confirm` arm body:

```rust
                    InputCommand::Confirm => {
                        match std::mem::take(&mut self.export_stage) {
                            ExportStage::Hostname => {
                                let hostname = self.input.value().trim().to_string();
                                if hostname.is_empty() {
                                    self.export_stage = ExportStage::Hostname;
                                } else {
                                    self.export_stage = ExportStage::OutputDir { hostname };
                                    self.input = Input::default();
                                }
                            }
                            ExportStage::OutputDir { hostname } => {
                                let value = self.input.value().trim().to_string();
                                if value.is_empty() {
                                    self.export_stage = ExportStage::OutputDir { hostname };
                                } else {
                                    let request = ExportRequest {
                                        source: self.config.source.clone(),
                                        hostname,
                                        output_dir: trim::expand_tilde(&value),
                                        modules: self
                                            .config
                                            .modules
                                            .iter()
                                            .filter(|module| module.added)
                                            .map(|module| ExportModule {
                                                name: module.name.clone(),
                                                path: module.path.clone().map(PathBuf::from),
                                            })
                                            .collect(),
                                    };
                                    let result =
                                        trim::export(&request).map_err(|error| error.to_string());
                                    self.export_stage = ExportStage::Done(result);
                                    self.input = Input::default();
                                    self.input_mode = false;
                                }
                            }
                            stage => {
                                self.export_stage = stage;
                                self.input_mode = false;
                            }
                        }
                    }
```

(`std::mem::take` needs the `#[default]` on `Idle` from Step 2. `PathBuf` is already imported in this file; if not: `use std::path::PathBuf;`.)

Extend the `InputCommand::Exit` arm:

```rust
                    InputCommand::Exit => {
                        self.input = Input::default();
                        self.input_mode = false;
                        self.export_stage = ExportStage::Idle;
                    }
```

Add to `get_key_bindings()` for `Tab::General`, after the `("⏎ ", "Add module")` entry:

```rust
                    ("e", "Export"),
```

- [ ] **Step 4: Render prompt labels, status, and popup**

In `src/tui/ui.rs`:

Import: add `Clear` to the `ratatui::widgets` import list and `ExportStage` to the state import: `use crate::tui::state::{ExportStage, Panel, State};`

Update `get_input_line` to show the status message and stage-specific labels:

```rust
/// Returns the input line.
fn get_input_line<'a>(state: &'a State) -> Line<'a> {
    if let Some(status) = &state.status {
        return Line::from(vec![
            "|".fg(Color::Rgb(100, 100, 100)),
            status.clone().yellow(),
            "|".fg(Color::Rgb(100, 100, 100)),
        ]);
    }
    let label = match &state.export_stage {
        ExportStage::Hostname => "hostname: ",
        ExportStage::OutputDir { .. } => "output dir: ",
        _ => "search: ",
    };
    if !state.input.value().is_empty() || state.input_mode {
        Line::from(vec![
            "|".fg(Color::Rgb(100, 100, 100)),
            label.yellow(),
            state.input.value().fg(state.accent_color),
            if state.input_mode { " " } else { "" }.into(),
            "|".fg(Color::Rgb(100, 100, 100)),
        ])
    } else {
        Line::default()
    }
}
```

In `render`, replace the placeholder `files` paragraph in the header's right chunk (`let mut files = Vec::new(); … frame.render_widget(Paragraph::new(Line::from(files))…`) with:

```rust
        frame.render_widget(
            Paragraph::new(get_input_line(state)).alignment(Alignment::Right),
            chunks[1],
        );
```

Add at the end of `render` (after `render_key_bindings`):

```rust
    render_export_popup(state, frame);
```

Add the popup renderer:

```rust
/// Renders the export result popup, if the export flow just finished.
fn render_export_popup(state: &State, frame: &mut Frame) {
    let ExportStage::Done(result) = &state.export_stage else {
        return;
    };
    let mut lines = match result {
        Ok(report) => {
            let mut lines = vec![
                Line::from(vec![
                    "Exported to ".into(),
                    report.output_dir.display().to_string().green(),
                ]),
                Line::from(format!("{} files written", report.files_written.len())),
            ];
            if !report.warnings.is_empty() {
                lines.push(Line::from(
                    format!("{} warning(s):", report.warnings.len()).yellow(),
                ));
                lines.extend(
                    report
                        .warnings
                        .iter()
                        .take(8)
                        .map(|warning| Line::from(format!("- {warning}"))),
                );
                if report.warnings.len() > 8 {
                    lines.push(Line::from(format!(
                        "… and {} more",
                        report.warnings.len() - 8
                    )));
                }
            }
            lines
        }
        Err(error) => vec![
            Line::from("Export failed".red()),
            Line::from(error.as_str()),
        ],
    };
    lines.push(Line::from(
        "press any key to close".fg(Color::Rgb(100, 100, 100)),
    ));
    let area = frame.area();
    let width = (area.width / 2).clamp(40.min(area.width), area.width.saturating_sub(4));
    let height = (lines.len() as u16 + 2).min(area.height.saturating_sub(2));
    let popup = Rect::new(
        area.width.saturating_sub(width) / 2,
        area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(
                Block::bordered()
                    .title(vec![
                        "|".fg(Color::Rgb(100, 100, 100)),
                        "Export".fg(state.accent_color).bold(),
                        "|".fg(Color::Rgb(100, 100, 100)),
                    ])
                    .title_alignment(Alignment::Center),
            ),
        popup,
    );
}
```

- [ ] **Step 5: Verify it builds and tests pass**

Run: `cargo test 2>&1 | tail -5`
Expected: all tests PASS, no compile errors.

Run: `cargo clippy 2>&1 | tail -5`
Expected: no errors (pre-existing warnings OK, no new warnings from `src/trim/` or the touched TUI files).

- [ ] **Step 6: Commit**

```bash
jj split src/tui/command.rs src/tui/state.rs src/tui/ui.rs -m "feat(tui): export flow with hostname/output prompts and report popup"
```

---

### Task 6: Keybinding doc + end-to-end verification

**Files:**
- Modify: `config.toml` (document the export key alongside the other bindings)

**Interfaces:**
- Consumes: everything above.
- Produces: nothing new — verification only.

- [ ] **Step 1: Document the keybinding**

Add to the `[keybindings]` section of `config.toml`:

```toml
export = "e"
```

(Note: keybindings in `config.toml` are documentation today — `command.rs` hardcodes keys; no loader exists. Keep the file consistent anyway.)

- [ ] **Step 2: Full test suite**

Run: `cargo test`
Expected: all tests PASS.

- [ ] **Step 3: Manual end-to-end verification**

```bash
cargo run -- --config <path-to-a-real-nix-config>
```

In the TUI: toggle 2–3 modules with Enter → press `e` → type a hostname → Enter → type `/tmp/claude-export-check` → Enter. Expect the popup with the output path and any warnings.

Then verify the generated flake evaluates:

```bash
nix flake check --no-build /tmp/claude-export-check || nix eval /tmp/claude-export-check#nixosConfigurations --apply builtins.attrNames
```

Expected: module list evaluates; `flake check` may fail only on the placeholder `hardware-configuration.nix` / missing `fileSystems`, which is expected for a scaffold (the user replaces it with `nixos-generate-config` output on the real machine).

- [ ] **Step 4: Commit**

```bash
jj split config.toml -m "docs: document export keybinding"
```
