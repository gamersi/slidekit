// slidekit showcase: a talk about slidekit, built with slidekit.
//   slidekit build examples/showcase        → dist/slides.pdf, dist/handout.pdf, dist/index.html
//   slidekit serve examples/showcase --open → live player
#import "lib/slidekit.typ" as sk
#import "theme.typ": *

#show: deck.with(title: "slidekit: slides as code")

// Handout front matter --------------------------------------------------------------------

#if sk.is-handout {
  text(font: mono, size: 9pt, fill: accent, weight: "bold", tracking: 0.14em)[SPEAKER HANDOUT]
  v(0.1cm)
  text(size: 28pt, weight: "bold")[slidekit: slides as code]
  v(0.05cm)
  text(size: 13pt, style: "italic", fill: luma(90))[One Typst file → slides PDF, this handout and an offline HTML player.]
  v(0.4cm)
  context {
    let all = sk.slides()
    let total = all.map(s => if s.minutes == none { 0 } else { s.minutes }).sum()
    grid(
      columns: (1fr, 1fr, 1fr, 1fr),
      column-gutter: 0.4cm,
      ..(
        ([Slides], [#all.len()]),
        ([Planned], [#total min]),
        ([Speakers], [#speakers.len()]),
        ([Videos], [#query(<slidekit-video>).len()]),
      ).map(((k, v)) => block(width: 100%, fill: luma(245), radius: 4pt, inset: 8pt)[
        #text(font: mono, size: 7.5pt, fill: luma(110), upper(k)) \
        #text(size: 16pt, weight: "bold", v)
      ]),
    )
  }
  v(0.4cm)
  text(font: mono, size: 9pt, fill: accent, weight: "bold", tracking: 0.14em)[RUN SHEET]
  v(-0.2cm)
  {
    set text(size: 9.5pt)
    show table.cell.where(y: 0): set text(font: mono, size: 8pt)
    context sk.run-sheet(stroke: 0.5pt + luma(190))
  }
  v(0.3cm)
  text(size: 9pt, fill: luma(90))[Speaker order: ] + speakers.keys().map(n => {
    box(baseline: 0.05em, circle(radius: 3pt, fill: speaker-color(n))); h(3pt); n
  }).join([ #sym.arrow.r ])
  pagebreak()
}

// 1 · Title ------------------------------------------------------------------------------

#slide(
  speaker: "Mira", minutes: 1, transition: "fade", anchor: <s-title>, layout: "title", agenda-link: false,
  title: [slidekit: slides as code],
  notes: [
    - Welcome. Three of us today: *Mira*, *Jonas* and *Ana*.
    - This deck _is_ the demo: every slide shows one slidekit feature and these notes say how
      to try it.
    - Press `P` now to open the presenter view in its own window; it shows these notes.
  ],
)[
  #v(1fr)
  #kicker[a talk in 18 slides · built with itself]
  #v(0.5cm)
  #text(size: 120pt, weight: "bold")[slidekit]
  #v(0.5cm)
  #text(size: 46pt, style: "italic", fill: accent)[slides as code]
  #v(0.9cm)
  #block(width: 60%, text(size: 22pt, fill: muted)[
    Write the talk in Typst. Get a PDF, a speaker handout and an offline HTML player
    with videos, build steps, transitions and a presenter view.
  ])
  #v(1fr)
  #for n in speakers.keys() { speaker-chip(n, size: 15pt, fill: ink); h(1.2cm) }
]

// 2 · Agenda ------------------------------------------------------------------------------

#let agenda-item(n, title, sub, target) = link(target, block(
  width: 100%, inset: (y: 0.42cm), stroke: (bottom: 1pt + rule),
  grid(
    columns: (2.2cm, 1fr, auto),
    align: (left + horizon, left + horizon, right + horizon),
    text(font: mono, size: 26pt, fill: accent, weight: "bold", two(n)),
    [#text(size: 30pt, weight: "bold", fill: ink, title) #h(0.5em) #text(size: 19pt, fill: muted, style: "italic", sub)],
    text(font: mono, size: 13pt, fill: muted)[slide #context two(slide-no.at(target).first()) #h(0.3em) #text(fill: accent, sym.arrow.r)],
  ),
))

#slide(
  title: [Agenda], kicker: [click a line to jump], speaker: "Mira", minutes: 1,
  transition: "wipe", anchor: <agenda>, agenda-link: false,
  notes: [
    - Every line is an *internal link* (`link(<sec-why>)[..]`): click it in the player and it
      jumps straight to that section. It works in the PDF, too.
    - Every later slide has a small `agenda ↑` link in its footer that comes back here.
    - This slide enters with the *wipe* transition.
  ],
)[
  #agenda-item(1, [Why slides as code], [one source, many outputs], <sec-why>)
  #agenda-item(2, [Motion], [transitions and build steps], <sec-motion>)
  #agenda-item(3, [Media & links], [video, ambient clips, links], <sec-media>)
  #agenda-item(4, [On stage], [player, presenter view, handout], <sec-stage>)
]

// 3 · Section 01 --------------------------------------------------------------------------

#section(1, [Why slides as code], [Your talk is a program. Treat it like one.], anchor: <sec-why>, speaker: "Mira", notes: [
  Section dividers enter with *zoom*. Diffable, reviewable, versioned: slides live next to the
  code they talk about.
])

// 4 · One source, three outputs (diagram, 3 steps) ----------------------------------------------

#let node(title, sub, color, on: true) = block(
  width: 100%, fill: surface, radius: 8pt, inset: (x: 0.6cm, y: 0.55cm),
  stroke: (left: 5pt + if on { color } else { dim-color }, rest: 1pt + rule),
)[
  #text(font: mono, size: 21pt, fill: if on { color } else { dim-color }, weight: "bold", title)
  #v(0.2cm, weak: true)
  #text(size: 21pt, fill: if on { muted } else { dim-color }, sub)
]
#let arrow = text(font: mono, size: 34pt, fill: muted, sym.arrow.r)

#slide(
  title: [One source, three outputs], kicker: [build steps · a diagram that grows],
  speaker: "Mira", minutes: 2, steps: 3,
  notes: [
    This slide has *3 build steps*: press `→` to reveal the next part of the diagram. The
    player counts it as one slide, so the slide number does not change.
    + Source: one `slides.typ` plus the vendored `lib/slidekit.typ`.
    + slidekit compiles it twice, in `slides` and `handout` mode, in one process.
    + Out come PDF, handout and the player (slide images + `index.html`).
    The handout shows this slide once, at its last step.
  ],
i => {
  v(1fr)
  grid(
    columns: (1fr, auto, 1fr, auto, 1fr),
    column-gutter: 0.55cm,
    align: horizon,
    // `sk.shows` answers "is step i in this range?", so colours can follow the steps too.
    stack(spacing: 0.45cm,
      node([slides.typ], [your talk, theme & notes], accent, on: sk.shows("1, 3", i)),
      node([lib/slidekit.typ], [the contract], accent, on: sk.shows("1, 3", i)),
    ),
    sk.uncover(2, arrow),
    sk.uncover(2, stack(spacing: 0.45cm,
      node([mode = slides], [one page per step], blue),
      node([mode = handout], [your handout design], blue),
    )),
    sk.uncover(3, arrow),
    sk.uncover(3, stack(spacing: 0.45cm,
      node([slides.pdf], [to share], green),
      node([handout.pdf], [for the speakers], green),
      node([index.html], [offline player], green),
    )),
  )
  v(1fr)
  set text(size: 22pt, fill: muted, style: "italic")
  sk.only("1")[You write one `.typ` file, styled however you like.]
  sk.only("2")[slidekit compiles it twice, sharing one typst session: rebuilds take milliseconds.]
  sk.only("3")[… and writes everything a talk needs into `dist/`. Copy it onto a USB stick.]
})

// 5 · Typst does the heavy lifting: math + code --------------------------------------------

#slide(
  title: [Typst does the heavy lifting], kicker: [math · code · syntax highlighting],
  speaker: "Mira", minutes: 2, transition: "slide",
  notes: [
    - Anything Typst can typeset works on a slide: *math*, *code*, tables, drawings.
    - The left formula is real: a 33.867 cm slide at the default `ppi = 216` becomes a 2880 px
      wide image in the player.
    - The code colours come from a small `night.tmTheme` next to the deck (`set raw(theme: ..)`).
    - Enters with *slide* at the deck's default duration.
  ],
)[
  #grid(
    columns: (1fr, 1.15fr),
    column-gutter: 1.2cm,
    [
      #kicker(fill: muted)[pixels per slide]
      #v(0.2cm)
      $ w_"px" = (33.867 "cm") / (2.54 "cm/in") dot 216 "ppi" approx 2880 $
      #v(0.5cm)
      #kicker(fill: muted)[or anything else]
      #v(0.2cm)
      $ integral_(-oo)^oo e^(-x^2) dif x = sqrt(pi) quad quad
        sum_(k=1)^n k = (n(n+1)) / 2 $
      #v(0.5cm)
      #text(size: 20pt, fill: muted)[Typeset by Typst, rendered once to JPEG for the player,
        kept as vectors in the PDF.]
    ],
    panel(inset: 0.6cm)[
      ```typ
      #import "lib/slidekit.typ" as sk

      #sk.stepped(2, i => page[
        #sk.slide-info(
          title: [Hello], speaker: "Mira",
          minutes: 2, step: i,
          transition: (kind: "slide", duration: 0.8),
        )
        = Hello
        #sk.uncover(2)[world]
      ])
      ```
    ],
  )
]

