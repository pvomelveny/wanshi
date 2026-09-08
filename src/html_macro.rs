// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Spore (@s-cerevisiae)

/// Escape a value on its way into an attribute position.
///
/// Every attribute [`html!`] writes goes through this, so a caller can hand
/// over plain text — a note's title, say — without a stray `"` terminating the
/// attribute and leaking the rest into the tag. Element *bodies* are untouched:
/// they legitimately carry markup, and the macro's `(expr)` form is how
/// pre-built HTML is spliced in.
pub(crate) fn escape_attr(value: impl std::fmt::Display) -> String {
    htmlize::escape_attribute(value.to_string()).into_owned()
}

/// Example (illustrative; doctests do not run in a binary crate):
/// ```rust,ignore
/// let mut s = String::from("a");
/// html_write!(s; br);
/// assert_eq!(s, "a<br />");
/// ```
macro_rules! html_write {
    // Match a single HTML element with attributes and children
    ($str:expr; $tag:ident $($attr:ident = $val:tt)* { $($inner:tt)* } $($rest:tt)*) => {
        write!($str, "<{}", stringify!($tag)).unwrap();

        // Add attributes
        $(
            write!(
                $str,
                " {}=\"{}\"",
                stringify!($attr).replace("_", "-"),
                $crate::html_macro::escape_attr(&$val)
            )
            .unwrap();
        )*

        $str.push('>');

        // Add children (recursively process inner HTML)
        $crate::html_macro::html_write!($str; $($inner)*);

        write!($str, "</{}>", stringify!($tag)).unwrap();

        $crate::html_macro::html_write!($str; $($rest)*);
    };

    // Match a single HTML element without attributes (self-closing tag)
    ($str:expr; $tag:ident $($rest:tt)*) => {{
        write!($str, "<{} />", stringify!($tag)).unwrap();
        $crate::html_macro::html_write!($str; $($rest)*);
    }};

    // Match plain text content
    ($str:expr; $lit:literal $($rest:tt)*) => {{
        write!($str, "{}", $lit).unwrap();
        $crate::html_macro::html_write!($str; $($rest)*);
    }};

    // Match arbitrary expression
    ($str:expr; ($text:expr) $($rest:tt)*) => {{
        write!($str, "{}", $text).unwrap();
        $crate::html_macro::html_write!($str; $($rest)*);
    }};

    // Nothing more, ends here
    ($str:expr;) => {};
}

/// Example (illustrative; doctests do not run in a binary crate):
/// ```rust,ignore
/// let value = 1;
/// let id = "some_id".to_string();
/// let html = html!(
///     p class="c" id={id} { (value) }
///     br
/// );
/// assert_eq!(html, r#"<p class="c" id="some_id">1</p><br />"#);
/// ```
macro_rules! html {
    ($($args:tt)*) => {{
        use ::std::fmt::Write as _;
        let mut html = String::new();
        $crate::html_macro::html_write!(html; $($args)*);
        html
    }};
}

pub(crate) use {html, html_write};

#[cfg(test)]
mod tests {

    /// The values that reach attributes are plain text — titles above all — so
    /// the macro escapes them; a quote in a title must not terminate the
    /// attribute and leak the rest into the tag.
    #[test]
    fn test_attribute_values_are_escaped() {
        let title = r#"A "quoted" & <odd> title"#;
        let html = html!(a title={title} { "x" });
        assert_eq!(
            html,
            r#"<a title="A &quot;quoted&quot; &amp; &lt;odd&gt; title">x</a>"#
        );
    }

    /// Bodies are where pre-built HTML is spliced in, so they pass through
    /// untouched — escaping belongs to whoever built the fragment.
    #[test]
    fn test_bodies_pass_through_unescaped() {
        let inner = "<em>x</em>";
        let html = html!(span class="title" { (inner) });
        assert_eq!(html, r#"<span class="title"><em>x</em></span>"#);
    }

    #[test]
    fn test_underscored_attribute_names_become_kebab_case() {
        let html = html!(div data_taxon="definition" { "x" });
        assert_eq!(html, r#"<div data-taxon="definition">x</div>"#);
    }
}
