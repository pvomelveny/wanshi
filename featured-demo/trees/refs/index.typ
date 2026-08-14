#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "References",
  "date": "2026-08-14",
  // An index whose job is to link everything would otherwise put a backlink to
  // itself on every note it lists. This keeps the listings clean.
  "asback": "false",
))

Notes here are marked `"asref": "true"`, so linking to one cites it: the link
drops into the citing page's footer instead of interrupting the sentence. See
#local("/guide/links") for the mechanism.

#by-taxon("reference", title: none)
