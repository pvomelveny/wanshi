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

/// Label a worked example so the prose around it reads consistently.
///
/// `raw` maps onto `<pre><code>` in Typst's HTML export, so code samples come
/// out as real code blocks rather than pictures of code.
#let source(code) = raw(code.text, lang: "typst", block: true)
