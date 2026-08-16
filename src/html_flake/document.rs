// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{cli::serve, environment, html_macro::html};

const MAIN_SCRIPT: &str = include_str!("../include/main.js");
const MAIN_STYLE: &str = include_str!("../include/main.css");

pub fn html_doc(
    page_title: &str,
    header_html: &str,
    article_inner: &str,
    footer_html: &str,
    catalog_html: &str,
) -> String {
    let mut toc_class: Vec<&str> = vec![];
    if environment::is_toc_sticky() {
        toc_class.push("sticky-nav");
    }
    if environment::is_toc_mobile_sticky() {
        toc_class.push("mobile-sticky-nav");
    }

    let doc_type = "<!DOCTYPE html>";

    let nav_html = html_nav(toc_class, catalog_html);
    let html = html!(html lang="en-US" {
        head {
            r#"
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<meta name="viewport" content="width=device-width">"#
            (format!("<title>{page_title}</title>"))
            (html_favicon_link())
            (html_feed_link())
            (html_import_meta())
            (html_scripts())
            (html_live_reload())
            // math should be loaded after scripts to handle dynamic content
            (html_import_math())
            // main styles should be loaded after math to override formula font size
            (html_static_css())
            (html_dynamic_css())
            // fonts should be loaded after `static_css` to handle override default fonts
            (html_import_fonts())
            // custom styles should be loaded last to override other styles
            (html_import_style())
        }
        body {
            // Above the breadcrumb, so a host site's navigation reads as the
            // outermost chrome rather than something nested inside wanshi's.
            (html_import_header())
            (header_html)
            (html_body_inner(&nav_html, article_inner, footer_html))
            (html_import_footer())
        }
    });
    format!("{}\n{}", doc_type, html)
}

fn html_body_inner(nav: &str, article_inner: &str, footer: &str) -> String {
    let base_url = environment::base_url_raw();
    let style = grid_wrapper_style();

    html!(div id="grid-wrapper" style={style} data_base_url={base_url} {
        (nav) "\n\n" article { (article_inner) (footer) }
    })
}

pub fn grid_wrapper_style() -> &'static str {
    if environment::is_toc_left() {
        "grid-template-areas: 'toc article';"
    } else {
        "grid-template-areas: 'article toc';"
    }
}

pub fn html_static_css() -> String {
    if environment::inline_css() {
        html!(style { (html_main_style()) })
    } else {
        let base_url = environment::base_url();
        format!(r#"<link rel="stylesheet" href="{}main.css">"#, base_url)
    }
}

pub fn html_dynamic_css() -> String {
    let toc_max_width = environment::toc_max_width();
    // The sidebar track is `minmax(0, …)` so it can give ground when the window
    // is not wide enough for both columns at full size. As a fixed track it
    // kept its full width and pushed itself past the edge of the window, where
    // entries were clipped rather than wrapped. The article track stays fixed,
    // which is what gives it priority over the sidebar as space runs out.
    let grid_columns_value = if environment::is_toc_left() {
        "minmax(0, max-content) var(--article-max-width)"
    } else {
        "var(--article-max-width) minmax(0, var(--toc-max-width))"
    };
    let theme_lock = if environment::theme_lock() {
        "\n#theme-options { display: none; }"
    } else {
        ""
    };

    let grid_wrapper = format!(
        r#"@media only screen and (min-width: 1000px) {{
  #grid-wrapper {{ grid-template-columns: {grid_columns_value}; }}
  nav#toc {{ max-width: {toc_max_width}; }}
}}{theme_lock}"#
    );

    format!("<style>\n{grid_wrapper}\n</style>")
}

pub fn html_import_meta() -> String {
    environment::import_meta_html()
}

pub fn html_import_style() -> String {
    environment::import_style_html()
}

pub fn html_import_fonts() -> String {
    environment::import_fonts_html()
}

pub fn html_import_math() -> String {
    environment::import_math_html()
}

pub fn html_import_header() -> String {
    environment::import_header_html()
}

pub fn html_import_footer() -> String {
    environment::import_footer_html()
}

pub fn html_live_reload() -> String {
    if environment::is_serve() && *serve::live_reload() {
        include_str!("../include/reload.html").to_string()
    } else {
        String::new()
    }
}

