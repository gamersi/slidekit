//! Link areas on each page, so the player can make links in slide images clickable.

use typst::layout::{Abs, Frame, FrameItem, Point, Transform};
use typst::model::Destination;
use typst_layout::PagedDocument;

/// Where a link goes.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Url(String),
    /// 1-based page number.
    Page(usize),
}

/// A link's bounding box in pt from the page's top left.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkArea {
    pub target: Target,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// All link areas, per page (same order as `doc.pages()`).
pub fn extract(doc: &PagedDocument) -> Vec<Vec<LinkArea>> {
    let introspector = doc.introspector();
    doc.pages()
        .iter()
        .map(|page| {
            let mut out = Vec::new();
            walk(&page.frame, Transform::identity(), &mut |dest, ts, size| {
                let target = match dest {
                    Destination::Url(url) => Target::Url(url.to_string()),
                    Destination::Position(p) => Target::Page(p.page.get()),
                    Destination::Location(loc) => match introspector.position(*loc) {
                        Some(p) => Target::Page(p.page.get()),
                        None => return,
                    },
                };
                // Bounding box of the transformed rectangle (handles rotate/scale).
                let corners = [
                    Point::zero(),
                    Point::with_x(size.x),
                    Point::with_y(size.y),
                    Point::new(size.x, size.y),
                ]
                .map(|p| p.transform(ts));
                let (mut x0, mut y0, mut x1, mut y1) =
                    (Abs::inf(), Abs::inf(), -Abs::inf(), -Abs::inf());
                for c in corners {
                    x0 = x0.min(c.x);
                    y0 = y0.min(c.y);
                    x1 = x1.max(c.x);
                    y1 = y1.max(c.y);
                }
                let area = LinkArea {
                    target,
                    x: x0.to_pt(),
                    y: y0.to_pt(),
                    w: (x1 - x0).to_pt(),
                    h: (y1 - y0).to_pt(),
                };
                if area.w > 0.0 && area.h > 0.0 && !out.contains(&area) {
                    out.push(area);
                }
            });
            out
        })
        .collect()
}

fn walk(
    frame: &Frame,
    ts: Transform,
    f: &mut impl FnMut(&Destination, Transform, typst::layout::Size),
) {
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Group(group) => {
                let ts = ts
                    .pre_concat(Transform::translate(pos.x, pos.y))
                    .pre_concat(group.transform);
                walk(&group.frame, ts, f);
            }
            FrameItem::Link(dest, size) => {
                f(
                    dest,
                    ts.pre_concat(Transform::translate(pos.x, pos.y)),
                    *size,
                );
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::world::{Mode, Session};

    #[test]
    fn finds_url_and_internal_links() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("main.typ"),
            r#"
#set page(width: 200pt, height: 100pt, margin: 10pt)
#link("https://typst.app")[Typst] and #link(<end>)[the end]
#pagebreak()
#move(dx: 20pt, dy: 30pt, link("https://example.com", box(width: 40pt, height: 10pt)))
#pagebreak()
= End <end>
"#,
        )
        .unwrap();
        let config = Config::parse("[deck]\nentry = \"main.typ\"", dir.path()).unwrap();
        let session = Session::new(&config).unwrap();
        let doc = crate::compile::compile(&session.world(Mode::Slides))
            .unwrap()
            .doc;
        let links = extract(&doc);
        assert_eq!(links.len(), 3);
        let targets: Vec<_> = links[0].iter().map(|l| l.target.clone()).collect();
        assert_eq!(
            targets,
            [Target::Url("https://typst.app".into()), Target::Page(3)]
        );
        let moved = &links[1][0];
        assert_eq!(moved.target, Target::Url("https://example.com".into()));
        assert!(
            (moved.x - 30.0).abs() < 0.01 && (moved.w - 40.0).abs() < 0.01,
            "{moved:?}"
        );
        assert!(moved.y > 30.0, "{moved:?}");
        assert!(links[2].is_empty());
    }
}
