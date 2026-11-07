//! Versioning of the deck-side library (`lib/slidekit.typ`).
//!
//! The library stamps its contract version into every `<slidekit-slide>`. A deck vendors a
//! copy, so it can fall behind the binary (or, with an old binary, be ahead of it):
//!
//! - an unmodified copy of an older library at the standard path is upgraded in place before
//!   the build, since nothing of the user's can be lost;
//! - otherwise the build warns and says which side to update.

use std::path::Path;

use anyhow::{Context, Result};

use crate::meta::Meta;
use crate::scaffold::{LIB, LIB_PATH};

/// The contract version this binary speaks (keep in sync with `contract` in `assets/slidekit.typ`).
pub const VERSION: u32 = 2;

/// Every library this binary has ever shipped before, oldest first (index + 1 = version).
const OLD: &[&str] = &[include_str!("../assets/contracts/v1.typ")];

/// What `upgrade` did.
#[derive(Debug, PartialEq)]
pub enum Upgrade {
    /// No vendored library at the standard path, or it is current or modified.
    None,
    /// Replaced an unmodified copy of this older version.
    From(u32),
}

/// Replaces `lib/slidekit.typ` in `deck_dir` with the current library if it is a byte-exact
/// (modulo line endings) copy of an older one.
pub fn upgrade(deck_dir: &Path) -> Result<Upgrade> {
    let path = deck_dir.join(LIB_PATH);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Upgrade::None);
    };
    let text = text.replace("\r\n", "\n");
    let Some(i) = OLD.iter().position(|old| old.replace("\r\n", "\n") == text) else {
        return Ok(Upgrade::None);
    };
    std::fs::write(&path, LIB).with_context(|| format!("cannot update {}", path.display()))?;
    Ok(Upgrade::From(i as u32 + 1))
}

/// The contract version declared by a library file (`#let contract = N`; v1 had none).
fn declared(text: &str) -> u32 {
    text.lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("#let contract = ")?
                .trim()
                .parse()
                .ok()
        })
        .unwrap_or(1)
}

/// For a failed compile: if `lib/slidekit.typ` in `deck_dir` speaks another contract version,
/// a hint that this may be why.
pub fn compile_hint(deck_dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(deck_dir.join(LIB_PATH)).ok()?;
    let lib = declared(&text);
    (lib != VERSION).then(|| mismatch(lib).replacen("warning: ", "hint: ", 1))
}

/// A warning if the deck's library speaks another contract version than this binary.
/// Decks without any slide info are not checked.
pub fn check(meta: &Meta) -> Option<String> {
    let first = meta.slides.values().next()?;
    let deck = first.contract.unwrap_or(1);
    (deck != VERSION).then(|| mismatch(deck))
}

fn mismatch(deck: u32) -> String {
    match deck.cmp(&VERSION) {
        std::cmp::Ordering::Less | std::cmp::Ordering::Equal => format!(
            "warning: the deck's slidekit.typ speaks contract v{deck}, this slidekit speaks v{VERSION}.\n  \
             Newer features (transitions, build steps, video options) need the new library: \
             run `slidekit lib` to update {LIB_PATH}\n  \
             (if the deck imports the library from somewhere else, copy {LIB_PATH} there).\n"
        ),
        std::cmp::Ordering::Greater => format!(
            "warning: the deck's slidekit.typ speaks contract v{deck}, but this slidekit {} only knows v{VERSION}.\n  \
             Update slidekit (https://github.com/gamersi/slidekit/releases); \
             until then some slide settings may be ignored.\n",
            env!("CARGO_PKG_VERSION")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_library_declares_this_version() {
        assert!(LIB.contains(&format!("contract v{VERSION}")));
        assert!(LIB.contains(&format!("#let contract = {VERSION}")));
        assert_eq!(OLD.len() as u32, VERSION - 1);
        assert_eq!(declared(LIB), VERSION);
        assert_eq!(declared(OLD[0]), 1);
    }

    #[test]
    fn upgrades_only_unmodified_old_copies() {
        let dir = tempfile::tempdir().unwrap();
        let lib = dir.path().join(LIB_PATH);
        assert_eq!(upgrade(dir.path()).unwrap(), Upgrade::None);

        std::fs::create_dir_all(lib.parent().unwrap()).unwrap();
        std::fs::write(&lib, OLD[0].replace('\n', "\r\n")).unwrap();
        assert_eq!(upgrade(dir.path()).unwrap(), Upgrade::From(1));
        assert_eq!(std::fs::read_to_string(&lib).unwrap(), LIB);
        assert_eq!(upgrade(dir.path()).unwrap(), Upgrade::None);

        let modified = format!("{}\n// my tweak\n", OLD[0]);
        std::fs::write(&lib, &modified).unwrap();
        assert_eq!(upgrade(dir.path()).unwrap(), Upgrade::None);
        assert_eq!(std::fs::read_to_string(&lib).unwrap(), modified);
    }
}
