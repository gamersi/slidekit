# Typst for slides

slidekit embeds Typst 0.15. These are the idioms and pitfalls that matter for decks. When
the Typst syntax for something is unclear, check https://typst.app/docs. Do not guess
function names: a wrong guess is a compile error at best and a silent layout bug at worst.

## Theme skeleton

```typ
// theme.typ
#import "lib/slidekit.typ" as sk

// tokens: the only place with literal colours and sizes
#let bg = rgb("#fbfaf7")
#let ink = rgb("#1d1f24")
#let muted = rgb("#6b7080")
#let accent = rgb("#d6336c")
#let W = 33.867cm          // 16:9
#let H = 19.05cm
#let pad = 2cm
#let sizes = (body: 26pt, small: 20pt, title: 44pt, display: 80pt)

/// One slide. `body` is content, or `i => content` together with `steps: n`.
#let slide(title: none, speaker: none, minutes: none, notes: none,
           transition: auto, steps: none, body) = {
  let content(i) = if type(body) == function { body(i) } else { body }
  let info(step) = sk.slide-info(title: title, speaker: speaker, minutes: minutes,
                                 notes: notes, transition: transition, step: step)
  let draw(i) = {
    if title != none { text(size: sizes.title, weight: "bold", title); v(0.8cm, weak: true) }
    content(i)
  }
  if sk.is-handout {
    // one entry per slide: small preview (last step) next to the notes
    block(breakable: false, below: 1cm, {
      place(info(none))
      grid(columns: (40%, 1fr), gutter: 0.6cm,
        box(stroke: 0.5pt + luma(180), scale(30%, reflow: true,
          box(width: W, height: H, fill: bg, inset: pad, {
            set text(size: sizes.body, fill: ink)
            if steps == none { draw(none) } else { sk.at-step(steps, draw(steps)) }
          }))),
        if notes != none { notes } else { text(fill: muted)[No notes.] })
    })
  } else if steps == none {
    page({ place(info(none)); draw(none) })
  } else {
    sk.stepped(steps, i => page({ place(info(i)); draw(i) }))
  }
}

#let deck(title: none, body) = {
  set document(title: title)
  set text(font: "Libertinus Serif", lang: "en")
  if sk.is-handout {
    set page(paper: "a4", margin: 2cm)
    set text(size: 10.5pt)
    body
  } else {
    set page(width: W, height: H, margin: pad, fill: bg)
    set text(size: sizes.body, fill: ink)
    show heading: set text(fill: accent)
    body
  }
}
```

```typ
// main.typ
#import "lib/slidekit.typ" as sk
#import "theme.typ": *
#show: deck.with(title: "My talk")

#if sk.is-handout { heading[Speaker handout]; context sk.run-sheet(); pagebreak() }

#slide(title: [Caching made builds 4× faster], speaker: "Ada", minutes: 2, notes: [
  - The headline number first, then how we measured it.
  - Next: what we cached.
])[ .. ]
```

`examples/showcase` in the slidekit repository is a complete, polished theme (dark,
speaker colours, section slides, agenda links, handout with run sheet). Copy from it
when you need more than the skeleton.

## Layout idioms

- **Vertical centring or spreading**: `v(1fr)` before and after, inside a page or a block
  with a fixed height.
- **Columns**: `grid(columns: (1fr, 1fr), column-gutter: 1.2cm, left, right)`. Use
  `(2fr, 3fr)` for unequal columns and `align: horizon` to centre them vertically.
- **Absolute placement** of decoration: `place(top + right, dx: -1cm, dy: 1cm, ..)`. It
  takes no space, so keep content out of `place` unless you mean to.
- **Full-bleed image**: on a page with margin 0, use `image("x.jpg", width: 100%, height:
  100%, fit: "cover")`. With margins, wrap it in `box(width: .., height: .., clip: true, ..)`.
- **Big statement**: `align(center + horizon, text(size: sizes.display, weight: "bold")[..])`.
- **Code highlight**: `show raw.line: it => if it.number == 3 { highlight(it) } else { it }`
  inside a scoped block, or reveal lines with build steps.
- **Simple bars from data**: `rect(width: value / max * 12cm, height: 0.6cm, fill: accent)`
  in a `grid`.
- **Text that must fit one line**: shorten it. Do not shrink the font below the floor in
  the skill's self-check list.

## Pitfalls

- **Fonts**: only Libertinus Serif, New Computer Modern (+ Math) and DejaVu Sans Mono are
  built in. **There is no built-in sans-serif.** For any other font, put the `.ttf`/`.otf`
  files in `fonts/` and set `fonts = ["fonts"]` in slidekit.toml. `system-fonts = true`
  works too, but the deck then builds differently on other machines. An unknown font is
  only a *warning* and Typst falls back silently, so `slidekit preview` failing on it is
  your signal.
- **Overflow**: content taller than the page silently spills onto a new page. That page
  has no slide-info, and `slidekit preview` warns. Content inside a fixed-height box
  overflows visibly instead, and preview warns when text crosses the page or a clip edge.
  Fix it by cutting content or splitting the slide, not by squeezing font sizes.
- **Spacing with big text**: paragraph and block spacing is in `em` of the surrounding
  text, so one huge `text(size: 80pt)[..]` paragraph gets huge gaps. Set absolute spacing
  in the theme (`set par(spacing: 0.55cm)`, `set block(spacing: 0.55cm)`), or use
  `v(.., weak: true)`.
- **Metadata takes a line**: wrap `sk.slide-info(..)`, counters and labels at the top of a
  slide in `place(..)` so they never push the layout down.
- **Build steps**: `make(i)` must produce exactly one `page(..)`. Keep content positions
  stable across steps (`uncover` reserves space; `only` does not and will move things).
- **Labels in stepped slides** occur once per step: attach them only when `i == 1`.
- **Handout vs slides**: anything under `if sk.is-handout` only shows in the handout.
  Preview it with `slidekit preview --handout`.
- **Images**: paths are relative to the file that calls `image(..)`. Use compressed JPEGs
  for photos (about 2000 px wide is plenty) and SVG for logos and diagrams.
- **Packages**: `#import "@preview/name:version"` works (slidekit downloads and caches
  them like the typst CLI). Look up the current version on https://typst.app/universe;
  do not invent one. Useful ones: `cetz` (drawings), `fletcher` (arrow diagrams), `lilaq`
  or `cetz-plot` (charts), `codly` (code blocks).
- **Equations**: `$ .. $` with spaces inside is display math; `$x$` is inline. Math
  inherits the text colour.
