// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli), Kokic (@kokic)
// Last modified time: 2026/03/06

/**
 * There are some external inputs:
 *   sys.inputs.path: relative path of the typst file
 *   sys.inputs.random: a random number in 0..INT64_MAX (note, it's a string)
 */

#let repri(r) = if type(r) == str { r } else {
  repr(r)
}

#let with-target-check(callback) = context {
  let target-value = if "target" in dictionary(std) { std.target() } else { "paged" }
  callback(target-value)
}

#let compatibled-html(f, content-provider) = with-target-check((export-target) => {
  let content = content-provider()
  if export-target == "html" { f()(content) } else {
    content
  }
})

#let auto-frame(content) = compatibled-html(() => html.frame, () => content)
#let auto-figure(content) = with-target-check((export-target) => {
  if export-target == "html" {
    html.figure(content) // main.css: `figure { text-align: center; }`
  } else {
    align(center, content)
  }
})

// 1em (:= 18px)
#let html-font-size = 13.5pt

// paged (print-facing output stays on a white page background per the
// "Parchment & walnut" design system; the ink/muted/accent colors carry over)
#let ink-color = rgb("#2E1F14")
#let muted-color = rgb("#6B5744")
#let accent-color = rgb("#822727")
#let paged-metadata-text-color = muted-color
#let small-block-below = 0.65em
#let heading-font-weight = "black"
#let slug-color = muted-color
#let taxon-color = accent-color

#let is_preset_key(key) = {
  (
    "title",
    "taxon",
    "parent",
    "page-title",
    "backlinks",
    "transparent-backlinks",
    "references",
    "asref",
    "asback",
    "footer-mode",
  ).contains(key)
}

#let dotted-stroke = (thickness: 0.1em, dash: ("dot", "dot")/* = thickness */)

#let span-slug(slug) = underline(stroke: dotted-stroke, text(size: 1.083em, fill: slug-color, raw("[" + slug + "]")))

#let taxon-upper(taxon) = upper(taxon.at(0)) + taxon.slice(1) + "."

#let metadata(table) = {
  let title = table.at("title", default: "")
  let taxon = table.at("taxon", default: none)

  let table-pairs = table.pairs()
  let custom-pairs = table-pairs.filter(e => not is_preset_key(e.at(0)))

  with-target-check(
    (export-target) => {
      if export-target == "html" {
        table-pairs.map(e => {
          let value = e.at(1)
          let v = value
          let attrs = (key: e.at(0))

          if type(value) != content {
            v = none
            attrs.insert("value", repri(value))
          }
          html.elem("wanshi-meta", v, attrs: attrs)
        }).join()
      } else {
        if taxon != none {
          text(weight: heading-font-weight, fill: taxon-color, size: 1.35em, taxon-upper(taxon))
        }
        block(above: small-block-below, below: small-block-below, text(fill: ink-color, size: 1.5em, weight: heading-font-weight, title))
        block(text(fill: muted-color, custom-pairs.map(e =>
        e.at(1)).join(text(" · "))))
      }
    },
  )
}

#let external(dest, content) = link(dest, underline(content))

///
/// - raw-tex (string): raw TeX math source code without delimiters
/// -> string
#let tex(raw-tex) = "$" + raw-tex.text + "$"

#let local(slug, text: none) = with-target-check((export-target) => {
  if export-target == "html" {
    html.elem(
      "span", // Make it an inline element. This is automatically removed by wanshi.
      {
        let v = if text == none { none } else { text }
        let attrs = (slug: slug)

        if text != none and type(text) != content {
          v = none
          attrs.insert("value", repri(text))
        }

        html.elem("wanshi-local", v, attrs: attrs)
      },
    )
  } else {
    let label = if text == none { slug } else { text }
    underline(stroke: dotted-stroke, label)
  }
})

/// Listing of other sections, resolved by wanshi after the whole graph is known.
///
/// - from: which sections to consider.
///     "children"    — sections whose parent is this one
///     "descendants" — the whole subtree beneath this one
///     "siblings"    — sections sharing this one's parent
///     "all"         — every visible section
///     "<prefix>/"   — every section whose slug starts with the prefix
/// - taxon: keep only sections with this taxon.
/// - key / value: keep only sections carrying this metadata key, and (with
///     `value`) only when it equals that value.
/// - sort: metadata key to order by; also "slug", "title", "taxon".
/// - order: "asc" or "desc".
/// - limit: keep at most this many.
/// - title: optional heading rendered above the list.
/// - include-indexes: whether directory index pages may appear in the results.
///     Set to `false` to list only ordinary notes, leaving out the hubs.
#let query(
  from: "all",
  taxon: none,
  key: none,
  value: none,
  sort: "date",
  order: "asc",
  limit: none,
  title: none,
  include-indexes: true,
) = with-target-check((export-target) => {
  if export-target == "html" {
    let attrs = (from: repri(from), sort: repri(sort), order: repri(order))
    if taxon != none { attrs.insert("taxon", repri(taxon)) }
    if key != none { attrs.insert("key", repri(key)) }
    if value != none { attrs.insert("value", repri(value)) }
    if limit != none { attrs.insert("limit", repri(limit)) }
    if title != none { attrs.insert("title", repri(title)) }
    if not include-indexes { attrs.insert("include-indexes", "false") }
    html.elem("wanshi-query", attrs: attrs)
  } else {
    // Paged output has no graph to query; show what would be listed.
    block(
      below: small-block-below,
      text(fill: muted-color, style: "italic", "[listing: " + repri(from) + "]"),
    )
  }
})

