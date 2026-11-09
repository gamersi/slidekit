//! `slidekit preview`: numbered PNGs of the pages (and optionally one contact sheet), a
//! page-by-page manifest and the layout checks. Made for looking at a deck without a
//! browser, e.g. by a coding agent that can read images.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform as SkTransform};
use typst::utils::Scalar;
use typst_layout::{Page, PagedDocument};
use typst_render::RenderOptions;

use crate::config::Config;
use crate::meta::Meta;
use crate::world::{Mode, Session};
use crate::{compile, contract, lint, notes};

pub struct Options {
    pub mode: Mode,
    /// Width of each page image in pixels.
    pub width: u32,
    /// 1-based pages to render; `None` means all.
    pub pages: Option<Vec<usize>>,
    /// Also write `sheet.png`, a grid of all rendered pages.
    pub sheet: bool,
    /// Output directory; defaults to `<output dir>/preview`.
    pub out: Option<PathBuf>,
}

pub struct Preview {
    /// Manifest: one line per page plus a header (for stdout).
    pub manifest: String,
    /// Compiler warnings and layout findings.
    pub warnings: Vec<String>,
}

/// Prefix of the page images; old ones are removed before writing new ones.
const PREFIX: &str = "page-";
const SHEET: &str = "sheet.png";
const SHEET_COLUMNS: u32 = 4;
const SHEET_THUMB: u32 = 480;
const SHEET_GAP: u32 = 16;
/// Longest page range `--pages` accepts (guards against "1-99999999999").
const MAX_PAGES: usize = 10_000;

pub fn run(config: &Config, options: &Options) -> Result<Preview> {
    let session = Session::new(config)?;
    let world = session.world(options.mode);
    let compiled =
        compile::compile(&world).map_err(|err| match contract::compile_hint(&config.deck_dir) {
            Some(hint) => anyhow::anyhow!("{err:#}\n{hint}"),
            None => err,
        })?;
    let doc = &compiled.doc;
    let total = doc.pages().len();
    let meta = Meta::extract(doc)?;

    let mut warnings = compiled.warnings.clone();
    if options.mode == Mode::Slides {
        warnings.extend(contract::check(&meta));
        warnings.extend(lint::check(doc, &meta));
    }

    let pages: Vec<usize> = match &options.pages {
        Some(list) => {
            if let Some(bad) = list.iter().find(|&&n| n == 0 || n > total) {
                bail!("page {bad} does not exist (the deck has {total} pages)");
            }
            list.clone()
        }
        None => (1..=total).collect(),
    };

    let dir = options
        .out
        .clone()
        .unwrap_or_else(|| config.out_dir.join("preview"));
    prepare(&dir, options.out.is_none())?;

    let digits = total.to_string().len().max(2);
    let name = |n: usize| format!("{PREFIX}{n:0digits$}.png");
    // Only the small sheet thumbnails are kept in memory, not the full pages.
    let thumbs: Vec<Option<Pixmap>> = pages
        .par_iter()
        .map(|&n| {
            let pixmap = render(&doc.pages()[n - 1], options.width);
            let path = dir.join(name(n));
            pixmap
                .save_png(&path)
                .with_context(|| format!("cannot write {}", path.display()))?;
            Ok(options.sheet.then(|| thumbnail(&pixmap)))
        })
        .collect::<Result<_>>()?;
    let thumbs: Vec<Pixmap> = thumbs.into_iter().flatten().collect();

    let mut manifest = String::new();
    let _ = writeln!(
        manifest,
        "{} · {} mode · {total} pages · {}",
        config.title,
        options.mode.as_str(),
        dir.display()
    );
    let slides = slide_numbers(doc, &meta);
    for &n in &pages {
        // A handout page holds any number of slides, so only slide pages get a description.
        let what = match options.mode {
            Mode::Slides => describe(n, &slides, &meta),
            Mode::Handout => String::new(),
        };
        let _ = writeln!(manifest, "{}  {what}", name(n));
    }

    if options.sheet && !thumbs.is_empty() {
        let sheet = contact_sheet(&thumbs);
        let path = dir.join(SHEET);
        sheet
            .save_png(&path)
            .with_context(|| format!("cannot write {}", path.display()))?;
        let columns = SHEET_COLUMNS.min(thumbs.len() as u32);
        let _ = writeln!(
            manifest,
            "{SHEET}  all {} pages above, {columns} per row, left to right",
            thumbs.len()
        );
    }

    Ok(Preview { manifest, warnings })
}

