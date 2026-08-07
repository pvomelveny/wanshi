# Publishing and Maintenance Workflows

## Recommended Authoring Loop

1. Run `wanshi serve` and leave it running while you write.
2. Create or edit `.typ` sections under the source tree — see [Writing Notes](writing-notes.md).
3. Run `wanshi check --strict` before publishing.
4. Run `wanshi build` for the final output.
5. Deploy the configured publish output directory.

This page covers steps 3–5. The authoring side of the loop is documented in
[Writing Notes](writing-notes.md) and [Links and References](links-and-references.md).

## Continuous Integration

`wanshi check --strict` exits non-zero on any warning, which makes it a suitable
gate. A minimal pipeline:

```sh
wanshi check --strict
wanshi build
```

Both commands need `typst` on `PATH`. Neither needs a server, so `miniserve` is
not a CI dependency.

## Static Hosting

wanshi output is static HTML, CSS, JavaScript, assets, and optional JSON/XML artifacts. Any static hosting provider can serve it.

Use:

```sh
wanshi build
```

Then publish the configured `[build].output` directory.

If your host serves the site under a subpath, configure:

```toml
[wanshi]
base-url = "/subpath/"
```

## Publishing a Feed

Two settings turn on an RSS feed:

```toml
[wanshi]
base-url = "https://example.com/"   # must be absolute

[publish]
rss = true
```

`base-url` has to be an absolute `http://` or `https://` URL with a host,
because a feed is read away from your site and every link in it must stand on
its own. A relative base URL is rejected before the feed is written, with a
message naming the setting — this is the most common way to get RSS wrong.

`wanshi build` then writes `feed.xml` beside your pages, and every page links to
it, so browsers and feed readers discover it automatically. Nothing else is
needed. Feeds are a publish-time artifact: `wanshi serve` does not write one, and
preview pages correctly do not advertise one.

### What Goes in the Feed

Every visible section becomes an item, except:

- the root `index` page,
- anything marked `"collect": "true"`, which is the switch for "this is a
  listing page, not a post", and
