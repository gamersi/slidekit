// slidekit.typ · deck-side half of slidekit (https://github.com/gamersi/slidekit), contract v2
//
// Decks import this file and stay free in how they look. The `slidekit` CLI only relies on
// the contract below; everything else (theme, layout, handout design) belongs to the deck.
//
//   sys.inputs."slidekit-mode"  "slides" (default) or "handout"
//   <slidekit-slide>            one metadata per slide page:
//                                 (page: int, title: content|none, speaker: str|none,
//                                  minutes: float|none, notes: content|none,
//                                  transition: (kind: str, duration: float|none)|none,
//                                  advance: float|none, step: int|none, contract: int)
//                               a page with step > 1 continues the slide on the page before
//                               (a build step): same slide number, no slide transition.
//   <slidekit-video>            one metadata per video area:
//                                 (page: int, file: str, x, y, w, h: float,
//                                  autoplay, loop, muted, controls: bool)
//                               file is relative to the deck root; x/y/w/h are in pt,
//                               measured from the top-left corner of the page.
//
// Links (`link(..)`) on a slide become clickable areas in the HTML player: URLs open in a
// new tab, links to a label or location jump to that slide.
//
// A plain `typst compile` of a deck still works: links point at the video files and the
// metadata is simply ignored.

/// The contract version this library speaks; the CLI warns when it differs from its own.
#let contract = 2

/// "slides" or "handout"; set by the CLI, defaults to "slides".
#let mode = sys.inputs.at("slidekit-mode", default: "slides")
#let is-handout = mode == "handout"

/// Transition kinds the player knows.
#let transitions = ("none", "fade", "slide", "cover", "zoom", "wipe")

#let _transition(t) = {
  if t == auto { return none }
  if t == none { return (kind: "none", duration: none) }
  let (kind, duration) = if type(t) == str { (t, none) } else {
    assert(type(t) == dictionary, message: "transition must be a string or (kind: .., duration: ..)")
    (t.at("kind"), t.at("duration", default: none))
  }
  assert(kind in transitions, message: "unknown transition `" + kind + "`, expected one of " + transitions.join(", "))
  if type(duration) == std.duration { duration = duration.seconds() }
  assert(duration == none or type(duration) in (int, float), message: "transition duration must be seconds or a duration")
  (kind: kind, duration: duration)
}

/// Attach information to the current slide. Call it once anywhere on the slide's page.
///
/// - transition: how this slide enters: a kind from `transitions`, `(kind: "slide",
///   duration: 0.6)` (seconds or a `duration`), `none` for a hard cut, or `auto` for the
///   deck default from slidekit.toml. Going back plays it in reverse.
/// - advance: move on automatically after this many seconds (waits for a playing video).
/// - step: build step of this page (1, 2, ..). Pages with step > 1 continue the slide.
#let slide-info(
  title: none, speaker: none, minutes: none, notes: none,
  transition: auto, advance: none, step: none,
) = {
  let transition = _transition(transition)
  // `place` so the (invisible) metadata never takes up a line in the slide's layout
  place(context [
    #metadata((
      page: here().page(), title: title, speaker: speaker, minutes: minutes, notes: notes,
      transition: transition, advance: advance, step: step, contract: contract,
    )) <slidekit-slide>
  ])
}

/// An area that plays `file` (relative to the deck root). `body` is what the PDF shows,
/// usually a poster frame; the HTML player plays the video in exactly this area.
///
/// `autoplay` starts the video when the slide appears (browsers only allow sound after the
/// first key press or click, so combine it with `muted` for a slide you open on).
/// `autoplay + loop + muted` with `controls: false` makes a moving background or GIF-like clip.
#let video(file, body, width: 100%, height: 100%, autoplay: false, loop: false, muted: false, controls: true) = box(width: width, height: height, {
  place(top + left, layout(size => context {
    let p = here().position()
    [#metadata((
      page: p.page, file: file, x: p.x / 1pt, y: p.y / 1pt, w: size.width / 1pt, h: size.height / 1pt,
      autoplay: autoplay, loop: loop, muted: muted, controls: controls,
    )) <slidekit-video>]
  }))
  link(file, box(width: 100%, height: 100%, body))
})

// Build steps ----------------------------------------------------------------------------
//
// A slide shown in steps is the same page emitted several times, each with a higher
// `step`. Inside it, `uncover` and `only` decide what is visible:
//
//   #sk.stepped(3, i => page[
//     #sk.slide-info(title: [Plan], step: i)
//     - always
//     - #sk.uncover(2)[from step 2 on]
//     #sk.only(3)[only in step 3]
//   ])

/// The step being built (an int), or `none` outside of `stepped`, where everything shows.
#let step = state("slidekit-step", none)

/// Whether `when` includes step `n`: an int, an array of steps, or a string like "2", "2-",
/// "-3", "2-4" or "1, 3-". An int means "from that step on" if `from` is true (`uncover`),
/// otherwise exactly that step (`only`, `focus`).
#let shows(when, n, from: true) = {
  if type(when) == int { return if from { n >= when } else { n == when } }
  if type(when) == array { return n in when }
  assert(type(when) == str, message: "steps must be an int, an array or a string like \"2-4\"")
  when.split(",").any(part => {
    let part = part.trim()
    if "-" in part {
      let (a, b) = part.split("-").map(str.trim)
      (a == "" or n >= int(a)) and (b == "" or n <= int(b))
    } else { n == int(part) }
  })
}

/// `body` from the given step on (or in the given steps); otherwise invisible, but it keeps
/// its space so nothing moves when it appears.
#let uncover(when, body) = context {
  let n = step.get()
  if n == none or shows(when, n) { body } else { hide(body) }
}

/// `body` only in the given steps (an int is exactly that step); otherwise it takes no space.
/// Labels inside a stepped slide exist once per step, so link to labels outside of it.
#let only(when, body) = context {
  let n = step.get()
  if n == none or shows(when, n, from: false) { body }
}

/// `body` as is in the given steps (an int is exactly that step) and dimmed otherwise: for
/// "this is where we are" lists. `dim` styles the other steps; the default greys text, which
/// suits light themes and doesn't reach explicitly coloured content (raw, shapes), so pass
/// your own for dark themes.
#let focus(when, body, dim: text.with(fill: luma(170))) = context {
  let n = step.get()
  if n == none or shows(when, n, from: false) { body } else { dim(body) }
}

/// Emits `count` pages by calling `make(i)` for i = 1..count with `step` set to i, then
/// resets `step`. `make(i)` must produce exactly one page and pass `step: i` to `slide-info`.
#let stepped(count, make) = {
  for i in range(1, count + 1) {
    step.update(i)
    make(i)
  }
  step.update(none)
}

/// Shows `body` as it looks in step `n` (e.g. the last step in a handout).
#let at-step(n, body) = {
  step.update(n)
  body
  step.update(none)
}

// Handout helpers ------------------------------------------------------------------------

/// All slide infos in document order, one per slide (build steps are folded). Needs `context`.
#let slides() = query(<slidekit-slide>).map(it => it.value).filter(s => s.at("step", default: none) in (none, 1))

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
