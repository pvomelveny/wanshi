// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::fs;

const DEFAULT_IMPORT_FONT_HTML: &str = include_str!("../include/import-font.html");
const DEFAULT_IMPORT_MATH_HTML: &str = include_str!("../include/import-math.html");

/// Every optional override file read from the project root.
///
/// Kept in one place because `wanshi serve` has to watch exactly this set: a
/// name that appears here but not in the watch list produces a hook that
/// silently stops live-reloading, which is hard to notice and harder to
/// diagnose.
pub const IMPORT_FILE_NAMES: [&str; 6] = [
    "import-meta.html",
    "import-style.html",
    "import-font.html",
    "import-math.html",
    "import-header.html",
    "import-footer.html",
];

pub fn import_meta_html() -> String {
    fs::read_to_string(super::root_dir().join("import-meta.html")).unwrap_or_default()
}

pub fn import_style_html() -> String {
    fs::read_to_string(super::root_dir().join("import-style.html")).unwrap_or_default()
}

pub fn import_fonts_html() -> String {
    fs::read_to_string(super::root_dir().join("import-font.html"))
        .unwrap_or_else(|_| DEFAULT_IMPORT_FONT_HTML.to_string())
}

/// KaTeX loader, absent unless the site asks for it.
///
/// Equations are MathML, which the browser renders without help, so loading
/// KaTeX by default would put two CDN requests and a render-blocking stylesheet
/// on every page for the benefit of the `tex()` helper alone. Sites that use
/// `tex()` opt in with `wanshi snip --katex`, which writes the loader.
pub fn import_math_html() -> String {
    fs::read_to_string(super::root_dir().join("import-math.html")).unwrap_or_default()
}

/// The bundled KaTeX loader, written into a project by `wanshi snip --katex`.
pub fn default_import_math_html() -> &'static str {
    DEFAULT_IMPORT_MATH_HTML
}

/// Markup placed at the very top of `<body>`, above wanshi's own breadcrumb.
///
/// The one extension point that is not in `<head>`: it exists so a site wanshi
/// is embedded in can put its own navigation on the page.
pub fn import_header_html() -> String {
    fs::read_to_string(super::root_dir().join("import-header.html")).unwrap_or_default()
}

/// Markup placed at the very end of `<body>`, outside the content grid.
///
/// Distinct from a section's own footer, which holds that note's backlinks and
/// references and lives inside the article.
pub fn import_footer_html() -> String {
    fs::read_to_string(super::root_dir().join("import-footer.html")).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_file_names_cover_every_hook() {
        // The watch list is derived from this constant, so a hook missing here
        // would build correctly and then never live-reload.
        for name in [
            "import-meta.html",
            "import-style.html",
            "import-font.html",
            "import-math.html",
            "import-header.html",
            "import-footer.html",
        ] {
            assert!(
                IMPORT_FILE_NAMES.contains(&name),
                "{name} is read at build time but not listed for watching"
            );
        }
    }
}
