//! slidekit: Typst deck → PDF, speaker handout and an offline HTML player with presenter view.

mod build;
mod compile;
mod config;
mod contract;
mod export;
mod links;
mod media;
mod meta;
mod notes;
mod player;
mod render;
mod scaffold;
mod serve;
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
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
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
            Ok(())
        }
        Command::Watch { dir } => watch::run(&dir, options, cli.quiet, |_| {}),
        Command::Serve { dir, port, open } => serve::run(&dir, port, open, options, cli.quiet),
        Command::Init { dir } => {
            let files = scaffold::init(&dir)?;
            eprintln!("created {} in {}", files.join(", "), dir.display());
            eprintln!("next: slidekit serve {} --open", dir.display());
            Ok(())
        }
        Command::Lib { dir } => {
            let changed = scaffold::lib(&dir)?;
            let path = dir.join(scaffold::LIB_PATH);
            eprintln!(
                "{} {}",
                if changed { "wrote" } else { "up to date:" },
                path.display()
            );
            Ok(())
        }
    }
}
