# slidekit

Turns a [Typst](https://typst.app) slide deck into

- the slides as **PDF**,
- a **speaker handout** PDF (the same deck compiled in handout mode),
- an **offline HTML player**: slides as images with transitions and build steps, videos
  playing in place, clickable links, overview, blackout, laser pointer and pen, and a
  presenter view with notes, speaker hand-overs and timers. It works from `file://`, so you can copy
  `dist/` onto a USB stick.

slidekit doesn't know anything about your theme. The deck talks to it only through a small
contract library, `lib/slidekit.typ`.

## Install

Prebuilt binaries for Windows, Linux and macOS are on the
[releases page](https://github.com/gamersi/slidekit/releases): unpack and put `slidekit`
(`slidekit.exe`) somewhere on your `PATH`, or run it from the deck folder. Or build it yourself:

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

`examples/minimal` is a complete four-slide deck with a video and build steps. `examples/showcase` uses
everything: every transition, build steps, two videos on a slide, an ambient video, links,
auto-advance and a handout with run sheet.

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

Transitions, build steps and auto-advance are slide settings:

```typ
#sk.slide-info(title: [Intro], transition: "slide")              // or (kind: "zoom", duration: 0.8), none, auto
#sk.slide-info(title: [Break], advance: 10)                      // next slide after 10 s

// a slide built up in 3 steps: one page per step, `step: i` tells the player they belong together
#sk.stepped(3, i => page[
  #sk.slide-info(title: [Plan], step: i)
  - always there
  - #sk.uncover(2)[from step 2 on (space is kept)]
  - #sk.only("3-")[from step 3 on (no space before)]
  #sk.focus(2)[dimmed except in step 2]
])
```

`uncover`/`only`/`focus` take a step number, an array of steps or a string like `"2-4"`,
`"-2"` or `"1, 3-"`. A number means "from then on" for `uncover` and "exactly that step" for
`only` and `focus`. Labels inside a stepped slide occur once per step, so put link targets
outside of it (or only in step 1). Outside of `stepped` they show everything, so a handout can
render the slide once (`make(n)` or `sk.at-step(n, ..)`).

Transitions: `none`, `fade`, `slide`, `cover`, `zoom`, `wipe`. A slide's transition plays when
it enters; going back plays it in reverse. Build steps of one slide cross-fade quickly.

| Piece                          | Meaning |
|--------------------------------|---------|
| `sys.inputs.slidekit-mode`     | `"slides"` (default) or `"handout"`; available as `sk.mode` / `sk.is-handout` |
| `<slidekit-slide>` metadata    | One per slide page: `(page, title, speaker, minutes, notes, transition, advance, step, contract)`. `title` and `notes` may be content. Call `sk.slide-info(..)` once anywhere on the page. A page with `step` > 1 continues the slide before it. |
| `<slidekit-video>` metadata    | One per video area: `(page, file, x, y, w, h, autoplay, loop, muted, controls)`, position in pt from the page's top left. `sk.video(file, poster)` emits it and links the poster to the file in the PDF. |
| links                          | Every `link(..)` on a slide is clickable in the player: URLs open in a new tab, links to a label or location jump to that slide. |
| `sk.slides()`, `sk.run-sheet()`| Helpers for the handout (both need `context`); build steps are folded. |

How the files are found and what slidekit expects:

- Video `file` paths are relative to the deck dir (or, if not found there, the typst root).
  They're copied to the same relative path in `dist/`, so the PDF links work too.
- Slide metadata is mapped by page. A page without `slide-info` still works, it just has
  no title, notes or minutes.
- The handout is the deck compiled with `slidekit-mode = "handout"`. What it looks like is
  up to the deck. The template shows a preview of each slide next to its notes, plus a run sheet.
- A plain `typst compile` of the deck still works; the metadata is then simply ignored.

### Contract versions

The library stamps its contract version (`sk.contract`, currently 2) into the slide metadata
and slidekit compares it with its own on every build:

- the vendored `lib/slidekit.typ` is an **unmodified older copy**: slidekit replaces it with
  the current one and says so (nothing of yours can be lost);
- it is **older and modified** (or lives elsewhere): a warning tells you to run `slidekit lib`;
- it is **newer** than the binary: a warning tells you to update slidekit.

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

[player]
transition = "fade"         # default transition: none | fade | slide | cover | zoom | wipe
transition-duration = 0.35  # seconds
progress = false            # thin progress bar along the bottom of the slides
loop = false                # after the last slide, start over (kiosk)
advance = 0                 # auto-advance every n seconds; 0 = off (slides can set `advance:`)
```

Relative paths resolve against the directory that contains `slidekit.toml`. Unknown keys
are an error.

## Player keys

| Key | Action |
|-----|--------|
| → / Space / PgDn, ← / PgUp / Backspace | next / previous step or slide (also swipe) |
| Shift + → / ← | next / previous slide, skipping build steps |
| Home / End | first / last slide |
| number + Enter | jump to slide (also `#n` or `#n.step` in the URL) |
| `O` / `G` | overview of all slides; arrows + Enter or click to jump |
| click on a video area, `V` | play / pause the video in place; once it has ended, `V` plays the next video on the slide (`Shift+V`: previous) |
| Esc | close videos, overview or help |
| `B` / `.`, `W` / `,` | black / white screen (any navigation lifts it) |
| `L` | laser pointer |
| `D`, `C` | draw on the slide (kept per slide), clear the drawing |
| `P` | presenter view in its own window (allow pop-ups for `file://`) |
| `F` | fullscreen |
| `T` | small elapsed-time display |
| `H` / `?` | help |
| `R` | reset timers |
| `+` / `-` (presenter) | notes font size |

The presenter view shows the current and next slide, who is speaking (highlighted when the
next slide has a different speaker), the notes, the total time against the plan, the time on
the current slide with a progress bar, and a clock. Navigation and `V` in the presenter window
drive the audience window, and with `L` or `D` the mouse over the presenter's copy of the
slide points or draws on the audience screen. Badges show a blank screen, laser, pen, the
video state ("V → video 2/2") and auto-advance. The two windows talk via `postMessage`, so this works from `file://` too.
The timer starts with the first step forward; time spent on slide 1 counts up to its planned
minutes. The timers survive a reload.

Videos with `autoplay` start when their slide is in place (muted if the browser blocks sound
before the first interaction); `autoplay + loop` videos are ambient and `V` leaves them alone.
Auto-advance waits while a video plays. Transitions respect `prefers-reduced-motion`.

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
contract  library version check, in-place upgrade of unmodified old copies
links     link areas from the page frames (URL or target page)
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