pub fn html_scripts() -> String {
    let template = html_theme_option_template();

    if environment::inline_script() {
        return format!("{template}<script>\n{MAIN_SCRIPT}\n</script>");
    }

    let base_url = environment::base_url();
    format!(r#"{template}<script src="{base_url}main.js"></script>"#)
}

fn html_theme_option_template() -> String {
    html!(template id="theme-option-template" {
        r#"<input type="radio" name="theme" /><label></label>"#
    })
}

pub fn html_main_script() -> &'static str {
    MAIN_SCRIPT
}

fn html_import_theme() -> String {
    environment::theme_paths()
        .iter()
        .map(|theme_path| match std::fs::read_to_string(theme_path) {
            Ok(content) => content,
            Err(err) => {
                color_print::ceprintln!(
                    "<y>Warning: Failed to read theme file at '{}': {}</>",
                    theme_path,
                    err
                );

                String::new()
            }
        })
        .collect()
}

fn html_themes() -> String {
    html!(div id="theme-options" { (html_import_theme()) })
}

pub fn html_nav(toc_class: Vec<&str>, catalog_html: &str) -> String {
    html!(nav id="toc" class={toc_class.join(" ")} {
        (html_themes()) (html_search()) (catalog_html)
    })
}

/// Favicon link, resolved against the configured assets directory.
///
/// Not hardcoded to `assets/`: the build publishes assets under whatever
/// `[wanshi].assets` is named, so a hardcoded segment sends every page to a
/// path wanshi does not write. Where a site's output shares a directory with
/// another site, that path may well exist and belong to something else.
fn html_favicon_link() -> String {
    let Some(assets) = environment::assets_dir_name() else {
        return String::new();
    };

    format!(
        r#"<link rel="icon" href="{}{}/favicon.ico" />"#,
        environment::base_url(),
        assets,
    )
}

/// Feed autodiscovery link.
///
/// Without this the feed is published but unreachable: browsers and readers
/// find a feed by looking for `rel="alternate"` in the page head, and nothing
/// else on the site points at `feed.xml`.
///
/// Only emitted in publish builds, matching where the feed is actually written.
/// The `title` attribute is omitted deliberately: it is optional, only useful
/// when a site publishes several feeds, and any value chosen here could
/// contradict the channel title, which is derived from the index page.
fn html_feed_link() -> String {
    if !environment::publish_rss() || !environment::is_publish() {
        return String::new();
    }

    let href = environment::full_url(environment::FEED_NAME);
    format!(
        r#"<link rel="alternate" type="application/rss+xml" href="{}" />"#,
        htmlize::escape_attribute(&href),
    )
}

/// Search affordance in the sidebar.
///
/// Rendered hidden and revealed by script, so a reader without JavaScript never
/// sees an input that cannot work. The index URL is resolved against `base-url`
/// here rather than in script, because only the build knows the deploy prefix.
fn html_search() -> String {
    if !environment::search_enabled() {
        return String::new();
    }

    let index_url = environment::full_url(environment::SEARCH_INDEX_NAME);
    let placeholder = environment::get_search_text();

    html!(div id="search" hidden="" data_index={index_url} {
        input
            id="search-input"
            type="search"
            autocomplete="off"
            spellcheck="false"
            placeholder={placeholder} {}
        div id="search-results" {}
    })
}

pub fn html_main_style() -> &'static str {
    MAIN_STYLE
}

#[cfg(test)]
mod tests {
    use super::{html_doc, html_dynamic_css, html_live_reload};
    use crate::environment;

