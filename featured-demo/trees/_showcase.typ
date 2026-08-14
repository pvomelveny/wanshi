// Shared Typst code for this forest.
//
// The leading underscore is what keeps this file from becoming a page: wanshi
// skips sources whose name starts with `_`, so helpers can live beside notes
// without turning into notes themselves. See
// docs/users/content-authoring.md — "Helpers: the `_` Prefix".

#import "/_lib/wanshi.typ": *

/// The preamble every note in this forest repeats.
///
/// Nothing magical: a plain Typst function, imported the way any Typst project
/// would import one. It exists to show that sharing code between notes needs no
/// wanshi-specific machinery.
#let showcase(doc) = {
  show: wanshi
  doc
}

// Code samples in this forest use plain backtick fences rather than a helper.
// Backticks take their contents literally, so no quote or backslash needs
// escaping; `raw()` is the function form, worth reaching for only when the text
// or the language is computed rather than written out. Both produce the same
// `<pre><code>` in Typst's HTML export.
