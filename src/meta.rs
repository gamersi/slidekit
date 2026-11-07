//! Slide and video metadata from the compiled document (see `assets/slidekit.typ`).

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use typst::foundations::{Label, Selector};
use typst::introspection::{Introspector, MetadataElem};
use typst::utils::PicoStr;
use typst_layout::PagedDocument;

pub const SLIDE_LABEL: &str = "slidekit-slide";
pub const VIDEO_LABEL: &str = "slidekit-video";

/// `<slidekit-slide>`: one per slide page. Title, speaker and notes may be content.
#[derive(Debug, Clone, Deserialize)]
pub struct SlideInfo {
    pub page: usize,
    #[serde(default)]
    pub title: serde_json::Value,
    #[serde(default)]
    pub speaker: serde_json::Value,
    #[serde(default)]
    pub minutes: Option<f64>,
    #[serde(default)]
    pub notes: serde_json::Value,
    /// `(kind, duration)` or null for the deck default.
    #[serde(default)]
    pub transition: Option<Transition>,
    /// Auto-advance after this many seconds.
    #[serde(default)]
    pub advance: Option<f64>,
    /// Build step (1-based); a page with step > 1 continues the slide before it.
    #[serde(default)]
    pub step: Option<usize>,
    /// Contract version of the library that emitted this (absent before v2).
    #[serde(default)]
    pub contract: Option<u32>,
}

impl SlideInfo {
    /// Whether this page is a later build step of the slide on the page before.
    pub fn continues(&self) -> bool {
        self.step.is_some_and(|s| s > 1)
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize, PartialEq)]
pub struct Transition {
    pub kind: String,
    /// Seconds; `None` means the deck default.
    #[serde(default)]
    pub duration: Option<f64>,
}

/// `<slidekit-video>`: one per video area, position and size in pt from the page's top left.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct VideoInfo {
    pub page: usize,
    pub file: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    #[serde(default)]
    pub autoplay: bool,
    #[serde(default, rename = "loop")]
    pub looped: bool,
    #[serde(default)]
    pub muted: bool,
    #[serde(default = "yes")]
    pub controls: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Default)]
pub struct Meta {
    /// Slide infos by 1-based page number (the first one wins if a page has several).
    pub slides: BTreeMap<usize, SlideInfo>,
    pub videos: Vec<VideoInfo>,
}

impl Meta {
    pub fn extract(doc: &PagedDocument) -> Result<Self> {
        let mut slides = BTreeMap::new();
        for info in query::<SlideInfo>(doc, SLIDE_LABEL)? {
            slides.entry(info.page).or_insert(info);
        }
        Ok(Self {
            slides,
            videos: query(doc, VIDEO_LABEL)?,
        })
    }

    pub fn videos_on(&self, page: usize) -> impl Iterator<Item = &VideoInfo> {
        self.videos.iter().filter(move |v| v.page == page)
    }
}

/// All `metadata` values with the given label, deserialized via their JSON form.
fn query<T: DeserializeOwned>(doc: &PagedDocument, label: &str) -> Result<Vec<T>> {
    let label = Label::new(PicoStr::intern(label)).expect("label is not empty");
    doc.introspector()
        .query(&Selector::Label(label))
        .iter()
        .filter_map(|elem| elem.to_packed::<MetadataElem>())
        .map(|meta| {
            let json = serde_json::to_value(&meta.value)?;
            serde_json::from_value(json.clone())
                .with_context(|| format!("unexpected <{}> metadata: {json}", label.resolve()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::world::{Mode, Session};

    const DECK: &str = r#"
#import "slidekit.typ" as sk
#set page(width: 200pt, height: 100pt, margin: 10pt)
#sk.slide-info(title: [Hello *world*], speaker: "Ada", minutes: 2, notes: [
  - one
  - two
])
First
#pagebreak()
No info here
#pagebreak()
#sk.slide-info(speaker: "Bob", minutes: 1.5, transition: (kind: "slide", duration: duration(seconds: 1)), advance: 5, step: 2)
#sk.video("clip.mp4", rect(), width: 50pt, height: 20pt, autoplay: true, loop: true)
#metadata((page: 3, title: "dup")) <slidekit-slide>
"#;

    #[test]
    fn extracts_from_compiled_deck() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("slidekit.typ"), crate::scaffold::LIB).unwrap();
        std::fs::write(dir.path().join("main.typ"), DECK).unwrap();
        let config = Config::parse("[deck]\nentry = \"main.typ\"", dir.path()).unwrap();
        let session = Session::new(&config).unwrap();
        let doc = crate::compile::compile(&session.world(Mode::Slides))
            .unwrap()
            .doc;
        assert_eq!(doc.pages().len(), 3);

        let meta = Meta::extract(&doc).unwrap();
        assert_eq!(meta.slides.keys().copied().collect::<Vec<_>>(), [1, 3]);
        let first = &meta.slides[&1];
        assert_eq!(crate::notes::to_text(&first.title), "Hello world");
        assert_eq!(first.speaker, "Ada");
        assert_eq!(first.minutes, Some(2.0));
        assert_eq!(
            crate::notes::to_html(&first.notes),
            "<ul><li>one</li><li>two</li></ul>"
        );
        let third = &meta.slides[&3];
        assert_eq!(third.speaker, "Bob");
        assert_eq!(third.minutes, Some(1.5));
        assert!(third.title.is_null());
        assert!(!first.continues() && third.continues());
        assert_eq!(first.transition, None);
        assert_eq!(first.contract, Some(crate::contract::VERSION));
        assert_eq!(
            third.transition,
            Some(Transition {
                kind: "slide".into(),
                duration: Some(1.0)
            })
        );
        assert_eq!(third.advance, Some(5.0));

        assert_eq!(meta.videos.len(), 1);
        let v = &meta.videos[0];
        assert_eq!((v.page, v.file.as_str()), (3, "clip.mp4"));
        assert_eq!((v.w, v.h), (50.0, 20.0));
        assert!(v.autoplay && v.looped && !v.muted && v.controls);
        assert!(v.x >= 10.0 && v.y >= 10.0, "{v:?}");
        assert_eq!(meta.videos_on(3).count(), 1);
    }
}
