#import "/_lib/wanshi.typ": *
#import "/_showcase.typ": showcase

#show: showcase

#metadata((
  "title": "A note that embeds another",
  "taxon": "remark",
  "date": "2026-08-12",
))

This note is embedded by #local("/guide/embeds"), and embeds one of its own.
Read it there and you are three levels down; read it here and it is an ordinary
page with one embed in it. Both are the same note.

#embed("/guide/chain-inner", "The note at the end of the chain")