// 6 · The deck knows itself: table + chart ------------------------------------------------------

#slide(
  title: [The deck knows itself], kicker: [tables · charts from the deck's own metadata],
  speaker: "Mira", minutes: 2,
  notes: [
    - The chart on the right is *not* typed in: it is computed at compile time with
      `sk.slides()`, which reads every slide's `minutes` and `speaker`.
    - Change a `minutes:` value, save, and `slidekit serve` redraws it live.
    - *Hand-over:* next up is *Jonas*. The presenter view highlights the speaker change.
  ],
)[
  #grid(
    columns: (1.1fr, 1fr),
    column-gutter: 1.4cm,
    [
      #kicker(fill: muted)[what `dist/` contains]
      #v(0.3cm)
      #set text(size: 22pt)
      #table(
        columns: (auto, 1fr),
        stroke: none,
        inset: (x: 0.3cm, y: 0.32cm),
        table.hline(stroke: 1pt + rule),
        table.header(text(font: mono, size: 13pt, fill: accent)[FILE], text(font: mono, size: 13pt, fill: accent)[WHAT IT IS]),
        table.hline(stroke: 1pt + rule),
        `slides.pdf`, [the deck, videos as links],
        `handout.pdf`, [notes, previews, run sheet],
        `index.html`, [offline player, no server],
        `slides/*.jpg`, [one image per page],
        `media/*`, [your videos, copied over],
        table.hline(stroke: 1pt + rule),
      )
    ],
    [
      #kicker(fill: muted)[planned minutes per speaker]
      #v(0.5cm)
      #context {
        let per = (:)
        for s in sk.slides() {
          let who = if s.speaker == none { "–" } else { s.speaker }
          per.insert(who, per.at(who, default: 0) + if s.minutes == none { 0 } else { s.minutes })
        }
        let top = calc.max(..per.values())
        for (who, m) in per {
          grid(
            columns: (2.6cm, 1fr),
            align: horizon,
            text(font: mono, size: 15pt, who),
            box(width: 100%, {
              box(width: 100% * (m / top) * 0.8, height: 0.95cm, radius: (right: 5pt), fill: speaker-color(who))
              h(0.3cm)
              text(font: mono, size: 15pt, fill: ink, weight: "bold")[#m]
            }),
          )
          v(0.35cm)
        }
        v(0.3cm)
        text(size: 18pt, fill: muted, style: "italic")[Total: #per.values().sum() minutes, from `minutes:` on each slide.]
      }
    ],
  )
]

