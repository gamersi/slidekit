//! Rendering pages to SVG or JPEG, in parallel and content-addressed.
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

use crate::config::ImageFormat;

/// The directory (inside the output dir) holding the slide images.
pub const DIR: &str = "slides";

#[derive(Debug, Default, Clone, Copy)]
pub struct Stats {
    pub rendered: usize,
    pub reused: usize,
    pub removed: usize,
}

/// A page's image format, plus its SVG if that was already encoded while deciding.
type Resolved = (ImageFormat, Option<String>);

/// Renders all pages into `dir` and returns the file name for each page.
pub fn render(
    doc: &PagedDocument,
    dir: &Path,
    format: ImageFormat,
    ppi: f32,
    quality: u8,
) -> Result<(Vec<String>, Stats)> {
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    // `Auto` decides from the SVG itself: anything typst-svg embeds as `<image>` (pictures,
    // bitmap emoji, image tilings) is decoded lazily by browsers, so such pages become JPEG.
    // The SVG of the other pages is kept and written below instead of encoding it twice.
    let resolved: Vec<Resolved> = doc
        .pages()
        .par_iter()
        .map(|page| match format {
            ImageFormat::Auto if has_image(&page.frame) => (ImageFormat::Jpeg, None),
            ImageFormat::Auto => {
                let svg = encode_svg(page);
                if svg.contains("<image") {
                    (ImageFormat::Jpeg, None)
                } else {
                    (ImageFormat::Svg, Some(svg))
                }
            }
            other => (other, None),
        })
        .collect();
    let formats: Vec<ImageFormat> = resolved.iter().map(|(f, _)| *f).collect();
    let names: Vec<String> = doc
        .pages()
        .iter()
        .zip(&formats)
        .map(|(page, &format)| file_name(page, format, ppi, quality))
        .collect();

    // Identical pages (e.g. a build step that changes nothing visible) share a file name;
    // render each name once, or two workers would race on the same temp file.
    let mut seen = HashSet::new();
    let jobs: Vec<(&Page, &String, &Resolved)> = doc
        .pages()
        .iter()
        .zip(&names)
        .zip(&resolved)
        .filter(|((_, name), _)| seen.insert(name.as_str()))
        .map(|((page, name), resolved)| (page, name, resolved))
        .collect();
    let duplicates = names.len() - jobs.len();

    let results: Vec<Result<bool>> = jobs
        .into_par_iter()
        .map(|(page, name, (format, svg))| {
            let path = dir.join(name);
            if path.exists() {
                return Ok(false);
            }
            let bytes = match format {
                ImageFormat::Svg => svg.clone().unwrap_or_else(|| encode_svg(page)).into_bytes(),
                ImageFormat::Jpeg => encode(page, ppi, quality)?,
                ImageFormat::Auto => unreachable!("resolved per page above"),
            };
            // Write next to the target and rename, so a server never sees half a file.
            let tmp = dir.join(format!("{name}.tmp"));
            std::fs::write(&tmp, bytes)
                .with_context(|| format!("cannot write {}", tmp.display()))?;
            std::fs::rename(&tmp, &path).with_context(|| {
                format!("cannot rename {} to {}", tmp.display(), path.display())
            })?;
            Ok(true)
        })
        .collect();

    let mut stats = Stats {
        reused: duplicates,
        ..Stats::default()
    };
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
        let base = name.strip_suffix(".tmp").unwrap_or(&name);
        let ours = base.ends_with(".jpg") || base.ends_with(".svg");
        if ours && !keep.contains(name.as_ref()) {
            std::fs::remove_file(entry.path())
                .with_context(|| format!("cannot remove {}", entry.path().display()))?;
            stats.removed += 1;
        }
    }
    Ok((names, stats))
}

fn file_name(page: &Page, format: ImageFormat, ppi: f32, quality: u8) -> String {
    let fill = page.fill_or_white();
    // ppi and quality only matter for JPEG; leave them out for SVG so changing them
    // doesn't re-render anything.
    let (settings, ext) = match format {
        ImageFormat::Svg | ImageFormat::Auto => (None, "svg"),
        ImageFormat::Jpeg => (Some((ppi.to_bits(), quality)), "jpg"),
    };
    let hash = hash128(&(
        Visual(&page.frame),
        fill,
        format,
        settings,
        env!("CARGO_PKG_VERSION"),
    ));
    format!("{hash:032x}.{ext}")
}

/// Whether a frame contains an image (raster or SVG) anywhere.
fn has_image(frame: &Frame) -> bool {
    frame.items().any(|(_, item)| match item {
        FrameItem::Image(..) => true,
        FrameItem::Group(group) => has_image(&group.frame),
        _ => false,
    })
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

/// Renders one page as a self-contained SVG (images are embedded as data URLs).
pub fn encode_svg(page: &Page) -> String {
    typst_svg::svg(page, &typst_svg::SvgOptions::default())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::world::{Mode, Session};

    #[test]
    fn auto_uses_jpeg_only_for_pages_with_images() {
        let dir = tempfile::tempdir().unwrap();
        let mut px = tiny_skia::Pixmap::new(4, 4).unwrap();
        px.fill(tiny_skia::Color::from_rgba8(200, 30, 40, 255));
        px.save_png(dir.path().join("photo.png")).unwrap();
        std::fs::write(
            dir.path().join("main.typ"),
            "#set page(width: 200pt, height: 100pt)\nText only\n#pagebreak()\n#box(image(\"photo.png\", width: 50pt))\n#pagebreak()\n#rect(width: 40pt, height: 40pt, fill: tiling(size: (8pt, 8pt), image(\"photo.png\", width: 8pt)))",
        )
        .unwrap();
        let config = Config::parse("[deck]\nentry = \"main.typ\"", dir.path()).unwrap();
        let session = Session::new(&config).unwrap();
        let doc = crate::compile::compile(&session.world(Mode::Slides))
            .unwrap()
            .doc;
        let out = dir.path().join("slides");
        let (names, stats) = render(&doc, &out, ImageFormat::Auto, 72.0, 90).unwrap();
        assert!(
            names[0].ends_with(".svg") && names[1].ends_with(".jpg"),
            "{names:?}"
        );
        assert_eq!(stats.rendered, 3);
        assert!(
            std::fs::read(out.join(&names[1]))
                .unwrap()
                .starts_with(&[0xFF, 0xD8])
        );
    }

    #[test]
    fn identical_pages_render_once() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("main.typ"),
            "#set page(width: 200pt, height: 100pt)\n#for _ in range(16) { page[Same] }",
        )
        .unwrap();
        let config = Config::parse("[deck]\nentry = \"main.typ\"", dir.path()).unwrap();
        let session = Session::new(&config).unwrap();
        let doc = crate::compile::compile(&session.world(Mode::Slides))
            .unwrap()
            .doc;
        let (names, stats) =
            render(&doc, &dir.path().join("slides"), ImageFormat::Svg, 72.0, 90).unwrap();
        assert!(names.iter().all(|n| *n == names[0]));
        assert_eq!((stats.rendered, stats.reused), (1, 15));
    }
}
