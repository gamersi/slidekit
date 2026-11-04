//! `init` (new example deck) and `lib` (rewrite the vendored contract library).

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::config;

/// The deck-side contract library.
pub const LIB: &str = include_str!("../assets/slidekit.typ");
/// Where decks vendor the library, relative to the deck dir.
pub const LIB_PATH: &str = "lib/slidekit.typ";

const TEMPLATE: &[(&str, &str)] = &[
    (
        config::FILE_NAME,
        include_str!("../assets/template/slidekit.toml"),
    ),
    ("slides.typ", include_str!("../assets/template/slides.typ")),
    (LIB_PATH, LIB),
    (".gitignore", "/dist\n"),
];

/// Creates a minimal deck in `dir` (created if missing). Never overwrites files.
pub fn init(dir: &Path) -> Result<Vec<String>> {
    if let Some((name, _)) = TEMPLATE.iter().find(|(name, _)| dir.join(name).exists()) {
        bail!(
            "{} already exists, not overwriting anything",
            dir.join(name).display()
        );
    }
    for (name, text) in TEMPLATE {
        write(&dir.join(name), text)?;
    }
    Ok(TEMPLATE.iter().map(|(name, _)| name.to_string()).collect())
}

/// (Re)writes `lib/slidekit.typ`; returns whether the file changed.
pub fn lib(dir: &Path) -> Result<bool> {
    let path = dir.join(LIB_PATH);
    if std::fs::read_to_string(&path).is_ok_and(|old| old == LIB) {
        return Ok(false);
    }
    write(&path, LIB)?;
    Ok(true)
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_then_lib() {
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("talk");
        init(&deck).unwrap();
        assert!(deck.join("slidekit.toml").is_file());
        assert!(init(&deck).is_err(), "init must not overwrite");
        assert!(!lib(&deck).unwrap());
        std::fs::write(deck.join(LIB_PATH), "old").unwrap();
        assert!(lib(&deck).unwrap());
        assert_eq!(std::fs::read_to_string(deck.join(LIB_PATH)).unwrap(), LIB);
        config::Config::load(&deck).unwrap();
    }
}
