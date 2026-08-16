# Command Reference

wanshi commands accept the usual `--help` flag. Most commands also have visible aliases for shorter interactive use.

| Command | Alias | Purpose |
| --- | --- | --- |
| `wanshi new` | `n` | Create a site, config file, note, or KaTeX loader |
| `wanshi init` | `i` | Initialize an existing directory as a site |
| `wanshi build` | `b` | Compile the site to HTML |
| `wanshi check` | `c` | Validate sections and graph without writing output |
| `wanshi serve` | `s` | Preview locally with watch and live reload |
| `wanshi snip` | — | Generate VS Code snippet files |
| `wanshi upgrade` | `u` | Upgrade config shape and sync the Typst library |

## `wanshi new`

Creates site files, config files, or sections.

```sh
wanshi new site <path>
wanshi new config [path]
wanshi new post <path>
wanshi new katex
```

Aliases: `wanshi n`, and `s` / `c` / `p` for the three subcommands (so
`wanshi n p notes/alice` is the shortest form of `wanshi new post notes/alice`).

### `new site`

Creates a new directory and writes a default wanshi project into it, including the bundled Typst library.

### `new config`

Writes a default configuration file. The default path is `Wanshi.toml`.

### `new post`

Creates a new section under the configured source tree.

Options:

- `--template <path>`, short `-t`: template file (default `./template`). The placeholder `<FILE_NAME>` is replaced with the new file stem. If the default path does not exist, a built-in skeleton is used.
- `--config <path>`, short `-c`: configuration file.

The path is given a `.typ` extension automatically if it has none; any other extension is rejected. A path that already begins with the source tree name is accepted — the prefix is stripped rather than doubled. Intermediate directories are created; existing files are never overwritten.

### `new katex`

Writes `import-math.html`, the KaTeX loader that makes the `tex()` helper render.
Nothing loads KaTeX otherwise: ordinary equations are MathML and need no library,
so a site that never calls `tex()` makes no request to a maths CDN.

An existing `import-math.html` is left alone, so the command is safe to re-run.

Options:

- `--config <path>`, short `-c`: configuration file.

## `wanshi init`

```sh
wanshi init [path]
```

Initializes an existing directory as a wanshi project, including the bundled Typst library. The directory must already exist. The path defaults to `./`.

Alias: `wanshi i`.

## `wanshi build`

```sh
wanshi build
```

Compiles the current site into the publish output directory.

Options:

- `--config <path>`, short `-c`: configuration file.
- `--verbose`, short `-v`: print build output.
- `--verbose-skip`: print skip output.
- `--no-cache`, alias `--nc`: rebuild all files without using caches.
- `--indexes`: generate `wanshi.json`.
- `--no-indexes`: skip `wanshi.json`.
- `--graph`: generate `wanshi.graph.json`.
- `--no-graph`: skip `wanshi.graph.json`.

Alias: `wanshi b`.

## `wanshi check`

```sh
wanshi check
```

Validates the site without generating build artifacts.

Options:

- `--config <path>`, short `-c`: configuration file.
- `--strict`: treat warnings as errors.

Alias: `wanshi c`.

## `wanshi serve`

```sh
wanshi serve
```

Builds the site into the serve output directory, starts the configured static server, watches source/config/theme/assets files, and rebuilds on changes.

Options:

- `--config <path>`, short `-c`: configuration file.
- `--verbose`, short `-v`: print build output.
- `--verbose-skip`: print skip output.
- `--disable-reload`, short `-d`: disable live reload.
- `--watch-stats`, short `-w`: print dirty-path analysis for each watch batch.
- `--no-server`: build and watch the serve output without starting the configured static server. Useful when something else is already serving the directory, or when `miniserve` is not installed.
- `--print-json`: print line-delimited JSON events for editor integrations. Requires `--no-server` so stdout stays machine-readable.
- `--indexes`: generate `wanshi.json` during serve.
- `--no-indexes`: skip `wanshi.json`.
- `--graph`: generate `wanshi.graph.json` during serve.
- `--no-graph`: skip `wanshi.graph.json`.

Alias: `wanshi s`.

### Editor Integration Events

`wanshi serve --no-server --print-json` emits one JSON object per line: a `ready`
event after the initial build, then a `rebuilt` event after each successful
rebuild.

```json
{"config":"./Wanshi.toml","event":"ready","output":"./.cache/publish","reload":"./.cache/publish/wanshi.reload","root":"."}
```

`reload` is the path of the live-reload marker file, which is touched on every
rebuild — a tool can watch it instead of parsing events.

## `wanshi snip`

```sh
wanshi snip --katex
```

Generates VS Code snippet files in `.vscode/`. Editor convenience only — it does
not change what a page loads. To make `tex()` render, use
[`wanshi new katex`](#wanshi-new).

Options:

- `--config <path>`, short `-c`: configuration file.
- `--katex`, short `-k`: write `.vscode/katex.code-snippets`.

`--katex` is currently the only generator, and the command does nothing without
it. (Upstream kodama also shipped Markdown section snippets; wanshi is
Typst-only, so those were removed.)

## `wanshi upgrade`

```sh
wanshi upgrade
wanshi upgrade all
wanshi upgrade config
wanshi upgrade typst-lib
```

Upgrades configuration shape and/or syncs the bundled Typst library into the configured source tree.

Subcommands:

- `all`, alias `a`: upgrade config and sync Typst library. This is the default when no subcommand is supplied.
- `config`, alias `c`: upgrade config only.
- `typst-lib`, alias `t`: sync `wanshi.typ` only.

Options for config upgrades:

- `--config <path>`, short `-c`: source configuration file.
- `--output <path>`, short `-o`: write upgraded config to another path instead of overwriting.

Alias: `wanshi u`.
