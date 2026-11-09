//! slidekit: Typst deck → PDF, speaker handout and an offline HTML player with presenter view.

mod agents;
mod build;
mod compile;
mod config;
mod contract;
mod export;
mod links;
mod lint;
mod media;
mod meta;
mod notes;
mod player;
mod preview;
mod render;
mod scaffold;
mod serve;
mod update;
mod watch;
mod world;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Only print warnings and errors.
    #[arg(long, short, global = true)]
    quiet: bool,
    /// Skip the speaker handout.
    #[arg(long, global = true)]
    no_handout: bool,
    /// Skip the HTML player (and slide images).
    #[arg(long, global = true)]
    no_html: bool,
    /// Skip the slides PDF.
    #[arg(long, global = true)]
    no_pdf: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Build all outputs once. DIR must contain slidekit.toml.
    Build {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Build, then rebuild incrementally on every change.
    Watch {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Watch and serve the output dir over HTTP with live reload.
    Serve {
        #[arg(default_value = ".")]
        dir: PathBuf,
        #[arg(long, default_value_t = 8765)]
        port: u16,
        /// Open the player in the browser after the first build.
        #[arg(long)]
        open: bool,
    },
    /// Create a minimal example deck.
    Init {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// (Re)write the vendored lib/slidekit.typ.
    Lib {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Render pages to numbered PNGs, print a page manifest and check the layout.
    ///
    /// Meant for looking at a deck without a browser (e.g. by a coding agent). Exits with
    /// status 2 if there are warnings, so scripts can treat a clean preview as a gate.
    Preview {
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Pages to render, e.g. "3" or "1,4-6" (default: all).
        #[arg(long)]
        pages: Option<String>,
        /// Width of each image in pixels.
        #[arg(long, default_value_t = 1280)]
        width: u32,
        /// Also write sheet.png: all rendered pages in one grid image.
        #[arg(long)]
        sheet: bool,
        /// Preview the handout instead of the slides.
        #[arg(long)]
        handout: bool,
        /// Output directory (default: <output dir>/preview).
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Replace this binary with the latest release from GitHub.
    Update {
        /// Only report whether a newer release exists.
        #[arg(long)]
        check: bool,
        /// Reinstall the latest release even if it is not newer.
        #[arg(long)]
        force: bool,
    },
    /// Add coding-agent instructions to a deck: AGENTS.md, CLAUDE.md, a Claude Code skill
    /// and a slide-critic subagent. Existing files that differ are kept unless --force.
    Agents {
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Overwrite files that differ from the built-in copies.
        #[arg(long)]
        force: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let options = build::Options {
        pdf: !cli.no_pdf,
        handout: !cli.no_handout,
        html: !cli.no_html,
    };
    match cli.command {
        Command::Build { dir } => {
            let mut builder = build::Builder::new(config::Config::load(&dir)?, options)?;
            let summary = builder.build()?;
            if cli.quiet {
                summary.warnings.iter().for_each(|w| eprint!("{w}"));
            } else {
                eprintln!("{}", summary.report());
                for path in &summary.outputs {
                    eprintln!("  → {}", path.display());
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Watch { dir } => {
            watch::run(&dir, options, cli.quiet, |_| {})?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Serve { dir, port, open } => {
            serve::run(&dir, port, open, options, cli.quiet)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Init { dir } => {
            let files = scaffold::init(&dir)?;
            eprintln!("created {} in {}", files.join(", "), dir.display());
            eprintln!("next: slidekit serve {} --open", dir.display());
            eprintln!("using a coding agent? slidekit agents {}", dir.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Preview {
            dir,
            pages,
            width,
            sheet,
            handout,
            out,
        } => {
            let config = config::Config::load(&dir)?;
            let options = preview::Options {
                mode: if handout {
                    world::Mode::Handout
                } else {
                    world::Mode::Slides
                },
                width: width.clamp(64, 8192),
                pages: pages.as_deref().map(preview::parse_pages).transpose()?,
                sheet,
                out,
            };
            let result = preview::run(&config, &options)?;
            print!("{}", result.manifest);
            result.warnings.iter().for_each(|w| eprint!("{w}"));
            if result.warnings.is_empty() {
                if !cli.quiet {
                    eprintln!("no warnings");
                }
                Ok(ExitCode::SUCCESS)
            } else {
                eprintln!("{} warning(s)", result.warnings.len());
                Ok(ExitCode::from(2))
            }
        }
        Command::Update { check, force } => {
            update::run(check, force)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Agents { dir, force } => {
            let results = agents::install(&dir, force)?;
            for (path, status) in &results {
                eprintln!("{status:>9}  {}", dir.join(path).display());
            }
            if results.iter().any(|(_, s)| *s == agents::Status::Kept) {
                eprintln!("kept files differ from this slidekit's copies; --force overwrites them");
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Lib { dir } => {
            let changed = scaffold::lib(&dir)?;
            let path = dir.join(scaffold::LIB_PATH);
            eprintln!(
                "{} {}",
                if changed { "wrote" } else { "up to date:" },
                path.display()
            );
            Ok(ExitCode::SUCCESS)
        }
    }
}