// 7 · Section 02 --------------------------------------------------------------------------

#section(2, [Motion], [Transitions between slides, steps within them.], anchor: <sec-motion>, speaker: "Jonas",
  transition: (kind: "slide", duration: 0.8),
  notes: [
    - *Jonas* takes over here.
    - This divider uses `transition: (kind: "slide", duration: 0.8)`: a slower push than the
      deck default. Go back with `←` and forward again to watch it; going back plays it reversed.
  ],
)

// 8 · Transitions -------------------------------------------------------------------------

#let transition-card(kind, desc, target, color) = panel(inset: 0.5cm, height: 100%)[
  #text(font: mono, size: 20pt, fill: color, weight: "bold", kind)
  #v(0.15cm, weak: true)
  #text(size: 18pt, fill: muted, desc)
  #v(1fr)
  #pill(target, fill: color)[see slide #context two(slide-no.at(target).first()) #sym.arrow.r]
]

#slide(
  title: [Six ways in], kicker: [transitions · per slide or deck-wide],
  speaker: "Jonas", minutes: 2, transition: "cover", anchor: <transitions>,
  notes: [
    - Each card links to a slide that *uses* that transition: click a pill to jump there, then
      use the footer's `agenda ↑` or `←` to come back.
    - The deck default lives in `slidekit.toml`: `[player] transition = "fade"`.
    - Per slide: `sk.slide-info(transition: "zoom")`, `(kind: "slide", duration: 0.8)`,
      a `duration` like `800ms`, or `none` for a hard cut.
    - This slide itself came in with *cover*.
  ],
)[
  #grid(
    columns: (1fr, 1fr, 1fr),
    rows: (5.6cm, 5.6cm),
    gutter: 0.55cm,
    transition-card("fade", [Cross-dissolve. The deck default from `slidekit.toml`.], <s-title>, accent),
    transition-card("slide", [Pushes the old slide out, here with `duration: 0.8`.], <sec-motion>, blue),
    transition-card("cover", [The new slide slides in over the old one, like this one.], <transitions>, violet),
    transition-card("zoom", [Grows in from the centre. Every section divider.], <sec-why>, green),
    transition-card("wipe", [Reveals the new slide from left to right.], <agenda>, accent),
    transition-card("none", [A hard cut, for things that should just _be there_.], <keys>, blue),
  )
]

