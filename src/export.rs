//! PDF export for slides and handout.

use std::path::Path;

use anyhow::{Context, Result};
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::compile;
use crate::world::ModeWorld;

pub fn pdf(world: &ModeWorld, doc: &PagedDocument, path: &Path) -> Result<()> {
    let bytes = compile::check(world, typst_pdf::pdf(doc, &PdfOptions::default()))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Replace atomically so a viewer or server never reads a half-written file.
    let tmp = path.with_extension("pdf.tmp");
    std::fs::write(&tmp, bytes).with_context(|| format!("cannot write {}", tmp.display()))?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
