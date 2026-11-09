//! `slidekit agents`: instructions for coding agents, written into a deck.

use std::path::Path;

use anyhow::{Context, Result};

/// Files written into the deck, relative to the deck dir.
pub const FILES: &[(&str, &str)] = &[
    ("AGENTS.md", include_str!("../assets/agents/AGENTS.md")),
    ("CLAUDE.md", "@AGENTS.md\n"),
    (
        ".claude/skills/slidekit/SKILL.md",
        include_str!("../assets/agents/skills/slidekit/SKILL.md"),
    ),
    (
        ".claude/skills/slidekit/design.md",
        include_str!("../assets/agents/skills/slidekit/design.md"),
    ),
    (
        ".claude/skills/slidekit/typst.md",
        include_str!("../assets/agents/skills/slidekit/typst.md"),
    ),
    (
        ".claude/agents/slide-critic.md",
        include_str!("../assets/agents/agents/slide-critic.md"),
    ),
];

/// What happened to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Created,
    Updated,
    Unchanged,
    /// Exists and differs; left alone because `--force` was not given.
    Kept,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(match self {
            Status::Created => "created",
            Status::Updated => "updated",
            Status::Unchanged => "unchanged",
            Status::Kept => "kept",
        })
    }
}

/// Writes the files that are missing; files that differ are only replaced with `force`.
pub fn install(dir: &Path, force: bool) -> Result<Vec<(&'static str, Status)>> {
    let mut out = Vec::new();
    for (name, text) in FILES {
        let path = dir.join(name);
        let status = match std::fs::read(&path) {
            Ok(old) if old == text.as_bytes() => Status::Unchanged,
            Ok(_) if !force => Status::Kept,
            Ok(_) => Status::Updated,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Status::Created,
            Err(e) => return Err(e).with_context(|| format!("cannot read {}", path.display())),
        };
        if matches!(status, Status::Created | Status::Updated) {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, text)
                .with_context(|| format!("cannot write {}", path.display()))?;
        }
        out.push((*name, status));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_keeps_edits_unless_forced() {
        let dir = tempfile::tempdir().unwrap();
        let all = |r: Vec<(&str, Status)>, s| r.iter().all(|(_, st)| *st == s);
        assert!(all(install(dir.path(), false).unwrap(), Status::Created));
        assert!(all(install(dir.path(), false).unwrap(), Status::Unchanged));
        std::fs::write(dir.path().join("AGENTS.md"), "mine").unwrap();
        let r = install(dir.path(), false).unwrap();
        assert_eq!(r[0], ("AGENTS.md", Status::Kept));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap(),
            "mine"
        );
        assert_eq!(
            install(dir.path(), true).unwrap()[0],
            ("AGENTS.md", Status::Updated)
        );
    }
}