// 9 · Build steps: uncover + focus ----------------------------------------------------------

#slide(
  title: [Build steps], kicker: [uncover · focus · only],
  speaker: "Jonas", minutes: 2, steps: 5,
  notes: [
    Five steps on one slide. Press `→` to step forward, `←` to step back.
    - Each point appears with `sk.uncover(k)`, which keeps its space so nothing jumps.
    - `sk.focus("k, 5")` dims the earlier points while you talk about the current one; in the
      last step everything lights up again.
    - The dots on the right are drawn from the step number `i` itself.
  ],
i => grid(
  columns: (1.35fr, 1fr),
  column-gutter: 1.2cm,
  {
    set text(size: 23pt)
    let items = (
      [`stepped(n, make)` emits the page _n_ times, one per step.],
      [`uncover(2)` hides a part until step 2 but keeps its space.],
      [`focus("3")` dims everything except the point you are on.],
      [`only("4")` exists in step 4 only and takes no space otherwise.],
    )
    for (k, it) in items.enumerate(start: 1) {
      sk.uncover(k, sk.focus(str(k) + ", 5", dim: dim, block(below: 0.55cm, grid(
        columns: (1.3cm, 1fr),
        text(font: mono, size: 18pt, fill: accent, weight: "bold", two(k)),
        it,
      ))))
    }
    v(0.3cm)
    sk.uncover(5, panel(fill: accent.transparentize(88%), stroke: 1pt + accent, inset: 0.45cm,
      text(size: 20pt)[The player counts all five as *one* slide: one number, no transition between steps.]))
  },
  panel(inset: 0.6cm)[
    #kicker(fill: muted)[step]
    #v(0.1cm)
    #text(size: 96pt, weight: "bold", fill: accent)[#i]#text(size: 40pt, fill: muted)[ \/ 5]
    #v(0.2cm)
    #for k in range(1, 6) {
      box(circle(radius: 0.32cm, fill: if k <= i { accent } else { none }, stroke: 1.5pt + if k <= i { accent } else { rule }))
      h(0.3cm)
    }
    #v(0.6cm)
    ```typ
    #sk.stepped(5, i => page[
      #sk.slide-info(step: i, ..)
      #sk.uncover(2)[..]
      #sk.focus("3")[..]
      #sk.only("4")[..]
    ])
    ```
  ],
))

