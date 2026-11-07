//! The offline HTML player: the embedded template plus the slide data as JSON.

use std::path::{Component, Path};

use anyhow::{Context, Result};
use serde::Serialize;
use typst_layout::PagedDocument;

use crate::config::Player;
use crate::links::{LinkArea, Target};
use crate::meta::{Meta, Transition};
use crate::notes;
use crate::render;

pub const TEMPLATE: &str = include_str!("../assets/player.html");
const DATA_MARKER: &str = "__SLIDEKIT_DATA__";
const TITLE_MARKER: &str = "__SLIDEKIT_TITLE__";
/// The template line `serve` rewrites to enable live reload.
pub const LIVE_RELOAD_OFF: &str = "const LIVE_RELOAD = false;";
pub const LIVE_RELOAD_ON: &str = "const LIVE_RELOAD = true; ";

#[derive(Debug, Serialize)]
pub struct Deck {
    pub title: String,
    /// Page width / height of the first page.
    pub aspect: f64,
    /// Sum of all planned minutes.
    pub planned: f64,
    pub player: Player,
    /// One entry per page.
    pub slides: Vec<Slide>,
}

#[derive(Debug, Serialize)]
pub struct Slide {
    pub img: String,
    pub title: String,
    pub speaker: String,
    pub minutes: f64,
    /// Planned minutes of all slides before this one (build steps count once).
    pub before: f64,
    pub notes: String,
    /// Build step, 1 for a page that starts a slide.
    pub step: usize,
    /// How the page enters; `None` = deck default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
    /// Auto-advance after this many seconds; `None` = deck default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advance: Option<f64>,
    pub videos: Vec<Video>,
    pub links: Vec<Link>,
}

/// A video area in % of the page.
#[derive(Debug, Serialize)]
pub struct Video {
    pub src: String,
    pub l: f64,
    pub t: f64,
    pub w: f64,
    pub h: f64,
    pub autoplay: bool,
    #[serde(rename = "loop")]
    pub looped: bool,
    pub muted: bool,
    pub controls: bool,
}

/// A clickable link area in % of the page: either `href` (URL) or `page` (0-based index).
#[derive(Debug, Serialize)]
pub struct Link {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    pub l: f64,
    pub t: f64,
    pub w: f64,
    pub h: f64,
}

/// What the player shows besides the pages themselves.
pub struct Inputs<'a> {
    pub title: &'a str,
    pub meta: &'a Meta,
    /// Image file name per page.
    pub images: &'a [String],
    /// Link areas per page.
    pub links: &'a [Vec<LinkArea>],
    pub player: &'a Player,
}

