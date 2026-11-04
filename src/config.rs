//! `slidekit.toml`: loading, defaults, validation and path resolution.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

pub const FILE_NAME: &str = "slidekit.toml";

/// The raw file as written by the user.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    deck: RawDeck,
    #[serde(default)]
    output: RawOutput,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct RawDeck {
    entry: PathBuf,
    title: Option<String>,
    #[serde(default = "dot")]
    root: PathBuf,
    #[serde(default)]
    fonts: Vec<PathBuf>,
    #[serde(default)]
    system_fonts: bool,
    #[serde(default)]
    inputs: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case", default)]
struct RawOutput {
    dir: PathBuf,
    pdf: String,
    handout: String,
    html: String,
    ppi: f32,
    jpeg_quality: u8,
}

impl Default for RawOutput {
    fn default() -> Self {
        Self {
            dir: "dist".into(),
            pdf: "slides.pdf".into(),
            handout: "handout.pdf".into(),
            html: "index.html".into(),
            ppi: 216.0,
            jpeg_quality: 92,
        }
    }
}

fn dot() -> PathBuf {
    ".".into()
}

/// A validated configuration with absolute paths.
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory containing `slidekit.toml`.
    pub deck_dir: PathBuf,
    /// Typst project root (may be a parent of `deck_dir`).
    pub root: PathBuf,
    /// Main typst file.
    pub entry: PathBuf,
    pub title: String,
    pub fonts: Vec<PathBuf>,
    pub system_fonts: bool,
    pub inputs: BTreeMap<String, String>,
    pub out_dir: PathBuf,
    /// Output files; `None` when disabled in the config.
    pub pdf: Option<PathBuf>,
    pub handout: Option<PathBuf>,
    pub html: Option<PathBuf>,
    pub ppi: f32,
    pub jpeg_quality: u8,
}

impl Config {
    /// Loads `slidekit.toml` from `dir`.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(FILE_NAME);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let deck_dir = std::path::absolute(dir)?;
        Self::parse(&text, &deck_dir).with_context(|| format!("invalid {}", path.display()))
    }

    /// Parses and validates the config text; relative paths are resolved against `deck_dir`.
    pub fn parse(text: &str, deck_dir: &Path) -> Result<Self> {
        let raw: RawConfig = toml::from_str(text)?;
        let deck_dir = normalize(deck_dir);
        let resolve = |p: &Path| normalize(&deck_dir.join(p));

        let root = resolve(&raw.deck.root);
        let entry = resolve(&raw.deck.entry);
        if !entry.starts_with(&root) {
            bail!(
                "deck.entry {} is outside deck.root {}",
                entry.display(),
                root.display()
            );
        }
        if !entry.is_file() {
            bail!("deck.entry {} does not exist", entry.display());
        }

        let out_dir = resolve(&raw.output.dir);
        if out_dir == deck_dir || out_dir == root || deck_dir.starts_with(&out_dir) {
            bail!("output.dir must be a separate directory inside or next to the deck");
        }
        if !(36.0..=1200.0).contains(&raw.output.ppi) {
            bail!(
                "output.ppi must be between 36 and 1200, got {}",
                raw.output.ppi
            );
        }
        if !(1..=100).contains(&raw.output.jpeg_quality) {
            bail!("output.jpeg-quality must be between 1 and 100");
        }
        if raw.deck.inputs.contains_key(crate::world::MODE_INPUT) {
            bail!(
                "deck.inputs must not set `{}`, slidekit sets it",
                crate::world::MODE_INPUT
            );
        }

        let output_file = |name: &str| -> Result<Option<PathBuf>> {
            if name.is_empty() {
                return Ok(None);
            }
            let p = Path::new(name);
            if p.is_absolute()
                || p.components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                bail!("output file `{name}` must be a plain relative path");
            }
            Ok(Some(out_dir.join(p)))
        };

        let title = raw.deck.title.unwrap_or_else(|| {
            entry
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        });

        Ok(Self {
            fonts: raw.deck.fonts.iter().map(|p| resolve(p)).collect(),
            system_fonts: raw.deck.system_fonts,
            inputs: raw.deck.inputs,
            pdf: output_file(&raw.output.pdf)?,
            handout: output_file(&raw.output.handout)?,
            html: output_file(&raw.output.html)?,
            ppi: raw.output.ppi,
            jpeg_quality: raw.output.jpeg_quality,
            deck_dir,
            root,
            entry,
            title,
            out_dir,
        })
    }
}

/// Lexically removes `.` and `..` components (no symlink resolution).
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deck() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.typ"), "= Hi").unwrap();
        dir
    }

    #[test]
    fn defaults() {
        let dir = deck();
        let c = Config::parse("[deck]\nentry = \"main.typ\"\n", dir.path()).unwrap();
        assert_eq!(c.title, "main");
        assert_eq!(c.root, normalize(dir.path()));
        assert_eq!(c.out_dir, dir.path().join("dist"));
        assert_eq!(c.pdf, Some(dir.path().join("dist/slides.pdf")));
        assert_eq!(c.handout, Some(dir.path().join("dist/handout.pdf")));
        assert_eq!(c.html, Some(dir.path().join("dist/index.html")));
        assert_eq!(c.ppi, 216.0);
        assert_eq!(c.jpeg_quality, 92);
        assert!(!c.system_fonts);
    }

    #[test]
    fn full() {
        let dir = deck();
        let sub = dir.path().join("talk");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("deck.typ"), "").unwrap();
        let text = r#"
            [deck]
            entry = "deck.typ"
            title = "Talk"
            root = ".."
            fonts = ["../fonts"]
            system-fonts = true
            inputs = { version = "long" }
            [output]
            dir = "out"
            pdf = ""
            handout = "notes.pdf"
            ppi = 144
            jpeg-quality = 80
        "#;
        let c = Config::parse(text, &sub).unwrap();
        assert_eq!(c.title, "Talk");
        assert_eq!(c.root, normalize(dir.path()));
        assert_eq!(c.fonts, vec![dir.path().join("fonts")]);
        assert!(c.system_fonts);
        assert_eq!(c.inputs["version"], "long");
        assert_eq!(c.pdf, None);
        assert_eq!(c.handout, Some(sub.join("out/notes.pdf")));
        assert_eq!(c.ppi, 144.0);
        assert_eq!(c.jpeg_quality, 80);
    }

    #[test]
    fn rejects_bad_configs() {
        let dir = deck();
        let bad = [
            "[deck]\nentry = \"missing.typ\"\n",
            "[deck]\nentry = \"main.typ\"\nunknown = 1\n",
            "[deck]\nentry = \"main.typ\"\n[output]\ndir = \".\"\n",
            "[deck]\nentry = \"main.typ\"\n[output]\njpeg-quality = 0\n",
            "[deck]\nentry = \"main.typ\"\n[output]\npdf = \"../x.pdf\"\n",
            "[deck]\nentry = \"main.typ\"\ninputs = { slidekit-mode = \"x\" }\n",
            "[output]\ndir = \"x\"\n",
        ];
        for text in bad {
            assert!(Config::parse(text, dir.path()).is_err(), "accepted: {text}");
        }
    }
}
