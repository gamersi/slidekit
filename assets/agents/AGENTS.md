# Agent guide: this is a slidekit deck

This folder is a presentation written in [Typst](https://typst.app) and built with
[slidekit](https://github.com/gamersi/slidekit). `slidekit.toml` names the entry file;
`dist/` is build output (never edit it).

The full guide is `.claude/skills/slidekit/SKILL.md`, with `design.md` (presentation
craft) and `typst.md` (Typst idioms and pitfalls) next to it. Read it before changing
slides, whichever agent you are.

## Commands

```sh
slidekit preview . --sheet       # dist/preview/page-NN.png + sheet.png; exit 2 on warnings
slidekit preview . --pages 3-5   # only the pages you touched
slidekit build .                 # slides PDF, handout PDF, HTML player into dist/
slidekit serve . --open          # live player for humans (long-running)
```

## Rules

1. Look at what you changed: run `slidekit preview` and open the PNGs before you say a
   slide is done. A successful compile does not mean the slides are readable.
2. Fix every preview warning (overflow, text off the slide, unknown fonts, broken steps).
3. Do not edit `lib/slidekit.typ`; `slidekit lib` replaces it.
4. Style lives in the theme (tokens and a `slide` function); slides do not hard-code
   colours or sizes.
5. Every slide has a takeaway title, minutes and speaker notes; minutes add up to the
   talk length.
6. One idea per slide, body text ≥ 24 pt, at most about 6 lines. Move the rest into the
   notes.
