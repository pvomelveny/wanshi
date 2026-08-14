# featured-demo

A small forest that documents itself. Every note demonstrates one wanshi feature
and explains, in its own text, how it was written — so reading a built page
beside its source in `trees/` should be enough to copy the technique.

```sh
wanshi serve    # preview at http://localhost:8087 with live reload
wanshi build    # write ./publish
wanshi check --strict
```

Requires `typst` on your `PATH`. The diagram in `math/fixed-point.typ` pulls
`fletcher` from the Typst package registry, so the first build needs network
access; everything else is offline.

## Where each feature lives

| Feature | Source | Rendered |
| --- | --- | --- |
| Slugs, file layout, `_` prefix | `trees/guide/slugs.typ` | `/guide/slugs.html` |
| Metadata keys, custom keys, links in metadata | `trees/guide/metadata.typ` | `/guide/metadata.html` |
| `local`, `external`, references, backlinks | `trees/guide/links.typ` | `/guide/links.html` |
| `embed`, and its options | `trees/guide/embeds.typ` | `/guide/embeds.html` |
| `subtree`, anonymous and named, semantic helpers | `trees/guide/subtrees.typ` | `/guide/subtrees.html` |
| `children`, `recent`, `by-taxon`, `orphans`, `query` | `trees/guide/listings.typ` | `/guide/listings.html` |
| Directory index, inferred parents, directory URLs | `trees/guide/index.typ` | `/guide/` |
| Typst maths, `tex()`, diagrams, `auto-frame` | `trees/math/fixed-point.typ` | `/math/fixed-point.html` |
| Notes marked `asref`, collected as citations | `trees/refs/` | `/refs/` |
| An orphan, for the orphan listing to find | `trees/strays/unlinked.typ` | `/strays/unlinked.html` |
| Sharing Typst code between notes | `trees/_showcase.typ` | *(not a page — that is the point)* |
| Per-page `footer-mode` override | `trees/guide/links.typ` metadata | `/guide/links.html` |
| Site configuration, commented | `Wanshi.toml` | — |

## Things the demo shows by existing

- **`trees/_showcase.typ` and `trees/_lib/` produce no pages.** The leading
  underscore is the whole mechanism.
- **`guide/subtrees-named.html` and `math/contraction.html` have no source
  files.** Both are subtrees declared inside another note, promoted to pages by
  being given a slug.
- **`/guide/` and `/refs/` are directory URLs**, served without a filename,
  while their slugs remain `guide/index` and `refs/index`. Links use the slug.

## Deliberately switched off

`Wanshi.toml` leaves two things off, with comments explaining why:

- **`[publish].rss`** — a feed needs an absolute `base-url` to be useful, and
  this demo uses `/` so it can be previewed from disk.
- **`[build].asref`** — turning it on globally makes *every* linked note a
  citation. The demo instead marks individual notes, which is what `refs/`
  illustrates.