/// Direct children of this section, oldest first.
#let children(sort: "date", order: "asc", limit: none, title: none, include-indexes: true) = query(
  from: "children",
  sort: sort,
  order: order,
  limit: limit,
  title: title,
  include-indexes: include-indexes,
)

/// The most recently dated sections in the whole forest.
#let recent(count: 10, title: none, include-indexes: true) = query(
  from: "all",
  sort: "date",
  order: "desc",
  limit: count,
  title: title,
  include-indexes: include-indexes,
)

/// Every section carrying a given taxon.
#let by-taxon(taxon, sort: "title", order: "asc", limit: none, title: none, include-indexes: true) = query(
  from: "all",
  taxon: taxon,
  sort: sort,
  order: order,
  limit: limit,
  title: title,
  include-indexes: include-indexes,
)

/// Sections nothing links to and nothing embeds: written, then lost track of.
///
/// Unlinked directory index pages count as orphans by default: being a parent
/// makes a hub reachable *from* its children, not *to* it. Pass
/// `include-indexes: false` to list only ordinary notes.
#let orphans(sort: "slug", order: "asc", limit: none, title: none, include-indexes: true) = query(
  from: "orphans",
  sort: sort,
  order: order,
  limit: limit,
  title: title,
  include-indexes: include-indexes,
)

#let embed(url, title, numbering: false, open: true, catalog: true, display-options: false) = {
  with-target-check((export-target) => {
    if export-target == "html" {
      let v = title
      let attrs = (url: url, numbering: repri(numbering), open: repri(open), catalog: repri(catalog))

      if type(title) != content {
        v = none
        attrs.insert("value", repri(title))
      }

      html.elem("wanshi-embed", v, attrs: attrs)
    } else {
      block(below: small-block-below, text(fill: ink-color, size: 1.083em, weight: heading-font-weight, title))
      if display-options {
        block(text(fill: paged-metadata-text-color)[`numbering:` #numbering ~ `open:` #open ~ `toc:` #catalog])
      }
    }
  })
}

#let subtree(
  slug: none, // default: anonymous subtree
  title: none,
  taxon: none,
  numbering: false,
  open: true,
  catalog: true,
  content,
) = with-target-check((export-target) => {
  if export-target == "html" {
    let attrs = (numbering: repri(numbering), open: repri(open), catalog: repri(catalog))
    if slug != none { attrs.insert("slug", repri(slug)) }
    if title != none { attrs.insert("title", repri(title)) }
    if taxon != none { attrs.insert("taxon", repri(taxon)) }
    html.elem("wanshi-subtree", content, attrs: attrs)
  } else {
    block(below: small-block-below)[
      #if taxon != none {
        text(size: 1.083em, weight: heading-font-weight, fill: taxon-color, taxon-upper(taxon))
      }
      #text(size: 1.083em, weight: heading-font-weight, fill: ink-color, title)
      #if slug != none { span-slug(slug) }
    ]
    content
  }
})

// Semantic subtree sugar helpers for common note taxons.
#let exegesis(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "exegesis" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let definition(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "definition" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let proposition(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "proposition" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let remark(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "remark" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let conjecture(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "conjecture" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let postulate(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "postulate" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let claim(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "claim" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let observation(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "observation" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let fact(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "fact" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let hypothesis(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "hypothesis" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let axiom(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "axiom" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let lemma(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "lemma" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let theorem(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "theorem" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let corollary(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "corollary" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let example(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "example" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)

#let proof(slug: none, title: none, taxon: none, numbering: false, open: true, catalog: true, content) = subtree(
  slug: slug,
  title: title,
  taxon: if taxon == none { "proof" } else { taxon },
  numbering: numbering,
  open: open,
  catalog: catalog,
  content,
)


/**
 * Document wrapper.
 *
 * Equations are deliberately left alone in HTML: Typst emits MathML for them,
 * which the browser lays out as text on the surrounding baseline, in the page's
 * own font and colour, and which stays selectable and legible to a screen
 * reader.
 *
 * They used to be replaced with `html.frame` SVG, to match paged output
 * exactly. That cost roughly six times the page weight in outlined glyphs, and
 * an SVG box sits on the baseline by its bottom edge, so every formula floated
 * above the line by its own depth. The correction for that measured a position
 * on the page — something HTML export does not have, so it silently never ran.
 *
 * `auto-frame` still produces SVG, which is what diagrams and other arbitrary
 * content need.
 */

#let wanshi(doc) = {
  with-target-check((export-target) => {
    if export-target == "paged" {
      set page(margin: 2em, paper: "iso-b6", height: auto)
      set par(spacing: 1.5em)
      doc
    } else {
      doc
    }
  })
}
