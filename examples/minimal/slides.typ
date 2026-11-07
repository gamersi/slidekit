// A minimal slidekit deck. Build it with `slidekit build`, iterate with `slidekit serve --open`.
#import "lib/slidekit.typ" as sk

#let accent = rgb("#e5484d")
#let slide-size = (width: 29.7cm, height: 16.7cm)

#set document(title: "slidekit minimal example")
#set text(font: "Libertinus Serif", size: if sk.is-handout { 11pt } else { 24pt })
#set page(
  ..if sk.is-handout { (paper: "a4", margin: 2cm) } else { (..slide-size, margin: 1.6cm) },
)
#show heading: set text(fill: accent)

/// One slide. In handout mode it becomes a small preview with the speaker notes.
/// `steps: n` shows it in n build steps (use `sk.uncover(2)[..]`, `sk.only(3)[..]` inside);
/// `transition` is how it enters (see slidekit.toml for the default).
#let slide(title: none, speaker: none, minutes: none, notes: none, steps: 1, transition: auto, body) = {
  let info(step) = sk.slide-info(
    title: title, speaker: speaker, minutes: minutes, notes: notes, transition: transition, step: step,
  )
  let content = {
    if title != none { heading(title) }
    body
  }
  if sk.is-handout {
    block(breakable: false, below: 1.5em, {
      info(none)
      if title != none { heading(level: 2, title) }
      text(fill: gray)[#speaker · #minutes min]
      grid(
        columns: (40%, 1fr),
        gutter: 1em,
        box(stroke: 0.5pt + gray, scale(30%, reflow: true, box(..slide-size, inset: 1.6cm, {
          set text(24pt)
          content
        }))),
        notes,
      )
    })
  } else {
    sk.stepped(steps, i => page(info(i) + content))
  }
}

#if sk.is-handout [
  = Speaker handout
  #context sk.run-sheet()
]

#slide(title: [Hello], speaker: "Ada", minutes: 1, notes: [
  - Welcome everyone
  - Say what the talk is about
])[
  A talk built with *Typst* and *slidekit*.
]

#slide(title: [Main point], speaker: "Ada", minutes: 3, steps: 3, transition: "slide", notes: [
  Take your time here. The points appear one by one.
])[
  - First
  - #sk.uncover(2)[Second]
  - #sk.uncover(3)[Third]
]

#slide(title: [A video], speaker: "Bob", minutes: 2, notes: [
  Click the video area (or press *V*) to play it. In the PDF it is a link to `media/clip.mp4`.
])[
  #align(center, sk.video("media/clip.mp4", width: 16cm, height: 9cm, rect(
    width: 100%, height: 100%, fill: luma(30), radius: 4pt,
    align(center + horizon, text(fill: white)[▶ media/clip.mp4]),
  )))
]

#slide(title: [Thanks], speaker: "Bob", minutes: 1, notes: [Questions?])[
  Questions?
]
