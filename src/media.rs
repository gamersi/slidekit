//! Copying video files referenced by `<slidekit-video>` next to the outputs.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::Config;

#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub copied: usize,
    pub unchanged: usize,
    /// Resolved source files (watch mode rebuilds when they change).
    pub sources: Vec<PathBuf>,
}

/// Copies each file to the same relative path in the output dir. Files are looked up
/// relative to the deck dir first, then relative to the typst root.
pub fn copy<'a>(config: &Config, files: impl IntoIterator<Item = &'a str>) -> Result<Stats> {
    let mut stats = Stats::default();
    let mut seen = std::collections::HashSet::new();
    for file in files {
        if !seen.insert(file) {
            continue;
        }
        let rel = Path::new(file);
        if !rel
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        {
            bail!("video path `{file}` must be relative and stay inside the deck");
        }
        let src = [&config.deck_dir, &config.root]
            .iter()
            .map(|dir| dir.join(rel))
            .find(|p| p.is_file())
            .with_context(|| {
                format!("video `{file}` not found in {}", config.deck_dir.display())
            })?;
        stats.sources.push(src.clone());
        let dst = config.out_dir.join(rel);
        if same_file(&src, &dst) {
            stats.unchanged += 1;
            continue;
        }
        if let Some(dir) = dst.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::copy(&src, &dst)
            .with_context(|| format!("cannot copy {} to {}", src.display(), dst.display()))?;
        // Carry the mtime over so the next build can skip the copy.
        let mtime = std::fs::metadata(&src)?.modified()?;
        std::fs::File::options()
            .write(true)
            .open(&dst)?
            .set_modified(mtime)?;
        stats.copied += 1;
    }
    Ok(stats)
}

/// Same size and modification time.
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.len() == b.len() && a.modified().ok() == b.modified().ok(),
        _ => false,
    }
}
