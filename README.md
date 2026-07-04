xtract_modules keeps its current call for names, adds the batched path eval, then reads each resolved file's content. Unresolvable modules keep None and the UI shows "source not available".

State (tui/state.rs): The existing SelectableList keeps driving the left list; the already-present-but-unused selected_option_scroll_index becomes the right panel's scroll offset, reset to 0 whenever the highlighted module changes (j/k, top/bottom, mouse scroll).

UI (tui/ui.rs): In the General tab, below the "Nixos Config" info box, split the remaining area horizontally 50/50 — left: the existing module table, retitled "Remaining Modules"; right: a bordered Paragraph titled with the highlighted module's name, showing its source with a vertical scrollbar. h/l (currently ScrollType::Table) scroll the content panel up/down; scroll is clamped to content height.

Testing: unit test for the module-path resolution parsing (feed it canned nix eval JSON), and a snapshot-style check that the layout splits without panicking on small terminals. Manual verification against your real flake.
