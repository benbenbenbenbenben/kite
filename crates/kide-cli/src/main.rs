#[cfg(not(target_arch = "wasm32"))]
mod lsp;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kide", version, about = "Continuous DDD architecture verifier")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate a .kide spec file.
    Check {
        #[arg(default_value = "domain/main.kide")]
        file: PathBuf,
    },
    /// Auto-format a .kide spec file.
    Fmt {
        #[arg(default_value = "domain/main.kide")]
        file: PathBuf,
        /// Write changes in place (default: print to stdout)
        #[arg(long)]
        write: bool,
    },
    /// Start the integrated Language Server Protocol endpoint over stdio.
    StartLsp,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file } => {
            let report = kide_core::check_file(&file)?;
            if report.violations.is_empty() {
                println!(
                    "✨ All contexts crystallized. {} context(s) parsed.",
                    report.contexts
                );
            } else {
                for violation in &report.violations {
                    println!(
                        "{} [{}] {}",
                        violation.severity.as_str(),
                        violation.code,
                        violation.message
                    );
                }

                if report.has_errors() {
                    std::process::exit(1);
                }
            }
        }
        Commands::Fmt { file, write } => {
            let source = std::fs::read_to_string(&file)?;
            let formatted = kide_core::format_source(&source)?;
            if write {
                std::fs::write(&file, &formatted)?;
                println!("✨ Formatted {}", file.display());
            } else {
                print!("{}", formatted);
            }
        }
        Commands::StartLsp => run_lsp()?,
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn run_lsp() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(lsp::run_stdio())
}

#[cfg(target_arch = "wasm32")]
fn run_lsp() -> Result<()> {
    Err(anyhow::anyhow!("start-lsp is not available in wasm builds"))
}
