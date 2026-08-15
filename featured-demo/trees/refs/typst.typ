#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "Typst",
  "taxon": "reference",
  "date": "2023-03-21",
  "asref": "true",
  // Declared because `guide/embeds` embeds this note, and embedding otherwise
  // makes the embedding page the parent — which would file a reference under
  // the guide. See that note's "Embedding and parents".
  "parent": "refs/index",
))

The markup-based typesetting system every note here is written in. wanshi shells
out to it for each source file and embeds the result, so any Typst feature —
packages, scripting, diagrams, maths — is available inside a note without wanshi
knowing anything about it.

#external("https://typst.app", "typst.app")