// 10 · only: alternation --------------------------------------------------------------------

#let quiz-card(label, body, color) = panel(fill: color.transparentize(88%), stroke: 1.5pt + color, inset: 1cm, height: 10.5cm)[
  #text(font: mono, size: 14pt, fill: color, weight: "bold", label)
  #v(0.3cm)
  #body
]

#slide(
  title: [Swap, don't stack], kicker: [only · alternatives in the same spot],
  speaker: "Jonas", minutes: 1.5, steps: 3,
  notes: [
    - Three steps. The big card is *three different cards*, each wrapped in `sk.only("n")`, so
      they replace each other in the same place.
    - The step list on the right uses `sk.focus("n")` to show which `only` is active.
    - *Hand-over:* after this, *Ana* takes section 3.
  ],
i => grid(
  columns: (1.6fr, 1fr),
  column-gutter: 1.2cm,
  {
    set text(size: 36pt)
    sk.only("1", quiz-card([QUESTION], [How often does slidekit compile this deck per build?], blue))
    sk.only("2", quiz-card([ANSWER], [*Twice*: once as slides, once as the handout, concurrently.], accent))
    sk.only("3", quiz-card([AND THEN], [Only pages whose *pixels* changed are rendered again.], green))
  },
  {
    set text(font: mono, size: 17pt)
    v(0.5cm)
    for (k, label, color) in ((1, "only(\"1\")", blue), (2, "only(\"2\")", accent), (3, "only(\"3\")", green)) {
      sk.focus(str(k), dim: dim, block(below: 0.6cm, {
        box(width: 0.6cm, height: 0.6cm, radius: 3pt, fill: color, baseline: 0.12cm)
        h(0.4cm)
        label
      }))
    }
    v(0.4cm)
    text(font: serif, size: 19pt, fill: muted, style: "italic")[`uncover` reveals; `only` swaps.]
  },
))

// 11 · Section 03 -------------------------------------------------------------------------

#section(3, [Media & links], [Real video in the player, links that go somewhere.], anchor: <sec-media>, speaker: "Ana", notes: [
  *Ana* takes over. Zoom transition again, like every divider.
])

// 12 · Two videos -------------------------------------------------------------------------

#let clip(file, poster, caption) = [
  #block(radius: 8pt, clip: true, stroke: 1pt + rule,
    sk.video(file, image(poster, width: 100%, height: 100%, fit: "cover"), width: 100%, height: 7.9cm))
  #v(0.15cm, weak: true)
  #text(font: mono, size: 14pt, fill: muted, caption)
]

