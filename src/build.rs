//! One build: compile both modes concurrently, then export, render and write in parallel.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::compile::{self, Compiled};
use crate::config::Config;
use crate::meta::Meta;
use crate::world::{Mode, Session};
use crate::{export, media, player, render};

/// Which outputs to produce (on top of what the config enables).
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub pdf: bool,
    pub handout: bool,
    pub html: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            pdf: true,
            handout: true,
            html: true,
        }
    }
}

/// A deck plus the long-lived compilation state (reused by watch mode).
pub struct Builder {
    pub config: Config,
    session: Session,
    options: Options,
}

#[derive(Debug, Default)]
pub struct Summary {
    pub pages: usize,
    pub slide_infos: usize,
    pub videos: usize,
    pub planned_minutes: f64,
    pub images: Option<render::Stats>,
    pub media: Option<media::Stats>,
    pub warnings: Vec<String>,
    /// Phase name and duration, in execution order.
    pub timings: Vec<(&'static str, Duration)>,
    pub outputs: Vec<PathBuf>,
}

impl Builder {
    pub fn new(config: Config, options: Options) -> Result<Self> {
        let session = Session::new(&config)?;
        Ok(Self {
            config,
            session,
            options,
        })
    }

    pub fn build(&mut self) -> Result<Summary> {
        let total = Instant::now();
        self.session.reset();
        let config = &self.config;
        let pdf = config.pdf.as_ref().filter(|_| self.options.pdf);
        let handout = config.handout.as_ref().filter(|_| self.options.handout);
        let html = config.html.as_ref().filter(|_| self.options.html);
        let need_slides = pdf.is_some() || html.is_some();

        let slides_world = self.session.world(Mode::Slides);
        let handout_world = self.session.world(Mode::Handout);
        let timed_compile = |enabled: bool, world| {
            enabled.then(|| {
                let start = Instant::now();
                compile::compile(world).map(|c| (c, start.elapsed()))
            })
        };
        let (slides, handout_doc) = rayon::join(
            || timed_compile(need_slides, &slides_world),
            || timed_compile(handout.is_some(), &handout_world),
        );
        let slides = slides.transpose().context("slides")?;
        let handout_doc = handout_doc.transpose().context("handout")?;

        let mut summary = Summary::default();
        if let Some((c, t)) = &slides {
            summary.timings.push(("compile", *t));
            summary.warnings.extend(c.warnings.iter().cloned());
        }
        if let Some((c, t)) = &handout_doc {
            summary.timings.push(("compile handout", *t));
            for w in &c.warnings {
                if !summary.warnings.contains(w) {
                    summary.warnings.push(w.clone());
                }
            }
        }

        let timed = |f: &dyn Fn() -> Result<()>| -> Result<Duration> {
            let start = Instant::now();
            f().map(|()| start.elapsed())
        };

        // Slides PDF, handout PDF and the player are independent of each other.
        let ((pdf_time, handout_time), html_result) = rayon::join(
            || {
                rayon::join(
                    || match (pdf, &slides) {
                        (Some(path), Some((c, _))) => {
                            Some(timed(&|| export::pdf(&slides_world, &c.doc, path)))
                        }
                        _ => None,
                    },
                    || match (handout, &handout_doc) {
                        (Some(path), Some((c, _))) => {
                            Some(timed(&|| export::pdf(&handout_world, &c.doc, path)))
                        }
                        _ => None,
                    },
                )
            },
            || {
                slides
                    .as_ref()
                    .map(|(c, _)| self.player_and_media(c, html.is_some()))
            },
        );

        if let Some(t) = pdf_time.transpose().context("slides PDF")? {
            summary.timings.push(("pdf", t));
            summary.outputs.extend(pdf.cloned());
        }
        if let Some(t) = handout_time.transpose().context("handout PDF")? {
            summary.timings.push(("handout pdf", t));
            summary.outputs.extend(handout.cloned());
        }
        if let Some(result) = html_result {
            let part = result?;
            summary.pages = part.pages;
            summary.slide_infos = part.slide_infos;
            summary.videos = part.videos;
            summary.planned_minutes = part.planned;
            summary.images = part.images;
            summary.media = Some(part.media);
            summary.timings.extend(part.timings);
            summary.outputs.extend(html.cloned());
        } else if let Some((c, _)) = &handout_doc {
            summary.pages = c.doc.pages().len();
        }
        summary.timings.push(("total", total.elapsed()));
        Ok(summary)
    }

