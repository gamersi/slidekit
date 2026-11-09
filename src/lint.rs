//! Layout checks on the compiled slides: things that are almost always mistakes and easy to
//! miss when nobody looks at every page (text running off the slide, content that spilled
//! onto an extra page, broken build-step numbering).

use typst::layout::{Abs, Frame, FrameItem, Point, Transform};
use typst_layout::PagedDocument;

use crate::meta::Meta;

/// How far (in pt) text may poke out of the page or a clipping box before we complain.
const TOLERANCE: f64 = 2.0;

/// All findings, one formatted `warning: ..` line (ending in a newline) per problem.
pub fn check(doc: &PagedDocument, meta: &Meta) -> Vec<String> {
    let mut out = Vec::new();
    let pages = doc.pages();

    if !meta.slides.is_empty() {
        let mut prev_step: Option<usize> = None;
        for n in 1..=pages.len() {
            match meta.slides.get(&n) {
                None => {
                    let cause = if n == 1 {
                        String::new()
                    } else {
                        format!(
                            "content probably overflowed from page {} (make it fit or split the \
                             slide), or ",
                            n - 1
                        )
                    };
                    out.push(format!(
                        "warning: page {n} has no slide-info: {cause}the page was made without \
                         `sk.slide-info`\n"
                    ));
                    prev_step = None;
                }
                Some(info) => {
                    if let Some(step) = info.step.filter(|&s| s > 1)
                        && prev_step != Some(step - 1)
                    {
                        out.push(format!(
                            "warning: page {n} is build step {step}, but the page before is not step {}\n",
                            step - 1
                        ));
                    }
                    prev_step = info.step;
                }
            }
        }
    }

    for (i, page) in pages.iter().enumerate() {
        let size = page.frame.size();
        let bounds = Bounds {
            min: Point::zero(),
            max: Point::new(size.x, size.y),
        };
        let mut found = Outside::default();
        walk(&page.frame, Transform::identity(), bounds, true, &mut found);
        if let Some(sample) = found.sample {
            let n = i + 1;
            let runs = found.runs;
            let s = if runs == 1 { "" } else { "s" };
            out.push(format!(
                "warning: page {n}: {runs} text run{s} outside the {} (first: \"{sample}\"); \
                 the text is cut off or off-slide\n",
                if found.clipped_only {
                    "clipping box"
                } else {
                    "page"
                }
            ));
        }
    }
    out
}

#[derive(Clone, Copy)]
struct Bounds {
    min: Point,
    max: Point,
}

impl Bounds {
    /// Axis-aligned bounds of the rectangle `min`..`max` after `ts` (all four corners, so
    /// rotations are covered).
    fn around(ts: Transform, min: Point, max: Point) -> Bounds {
        let corners =
            [min, Point::new(max.x, min.y), Point::new(min.x, max.y), max].map(|p| p.transform(ts));
        let mut b = Bounds {
            min: corners[0],
            max: corners[0],
        };
        for c in &corners[1..] {
            b.min = Point::new(b.min.x.min(c.x), b.min.y.min(c.y));
            b.max = Point::new(b.max.x.max(c.x), b.max.y.max(c.y));
        }
        b
    }

    fn intersect(self, other: Bounds) -> Bounds {
        Bounds {
            min: Point::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y)),
            max: Point::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y)),
        }
    }
}

#[derive(Default)]
struct Outside {
    runs: usize,
    sample: Option<String>,
    /// Whether every run found is inside the page but outside a clipping box.
    clipped_only: bool,
}

