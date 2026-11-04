//! The offline HTML player: the embedded template plus the slide data as JSON.

use std::path::{Component, Path};

use anyhow::{Context, Result};
use serde::Serialize;
use typst_layout::PagedDocument;

use crate::meta::Meta;
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
    pub slides: Vec<Slide>,
}

#[derive(Debug, Serialize)]
pub struct Slide {
    pub img: String,
    pub title: String,
    pub speaker: String,
    pub minutes: f64,
    /// Planned minutes of all slides before this one.
    pub before: f64,
    pub notes: String,
    pub videos: Vec<Video>,
}

/// A video area in % of the page.
#[derive(Debug, Serialize)]
pub struct Video {
    pub src: String,
    pub l: f64,
    pub t: f64,
    pub w: f64,
    pub h: f64,
}

/// Builds the player data. `prefix` is the path from the HTML file to the output dir.
pub fn deck(
    title: &str,
    doc: &PagedDocument,
    meta: &Meta,
    images: &[String],
    prefix: &str,
) -> Deck {
    let round = |x: f64| (x * 1000.0).round() / 1000.0;
    let mut before = 0.0;
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
            let slide = Slide {
                img: format!("{prefix}{}/{img}", render::DIR),
                title: info.map(|s| notes::to_text(&s.title)).unwrap_or_default(),
                speaker: info.map(|s| notes::to_text(&s.speaker)).unwrap_or_default(),
                minutes,
                before: round(before),
                notes: info.map(|s| notes::to_html(&s.notes)).unwrap_or_default(),
                videos: meta
                    .videos_on(number)
                    .map(|v| Video {
                        src: format!("{prefix}{}", url_path(&v.file)),
                        l: round(v.x / pw * 100.0),
                        t: round(v.y / ph * 100.0),
                        w: round(v.w / pw * 100.0),
                        h: round(v.h / ph * 100.0),
                    })
                    .collect(),
            };
            before += minutes;
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
pub fn write(
    path: &Path,
    out_dir: &Path,
    title: &str,
    doc: &PagedDocument,
    meta: &Meta,
    images: &[String],
) -> Result<Deck> {
    let depth = path.strip_prefix(out_dir).map_or(0, |rel| {
        rel.components()
            .filter(|c| matches!(c, Component::Normal(_)))
            .count()
            .saturating_sub(1)
    });
    let deck = deck(title, doc, meta, images, &"../".repeat(depth));
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
            slides: vec![Slide {
                img: "slides/x.jpg".into(),
                title: String::new(),
                speaker: String::new(),
                minutes: 0.0,
                before: 0.0,
                notes: "</script><script>alert(1)</script>".into(),
                videos: vec![],
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