#slide(
  title: [Two clips, one slide], kicker: [video · plays in place],
  speaker: "Ana", minutes: 2,
  notes: [
    - Each area is `sk.video(file, poster)`. In the PDF the poster is a link to the file; in
      the player the video plays exactly in that rectangle.
    - Press `V`: the *left* clip plays. Press `V` again when it ends (or to pause) and the
      *right* clip is next. Or just click a clip.
    - The left clip has a quiet tone; the right one is silent.
  ],
)[
  #grid(
    columns: (1fr, 1fr),
    column-gutter: 1cm,
    clip("media/mandelbrot.mp4", "media/mandelbrot.jpg")[mandelbrot.mp4 · 5 s · with sound],
    clip("media/life.mp4", "media/life.jpg")[life.mp4 · 6 s · silent],
  )
  #v(1fr)
  #text(size: 20pt, fill: muted)[Press #key[V] for the first, #key[V] again for the second, or click either clip.]
]

// 13 · Ambient video ----------------------------------------------------------------------

#slide(
  title: [Ambient clips], kicker: [autoplay · loop · muted · no controls],
  speaker: "Ana", minutes: 1,
  notes: [
    - The panel on the right starts by itself, loops, has no sound and no controls:
      `sk.video(.., autoplay: true, loop: true, muted: true, controls: false)`.
    - Browsers only allow autoplay with sound after a key press, so ambient clips are always `muted`.
    - Good for a moving background, a looping demo, or an animated diagram.
  ],
)[
  #grid(
    columns: (1fr, 1.25fr),
    column-gutter: 1.2cm,
    [
      #set text(size: 22pt)
      #v(0.3cm)
      Like a GIF, but a real video file: small, sharp and smooth.
      #v(0.5cm)
      ```typ
      #sk.video(
        "media/ambient.mp4",
        image("media/ambient.jpg"),
        autoplay: true, loop: true,
        muted: true, controls: false,
      )
      ```
    ],
    block(radius: 12pt, clip: true, stroke: 1pt + rule, sk.video(
      "media/ambient.mp4", image("media/ambient.jpg", width: 100%, height: 100%, fit: "cover"),
      width: 100%, height: 9.9cm, autoplay: true, loop: true, muted: true, controls: false,
    )),
  )
]

// 14 · Section 04 -------------------------------------------------------------------------

#section(4, [On stage], [The player, the presenter view and the handout.], anchor: <sec-stage>, speaker: "Ana", notes: [
  Last section. From here on, everything is about *presenting*.
])

// 15 · Player keys ------------------------------------------------------------------------

#let key-row(keys, what) = (
  align(right, keys.map(key).join(h(0.2cm))),
  text(size: 21pt, fill: muted, what),
)

#slide(
  title: [The player at your fingertips], kicker: [keyboard · try them now],
  speaker: "Ana", minutes: 2, transition: none, anchor: <keys>,
  notes: [
    This slide arrives with a *hard cut* (`transition: none`). Try the keys while it is up:
    - `O` for the overview grid, `B`/`W` to black or white out the screen.
    - `L` turns the mouse into a laser pointer; `D` draws, `C` clears the drawing.
    - Type `3` then `Enter` to jump to slide 3; `Esc` leaves any mode.
  ],
)[
  #set text(size: 28pt)
  #v(0.5cm)
  #grid(
    columns: (auto, 1fr, auto, 1fr, auto, 1fr),
    column-gutter: (0.45cm, 1cm, 0.45cm, 1cm, 0.45cm),
    row-gutter: 1.05cm,
    align: horizon,
    ..key-row(([→], [Space]), [next]),
    ..key-row(([←],), [back]),
    ..key-row(([3], [Enter]), [go to slide 3]),
    ..key-row(([O],), [overview]),
    ..key-row(([B], [W]), [black / white]),
    ..key-row(([F],), [fullscreen]),
    ..key-row(([L],), [laser pointer]),
    ..key-row(([D],), [draw]),
    ..key-row(([C],), [clear drawing]),
    ..key-row(([V],), [play video]),
    ..key-row(([P],), [presenter]),
    ..key-row(([T],), [timer]),
    ..key-row(([Esc],), [leave a mode]),
    ..key-row(([Home], [End]), [first / last]),
    ..key-row(([H], [?]), [help]),
  )
]

