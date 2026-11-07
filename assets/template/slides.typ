// A minimal slidekit deck. Build it with `slidekit build`, iterate with `slidekit serve --open`.
#import "lib/slidekit.typ" as sk

#let accent = rgb("#e5484d")
#let slide-size = (width: 29.7cm, height: 16.7cm)

#set document(title: "My talk")
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

#slide(title: [Main point], speaker: "Ada", minutes: 3, notes: [
  Take your time here.
])[
  - First
  - Second
  - Third
  // Step by step: #slide(steps: 2, ..)[ - First  - #sk.uncover(2)[Second] ]
  // Videos: #sk.video("media/clip.mp4", image("poster.png"), width: 16cm, height: 9cm)
]

#slide(title: [Thanks], speaker: "Bob", minutes: 1, notes: [Questions?])[
  Questions?
]