    #[test]
    fn test_html_doc_places_body_hooks_outside_the_content_grid() {
        use std::fs;

        let root = crate::test_io::case_dir("document-body-hooks");
        fs::create_dir_all(root.as_std_path()).unwrap();
        let config_path = root.join("Wanshi.toml");
        fs::write(config_path.as_std_path(), "[wanshi]\n").unwrap();
        fs::write(
            root.join("import-header.html").as_std_path(),
            "<nav id=\"host-nav\">host</nav>",
        )
        .unwrap();
        fs::write(
            root.join("import-footer.html").as_std_path(),
            "<footer id=\"host-footer\">host</footer>",
        )
        .unwrap();

        environment::with_test_environment(root.clone(), environment::BuildMode::Publish, || {
            environment::init_environment(config_path.clone(), environment::BuildMode::Publish)
                .unwrap();

            let html = html_doc("Title", "<header>crumb</header>", "body", "", "");

            let header = html.find("host-nav").expect("header hook should render");
            let crumb = html.find("crumb").expect("breadcrumb should render");
            // Not bare `grid-wrapper`: the dynamic stylesheet names it in the
            // head, long before the element itself appears.
            let grid = html
                .find(r#"id="grid-wrapper""#)
                .expect("grid should render");
            let footer = html.find("host-footer").expect("footer hook should render");

            // The host's chrome wraps wanshi's, rather than nesting inside it.
            assert!(header < crumb, "header hook should precede the breadcrumb");
            assert!(crumb < grid, "breadcrumb should precede the grid");
            assert!(grid < footer, "footer hook should follow the grid");
        });

        let _ = fs::remove_dir_all(root.as_std_path());
    }

    #[test]
    fn test_html_favicon_link_follows_the_configured_assets_directory() {
        use std::fs;

        let root = crate::test_io::case_dir("document-favicon-assets");
        fs::create_dir_all(root.as_std_path()).unwrap();
        let config_path = root.join("Wanshi.toml");
        fs::write(
            config_path.as_std_path(),
            r#"
[wanshi]
assets = "note-assets"
base-url = "/notes/"
"#,
        )
        .unwrap();

        environment::with_test_environment(root.clone(), environment::BuildMode::Publish, || {
            environment::init_environment(config_path.clone(), environment::BuildMode::Publish)
                .unwrap();

            let html = html_doc("Title", "", "body", "", "");

            // The build copies assets to `<output>/note-assets`, so a link to
            // `/notes/assets/` would point at a path wanshi never writes.
            assert!(
                html.contains(r#"<link rel="icon" href="/notes/note-assets/favicon.ico" />"#),
                "favicon should follow [wanshi].assets"
            );
        });

        let _ = fs::remove_dir_all(root.as_std_path());
    }

    #[test]
    fn test_html_doc_omits_body_hooks_when_files_are_absent() {
        use std::fs;

        // `with_test_environment` rather than `mock_environment`: only the
        // former holds the environment lock for the whole closure, and without
        // it a sibling test's project root leaks in and supplies the hooks.
        let root = crate::test_io::case_dir("document-body-hooks-absent");
        fs::create_dir_all(root.as_std_path()).unwrap();

        environment::with_test_environment(root.clone(), environment::BuildMode::Publish, || {
            let html = html_doc("Title", "<header>crumb</header>", "body", "", "");

            assert!(html.contains(r#"id="grid-wrapper""#));
            assert!(!html.contains("host-nav"));
            assert!(!html.contains("host-footer"));
        });

        let _ = fs::remove_dir_all(root.as_std_path());
    }

    #[test]
    fn test_html_live_reload_disabled_outside_serve_mode() {
        environment::mock_environment().unwrap();
        assert!(html_live_reload().is_empty());
    }

    #[test]
    fn test_html_dynamic_css_hides_theme_options_when_theme_lock_enabled() {
        use std::fs;

        let root = crate::test_io::case_dir("document-theme-lock-enabled");
        fs::create_dir_all(root.as_std_path()).unwrap();
        let config_path = root.join("Wanshi.toml");
        fs::write(
            config_path.as_std_path(),
            r#"
[wanshi]
theme-lock = true
"#,
        )
        .unwrap();

        environment::with_test_environment(root.clone(), environment::BuildMode::Publish, || {
            environment::init_environment(config_path.clone(), environment::BuildMode::Publish)
                .unwrap();

            let css = html_dynamic_css();
            assert!(css.contains("#theme-options { display: none; }"));
        });

        let _ = fs::remove_dir_all(root.as_std_path());
    }

    #[test]
    fn test_html_dynamic_css_hides_theme_options_by_default() {
        environment::mock_environment().unwrap();

        let css = html_dynamic_css();
        assert!(css.contains("#theme-options { display: none; }"));
    }
}
