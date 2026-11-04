//! The `typst::World` implementation.
//!
//! A [`Session`] owns everything that is shared between compilations (file cache, fonts,
//! packages) and lives as long as the process, so that watch mode benefits from comemo's
//! memoization. Each compilation borrows a [`ModeWorld`], which only adds the standard library
//! with the mode-specific `sys.inputs`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use typst::diag::FileResult;
use typst::foundations::{Bytes, Datetime, Dict, Duration, IntoValue};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::datetime::Time;
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::downloader::SystemDownloader;
use typst_kit::files::{FileStore, FsRoot, SystemFiles};
use typst_kit::fonts::{self, FontStore};
use typst_kit::packages::{FsPackages, SystemPackages, UniversePackages};

use crate::config::Config;

/// The `sys.inputs` key that tells the deck what to produce.
pub const MODE_INPUT: &str = "slidekit-mode";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Slides,
    Handout,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Slides => "slides",
            Mode::Handout => "handout",
        }
    }
}

/// State shared by all compilations of one deck.
pub struct Session {
    main: FileId,
    files: FileStore<SystemFiles>,
    fonts: FontStore,
    time: Time,
    slides: LazyHash<Library>,
    handout: LazyHash<Library>,
}

impl Session {
    pub fn new(config: &Config) -> Result<Self> {
        let vpath = VirtualPath::virtualize(&config.root, &config.entry)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("entry file is not inside the root")?;
        let main = RootedPath::new(VirtualRoot::Project, vpath).intern();

        let downloader = SystemDownloader::new(concat!("slidekit/", env!("CARGO_PKG_VERSION")));
        let env_dir = |var: &str| std::env::var_os(var).map(FsPackages::new);
        let packages = SystemPackages::from_parts(
            env_dir("TYPST_PACKAGE_PATH").or_else(FsPackages::system_data),
            env_dir("TYPST_PACKAGE_CACHE_PATH").or_else(FsPackages::system_cache),
            UniversePackages::new(downloader),
        );
        let files = FileStore::new(SystemFiles::new(FsRoot::new(config.root.clone()), packages));

        let mut fonts = FontStore::new();
        for dir in &config.fonts {
            fonts.extend(fonts::scan(dir));
        }
        if config.system_fonts {
            fonts.extend(fonts::system());
        }
        fonts.extend(fonts::embedded());

        let library = |mode: Mode| {
            let mut inputs: Dict = config
                .inputs
                .iter()
                .map(|(k, v)| (k.as_str().into(), v.as_str().into_value()))
                .collect();
            inputs.insert(MODE_INPUT.into(), mode.as_str().into_value());
            LazyHash::new(Library::builder().with_inputs(inputs).build())
        };

        Ok(Self {
            main,
            files,
            fonts,
            time: Time::system(),
            slides: library(Mode::Slides),
            handout: library(Mode::Handout),
        })
    }

    pub fn world(&self, mode: Mode) -> ModeWorld<'_> {
        let library = match mode {
            Mode::Slides => &self.slides,
            Mode::Handout => &self.handout,
        };
        ModeWorld {
            session: self,
            library,
        }
    }

    /// Marks all files as stale before the next compilation (sources are edited in place,
    /// which keeps incremental compilation fast).
    pub fn reset(&mut self) {
        self.files.reset();
        self.time.reset();
    }

    /// Real paths of all project and package files read since the last reset.
    pub fn dependencies(&mut self) -> Vec<PathBuf> {
        let (loader, ids) = self.files.dependencies();
        let mut paths: Vec<_> = ids.filter_map(|id| loader.resolve(id).ok()).collect();
        paths.sort();
        paths
    }

    /// Resolves a file id to a path for display.
    fn path(&self, id: FileId) -> Option<PathBuf> {
        self.files.loader().resolve(id).ok()
    }
}

/// A [`World`] for one mode, borrowing the shared session.
pub struct ModeWorld<'a> {
    session: &'a Session,
    library: &'a LazyHash<Library>,
}

impl World for ModeWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.session.fonts.book()
    }

    fn main(&self) -> FileId {
        self.session.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.session.files.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.session.files.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.session.fonts.font(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.session.time.today(offset)
    }
}

impl DiagnosticWorld for ModeWorld<'_> {
    fn name(&self, id: FileId) -> String {
        let fallback = || format!("{:?}", id.get());
        let Some(path) = self.session.path(id) else {
            return fallback();
        };
        let cwd = std::env::current_dir().unwrap_or_default();
        path.strip_prefix(&cwd)
            .map(Path::to_path_buf)
            .unwrap_or(path)
            .display()
            .to_string()
    }
}
