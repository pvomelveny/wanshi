#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "Slugs and file layout",
  "taxon": "definition",
  "date": "2026-08-10",
))

A *slug* is a source file's path inside `trees/`, minus the extension. This file
is `trees/guide/slugs.typ`, so its slug is `guide/slugs`. That is the whole
rule; there is no separate identifier to assign and keep in sync.

Slugs are the address space. Links, parents, listings, and the published URL all
refer to a note by its slug, so moving a file renames the note.

= Writing a slug

Root-absolute slugs begin with `/` and are resolved from `trees/`:

```typst
#local("/guide/listings")
```

Relative slugs are resolved from the linking note's own directory:

```typst
#local("./listings")
```

Prefer root-absolute. A relative link breaks the moment the file moves, which is
exactly when you are least likely to notice.

= Two names with special meaning

#subtree(title: "index", taxon: "observation")[
  A file called `index.typ` is a directory index — the parent of everything
  beside it, published at the directory's own URL. `trees/index.typ` is the root
  of the forest. See #local("/guide/index") for what that buys you.
]

#subtree(title: "A leading underscore", taxon: "observation")[
  Files whose name starts with `_` are skipped. They carry Typst code, not
  notes, so they never become pages. The bundled library lives at
  `trees/_lib/wanshi.typ` for that reason, and this forest keeps its shared
  helpers in `trees/_showcase.typ`.

  Dot-prefixed files are skipped the same way.
]

= Grouping

Directories are the grouping mechanism, and they nest as deeply as you like.
Nothing stops a note in one directory from linking to a note in another —
#local("/math/fixed-point") is one directory over, and the link is the same
shape as any other. Grouping affects navigation, not reachability.