    /// Metadata, slide images, `index.html` and media copies.
    fn player_and_media(&self, slides: &Compiled, html: bool) -> Result<PlayerPart> {
        let config = &self.config;
        let mut part = PlayerPart {
            pages: slides.doc.pages().len(),
            ..Default::default()
        };
        let meta = Meta::extract(&slides.doc)?;
        part.slide_infos = meta.slides.len();
        part.videos = meta.videos.len();
        part.planned = meta.slides.values().filter_map(|s| s.minutes).sum();

        let start = Instant::now();
        part.media = media::copy(config, meta.videos.iter().map(|v| v.file.as_str()))?;
        part.timings.push(("media", start.elapsed()));

        if let (true, Some(path)) = (html, &config.html) {
            let start = Instant::now();
            let dir = config.out_dir.join(render::DIR);
            let (images, stats) =
                render::render(&slides.doc, &dir, config.ppi, config.jpeg_quality)?;
            part.timings.push(("render", start.elapsed()));
            part.images = Some(stats);

            let start = Instant::now();
            player::write(
                path,
                &config.out_dir,
                &config.title,
                &slides.doc,
                &meta,
                &images,
            )?;
            part.timings.push(("html", start.elapsed()));
        }
        Ok(part)
    }

    /// Real paths of every file the last build read.
    pub fn dependencies(&mut self) -> Vec<PathBuf> {
        self.session.dependencies()
    }
}

#[derive(Default)]
struct PlayerPart {
    pages: usize,
    slide_infos: usize,
    videos: usize,
    planned: f64,
    images: Option<render::Stats>,
    media: media::Stats,
    timings: Vec<(&'static str, Duration)>,
}

impl Summary {
    /// Human-readable multi-line report.
    pub fn report(&self) -> String {
        let mut out = String::new();
        for w in &self.warnings {
            out.push_str(w);
        }
        let _ = write!(out, "{} pages", self.pages);
        if self.slide_infos > 0 || self.videos > 0 {
            let _ = write!(
                out,
                " · {} slide infos · {} videos · {} min planned",
                self.slide_infos,
                self.videos,
                fmt_minutes(self.planned_minutes)
            );
        } else if self.pages > 0 {
            out.push_str(" · no <slidekit-slide> metadata (deck does not use slidekit.typ?)");
        }
        if let Some(i) = self.images {
            let _ = write!(
                out,
                " · images: {} rendered, {} reused",
                i.rendered, i.reused
            );
            if i.removed > 0 {
                let _ = write!(out, ", {} removed", i.removed);
            }
        }
        if let Some(m) = self.media.as_ref().filter(|m| m.copied > 0) {
            let _ = write!(out, " · {} media copied", m.copied);
        }
        out.push('\n');
        let timings: Vec<String> = self
            .timings
            .iter()
            .map(|(name, d)| format!("{name} {}", fmt_duration(*d)))
            .collect();
        out.push_str(&timings.join(" · "));
        out
    }
}

fn fmt_minutes(m: f64) -> String {
    let s = format!("{m:.1}");
    s.strip_suffix(".0").map(str::to_owned).unwrap_or(s)
}

pub fn fmt_duration(d: Duration) -> String {
    if d.as_secs_f64() >= 1.0 {
        format!("{:.2}s", d.as_secs_f64())
    } else {
        format!("{}ms", d.as_millis())
    }
}