/// Visits every text run with its page-space transform. `visible` is the area the text may
/// occupy (the page, narrowed by clipping groups); `page_only` is false inside a clip.
fn walk(frame: &Frame, ts: Transform, visible: Bounds, page_only: bool, found: &mut Outside) {
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Group(group) => {
                let inner = ts
                    .pre_concat(Transform::translate(pos.x, pos.y))
                    .pre_concat(group.transform);
                let (visible, page_only) = if group.clip.is_some() {
                    let size = group.frame.size();
                    let clip = Bounds::around(inner, Point::zero(), Point::new(size.x, size.y));
                    (visible.intersect(clip), false)
                } else {
                    (visible, page_only)
                };
                walk(&group.frame, inner, visible, page_only, found);
            }
            FrameItem::Text(text) => {
                if text.text.trim().is_empty() {
                    continue;
                }
                let local = ts.pre_concat(Transform::translate(pos.x, pos.y));
                // Real glyph outlines (bbox is relative to the baseline start, y down).
                let bbox = text.bbox();
                if !bbox.min.x.to_raw().is_finite() || bbox.min.x > bbox.max.x {
                    continue;
                }
                let Bounds { min: lo, max: hi } = Bounds::around(local, bbox.min, bbox.max);
                let tol = Abs::pt(TOLERANCE);
                let out = lo.x < visible.min.x - tol
                    || lo.y < visible.min.y - tol
                    || hi.x > visible.max.x + tol
                    || hi.y > visible.max.y + tol;
                if out {
                    if found.runs == 0 {
                        found.clipped_only = !page_only;
                    } else if page_only {
                        found.clipped_only = false;
                    }
                    found.runs += 1;
                    found.sample.get_or_insert_with(|| {
                        let t = text.text.trim();
                        match t.char_indices().nth(40) {
                            Some((i, _)) => format!("{}…", &t[..i]),
                            None => t.to_string(),
                        }
                    });
                }
            }
            FrameItem::Shape(..)
            | FrameItem::Image(..)
            | FrameItem::Link(..)
            | FrameItem::Tag(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::meta::Meta;
    use crate::world::{Mode, Session};

    fn lint(deck: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("slidekit.typ"), crate::scaffold::LIB).unwrap();
        std::fs::write(dir.path().join("main.typ"), deck).unwrap();
        let config = Config::parse("[deck]\nentry = \"main.typ\"", dir.path()).unwrap();
        let session = Session::new(&config).unwrap();
        let doc = crate::compile::compile(&session.world(Mode::Slides))
            .unwrap()
            .doc;
        super::check(&doc, &Meta::extract(&doc).unwrap())
    }

    const HEAD: &str =
        "#import \"slidekit.typ\" as sk\n#set page(width: 200pt, height: 100pt, margin: 10pt)\n";

    #[test]
    fn clean_deck() {
        let w = lint(&format!(
            "{HEAD}#sk.slide-info(title: [A])\nHello\n#pagebreak()\n\
             #sk.stepped(2, i => page[#sk.slide-info(step: i) #sk.uncover(2)[x]])"
        ));
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn overflow_onto_next_page() {
        let w = lint(&format!(
            "{HEAD}#sk.slide-info(title: [A])\n#for i in range(30) [line #i \\ ]"
        ));
        assert!(
            w.iter().any(|w| w.contains("page 2 has no slide-info")),
            "{w:?}"
        );
    }

    #[test]
    fn text_off_the_page_and_clipped() {
        let w = lint(&format!(
            "{HEAD}#sk.slide-info(title: [A])\n#place(dx: 150pt)[far too wide to fit]\n#pagebreak()\n\
             #sk.slide-info(title: [B])\n#box(width: 40pt, height: 12pt, clip: true)[clipped away here]"
        ));
        assert!(
            w.iter()
                .any(|w| w.contains("page 1: 1 text run outside the page")),
            "{w:?}"
        );
        assert!(
            w.iter()
                .any(|w| w.contains("page 2:") && w.contains("clipping box")),
            "{w:?}"
        );
    }

    #[test]
    fn big_text_and_rotation_are_not_false_positives() {
        let w = lint(
            "#import \"slidekit.typ\" as sk\n#set page(width: 400pt, height: 300pt, margin: 0pt)\n\
             #sk.slide-info(title: [A])\n#text(80pt)[TITLE]\n\
             #box(clip: true)[#text(40pt)[TITLE]]\n\
             #rotate(45deg, reflow: true, box(width: 200pt, height: 60pt, clip: true, inset: 10pt)[Hi \\ \\ Lo])",
        );
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn broken_steps() {
        let w = lint(&format!(
            "{HEAD}#sk.slide-info(step: 1)\n#pagebreak()\n#sk.slide-info(step: 3)"
        ));
        assert!(
            w.iter().any(|w| w.contains("page 2 is build step 3")),
            "{w:?}"
        );
    }
}
