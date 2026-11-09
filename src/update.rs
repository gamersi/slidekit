//! `slidekit update`: replace this binary with the latest GitHub release.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use typst_kit::downloader::{Downloader, SystemDownloader};

const REPO: &str = "gamersi/slidekit";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The release target this binary was built for, if releases ship one.
const TARGET: Option<&str> = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Some("x86_64-unknown-linux-gnu")
} else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
    Some("x86_64-pc-windows-msvc")
} else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    Some("aarch64-apple-darwin")
} else {
    None
};

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    assets: Vec<Asset>,
}

#[derive(serde::Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Checks for a newer release and, unless `check_only`, installs it. `force` reinstalls
/// the latest release even if it is not newer.
pub fn run(check_only: bool, force: bool) -> Result<()> {
    remove_leftover();
    let downloader = SystemDownloader::new(concat!("slidekit/", env!("CARGO_PKG_VERSION")));
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let bytes = downloader
        .download(&(), &url)
        .with_context(|| format!("cannot reach {url}"))?;
    let release: Release =
        serde_json::from_slice(&bytes).context("unexpected answer from the GitHub API")?;
    let latest = release.tag_name.trim_start_matches('v');

    let newer = parse(latest) > parse(VERSION);
    if !newer && !force {
        eprintln!("slidekit {VERSION} is up to date (latest release: {latest})");
        return Ok(());
    }
    if newer {
        eprintln!("slidekit {latest} is available (this is {VERSION})");
    }
    if check_only {
        eprintln!(
            "run `slidekit update` to install it, or see {}",
            release.html_url
        );
        return Ok(());
    }

    let Some(target) = TARGET else {
        bail!(
            "there is no prebuilt binary for this platform; update with\n  \
             cargo install --git https://github.com/{REPO}"
        );
    };
    let exe_suffix = std::env::consts::EXE_SUFFIX;
    let name = format!("slidekit-{}-{target}{exe_suffix}", release.tag_name);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == name)
        .with_context(|| {
            format!(
                "release {} has no {name} (yet?); download it by hand from {}",
                release.tag_name, release.html_url
            )
        })?;

    let current = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .context("cannot find the running slidekit binary")?;
    let dir = current.parent().context("binary has no parent directory")?;

    eprintln!("downloading {}", asset.browser_download_url);
    let binary = downloader
        .download(&(), &asset.browser_download_url)
        .with_context(|| format!("cannot download {}", asset.browser_download_url))?;

    let new = dir.join(format!(".slidekit-update{exe_suffix}"));
    write_executable(&new, &binary)?;
    // Make sure the download runs before it replaces anything.
    let check = std::process::Command::new(&new).arg("--version").output();
    match check {
        Ok(out) if out.status.success() => {}
        _ => {
            let _ = std::fs::remove_file(&new);
            bail!("the downloaded binary does not run on this machine; nothing was changed");
        }
    }
    replace(&current, &new)?;
    eprintln!("updated {} to {latest}", current.display());
    eprintln!(
        "decks: builds upgrade an unmodified lib/slidekit.typ by themselves; `slidekit lib DIR` \
         forces it, `slidekit agents DIR --force` refreshes the agent files"
    );
    Ok(())
}

/// "0.2.10" → (0, 2, 10); anything unparsable counts as 0.
fn parse(version: &str) -> (u64, u64, u64) {
    let mut parts = version
        .split(['.', '-', '+'])
        .map(|p| p.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn write_executable(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes).with_context(|| {
        format!(
            "cannot write {} (no permission? then reinstall where you can write)",
            path.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Where the replaced binary is parked on Windows, which cannot delete a running exe.
fn parked(current: &Path) -> PathBuf {
    current.with_file_name(format!(".slidekit-old{}", std::env::consts::EXE_SUFFIX))
}

/// Swaps `new` into the place of `current`. Unix can rename over a running binary;
/// Windows can only rename it away first.
fn replace(current: &Path, new: &Path) -> Result<()> {
    if cfg!(windows) {
        let old = parked(current);
        let _ = std::fs::remove_file(&old);
        std::fs::rename(current, &old)
            .with_context(|| format!("cannot move {} aside", current.display()))?;
        if let Err(err) = std::fs::rename(new, current) {
            let _ = std::fs::rename(&old, current);
            return Err(err).with_context(|| format!("cannot replace {}", current.display()));
        }
    } else {
        std::fs::rename(new, current)
            .with_context(|| format!("cannot replace {}", current.display()))?;
    }
    Ok(())
}

/// Best effort: delete a binary parked by an earlier update.
fn remove_leftover() {
    if let Ok(current) = std::env::current_exe() {
        let _ = std::fs::remove_file(parked(&current));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(parse("0.2.10") > parse("0.2.9"));
        assert!(parse("1.0.0") > parse("0.99.99"));
        assert_eq!(parse("0.2.2"), (0, 2, 2));
        assert_eq!(parse("0.3"), (0, 3, 0));
    }
}