// 16 · Presenter view & handout ----------------------------------------------------------------

#slide(
  title: [Two views for the speakers], kicker: [presenter view · handout],
  speaker: "Ana", minutes: 1.5,
  notes: [
    - Press `P`: the presenter view opens in its own window and drives this one, even from
      `file://`.
    - Open `dist/handout.pdf`: it is this deck compiled with `slidekit-mode = "handout"`,
      with a run sheet up front and each stepped slide shown *once*.
    - *Hand-over:* back to *Mira* for the wrap-up.
  ],
)[
  #grid(
    columns: (1fr, 1fr),
    column-gutter: 0.9cm,
    panel(height: 10.6cm, inset: 0.9cm)[
      #text(font: mono, size: 15pt, fill: accent, weight: "bold")[PRESENTER VIEW #h(0.3em) #key[P]]
      #v(0.35cm)
      #set text(size: 23pt)
      #set list(spacing: 0.9em)
      - current and next slide
      - notes with *bold*, lists and `code`
      - who speaks next, highlighted at a hand-over
      - time against plan, per slide and total
    ],
    panel(height: 10.6cm, inset: 0.9cm)[
      #text(font: mono, size: 15pt, fill: blue, weight: "bold")[HANDOUT.PDF]
      #v(0.35cm)
      #set text(size: 23pt)
      #set list(spacing: 0.9em)
      - the same source, `slidekit-mode = "handout"`
      - run sheet with minutes per speaker
      - each slide once, at its last build step
      - its look is up to your deck
    ],
  )
]

// 17 · Intermission (auto-advance) -------------------------------------------------------------

#slide(
  speaker: "Mira", minutes: 0.5, advance: 5, transition: (kind: "fade", duration: duration(seconds: 1)),
  layout: "section", title: [Questions in five],
  notes: [
    - `advance: 5`: the player moves on by itself after five seconds. Just wait.
    - It also enters with a slow fade: `transition: (kind: "fade", duration: duration(seconds: 1))`.
    - Deck-wide auto-advance (kiosk mode) is `[player] advance = 8` plus `loop = true`.
  ],
)[
  #set align(horizon)
  #v(1fr)
  #kicker[auto-advance · advance: 5]
  #v(0.4cm)
  #text(size: 76pt, weight: "bold")[Questions in five]
  #v(1cm)
  #for k in range(5) {
    box(rect(width: 2.2cm, height: 0.35cm, radius: 99pt, fill: accent.transparentize(k * 18%)))
    h(0.35cm)
  }
  #v(0.4cm)
  #text(size: 24pt, fill: muted, style: "italic")[No key needed: this slide moves on by itself.]
  #v(1fr)
]

// 18 · Thanks -----------------------------------------------------------------------------

#slide(
  title: [Questions?], speaker: "Mira", minutes: 2, layout: "title",
  notes: [
    - The pills are *external links*: in the player they open in a new tab.
    - `back to agenda` and `transitions` are *internal links* to labelled slides.
    - Thank the audience, then questions.
  ],
)[
  #v(1fr)
  #kicker[thank you]
  #v(0.3cm)
  #text(size: 96pt, weight: "bold")[Questions?]
  #v(0.9cm)
  #text(size: 26pt, fill: muted)[This deck is `examples/showcase` in the slidekit repository.]
  #v(1.2cm)
  #set text(size: 30pt)
  #pill("https://typst.app")[typst.app #sym.arrow.tr]
  #h(0.4cm)
  #pill("https://github.com/gamersi/slidekit", fill: blue)[github.com/gamersi/slidekit #sym.arrow.tr]
  #v(0.5cm)
  #pill(<agenda>, fill: violet)[#sym.arrow.l back to agenda]
  #h(0.4cm)
  #pill(<transitions>, fill: green)[#sym.arrow.l transitions]
  #v(1fr)
  #for n in speakers.keys() { speaker-chip(n, size: 15pt, fill: ink); h(1.2cm) }
]