/// Parses "1,3-5,8" into page numbers.
pub fn parse_pages(spec: &str) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let num = |s: &str| {
            s.trim()
                .parse::<usize>()
                .with_context(|| format!("bad page number `{s}` in `{spec}`"))
        };
        match part.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (num(a)?, num(b)?);
                if a > b || b - a > MAX_PAGES {
                    bail!("bad page range `{part}`");
                }
                out.extend(a..=b);
            }
            None => out.push(num(part)?),
        }
    }
    if out.is_empty() {
        bail!("no pages in `{spec}`");
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// Marks a folder as ours, so it may be cleaned on the next run.
const MARKER: &str = ".slidekit-preview";

/// Creates the output folder and removes our old images from it, so a shorter deck leaves
/// no stale pages behind. A user-chosen folder is only cleaned once slidekit has marked it;
/// an unmarked folder with files in it is refused rather than mixed with ours.
fn prepare(dir: &Path, default: bool) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let marked = dir.join(MARKER).exists();
    if !default && !marked && std::fs::read_dir(dir)?.next().is_some() {
        bail!(
            "{} is not empty and was not made by `slidekit preview`; pick a new or empty folder",
            dir.display()
        );
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let page = name
            .strip_prefix(PREFIX)
            .and_then(|rest| rest.strip_suffix(".png"))
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        if page || name == SHEET {
            std::fs::remove_file(entry.path())
                .with_context(|| format!("cannot remove {}", entry.path().display()))?;
        }
    }
    if !marked {
        std::fs::write(dir.join(MARKER), "")
            .with_context(|| format!("cannot write into {}", dir.display()))?;
    }
    Ok(())
}

/// A page scaled down to the sheet's column width.
fn thumbnail(page: &Pixmap) -> Pixmap {
    let s = SHEET_THUMB as f32 / page.width() as f32;
    let h = ((page.height() as f32 * s).ceil() as u32).max(1);
    let mut thumb = Pixmap::new(SHEET_THUMB, h).expect("non-empty thumbnail");
    let paint = PixmapPaint {
        quality: tiny_skia::FilterQuality::Bicubic,
        ..PixmapPaint::default()
    };
    thumb.draw_pixmap(
        0,
        0,
        page.as_ref(),
        &paint,
        SkTransform::from_scale(s, s),
        None,
    );
    thumb
}

fn render(page: &Page, width: u32) -> Pixmap {
    let pt = page.frame.width().to_pt().max(1.0);
    let options = RenderOptions {
        pixel_per_pt: Scalar::new(f64::from(width) / pt),
        render_bleed: false,
    };
    let mut pixmap = typst_render::render(page, &options);
    // Pages without a fill are transparent; show them on white like a PDF viewer would.
    let mut white = Pixmap::new(pixmap.width(), pixmap.height()).expect("non-empty page");
    white.fill(Color::WHITE);
    white.draw_pixmap(
        0,
        0,
        pixmap.as_ref(),
        &PixmapPaint::default(),
        SkTransform::identity(),
        None,
    );
    std::mem::swap(&mut pixmap, &mut white);
    pixmap
}

