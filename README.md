# slidekit

Turns a [Typst](https://typst.app) slide deck into

- the slides as **PDF**,
- a **speaker handout** PDF (the same deck compiled in handout mode),
- an **offline HTML player**: slides as images, videos playing in place, and a presenter
  view with notes, speaker hand-overs and timers. It works from `file://`, so you can copy
  `dist/` onto a USB stick.

slidekit doesn't know anything about your theme. The deck talks to it only through a small
contract library, `lib/slidekit.typ`.

## Install

```sh
cargo install --git https://github.com/gamersi/slidekit
```

typst 0.15.1 is built in, no separate `typst` binary needed. `@preview` packages are fetched
and cached the same way the typst CLI does it.

## Quickstart

```sh
slidekit init talk          # slidekit.toml, slides.typ, lib/slidekit.typ, .gitignore
cd talk
slidekit serve --open       # build, open the player, rebuild + live reload on every save
slidekit build              # one-off build into dist/
```

`examples/minimal` is a complete four-slide deck with a video.

## CLI

```
slidekit build  [DIR]                       # DIR defaults to "."; DIR must contain slidekit.toml
slidekit watch  [DIR]                       # build, then rebuild incrementally on change
slidekit serve  [DIR] [--port 8765] [--open]  # watch + HTTP server with live reload
slidekit init   [DIR]                       # new example deck (never overwrites)
slidekit lib    [DIR]                       # (re)write lib/slidekit.typ from the built-in copy

global flags: --quiet  --no-handout  --no-html  --no-pdf
```

Use `--no-handout --no-pdf` for the fastest loop while you are designing slides.

## The contract

Vendor `lib/slidekit.typ` (`slidekit lib`) and import it:

```typ
#import "lib/slidekit.typ" as sk

#sk.slide-info(title: [Intro], speaker: "Ada", minutes: 2, notes: [
  - say hello
])
#sk.video("media/clip.mp4", image("img/poster.jpg"), width: 16cm, height: 9cm)
```

| Piece                          | Meaning |
|--------------------------------|---------|
| `sys.inputs.slidekit-mode`     | `"slides"` (default) or `"handout"`; available as `sk.mode` / `sk.is-handout` |
| `<slidekit-slide>` metadata    | One per slide page: `(page, title, speaker, minutes, notes)`. `title` and `notes` may be content. Call `sk.slide-info(..)` once anywhere on the page. |
| `<slidekit-video>` metadata    | One per video area: `(page, file, x, y, w, h)` in pt from the page's top left. `sk.video(file, poster)` emits it and links the poster to the file in the PDF. |
| `sk.slides()`, `sk.run-sheet()`| Helpers for the handout (both need `context`). |

How the files are found and what slidekit expects:

- Video `file` paths are relative to the deck dir (or, if not found there, the typst root).
  They're copied to the same relative path in `dist/`, so the PDF links work too.
- Slide metadata is mapped by page. A page without `slide-info` still works, it just has
  no title, notes or minutes.
- The handout is the deck compiled with `slidekit-mode = "handout"`. What it looks like is
  up to the deck. The template shows a preview of each slide next to its notes, plus a run sheet.
- A plain `typst compile` of the deck still works; the metadata is then simply ignored.

## slidekit.toml

```toml
[deck]
entry = "main.typ"          # required
title = "My talk"           # player <title>; default: entry file stem
root = "."                  # typst project root, relative to the deck dir (may be a parent)
fonts = ["fonts"]           # extra font dirs (the typst default fonts are always included)
system-fonts = false        # also use installed system fonts
inputs = { }                # extra sys.inputs (string → string); slidekit-mode is reserved

[output]
dir = "dist"
pdf = "slides.pdf"          # "" disables
handout = "handout.pdf"     # "" disables; compiled with slidekit-mode=handout
html = "index.html"         # "" disables (also skips the slide images)
ppi = 216                   # 33.9 cm wide slide → 2880 px
jpeg-quality = 92
```

Relative paths resolve against the directory that contains `slidekit.toml`. Unknown keys
are an error.

## Player keys

| Key | Action |
|-----|--------|
| → / Space / PgDn, ← / PgUp / Backspace | next / previous slide |
| Home / End | first / last slide |
| number + Enter | jump to slide (also `#n` in the URL) |
| click on a video area, `V` | play / pause the video in place |
| Esc | close the video |
| `P` | presenter view in its own window (allow pop-ups for `file://`) |
| `F` | fullscreen |
| `T` | small elapsed-time display |
| `H` / `?` | help |
| `R` (presenter) | reset timers |

The presenter view shows the current and next slide, who is speaking (highlighted when the
next slide has a different speaker), the notes, the total time against the plan, the time on
the current slide with a progress bar, and a clock. Navigation and `V` in the presenter window
drive the audience window. The two windows talk via `postMessage`, so this works from `file://` too.
The timer starts with the first step forward; time spent on slide 1 counts up to its planned
minutes. The timers survive a reload.

`?debug` outlines the video areas and starts the first video muted.

When the player is served by `slidekit serve`, it reloads itself after every successful build
and stays on the current slide. Served over HTTP, the PDF's video links also work in Firefox.

## Architecture

```
main      clap CLI
config    slidekit.toml → validated Config with absolute paths
world     typst::World: Session (file cache, fonts, packages, today) shared across builds,
          ModeWorld per mode (library with sys.inputs incl. slidekit-mode)
compile   PagedDocument for a mode; diagnostics as file:line:col + source snippet
meta      <slidekit-slide>/<slidekit-video> via the introspector → typed structs
notes     typst content (JSON) → HTML for notes, plain text for titles
render    pages → JPEG (4:4:4) in parallel; file name = hash of the page's visible
          content + settings, so unchanged pages are skipped and stale ones removed
export    PDF via typst-pdf (slides and handout)
player    index.html from assets/player.html with the slide data as JSON
media     copy referenced videos into dist/ (skipped when size + mtime match)
build     one build: compile both modes concurrently, then PDFs / images / player in parallel
watch     notify + debounce; rebuilds when a file the last build read changes
serve     std-only HTTP server (range requests, SSE live reload at /__slidekit/events)
scaffold  init / lib
```

Incremental builds reuse the same `Session`, so typst's comemo memoization makes recompiles
take milliseconds. On top of that, only the pages whose pixels changed are re-rendered.
