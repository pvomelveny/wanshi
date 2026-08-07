# User Documentation

This section is for people using wanshi to create and publish a static note site.

**Start here**

- [Getting Started](getting-started.md) — install, scaffold a site, write your first note, build.

**Authoring**

- [Writing Notes](writing-notes.md) — the day-to-day workflow for producing a new note, from `wanshi new post` to publish.
- [Links and References](links-and-references.md) — `local`, `embed`, `external`, and how references and backlinks are derived.
- [Content Authoring](content-authoring.md) — reference for metadata keys and the Typst library.

**Operating**

- [Configuration Reference](configuration.md) — every `Wanshi.toml` key, plus the page-head customization hooks.
- [Command Reference](commands.md) — every command and flag.
- [Publishing and Maintenance Workflows](publishing-and-workflows.md) — deployment, RSS, caches, upgrades, troubleshooting.

## Project Shape

wanshi expects a project with a `Wanshi.toml` file, a source tree directory, and
an assets directory. By default those are `Wanshi.toml`, `trees/`, and `assets/`.

```
Wanshi.toml             configuration
trees/                  source tree — one .typ file per note
  index.typ           the entry-point section
  _lib/wanshi.typ       the bundled Typst library
assets/                 static files, copied verbatim into the output
publish/                build output (generated)
.cache/                 caches and serve output (generated)
```

Notes are written in Typst and nothing else — wanshi does not accept Markdown.
