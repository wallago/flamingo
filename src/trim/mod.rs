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
        .args([
            "flake",
            "metadata",
            "--json",
            "--no-write-lock-file",
            source,
        ])
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

    let mut seeds = Vec::new();
    let mut wired = Vec::new();
    for module in &request.modules {
        match module
            .path
            .as_ref()
            .and_then(|path| fs::canonicalize(path).ok())
        {
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
