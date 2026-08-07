# wanshi Documentation

wanshi is a Typst-only static site generator for Zettelkasten-style forests of
notes. The documentation is split by audience:

- [User documentation](users/README.md): installation assumptions, site creation, authoring, linking, configuration, local preview, validation, and publishing.
- [Developer documentation](developers/README.md): architecture, design model, compiler flow, maintenance guidance, and testing strategy.

## Quick Links

| I want to… | Go to |
| --- | --- |
| Set up a site and build it | [Getting Started](users/getting-started.md) |
| Write a new note, start to finish | [Writing Notes](users/writing-notes.md) |
| Link notes, cite sources, get backlinks | [Links and References](users/links-and-references.md) |
| Look up a metadata key or Typst helper | [Content Authoring](users/content-authoring.md) |
| Look up a config key or restyle the site | [Configuration](users/configuration.md) |
| Look up a command or flag | [Commands](users/commands.md) |
| Deploy, enable RSS, or upgrade | [Publishing and Workflows](users/publishing-and-workflows.md) |
| Understand how the compiler works | [Architecture](developers/architecture.md) |

The user documentation describes how to operate the current program. The
developer documentation avoids naming concrete source-code locations so it
remains useful across routine refactors.

wanshi is a fork of [kodama](https://github.com/kodama-community/kodama); see
`NOTICE.md` for attribution. Two things differ structurally from upstream:
**Typst is the only accepted input format** (all Markdown support is removed),
and the site ships a single built-in "Parchment & walnut" visual design.