- subtrees whose source note is itself in the feed, since their content already
  appears there — see [below](#notes-defined-inside-other-notes).

Items are newest first by `date`, falling back to slug order. The channel title
comes from your `index` page's title.

### Dates

`date` metadata becomes the item's `pubDate`, converted to the RFC 822 format
feeds require. `2026-08-06`, `10/12/2025`, and `October 12, 2025` all work.

A note with **no `date` gets no `pubDate`**. That is valid, but readers vary in
how they order such items, and they sort last in the feed. If you care about feed
ordering, set `date` on everything.

### Notes Defined Inside Other Notes

A named subtree has its own page, but its content is also rendered inside the
note that declared it. Listing both would hand a subscriber the same text twice,
so **a subtree is a feed item only when its source note is not**.

That gives you a choice per file, using `collect`:

```typst
// One post. The definitions are part of it, and the feed gets one item.
#metadata(( "title": "Notes on monoids", "date": "2026-05-02" ))

#definition(slug: "monoid", title: "Monoid")[...]
#theorem(slug: "free-monoid", title: "Free monoid")[...]
```

```typst
// A container of notes. The file itself is not a post, so the feed carries
// the notes inside it instead.
#metadata(( "title": "Algebra", "date": "2026-05-02", "collect": "true" ))

#definition(slug: "monoid", title: "Monoid")[...]
#theorem(slug: "free-monoid", title: "Free monoid")[...]
```

The first yields one item; the second yields two. Subtrees that do appear
inherit their source's `date`, so they sort with the note they were written in
rather than falling to the end of the feed.

Anonymous subtrees — those written without `slug:` — are never feed items, since
they have no page of their own.

### Mathematics in Summaries

Item summaries are plain text, and Typst renders mathematics as images, so
**formulas do not appear in the summary** — a sentence built around one can read
oddly in a reader's list view.

The full article does keep its mathematics: readers that show an item's whole
content render it normally. Only the short summary is affected. If it bothers
you, write the opening sentence or two of a note in prose.

### Checking the Result

`feed.xml` is plain XML; a quick look confirms the shape:

```sh
wanshi build
head -20 publish/feed.xml
```

Worth checking after your first build: that `<link>` values are absolute and
point at your real domain, and that `<pubDate>` values look right.

## Pretty URLs

Enable pretty URLs when your host maps extensionless paths to generated HTML pages:

```toml
[build]
pretty-urls = true
```

For local preview, make sure the configured static server command supports the same URL style.

## Cache and Incremental Builds

wanshi maintains cache data under `.cache`. Normal builds reuse caches and hash checks to avoid unnecessary work.

Deleting a note also deletes its published page: each build reconciles the output directory against the notes that currently exist, removing pages whose sources are gone and pruning directories left empty. Only files wanshi generated are ever removed, so anything else you keep in the publish directory — a `CNAME`, a hand-written `404.html`, the copied assets tree — is left alone.

Use:

```sh
wanshi build --no-cache
```

when investigating stale output or after making broad environment changes.

Serve mode also keeps an in-memory compile session and uses watcher dirty-path batches to avoid full rebuilds where possible.

## Upgrading Existing Sites

After installing a newer wanshi release, run:

```sh
wanshi upgrade
```

This rewrites the configuration into the current shape and syncs the bundled Typst library. To inspect a config upgrade first:

```sh
wanshi upgrade config --output Wanshi.upgraded.toml
```

To sync only the Typst library:

```sh
wanshi upgrade typst-lib
```

### Moving to the `.typ` Extension

Notes used to be `.typst`; new sites now use Typst's standard `.typ`, which is
what editors and language servers recognise without extra configuration. Both are
accepted, so **existing sites keep working untouched** and you can migrate at
your own pace, or not at all.

The migration is URL-safe: a slug is the source path *minus* the extension, so
renaming changes no slug, no page URL, no link, and no backlink. Link and embed
targets that spell out the old extension — `#embed("./alice.typst")` — keep
resolving too, since any recognised source extension is stripped from a target.

```sh
find trees -name '*.typst' -exec sh -c 'mv "$1" "${1%.typst}.typ"' _ {} \;
wanshi check --strict
```

One thing to know before you rename: `.typ` files are now notes, so any Typst
file in the tree that is *not* a note — shared macros, a reusable figure — must
be given a `_` prefix or moved into a `_`-prefixed directory, or it will turn
into a page. See [Helpers: the `_` Prefix](content-authoring.md#helpers-the-_-prefix).

### The Bundled Library

`trees/_lib/wanshi.typ` is a **copy** of the library that was current when the
site was scaffolded, not a live reference into the binary. New helpers added by a
wanshi release will not exist in your project until you run this. If you have
edited that file locally, diff before syncing — the upgrade overwrites it.

## Editor Integration

Every section header can carry an `[edit]` link that opens that note's source.
It is the fastest way to work: read the rendered page, spot something wrong,
click through to the exact file.

### How Edit Links Work

Two settings, for two different situations:

```toml
[serve]
edit = "vscode://file/"              # local preview — opens your editor

[build]
edit = "https://example.com/edit/"   # published pages — opens a web editor
```

- **`[serve].edit`** is a URL prefix with the file's **absolute path** appended,
  so it can address a file on your own machine. Defaults to `vscode://file/`.
- **`[build].edit`** is a prefix with the note's **repository-relative path**
  appended, which suits a forge URL like
  `https://github.com/you/notes/edit/main/trees/`. It is unset by default, so
  published pages carry no edit link unless you ask for one.

Anything that can be expressed as a URL works. The rest of this section is
per-editor detail; contributions welcome as more are worked out.

### VS Code

Works with no setup — it is the default, and VS Code registers the `vscode://`
scheme when installed:

```toml
[serve]
edit = "vscode://file/"
```

Variants are recognised too: `vscode-insiders://file/`, `vsc://file/`,
`vscodium://file/`.

### Neovim

Neovim is a terminal program, not a URL target, so it needs a small handler that
the browser can invoke. Point wanshi at a scheme of your choosing:

```toml
[serve]
edit = "nvim://file/"
```

Then register a handler for it. On macOS this must be an **app bundle**, because
the URL arrives as an Apple Event rather than as a command-line argument — a
bare shell script will never see it.

Compile a one-line AppleScript that forwards the URL to a helper:

```applescript
on open location this_URL
	set helper to (POSIX path of (path to home folder)) & ".local/bin/wanshi-edit"
	do shell script quoted form of helper & " " & quoted form of this_URL
end open location
```

```sh
osacompile -o ~/Applications/WanshiEdit.app handler.applescript
```

Declare the scheme in the bundle, re-sign it, and register it:

```sh
PLIST=~/Applications/WanshiEdit.app/Contents/Info.plist
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes array" "$PLIST"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes array" "$PLIST"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string nvim" "$PLIST"
codesign --force --deep -s - ~/Applications/WanshiEdit.app
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
  -f ~/Applications/WanshiEdit.app
```

The helper at `~/.local/bin/wanshi-edit` strips the scheme and hands the file to
Neovim. It tries hardest to land you where you are already working:

1. **A running Neovim session** — reuses your editor, with its plugins and
   preview already loaded.
2. **A running tmux session** — a new window beside your other work, rather than
   a stray terminal.
3. **A new terminal window** — only when neither is running.

```sh
#!/bin/bash
set -u
path="${1#nvim://}"; path="${path#file}"
path="$(printf '%b' "${path//%/\\x}")"        # percent-decode

raise() { open -a Ghostty 2>/dev/null || true; }
alive() { [ -S "$1" ] && nvim --server "$1" --remote-expr '1' >/dev/null 2>&1; }

# 1. A live Neovim session.
sock=""
marker="$HOME/.cache/nvim/last-server"        # written by an autocmd, if you add one
[ -r "$marker" ] && c="$(cat "$marker")" && alive "$c" && sock="$c"
if [ -z "$sock" ]; then
    for c in $(ls -t "${TMPDIR:-/tmp}"/nvim."$(id -un)"/*/nvim.*.0 2>/dev/null); do
        alive "$c" && { sock="$c"; break; }
    done
fi
if [ -n "$sock" ]; then
    nvim --server "$sock" --remote "$path"    # opens the buffer…
    raise                                     # …but does not raise the window
    exit 0
fi

# 2. A running tmux server: prefer an attached session, else the most recent.
if command -v tmux >/dev/null 2>&1 && tmux list-sessions >/dev/null 2>&1; then
    session="$(tmux list-sessions -F '#{session_attached} #{session_activity} #{session_name}' \
               2>/dev/null | sort -k1,1nr -k2,2nr | head -1 | awk '{print $3}')"
    if [ -n "$session" ]; then
        tmux new-window -t "$session" -- nvim "$path"
        raise
        exit 0
    fi
fi

# 3. Nothing running.
open -na Ghostty.app --args --window-save-state=never -e nvim "$path"
```

Substitute your terminal for Ghostty. Two things that cost time otherwise:

- **Ghostty cannot be launched from the CLI on macOS.** Use
  `open -na Ghostty.app --args …`; a direct `ghostty -e …` will not work.
- **`open -n` starts a new *application instance*, which restores its saved
  windows.** Without `--window-save-state=never` a single click can reopen
  several terminals at once — one measured launch produced five. The flag is in
  the script above; setting `window-save-state = never` in your Ghostty config
  has the same effect.
- **Ghostty strips arguments beginning with `+`**, since that is its own action
  syntax. `nvim +42 file` silently loses the line number; use `nvim -c 42 file`.

Each candidate socket is probed before use, so a stale socket left by a crashed
Neovim is skipped rather than swallowing the click.

For the handler to prefer the session you were **last using** rather than the
one most recently started — which differ once two are open — have Neovim record
it, since a Unix socket carries no usage timestamp:

```lua
vim.api.nvim_create_autocmd({ "FocusGained", "BufEnter" }, {
  callback = function()
    if vim.v.servername ~= "" then
      vim.fn.writefile({ vim.v.servername }, vim.fn.stdpath("cache") .. "/last-server")
    end
  end,
})
```

### Other Editors

Any editor registering a URL scheme works the same way — set `[serve].edit` to
its prefix. An editor without one needs a handler like the Neovim recipe above.

### Snippets

```sh
wanshi snip --katex
```

Writes `.vscode/katex.code-snippets`.

### Editor-Driven Builds

For tooling that drives wanshi itself, `wanshi serve --no-server --print-json`
emits line-delimited JSON build events; see the
[command reference](commands.md#editor-integration-events).

### Limitation: No Line Numbers

Edit links open a file at its beginning, never at a specific line. wanshi has
the machinery to append a position, but never records one, so this affects every
editor equally.

It matters most for **subtrees**: a subtree's edit link opens the top of the
file that declares it rather than the subtree itself. For a note that occupies
its own file — the common case — the beginning of the file is the right place
anyway.

## Troubleshooting

Run `wanshi check` when a build fails or generated links look wrong. It catches many graph and content issues before writing output.

Common issues:

- **Missing `index` section**: add `trees/index.typ`.
- **Dangling local link**: fix the target path or create the target section. The warning names the slug the link resolved to, which is usually enough to spot a relative-vs-absolute mistake — see [Links and References](links-and-references.md#slugs-are-the-address-space).
- **Cyclic embed**: the error prints the whole chain. Convert one embed in the cycle into a `local()` link.
- **Missing embed target**: unlike a dangling link, this fails the build. Create the target or remove the embed.
- **Duplicate slug**: two source files resolve to the same slug. Rename one.
- **Typst render error**: verify Typst is installed and that the source compiles on its own with `typst compile --root trees trees/<note>.typ`. If the failure is a "file not found" on the library import, the note is probably using the tree-relative `#import "_lib/wanshi.typ"`, which only resolves at the top of the tree; use the root-absolute `#import "/_lib/wanshi.typ"` instead. See [Import Paths](writing-notes.md#import-paths).
- **A helper turned into a page**: prefix the file or its directory with `_`. See [Helpers: the `_` Prefix](content-authoring.md#helpers-the-_-prefix).
- **RSS base URL error**: set `[wanshi].base-url` to an absolute `http://` or `https://` URL with a host.
- **`miniserve` not found**: install it, point `[serve].command` at another static server, or use `wanshi serve --no-server` and serve the output directory yourself.
- **CSS or fonts look wrong after an edit**: `main.css`, `main.js`, and `wanshi.typ` are compiled into the binary. Editing the copies in a built site works for that site, but changing wanshi's own bundled versions requires rebuilding the binary. To customize a site without rebuilding, use `import-style.html` — see [Customizing the Page Head](configuration.md#customizing-the-page-head).
- **Stale output**: run `wanshi build --no-cache`. Note that pages for deleted notes are removed automatically on every build, so this is only needed for genuinely stale *content*.