fn contact_sheet(pages: &[Pixmap]) -> Pixmap {
    let columns = SHEET_COLUMNS.min(pages.len() as u32);
    let rows = (pages.len() as u32).div_ceil(columns);
    let scale = |p: &Pixmap| SHEET_THUMB as f32 / p.width() as f32;
    let row_height = |r: u32| {
        pages
            .iter()
            .skip((r * columns) as usize)
            .take(columns as usize)
            .map(|p| (p.height() as f32 * scale(p)).ceil() as u32)
            .max()
            .unwrap_or(0)
    };
    let heights: Vec<u32> = (0..rows).map(row_height).collect();
    let width = columns * SHEET_THUMB + (columns + 1) * SHEET_GAP;
    let height = heights.iter().sum::<u32>() + (rows + 1) * SHEET_GAP;
    let mut sheet = Pixmap::new(width, height).expect("non-empty sheet");
    sheet.fill(Color::from_rgba8(128, 128, 128, 255));

    let paint = PixmapPaint {
        quality: tiny_skia::FilterQuality::Bicubic,
        ..PixmapPaint::default()
    };
    let mut y = SHEET_GAP;
    for (r, &h) in heights.iter().enumerate() {
        for c in 0..columns {
            let Some(page) = pages.get(r * columns as usize + c as usize) else {
                break;
            };
            let x = SHEET_GAP + c * (SHEET_THUMB + SHEET_GAP);
            let s = scale(page);
            let ts = SkTransform::from_row(s, 0.0, 0.0, s, x as f32, y as f32);
            sheet.draw_pixmap(0, 0, page.as_ref(), &paint, ts, None);
        }
        y += h + SHEET_GAP;
    }
    sheet
}

/// Per 1-based page with slide-info: (slide number, step, steps of that slide); build
/// steps are folded into one slide number.
struct SlideNumbers {
    by_page: Vec<Option<(usize, usize, usize)>>,
}

fn slide_numbers(doc: &PagedDocument, meta: &Meta) -> SlideNumbers {
    let total = doc.pages().len();
    let mut by_page = vec![None; total + 1];
    let mut steps = Vec::new(); // highest step per slide
    for (&n, info) in &meta.slides {
        if n > total {
            continue;
        }
        if !info.continues() || steps.is_empty() {
            steps.push(1);
        }
        let step = info.step.unwrap_or(1);
        let slide = steps.len();
        steps[slide - 1] = steps[slide - 1].max(step);
        by_page[n] = Some((slide, step, 0));
    }
    for (slide, _, count) in by_page.iter_mut().flatten() {
        *count = steps[*slide - 1];
    }
    SlideNumbers { by_page }
}

fn describe(n: usize, slides: &SlideNumbers, meta: &Meta) -> String {
    let Some(info) = meta.slides.get(&n) else {
        return if meta.slides.is_empty() {
            String::new()
        } else {
            "(no slide-info)".into()
        };
    };
    let mut out = String::new();
    if let Some(Some((slide, step, steps))) = slides.by_page.get(n) {
        let _ = write!(out, "slide {slide}");
        if *steps > 1 {
            let _ = write!(out, " step {step}/{steps}");
        }
    }
    let title = notes::to_text(&info.title);
    let _ = write!(
        out,
        "  {}",
        if title.is_empty() {
            "(untitled)".into()
        } else {
            format!("\"{title}\"")
        }
    );
    let speaker = notes::to_text(&info.speaker);
    if !speaker.is_empty() {
        let _ = write!(out, " · {speaker}");
    }
    if let Some(m) = info.minutes.filter(|_| !info.continues()) {
        let _ = write!(out, " · {m} min");
    }
    if !info.continues() && info.notes.is_null() {
        out.push_str(" · no notes");
    }
    let videos = meta.videos_on(n).count();
    if videos > 0 {
        let _ = write!(
            out,
            " · {videos} video{}",
            if videos == 1 { "" } else { "s" }
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_spec() {
        assert_eq!(parse_pages("1, 3-5,8").unwrap(), [1, 3, 4, 5, 8]);
        assert!(parse_pages("5-3").is_err());
        assert!(parse_pages("x").is_err());
        assert!(parse_pages(" ").is_err());
        assert_eq!(parse_pages("2,1-2,1").unwrap(), [1, 2]);
        assert!(parse_pages("1-99999999999").is_err());
    }
}
