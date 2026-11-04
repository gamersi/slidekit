// slidekit.typ · deck-side half of slidekit (https://github.com/gamersi/slidekit), contract v1
//
// Decks import this file and stay free in how they look. The `slidekit` CLI only relies on
// the contract below; everything else (theme, layout, handout design) belongs to the deck.
//
//   sys.inputs."slidekit-mode"  "slides" (default) or "handout"
//   <slidekit-slide>            one metadata per slide page:
//                                 (page: int, title: content|none, speaker: str|none,
//                                  minutes: float|none, notes: content|none)
//   <slidekit-video>            one metadata per video area:
//                                 (page: int, file: str, x, y, w, h: float)
//                               file is relative to the deck root; x/y/w/h are in pt,
//                               measured from the top-left corner of the page.
//
// A plain `typst compile` of a deck still works: links point at the video files and the
// metadata is simply ignored.

/// "slides" or "handout"; set by the CLI, defaults to "slides".
#let mode = sys.inputs.at("slidekit-mode", default: "slides")
#let is-handout = mode == "handout"

/// Attach information to the current slide. Call it once anywhere on the slide's page.
#let slide-info(title: none, speaker: none, minutes: none, notes: none) = context [
  #metadata((page: here().page(), title: title, speaker: speaker, minutes: minutes, notes: notes)) <slidekit-slide>
]

/// An area that plays `file` (relative to the deck root). `body` is what the PDF shows,
/// usually a poster frame; the HTML player plays the video in exactly this area.
#let video(file, body, width: 100%, height: 100%) = box(width: width, height: height, {
  place(top + left, layout(size => context {
    let p = here().position()
    [#metadata((page: p.page, file: file, x: p.x / 1pt, y: p.y / 1pt, w: size.width / 1pt, h: size.height / 1pt)) <slidekit-video>]
  }))
  link(file, box(width: 100%, height: 100%, body))
})

/// All slide infos in document order. Needs `context`.
#let slides() = query(<slidekit-slide>).map(it => it.value)

/// Running order: slide, title, speaker, planned minutes and running total. Needs `context`.
#let run-sheet(stroke: 0.5pt + gray, labels: (title: "Slide", speaker: "Speaker", per-speaker: "Per speaker")) = {
  let rows = ()
  let total = 0
  let per = (:)
  for (i, s) in slides().enumerate() {
    let m = if s.minutes == none { 0 } else { s.minutes }
    total += m
    let who = if s.speaker == none { "–" } else { s.speaker }
    per.insert(who, per.at(who, default: 0) + m)
    rows += ([#(i + 1)], if s.title == none [] else { s.title }, who, [#m], [#total])
  }
  table(
    columns: (auto, 1fr, auto, auto, auto), stroke: none, inset: (x: 5pt, y: 2.6pt),
    align: (right, left, left, right, right),
    table.hline(stroke: stroke),
    table.header([*\#*], strong(labels.title), strong(labels.speaker), [*min*], [*Σ*]),
    table.hline(stroke: stroke),
    ..rows,
    table.hline(stroke: stroke),
  )
  [#strong(labels.per-speaker): #per.pairs().map(((k, v)) => [#k #v min]).join([ · ])]
}
