//! Rasterising pages to JPEG, in parallel and content-addressed.
//!
//! Each image is named after a hash of its page (plus render settings), so an incremental
//! rebuild only renders pages whose content changed and stale images are removed.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::Path;

use anyhow::{Context, Result};
use rayon::prelude::*;
use typst::layout::{Frame, FrameItem};
use typst::utils::{Scalar, hash128};
use typst_layout::{Page, PagedDocument};
use typst_render::RenderOptions;

/// The directory (inside the output dir) holding the slide images.
pub const DIR: &str = "slides";

#[derive(Debug, Default, Clone, Copy)]
pub struct Stats {
    pub rendered: usize,
    pub reused: usize,
    pub removed: usize,
}

/// Renders all pages into `dir` and returns the file name for each page.
pub fn render(
    doc: &PagedDocument,
    dir: &Path,
    ppi: f32,
    quality: u8,
) -> Result<(Vec<String>, Stats)> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let names: Vec<String> = doc
        .pages()
        .iter()
        .map(|page| file_name(page, ppi, quality))
        .collect();

    let results: Vec<Result<bool>> = doc
        .pages()
        .par_iter()
        .zip(&names)
        .map(|(page, name)| {
            let path = dir.join(name);
            if path.exists() {
                return Ok(false);
            }
            let jpeg = encode(page, ppi, quality)?;
            // Write next to the target and rename, so a server never sees half a file.
            let tmp = dir.join(format!("{name}.tmp"));
            std::fs::write(&tmp, jpeg)
                .with_context(|| format!("cannot write {}", tmp.display()))?;
            std::fs::rename(&tmp, &path)?;
            Ok(true)
        })
        .collect();

    let mut stats = Stats::default();
    for result in results {
        if result? {
            stats.rendered += 1;
        } else {
            stats.reused += 1;
        }
    }

    let keep: HashSet<&str> = names.iter().map(String::as_str).collect();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let ours = name.ends_with(".jpg") || name.ends_with(".jpg.tmp");
        if ours && !keep.contains(name.as_ref()) {
            std::fs::remove_file(entry.path())?;
            stats.removed += 1;
        }
    }
    Ok((names, stats))
}

fn file_name(page: &Page, ppi: f32, quality: u8) -> String {
    let fill = page.fill_or_white();
    let hash = hash128(&(
        Visual(&page.frame),
        fill,
        ppi.to_bits(),
        quality,
        env!("CARGO_PKG_VERSION"),
    ));
    format!("{hash:032x}.jpg")
}

/// Hashes only what is visible in a frame. The derived `Hash` of a frame also covers spans,
/// introspection tags and links, which contain interned ids whose numbering can change
/// between processes; that would make file names unstable across builds.
struct Visual<'a>(&'a Frame);

impl Hash for Visual<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.size().hash(state);
        for (pos, item) in self.0.items() {
            match item {
                FrameItem::Group(group) => {
                    (0u8, pos, group.transform, &group.clip).hash(state);
                    Visual(&group.frame).hash(state);
                }
                FrameItem::Text(text) => {
                    (1u8, pos, &text.font, text.size, &text.fill, &text.stroke).hash(state);
                    for g in &text.glyphs {
                        (g.id, g.x_advance, g.x_offset, g.y_advance, g.y_offset).hash(state);
                    }
                }
                FrameItem::Shape(shape, _) => (2u8, pos, shape).hash(state),
                FrameItem::Image(image, size, _) => (3u8, pos, image, size).hash(state),
                FrameItem::Link(..) | FrameItem::Tag(_) => {}
            }
        }
    }
}

/// Renders one page and encodes it as JPEG without chroma subsampling (keeps red text crisp).
pub fn encode(page: &Page, ppi: f32, quality: u8) -> Result<Vec<u8>> {
    let options = RenderOptions {
        pixel_per_pt: Scalar::new(f64::from(ppi) / 72.0),
        render_bleed: false,
    };
    let pixmap = typst_render::render(page, &options);
    let (w, h) = (pixmap.width(), pixmap.height());
    let too_big = || anyhow::anyhow!("page is too large for JPEG ({w}×{h} px), lower output.ppi");
    let (w16, h16) = (
        u16::try_from(w).map_err(|_| too_big())?,
        u16::try_from(h).map_err(|_| too_big())?,
    );

    // Premultiplied RGBA composited over white: c + (255 - a).
    let rgb: Vec<u8> = pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|px| {
            let bg = 255 - px[3];
            [
                px[0].saturating_add(bg),
                px[1].saturating_add(bg),
                px[2].saturating_add(bg),
            ]
        })
        .collect();

    let mut out = Vec::with_capacity(rgb.len() / 8);
    let mut encoder = jpeg_encoder::Encoder::new(&mut out, quality);
    encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::R_4_4_4);
    encoder.encode(&rgb, w16, h16, jpeg_encoder::ColorType::Rgb)?;
    Ok(out)
}