/// Builds the player data. `prefix` is the path from the HTML file to the output dir.
pub fn deck(doc: &PagedDocument, inputs: &Inputs, prefix: &str) -> Deck {
    let Inputs {
        title,
        meta,
        images,
        links,
        player,
    } = *inputs;
    let round = |x: f64| (x * 1000.0).round() / 1000.0;
    // `before` of the current slide, and the running total after it.
    let (mut slide_before, mut before) = (0.0, 0.0);
    let slides = doc
        .pages()
        .iter()
        .zip(images)
        .enumerate()
        .map(|(i, (page, img))| {
            let number = i + 1;
            let size = page.frame.size();
            let (pw, ph) = (size.x.to_pt(), size.y.to_pt());
            let info = meta.slides.get(&number);
            let minutes = info.and_then(|s| s.minutes).unwrap_or(0.0);
            let continues = i > 0 && info.is_some_and(|s| s.continues());
            let videos: Vec<Video> = meta
                .videos_on(number)
                .map(|v| Video {
                    src: format!("{prefix}{}", url_path(&v.file)),
                    l: round(v.x / pw * 100.0),
                    t: round(v.y / ph * 100.0),
                    w: round(v.w / pw * 100.0),
                    h: round(v.h / ph * 100.0),
                    autoplay: v.autoplay,
                    looped: v.looped,
                    muted: v.muted,
                    controls: v.controls,
                })
                .collect();
            let video_files: Vec<&str> = meta.videos_on(number).map(|v| v.file.as_str()).collect();
            let links = links
                .get(i)
                .into_iter()
                .flatten()
                // `sk.video` links its poster to the file for the PDF; the player plays it in place.
                .filter(
                    |l| !matches!(&l.target, Target::Url(u) if video_files.contains(&u.as_str())),
                )
                .map(|l| {
                    let (href, page) = match &l.target {
                        Target::Url(u) => (Some(u.clone()), None),
                        Target::Page(p) => (None, Some(p - 1)),
                    };
                    Link {
                        href,
                        page,
                        l: round(l.x / pw * 100.0),
                        t: round(l.y / ph * 100.0),
                        w: round(l.w / pw * 100.0),
                        h: round(l.h / ph * 100.0),
                    }
                })
                .collect();
            let slide = Slide {
                img: format!("{prefix}{}/{img}", render::DIR),
                title: info.map(|s| notes::to_text(&s.title)).unwrap_or_default(),
                speaker: info.map(|s| notes::to_text(&s.speaker)).unwrap_or_default(),
                minutes,
                before: round(if continues { slide_before } else { before }),
                notes: info.map(|s| notes::to_html(&s.notes)).unwrap_or_default(),
                step: if continues {
                    info.and_then(|s| s.step).unwrap_or(1)
                } else {
                    1
                },
                transition: info.and_then(|s| s.transition.clone()),
                advance: info.and_then(|s| s.advance),
                videos,
                links,
            };
            if !continues {
                slide_before = before;
                before += minutes;
            }
            slide
        })
        .collect();
    let aspect = doc.pages().first().map_or(16.0 / 9.0, |p| {
        let size = p.frame.size();
        size.x.to_pt() / size.y.to_pt()
    });
    Deck {
        title: title.into(),
        aspect,
        planned: round(before),
        player: player.clone(),
        slides,
    }
}

/// Fills the template.
pub fn html(deck: &Deck) -> Result<String> {
    // `<` is escaped so that notes can never close the surrounding <script>.
    let json = serde_json::to_string(deck)?.replace('<', "\\u003c");
    Ok(TEMPLATE
        .replace(TITLE_MARKER, &notes::escape(&deck.title))
        .replace(DATA_MARKER, &json))
}

/// Writes the player to `path`, which lies inside `out_dir`.
pub fn write(path: &Path, out_dir: &Path, doc: &PagedDocument, inputs: &Inputs) -> Result<Deck> {
    let depth = path.strip_prefix(out_dir).map_or(0, |rel| {
        rel.components()
            .filter(|c| matches!(c, Component::Normal(_)))
            .count()
            .saturating_sub(1)
    });
    let deck = deck(doc, inputs, &"../".repeat(depth));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("html.tmp");
    std::fs::write(&tmp, html(&deck)?)
        .with_context(|| format!("cannot write {}", tmp.display()))?;
    std::fs::rename(&tmp, path)?;
    Ok(deck)
}

/// Escapes the characters that would end the path part of a relative URL.
fn url_path(file: &str) -> String {
    file.replace('%', "%25")
        .replace('#', "%23")
        .replace('?', "%3F")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_has_markers() {
        assert!(TEMPLATE.contains(DATA_MARKER));
        assert!(TEMPLATE.contains(TITLE_MARKER));
        assert_eq!(TEMPLATE.matches(LIVE_RELOAD_OFF).count(), 1);
        assert_eq!(LIVE_RELOAD_OFF.len(), LIVE_RELOAD_ON.len());
    }

    #[test]
    fn html_escapes_script_breakouts() {
        let deck = Deck {
            title: "A <b>talk</b>".into(),
            aspect: 1.5,
            planned: 0.0,
            player: Player::default(),
            slides: vec![Slide {
                img: "slides/x.jpg".into(),
                title: String::new(),
                speaker: String::new(),
                minutes: 0.0,
                before: 0.0,
                notes: "</script><script>alert(1)</script>".into(),
                step: 1,
                transition: None,
                advance: None,
                videos: vec![],
                links: vec![],
            }],
        };
        let html = html(&deck).unwrap();
        assert!(html.contains("<title>A &lt;b&gt;talk&lt;/b&gt;</title>"));
        assert_eq!(html.matches("</script>").count(), 1);
    }

    #[test]
    fn url_paths() {
        assert_eq!(url_path("media/a b#1?.mp4"), "media/a b%231%3F.mp4");
    }
}
