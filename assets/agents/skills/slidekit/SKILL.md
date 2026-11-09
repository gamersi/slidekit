---
name: slidekit
description: Create, edit, restyle or review a presentation built with slidekit (a Typst deck with slidekit.toml and lib/slidekit.typ). Use for any request about slides, a deck, a talk, a pitch, a lecture, speaker notes, a handout or the slide theme in a slidekit project — including "make a presentation about X" in a folder that has or should get a slidekit.toml.
---

# Making presentations with slidekit

slidekit compiles one Typst file into a slides PDF, a speaker handout PDF and an offline
HTML player (transitions, build steps, videos, presenter view). The deck talks to slidekit
only through `lib/slidekit.typ` (imported as `sk`); everything visual is ordinary Typst.

Two companion files hold the detail; read them when the step below says so:

- `design.md`: how to make a talk good (story, slide types, layout, type, colour, data, code).
- `typst.md`: Typst idioms and pitfalls for slides, plus a theme skeleton.

## The loop

Follow these steps in order. Do not skip step 5. A deck that compiles can still be
unreadable, and you only find out by looking at it.

### 1. Orient

- Find `slidekit.toml` (it names the `entry` file) and read the entry file and any theme
  file it imports. Reuse the deck's own `slide` function and theme tokens; do not invent a
  second style next to an existing one.
- No deck yet? Run `slidekit init <dir>` and build on top of it.
- Never edit `lib/slidekit.typ`. It is vendored and replaced on upgrades (`slidekit lib`).
  Its comments are the authoritative API reference; read it when unsure.

### 2. Brief

Settle these before you write slides. Ask the user if you can, otherwise pick sensible
values and state them in your reply:

- **Audience**: who, and what they already know.
- **Goal**: what the audience should think or do afterwards. Write it as one sentence.
- **Length**: minutes. Plan about 1–2 min per slide; a title or section slide takes 0.5 min.
- **Speakers**: names, if more than one person presents.
- **Look**: an existing brand or theme, light or dark, the projector or venue if known.

### 3. Outline before slides

Write the story as a list of slide titles, each one a full-sentence takeaway ("Builds got
4× faster after caching", not "Performance"), with minutes. Read only the titles: they
must tell the whole argument on their own. Fix the story here, where it is cheap. Read
`design.md` for structures and slide types.

### 4. Write

- Put the theme in its own file (`theme.typ`) with named tokens (colours, sizes, spacing)
  and a `slide(..)` function, and keep content in the entry file. Slides use tokens,
  never literal colours or sizes. See `typst.md` for a skeleton.
- Give every slide `sk.slide-info(title:, minutes:, notes:)` (via the theme's `slide`), plus
  `speaker:` when there are several speakers.
- The slide carries the point and the notes carry the words. Notes are what the speaker
  says: two to five bullets, including the transition to the next slide.
- Use build steps only when the order of revealing *is* the point (a diagram that grows, a
  list you discuss one item at a time). See the contract cheat sheet below.

### 5. Look at it

```sh
slidekit preview <dir> --sheet        # dist/preview/page-NN.png + sheet.png, manifest on stdout
slidekit preview <dir> --pages 4-6    # just the pages you changed
```

- Exit status 2 means warnings. Fix every one: compiler warnings (unknown fonts are
  common), text outside the page or a clipping box, pages without slide-info (content
  overflowed onto an extra page), broken step numbering.
- **Read `sheet.png`** for the whole deck: rhythm, consistency, slides that are too dense
  next to their neighbours. Then **read every page image you changed** at full size.
  Check them against the self-check list below.
- The manifest maps page images to slides and steps and flags `no notes`.

### 6. Get a second opinion

When the `slide-critic` subagent is available, run it after a first full draft and after
any larger restyle. Give it the deck dir and the brief, apply its findings, then go back
to step 5. A fresh pair of eyes catches what the author has stopped seeing.

### 7. Deliver

Run `slidekit build <dir>`. Report the outputs (`dist/slides.pdf`, `dist/handout.pdf`,
`dist/index.html`), the slide count, the planned minutes against the target, and any
assumptions you made in step 2. `slidekit serve <dir> --open` gives the user a live player.

## Self-check per slide

These hold for a 16:9 slide about 34 × 19 cm. Scale them for other sizes.

- [ ] The title is the takeaway, and a reader gets the point in about 5 seconds.
- [ ] One idea per slide. If you need "and" in the title, split the slide.
- [ ] Body text is ≥ 24 pt, nothing but footers or labels is under 18 pt, and titles are 36–48 pt.
- [ ] At most 6 lines of text. Prefer a diagram, number, image or code over bullets.
- [ ] Generous margins (≥ 1.5 cm). Nothing touches the edge or another element.
- [ ] Elements align to a shared left edge or grid; no random positions.
- [ ] One accent colour, used for the one thing to look at. Contrast is strong (dark on light or light on dark).
- [ ] Code: at most about 12 lines, with no line wrapping and the relevant line highlighted.
- [ ] Charts: one message each, axes labelled, the key series highlighted, the rest grey.
- [ ] Notes present, and minutes add up to the target length.

## Contract cheat sheet (`#import "lib/slidekit.typ" as sk`)

```typ
#sk.slide-info(title: [..], speaker: "Ada", minutes: 2, notes: [..],
  transition: "fade",  // none | "fade" | "slide" | "cover" | "zoom" | "wipe" | (kind:, duration:) | auto
  advance: 10)         // auto-advance after 10 s (optional)

// a slide in 3 build steps: one page per step, `step: i` ties them together
#sk.stepped(3, i => page[
  #sk.slide-info(title: [Plan], step: i, minutes: 2)
  - always there
  - #sk.uncover(2)[from step 2 on, space reserved]
  - #sk.only("3-")[from step 3 on, takes no space before]
  #sk.focus(2)[normal in step 2, dimmed otherwise]
])

#sk.video("media/clip.mp4", image("media/poster.jpg"), width: 16cm, height: 9cm,
  autoplay: false, loop: false, muted: false, controls: true)

#if sk.is-handout { .. }         // handout layout; sk.mode is "slides" or "handout"
#context sk.run-sheet()          // handout: running order with minutes per speaker
#context sk.slides()             // handout: all slide infos, build steps folded
```

- Step specs: `2`, `(1, 3)`, `"2-4"`, `"-2"`, `"1, 3-"`. An int means "from then on" for
  `uncover` and "exactly then" for `only` and `focus`.
- Labels inside a stepped slide exist once per step. Put link targets on step 1 only or
  outside the slide.
- `focus` dims with grey text by default, which suits light themes. On dark themes pass
  `dim:` with a colour that works.
- `link(<label>)[..]` jumps within the deck in the player and the PDF; URLs open in a tab.
- Video paths are relative to the deck dir and copied into `dist/`.

## Commands

| Command | Use |
|---|---|
| `slidekit preview [DIR] [--sheet] [--pages 1,3-5] [--width 1280] [--handout]` | PNGs + manifest + layout warnings (exit 2 on warnings) |
| `slidekit build [DIR] [--no-handout --no-pdf]` | All outputs into `dist/` |
| `slidekit serve [DIR] --open` | Live player for the human (long-running; do not block on it) |
| `slidekit init DIR` / `slidekit lib DIR` | New deck / refresh the vendored library |
