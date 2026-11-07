// "night" theme for the showcase deck: dark slides, warm accent, A4 handout.
// Everything here is ordinary Typst; slidekit only sees the metadata from lib/slidekit.typ.
#import "lib/slidekit.typ" as sk

// Palette -------------------------------------------------------------------------------
#let bg = rgb("#0e1120")
#let surface = rgb("#161a2c")
#let rule = rgb("#2a3050")
#let ink = rgb("#ece8e1")
#let muted = rgb("#8c92ad")
#let accent = rgb("#ff8a3d")
#let blue = rgb("#7aa2ff")
#let violet = rgb("#c9a3ff")
#let green = rgb("#b8e07a")
#let paper-ink = rgb("#1b1e2b")

#let speakers = (Mira: accent, Jonas: blue, Ana: violet)
#let speaker-color(name) = speakers.at(name, default: muted)

#let serif = "Libertinus Serif"
#let mono = "DejaVu Sans Mono"

// 16:9; with images = "jpeg" the default ppi turns this into 2880 × 1620 px.
#let W = 33.867cm
#let H = 19.05cm
#let pad-x = 2.4cm

#let slide-no = counter("showcase-slide")
#let current-section = state("showcase-section", none)

// Small pieces ------------------------------------------------------------------------------

/// Upper-case mono label above titles.
#let kicker(body, fill: accent) = text(font: mono, size: 12.5pt, fill: fill, tracking: 0.14em, weight: "bold", upper(body))

/// Coloured dot + speaker name.
#let speaker-chip(name, size: 12pt, fill: muted) = if name != none {
  box(baseline: 0.1em, circle(radius: size * 0.32, fill: speaker-color(name)))
  h(size * 0.4)
  text(font: mono, size: size, fill: fill, name)
}

/// A keyboard key.
#let key(k) = box(
  inset: (x: 0.32em, y: 0.22em), outset: (y: 0.12em), radius: 4pt,
  fill: surface, stroke: (paint: rule, thickness: 1pt),
  text(font: mono, size: 0.72em, fill: ink, weight: "bold", k),
)

/// Rounded panel on a slide.
#let panel(body, fill: surface, stroke: 1pt + rule, inset: 0.7cm, ..args) = block(
  fill: fill, stroke: stroke, radius: 10pt, inset: inset, width: 100%, ..args, body,
)

/// Internal or external link styled as a pill.
#let pill(dest, body, fill: accent) = link(dest, box(
  inset: (x: 0.55em, y: 0.32em), radius: 99pt, stroke: 1.2pt + fill,
  text(font: mono, size: 0.62em, fill: fill, weight: "bold", body),
))

/// Dimming for `sk.focus` on dark slides (the library default is made for light ones).
#let dim-color = rgb("#4c5272")
#let dim(it) = { set text(fill: dim-color); it }

#let two(n) = if n < 10 { "0" + str(n) } else { str(n) }

// The slide frame ------------------------------------------------------------------------

#let footer(speaker, show-agenda) = {
  set text(font: mono, size: 11pt, fill: muted)
  grid(
    columns: (1fr, auto, auto),
    column-gutter: 1.2cm,
    align: horizon,
    [#text(fill: accent, weight: "bold")[slidekit] #h(0.3em) slides as code
      #context {
        let s = current-section.get()
        if s != none [#h(0.6em) / #h(0.6em) #s]
      }],
    speaker-chip(speaker, size: 11pt),
    [
      #if show-agenda { link(<agenda>)[agenda #sym.arrow.t] ; h(1em) }
      #context text(fill: ink)[#two(slide-no.get().first())]#text(fill: rule)[ \/ ]#context two(slide-no.final().first())
    ],
  )
}

/// Draws one slide (without emitting metadata). `layout` is "content", "section" (rings) or "title" (rings, no footer).
#let frame(body, title: none, kicker-text: none, speaker: none, layout: "content", agenda-link: true) = block(
  width: W, height: H, fill: bg, clip: true, {
    set text(font: serif, size: 24pt, fill: ink)
    // Absolute spacing: a paragraph made of one big `text(size: ..)` would otherwise get
    // spacing in *its* em, i.e. huge gaps around display titles.
    set par(leading: 0.62em, spacing: 0.55cm)
    set block(spacing: 0.55cm)
    set list(marker: text(fill: accent, sym.square.filled.small), spacing: 0.75em, body-indent: 0.6em)
    set enum(numbering: n => text(font: mono, fill: accent, size: 0.8em, weight: "bold", two(n)))
    set raw(theme: "night.tmTheme")
    show raw: set text(font: mono, size: 0.68em)
    show raw.where(block: false): box.with(fill: surface, inset: (x: 3pt), outset: (y: 3pt), radius: 3pt)
    show strong: set text(fill: accent)
    show link: set text(fill: blue)

    // A thin gradient bar along the top edge.
    place(top + left, rect(width: 100%, height: 5pt, fill: gradient.linear(accent, violet, blue)))

    if layout in ("section", "title") {
      // Faint concentric rings in the corner.
      for (i, r) in (6cm, 9.5cm, 13cm, 16.5cm).enumerate() {
        place(top + left, dx: W - 7cm - r, dy: H * 0.5 - r,
          circle(radius: r, stroke: (paint: accent.transparentize(70% + i * 7%), thickness: 1.2pt)))
      }
    }

    block(width: 100%, height: 100%, inset: (x: pad-x, top: 1.7cm, bottom: 2.3cm), {
      if kicker-text != none { kicker(kicker-text); v(0.3cm) }
      if title != none and layout == "content" {
        text(size: 40pt, weight: "bold", title)
        v(0.7cm, weak: true)
      }
      body
    })

    if layout != "title" {
      place(bottom + left, dx: pad-x, dy: -0.85cm, box(width: W - 2 * pad-x, footer(speaker, agenda-link)))
    }
  },
)

