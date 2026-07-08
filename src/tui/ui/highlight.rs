use std::cell::LazyCell;

use ratatui::text::Text;
use syntect_assets::assets::HighlightingAssets;
use tui_syntax_highlight::Highlighter;

thread_local! {
    static ASSETS: LazyCell<HighlightingAssets> = LazyCell::new(HighlightingAssets::from_binary);
}

/// Highlights Nix source into styled lines, falling back to plain
/// text when the syntax or theme lookup fails.
pub fn highlight_nix(content: &str) -> Text<'static> {
    ASSETS.with(|assets| {
        let Ok(syntax_set) = assets.get_syntax_set() else {
            return Text::raw(content.to_string());
        };
        let Some(syntax) = syntax_set.find_syntax_by_extension("nix") else {
            return Text::raw(content.to_string());
        };
        Highlighter::new(assets.get_theme("ansi").clone())
            .highlight_reader(content.as_bytes(), syntax, syntax_set)
            .unwrap_or_else(|_| Text::raw(content.to_string()))
    })
}
