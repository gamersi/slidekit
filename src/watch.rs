//! Rebuilding on change. The [`Builder`] (and with it the typst session) is reused, so
//! comemo's memoization keeps incremental compiles fast.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;

use crate::build::{Builder, Options, Summary, fmt_duration};
use crate::config::{self, Config};

/// Builds once, then rebuilds whenever a relevant file changes. `on_build` runs after
/// every successful build (used by `serve` for live reload). Runs until the process ends.
pub fn run(
    dir: &Path,
    options: Options,
    quiet: bool,
    mut on_build: impl FnMut(&Summary),
) -> Result<()> {
    let mut builder = Builder::new(Config::load(dir)?, options)?;
    let (tx, rx) = mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(150), tx)?;
    let deck_dir = builder.config.deck_dir.clone();
    debouncer
        .watcher()
        .watch(&deck_dir, RecursiveMode::Recursive)
        .context("cannot watch the deck dir")?;

    let mut extra_watched: HashSet<PathBuf> = HashSet::new();
    loop {
        let start = Instant::now();
        let started = SystemTime::now();
        let result = builder.build();
        let ok = result.is_ok();
        report(&result, quiet, start.elapsed());
        let mut relevant: HashSet<PathBuf> = builder.dependencies().into_iter().collect();
        if let Ok(summary) = &result {
            relevant.extend(summary.media.iter().flat_map(|m| m.sources.iter().cloned()));
            on_build(summary);
        }
        // Keep memory bounded across many rebuilds, like the typst CLI does.
        comemo::evict(10);

        // Files outside the deck dir (e.g. with a parent `root`): watch their directories.
        let dirs: HashSet<PathBuf> = relevant
            .iter()
            .filter(|p| !p.starts_with(&deck_dir))
            .filter_map(|p| p.parent().map(Path::to_path_buf))
            .collect();
        for dir in dirs.difference(&extra_watched.clone()) {
            if debouncer
                .watcher()
                .watch(dir, RecursiveMode::NonRecursive)
                .is_ok()
            {
                extra_watched.insert(dir.clone());
            }
        }
        if !quiet {
            eprintln!("watching {} for changes…", deck_dir.display());
        }

        // Wait for a change that matters.
        let config_file = deck_dir.join(config::FILE_NAME);
        let changed = loop {
            let events = match rx.recv() {
                Ok(Ok(events)) => events,
                Ok(Err(err)) => {
                    eprintln!("watch error: {err}");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            let out_dir = &builder.config.out_dir;
            let paths: Vec<PathBuf> = events
                .into_iter()
                .map(|e| e.path)
                .filter(|p| !p.starts_with(out_dir) && modified_since(p, started))
                .collect();
            if paths.contains(&config_file) {
                break Change::Config;
            }
            // After a failed build the dependency list may be incomplete: any change counts.
            if paths
                .iter()
                .any(|p| relevant.contains(p) || (!ok && !is_temp_file(p)))
            {
                break Change::Sources;
            }
        };
        if let Change::Config = changed {
            match Config::load(dir).and_then(|c| Builder::new(c, options)) {
                Ok(b) => builder = b,
                Err(err) => eprintln!("error: {err:#}"),
            }
        }
    }
}

enum Change {
    Sources,
    Config,
}

/// Whether the file was written (or removed) after `time`. Events alone are not enough:
/// inotify also reports reads, e.g. typst reading the sources during the build.
fn modified_since(path: &Path, time: SystemTime) -> bool {
    match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(mtime) => mtime >= time,
        Err(_) => true,
    }
}

fn is_temp_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    name.starts_with(".#")
        || name.ends_with('~')
        || name.ends_with(".swp")
        || name.ends_with(".tmp")
}

fn report(result: &Result<Summary>, quiet: bool, elapsed: Duration) {
    match result {
        Ok(summary) if !quiet => eprintln!("{}", summary.report()),
        Ok(summary) => summary.warnings.iter().for_each(|w| eprint!("{w}")),
        Err(err) => eprintln!(
            "error: {err:#}\nbuild failed after {}",
            fmt_duration(elapsed)
        ),
    }
}
