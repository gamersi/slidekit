# Presentation design

Make it easy to follow and hard to forget. Every rule below serves one of three aims:
the audience understands the point, remembers it, and does not have to read while
listening.

## Story

Pick a structure that fits the goal, then write slide titles that walk through it.

| Goal | Structure |
|---|---|
| Convince (pitch, proposal) | Situation → complication → question → answer → evidence → ask |
| Explain (tech talk, lecture) | Why it matters → mental model → how it works → example → pitfalls → recap |
| Report (status, results) | Headline result → what changed → evidence → risks → next steps |
| Teach a tool | The problem → the smallest working example → build it up step by step → the real-world version → where to go next |

- Open with why the audience should care, not with an agenda or a bio. An agenda helps
  in talks over about 15 minutes; keep it to 3–5 items and link them (`link(<label>)`).
- One idea per slide. For a deep idea, use several slides or build steps rather than a
  denser slide.
- Section slides, at about 0.5 min each, give long talks a rhythm. Skip them under about 10 minutes.
- End on the takeaway or the ask, not "Questions?". A closing slide may show both: the one
  sentence to remember plus where to find more.
- Timing: 1–2 min per content slide. Check that the minutes add up (`slidekit preview`
  lists them, and the handout run sheet sums them per speaker).

## Slide types

Keep a small set and reuse it, so the audience learns the layout once. Typical set:

- **Title**: talk title, subtitle or promise, speakers, date or venue.
- **Section**: big number and title, one subtitle line.
- **Statement**: one sentence, very large. Use it for the key claims.
- **Assertion and evidence**: the title states the claim; the body proves it with a chart,
  diagram, screenshot, code or quote. This is the default content slide.
- **Comparison**: two or three columns with the same structure (before and after, options).
- **Big number**: one figure, huge, with one line of context.
- **Code**: a short listing; the line that matters is highlighted or revealed by steps.
- **Closing**: the takeaway, then links or contact.

Bullets are a fallback, not the default. If a slide is a list of bullets, ask whether it
is really a diagram (a process, hierarchy or comparison) or really one sentence.

## Layout

- Work on a grid. Define margins and column widths once in the theme. Shared left edges
  and baselines do more for polish than any decoration.
- Leave whitespace. Content should cover well under the full slide; crowding is the most
  common failure.
- Place things deliberately. The title goes in the same place on every content slide, and
  the most important element is the biggest or the only coloured one.
- A slide is read top-left to bottom-right; put the conclusion where the eye lands
  first, or make it the title.
- Footer (slide number, section, speaker) is small and quiet, or absent.

## Type

- Use one or two families: a text face and a mono face for code, optionally a display face
  for titles.
- Use a scale with clear steps, for example 24 / 32 / 44 / 72 pt on a 19 cm high slide, not
  24 / 26 / 28.
- Keep lines short (under about 60 characters) and avoid justified text on slides.
- Use bold or the accent colour for emphasis, not underline, all-caps paragraphs or both
  at once.

## Colour

- Use a background, an ink, a muted ink and one accent. Add a second accent only when it
  carries meaning (for example speaker colours, or good and bad in a comparison).
- Contrast: body text needs ≥ 4.5:1 against the background, and projectors wash colours out,
  so err stronger. Light-grey-on-white and dark-blue-on-black fail on a beamer.
- Dark themes look good on screens and in dim rooms; light themes survive bright rooms
  and printing. If the venue is unknown, either works, as long as contrast is high.
- Colour must mean something. If everything is coloured, nothing stands out.

## Data

- One chart, one message, stated in the title ("Signups doubled after the redesign").
- Highlight the series or bar that proves the message in the accent and draw the rest in
  grey. Label lines and bars directly instead of using a legend.
- Drop chart junk: 3D, gridlines that are not needed, borders, more ticks than needed.
  Start bar axes at zero.
- Small numbers of values often work better as big numbers or a simple table than as a
  chart.
- Draw simple bars with plain Typst (`rect` widths from data). For real plots, use a Typst
  package (see `typst.md`).

## Code

- Show at most about 12 lines, in a mono face at ≥ 18 pt, with no wrapping. Cut imports,
  boilerplate and error handling unless they are the point.
- Highlight the one line that matters, or reveal the listing in build steps.
- Use a syntax theme that matches the slide background, not the editor default on a
  slide of a different colour.

## Images and diagrams

- Use real screenshots and photos, cropped to the relevant part and large. Avoid clip art
  and decorative stock photos.
- Build diagrams from a few boxes and arrows in the theme's colours. For a complex
  diagram, reveal it in steps (a diagram that grows) so the audience follows the order.
- Put videos in `sk.video(..)` with a poster frame that makes sense in the PDF too.

## Notes and handout

- Notes are bullets of what to say, including the sentence that leads into the next slide.
  Do not write a full script unless the user asks for one.
- With several speakers, set `speaker:` on every slide. The presenter view highlights
  hand-overs.
- The handout is a design choice of the deck. The usual shape is a slide preview next to
  its notes plus `sk.run-sheet()`. It is what the speakers print, so it must be readable
  at A4.

## Anti-patterns

Fix these when you see them:

- **Slideument**: paragraphs meant to be read instead of heard. Move them to the notes.
- **Title as label**: "Results" instead of the result itself.
- **Wall of bullets**: more than six lines, or bullets nested two levels deep.
- **Rainbow**: several unrelated accent colours, gradients everywhere.
- **Center everything**: centred body text and lists. Centre only titles, statements and
  big numbers.
- **Tiny text** that the author could read on a laptop and the audience cannot.
- **Inconsistent layouts**: the title jumps between slides, or margins differ.
- **Decoration over content**: shadows, borders, icons and frames that carry no meaning.
- **Ending on "Questions?"** with nothing else on the slide.
