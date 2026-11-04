//! Compiling a deck in one mode and turning diagnostics into readable text.

use std::io::IsTerminal;

use typst::diag::{SourceDiagnostic, SourceResult};
use typst_kit::diagnostics::{self, DiagnosticFormat, termcolor};
use typst_layout::PagedDocument;

use crate::world::ModeWorld;

/// A successful compilation; `warnings` are already formatted.
pub struct Compiled {
    pub doc: PagedDocument,
    pub warnings: Vec<String>,
}

/// Compiles the deck; on failure the error carries the formatted diagnostics.
pub fn compile(world: &ModeWorld) -> anyhow::Result<Compiled> {
    let warned = typst::compile::<PagedDocument>(world);
    let warnings = format_each(world, &warned.warnings);
    match warned.output {
        Ok(doc) => Ok(Compiled { doc, warnings }),
        Err(errors) => {
            let mut text = warnings.concat();
            text.push_str(&format_each(world, &errors).concat());
            anyhow::bail!("compilation failed\n{}", text.trim_end())
        }
    }
}

/// Converts a typst result (e.g. from PDF export) into an `anyhow` result with diagnostics.
pub fn check<T>(world: &ModeWorld, result: SourceResult<T>) -> anyhow::Result<T> {
    result.map_err(|errors| anyhow::anyhow!("{}", format_each(world, &errors).concat().trim_end()))
}

/// Formats each diagnostic separately (`file:line:col` plus a source snippet and hints).
pub fn format_each(world: &ModeWorld, diags: &[SourceDiagnostic]) -> Vec<String> {
    diags
        .iter()
        .map(|diag| {
            let mut buf = if std::io::stderr().is_terminal() {
                termcolor::Buffer::ansi()
            } else {
                termcolor::Buffer::no_color()
            };
            match diagnostics::emit(&mut buf, world, [diag], DiagnosticFormat::Human) {
                Ok(()) => String::from_utf8_lossy(buf.as_slice()).into_owned(),
                Err(_) => format!("error: {}\n", diag.message),
            }
        })
        .collect()
}
