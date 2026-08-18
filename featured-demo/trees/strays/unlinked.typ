#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "A note nothing points at",
  "taxon": "remark",
  "date": "2026-08-08",
))

Nothing in this forest links to or embeds this note. It exists so that the
orphan listing on #local("/guide/listings") has something to report.

That listing is the reason to care: in a forest of any size, notes go missing
not by being deleted but by being written and never linked. `#orphans()`
on a page you actually read is how they come back.

Note that this page linking *out* to the guide does not rescue it. Orphanhood is
about inbound links.

= But there is a breadcrumb right above this

Look at the top of this page: a trail leading back to the root index. If it can
be reached from there, in what sense is it an orphan?

The answer is that the two are answering different questions, and wanshi
computes them separately.

#subtree(title: "The breadcrumb is structural", taxon: "observation")[
  A note's parent is resolved by falling through three candidates: a `parent`
  declared in metadata, then whatever embedded the note, then the nearest
  enclosing directory index.

  That last step always succeeds. There is no `trees/strays/index.typ`, so this
  note falls through to the root — and the root is the floor, so *every* note
  has a parent. There is no parentless state to be in.

  The breadcrumb answers "where does this sit?", which is a question about the
  file tree. It is always answerable.
]

#subtree(title: "Orphanhood is about the link graph", taxon: "observation")[
  A note is an orphan when nothing links to it and nothing embeds it. The
  directory-index fallback deliberately does not count — it is inferred from
  where the file happens to live, not written by anyone as a way of getting
  here.

  This question is "how would a reader ever arrive?", and the answer can be
  "they would not".
]

The distinction is the point. A note can be filed perfectly sensibly and still
be unreachable by anyone actually browsing — that is the failure #local("/guide/listings")
exists to catch. If the breadcrumb counted as reachability, nothing would ever
be an orphan and the listing would always be empty.

The clearest way to see it: this note appears in the orphan listing on a page
you can read, while sitting under a breadcrumb that says it belongs to the root.
Both are true at once.