// Handout entry ----------------------------------------------------------------------------

#let transition-label(t) = if t == auto { "deck default" } else if t == none { "none (cut)" } else if type(t) == str { t } else {
  let d = t.at("duration", default: none)
  if type(d) == duration { d = d.seconds() }
  t.kind + if d != none { " · " + str(d) + " s" }
}

#let handout-entry(rendered, title: none, speaker: none, minutes: none, notes: none, transition: auto, steps: none, advance: none) = block(
  breakable: false, width: 100%, below: 0.75cm, {
    set text(fill: paper-ink)
    grid(
      columns: (auto, 1fr, auto),
      column-gutter: 0.35cm,
      align: horizon,
      box(fill: bg, radius: 3pt, inset: (x: 5pt, y: 3pt),
        context text(font: mono, size: 8.5pt, fill: accent, weight: "bold", two(slide-no.get().first()))),
      text(size: 12.5pt, weight: "bold", if title == none [(untitled)] else { title }),
      text(font: mono, size: 7.5pt, fill: luma(90), {
        if speaker != none { box(baseline: 0.05em, circle(radius: 3pt, fill: speaker-color(speaker))); h(3pt); speaker }
        if minutes != none [ · #minutes min]
        if steps != none [ · #steps steps]
        if advance != none [ · auto #advance s]
        [ · #transition-label(transition)]
      }),
    )
    v(0.2cm)
    grid(
      columns: (8.47cm, 1fr),
      column-gutter: 0.6cm,
      box(radius: 4pt, clip: true, stroke: 0.5pt + luma(180), scale(25%, reflow: true, rendered)),
      {
        set text(size: 9.5pt)
        set list(marker: text(fill: accent, sym.bullet), spacing: 0.5em)
        show raw: set text(font: mono, size: 8.5pt)
        show raw.where(block: false): box.with(fill: luma(238), inset: (x: 2pt), outset: (y: 2pt), radius: 2pt)
        if notes != none { notes } else { text(fill: luma(140), style: "italic")[No notes.] }
      },
    )
  },
)

// Public API -------------------------------------------------------------------------------

/// One slide. `body` is content, or `i => content` for a slide with `steps` build steps.
/// `anchor` is a label for internal links; it is placed on the first step only, so a stepped
/// slide still has exactly one target.
/// In handout mode the slide becomes one entry (at its final step) with notes.
#let slide(
  title: none, kicker: none, speaker: none, minutes: none, notes: none,
  transition: auto, advance: none, steps: none, anchor: none,
  layout: "content", agenda-link: true, body,
) = {
  let content(i) = if type(body) == function { body(i) } else { body }
  let anchor-mark = if anchor != none [#metadata("anchor")#anchor]
  let draw(i) = frame(content(i), title: title, kicker-text: kicker, speaker: speaker, layout: layout, agenda-link: agenda-link)
  let info(step) = sk.slide-info(
    title: title, speaker: speaker, minutes: minutes, notes: notes,
    transition: transition, advance: advance, step: step,
  )

  if sk.is-handout {
    slide-no.step()
    handout-entry(
      { place({ info(none); anchor-mark }); if steps == none { draw(none) } else { sk.at-step(steps, draw(steps)) } },
      title: title, speaker: speaker, minutes: minutes, notes: notes,
      transition: transition, steps: steps, advance: advance,
    )
  } else if steps == none {
    // The metadata goes into a `place` so it cannot take up a line above the frame.
    page({ place({ slide-no.step(); info(none); anchor-mark }); draw(none) })
  } else {
    sk.stepped(steps, i => page({
      place({
        if i == 1 { slide-no.step(); anchor-mark }
        info(i)
      })
      draw(i)
    }))
  }
}

/// A section divider: big number, title and one line of subtitle.
#let section(number, title, subtitle, anchor: none, speaker: none, minutes: 0.5, notes: none, transition: "zoom") = {
  current-section.update(title)
  slide(
    speaker: speaker, minutes: minutes, notes: notes, transition: transition, anchor: anchor,
    title: title, layout: "section",
  )[
    #set align(horizon)
    #v(1fr)
    #text(font: mono, size: 15pt, fill: accent, weight: "bold", tracking: 0.2em)[SECTION #two(number)]
    #v(0.5cm)
    #text(size: 76pt, weight: "bold", title)
    #v(0.7cm)
    #text(size: 26pt, fill: muted, style: "italic", subtitle)
    #v(1fr)
  ]
}

/// Document setup for both modes.
#let deck(title: none, body) = {
  set document(title: title)
  set text(font: serif, lang: "en")
  if sk.is-handout {
    set page(
      paper: "a4",
      margin: (x: 1.8cm, top: 2.3cm, bottom: 2cm),
      header: {
        set text(font: mono, size: 8pt, fill: luma(110))
        grid(columns: (1fr, auto), text(fill: accent, weight: "bold", title), [Speaker handout])
        v(-0.3em)
        line(length: 100%, stroke: 0.4pt + luma(200))
      },
      footer: context {
        set text(font: mono, size: 8pt, fill: luma(110))
        align(right, counter(page).display("1 / 1", both: true))
      },
    )
    set text(size: 10.5pt, fill: paper-ink)
    body
  } else {
    set page(width: W, height: H, margin: 0pt, fill: bg)
    body
  }
}
