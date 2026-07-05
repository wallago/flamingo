# Trim & Export — Design

**Date:** 2026-07-05
**Status:** Approved

## Purpose

Flamingo's core job: point it at a complete NixOS flake config (local path or URL),
pick the modules a new machine needs, and generate a fresh, self-contained config
repo outside the source — ready to build a host that does not exist in the source
config.

## Decisions (settled during brainstorming)

| Question | Decision |
|---|---|
| Output shape | Self-contained repo: files copied out, no dependency on the source repo |
| Selection | Manual pick in the TUI (already implemented via `Module.added` + Enter toggle) |
| Dependencies | Resolved and copied transitively |
| Dependency engine | Hybrid: existing nix-eval collector seeds a static Rust path-literal scanner |
| Export trigger | TUI: `e` key, then hostname + output dir prompts |
| Flake inputs | All source inputs copied verbatim; source `flake.lock` copied whole |

## Architecture

New `src/trim/` module, fully decoupled from the TUI. Pure "selection in →
files out + report"; no ratatui types. Reusable later for a non-interactive
CLI mode.

```
trim/
├── mod.rs       Exporter — orchestrates the pipeline, returns ExportReport
├── deps.rs      transitive local-dependency scanner (static)
└── generate.rs  flake.nix + host scaffold + flake.lock generation
```

### Pipeline

1. **Locate the source tree root.** `nix flake metadata --json` yields the
   flake's local path (repo dir for local sources, fetched
   `/nix/store/...-source` tree for URLs). Existing `Module.path` values point
   inside this tree; the root is used to compute repo-relative paths.
2. **Collect transitive deps** (`deps.rs`). Seeds are the added modules'
   resolved files (from the existing `extract_modules` eval collector). The
   scanner reads each file's text, extracts relative path literals
   (`./x.nix`, `../common`, `./dir`), resolves them against the file's
   directory, and recurses to fixpoint:
   - `.nix` file → copy + recurse into it
   - directory → copy + recurse into its `default.nix`
   - other file (asset: yaml, sh, …) → copy, no recursion
   - contains `${...}` (dynamic) → warning, skipped
   - escapes the repo root → warning, skipped
   - does not exist → warning, skipped
3. **Copy** every collected file into the output dir, **mirroring the source
   repo's relative layout**. This keeps all relative `imports` inside copied
   modules valid with zero Nix rewriting.
4. **Generate** `flake.nix`, `hosts/<host>/` scaffold, and copy `flake.lock`.
5. **Return `ExportReport`** — files written + warnings — rendered by the TUI.

### Why hybrid dependency resolution

The eval collector (`extract_modules`) is ground truth for mapping module
names to their entry file(s), but evaluation cannot see:

- imports inside function modules (`{ pkgs, ... }: { imports = [ ./y.nix ]; }`)
  without calling them with fabricated args (fragile),
- non-import file references: `sops.defaultSopsFile = ../secrets/host.yaml`,
  `home.file."x".source = ./dotfiles/x`, script paths, etc.

The static scanner catches all of those. Its own blind spot — computed paths —
degrades to a warning instead of a broken export.

## Generated output

Given hostname `myhost` and output dir `~/new-config`:

```
new-config/
├── flake.nix                # generated
├── flake.lock               # copied verbatim from source repo
├── hosts/
│   └── myhost/
│       ├── default.nix                 # generated scaffold
│       └── hardware-configuration.nix  # commented placeholder
└── <mirrored source paths>  # e.g. modules/networking.nix, secrets/…
```

### flake.nix

The `inputs = { ... };` block is lifted **textually** from the source
`flake.nix` via balanced-brace extraction (no Nix parsing — copied verbatim).
Outputs wire the added modules by their mirrored paths:

```nix
{
  description = "myhost — trimmed from <source> by flamingo";
  inputs = { /* copied from source flake.nix */ };
  outputs = { self, nixpkgs, ... }@inputs: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      specialArgs = { inherit inputs; };
      modules = [
        ./hosts/myhost
        ./modules/networking.nix   # one entry per added module
      ];
    };
  };
}
```

Baked-in defaults (trivially editable in the output): `system =
"x86_64-linux"`; `specialArgs = { inherit inputs; }`.

### Host scaffold

`hosts/myhost/default.nix`: sets `networking.hostName = "myhost"`, imports
`./hardware-configuration.nix`, and pins `system.stateVersion`. The release
value comes from one `nix eval <source>#inputs.nixpkgs.lib.release`; if that
eval fails, the line is emitted commented out and a warning is added to the
report. `hardware-configuration.nix` is a commented placeholder
instructing the user to replace it with `nixos-generate-config` output from
the target machine.

### flake.lock

Copied whole. All inputs are carried over, so the source lock is valid as-is
and every pin is preserved. Unused inputs are harmless and easy to delete by
hand.

## TUI export flow

Builds on existing checkboxes, panels, and the `Input` widget.

- **`e` triggers export** (new `Command::Export`) when ≥1 module is added;
  otherwise the status line shows "no modules added".
- **Two-step prompt** via a new state enum, reusing `Input`/`input_mode`:

```rust
pub enum ExportStage {
    Idle,
    Hostname,           // "hostname: " prompt
    OutputDir,          // "output dir: " prompt (~ expanded)
    Done(ExportReport), // result popup
}
```

- Enter confirms each stage; Esc cancels back to `Idle`. Prompts render in the
  same bottom slot as the existing `search:` line with a different label.
- Export runs synchronously on output-dir confirm (file I/O + one
  `nix flake metadata` call); a "trimming…" status shows meanwhile.
- **Result popup** (centered): success shows output path, file count, and
  warnings; failure shows the error. Any key dismisses.
- New keybinding in `config.toml` (`export = "e"`) and in `get_key_bindings()`.

## Error handling

**Hard errors** (abort, nothing half-written): output dir exists and is
non-empty; `nix flake metadata` failure; I/O errors. The exporter writes into
a temp dir first and renames into place, so a failed export never leaves a
broken half-repo.

**Warnings** (export succeeds, user informed): dynamic path literals;
references escaping the repo root; modules with `Module.path == None`
(listed in the report as skipped); hostname colliding with an existing
`nixosConfigurations` entry in the source (detected via
`nix eval <source>#nixosConfigurations --apply builtins.attrNames`; the
check is skipped if that eval fails).

## Testing

- Unit tests for `path_literals` / `resolve` in `deps.rs` against canned Nix
  snippets: imports, dir refs, assets, dynamic interpolation, `../` escapes.
- Unit test for textual `inputs` block extraction (nested braces, comments).
- Snapshot-style assertions on generated `flake.nix` and host scaffold.
- Integration test: build a small fixture flake in a tempdir, run the full
  `Exporter`, assert the exact resulting file set; if `nix` is on PATH, also
  run `nix flake check --no-build` on the output (skipped otherwise so CI
  without nix still passes).

## Out of scope (YAGNI)

- Overlay-flake output mode referencing the source repo
- Cloning an existing host as a selection template
- Detecting which flake inputs are actually used (all are copied)
- homeModules / home-manager standalone configs (nixosModules only, for now)
- Non-interactive CLI export mode (the `Exporter` API deliberately allows it later)
