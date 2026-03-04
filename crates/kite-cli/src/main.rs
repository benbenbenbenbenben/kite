#[cfg(not(target_arch = "wasm32"))]
mod lsp;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kite", version, about = "Continuous DDD architecture verifier")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Default, Debug)]
enum Format {
    #[default]
    Human,
    Tap,
    Ctrf,
    Junit,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse and validate .kite spec files (single file or directory).
    Check {
        #[arg(default_value = "domain")]
        file: PathBuf,
        /// Output format
        #[arg(short, long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    /// Auto-format a .kite spec file.
    Fmt {
        #[arg(default_value = "domain/main.kite")]
        file: PathBuf,
        /// Write changes in place (default: print to stdout)
        #[arg(long)]
        write: bool,
    },
    /// Scaffold a .kite domain file from an existing codebase.
    Init {
        /// Source directory to scan for code files
        #[arg(default_value = "src")]
        source: PathBuf,
        /// Output .kite file path
        #[arg(short, long, default_value = "domain/main.kite")]
        output: PathBuf,
    },
    /// Start the integrated Language Server Protocol endpoint over stdio.
    StartLsp,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { file, format } => {
            let report = kite_core::check_file(&file)?;
            match format {
                Format::Human => report_human(&report),
                Format::Tap => report_tap(&report),
                Format::Ctrf => report_ctrf(&report),
                Format::Junit => report_junit(&report),
            }

            if report.has_errors() {
                std::process::exit(1);
            }
        }
        Commands::Fmt { file, write } => {
            let source = std::fs::read_to_string(&file)?;
            let formatted = kite_core::format_source(&source)?;
            if write {
                std::fs::write(&file, &formatted)?;
                println!("🪁 Formatted {}", file.display());
            } else {
                print!("{}", formatted);
            }
        }
        Commands::Init { source, output } => {
            let output_dir = output.parent().unwrap_or_else(|| std::path::Path::new("."));
            let scaffold = kite_core::scaffold(&source, output_dir)?;
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&output, &scaffold)?;
            println!(
                "🪁 Scaffolded {} from {}",
                output.display(),
                source.display()
            );
        }
        Commands::StartLsp => run_lsp()?,
    }

    Ok(())
}

fn report_human(report: &kite_core::CheckReport) {
    if report.violations.is_empty() {
        println!(
            "🪁 \x1b[32mAll contexts verified.\x1b[0m {} context(s) parsed.",
            report.contexts
        );
    } else {
        let mut errors = 0;
        let mut warnings = 0;
        let mut infos = 0;

        for violation in &report.violations {
            let (prefix, color) = match violation.severity {
                kite_core::ViolationSeverity::Error => {
                    errors += 1;
                    ("error", "\x1b[31m")
                }
                kite_core::ViolationSeverity::Warning => {
                    warnings += 1;
                    ("warning", "\x1b[33m")
                }
                kite_core::ViolationSeverity::Information => {
                    infos += 1;
                    ("info", "\x1b[36m")
                }
            };

            println!(
                "{}{} [{}]\x1b[0m {}",
                color,
                prefix,
                violation.code,
                violation.message
            );

            if let Some(span) = violation.span {
                println!(
                    "  {} at line {}, col {}",
                    "\x1b[2m→\x1b[0m", span.start_line, span.start_column
                );
            }

            if let Some(hint) = &violation.hint {
                println!("  \x1b[2mhint: {}\x1b[0m", hint);
            }
        }

        println!();
        print!("🪁 Verified {} context(s) with ", report.contexts);
        if errors > 0 {
            print!("\x1b[31m{} error(s)\x1b[0m", errors);
        } else {
            print!("0 errors");
        }
        print!(", ");
        if warnings > 0 {
            print!("\x1b[33m{} warning(s)\x1b[0m", warnings);
        } else {
            print!("0 warnings");
        }
        if infos > 0 {
            print!(", \x1b[36m{} info(s)\x1b[0m", infos);
        }
        println!(".");
    }
}

fn report_tap(report: &kite_core::CheckReport) {
    println!("TAP version 13");
    let total = report.violations.len().max(1);
    println!("1..{}", total);

    if report.violations.is_empty() {
        println!("ok 1 - all contexts verified");
    } else {
        for (i, violation) in report.violations.iter().enumerate() {
            let ok = if violation.severity == kite_core::ViolationSeverity::Error {
                "not ok"
            } else {
                "ok"
            };
            println!("{} {} - {}", ok, i + 1, violation.code);
            println!("  ---");
            println!("  message: {:?}", violation.message);
            println!("  severity: {:?}", violation.severity.as_str());
            if let Some(span) = violation.span {
                println!("  at: line {}, col {}", span.start_line, span.start_column);
            }
            if let Some(hint) = &violation.hint {
                println!("  hint: {:?}", hint);
            }
            println!("  ...");
        }
    }
}

fn report_ctrf(report: &kite_core::CheckReport) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut tests = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    if report.violations.is_empty() {
        passed = 1;
        tests.push(serde_json::json!({
            "name": "Verification",
            "status": "passed",
            "duration": 0
        }));
    } else {
        for violation in &report.violations {
            let status = if violation.severity == kite_core::ViolationSeverity::Error {
                failed += 1;
                "failed"
            } else {
                passed += 1;
                "passed"
            };

            tests.push(serde_json::json!({
                "name": violation.code,
                "status": status,
                "duration": 0,
                "message": violation.message,
                "trace": violation.hint.clone().unwrap_or_default()
            }));
        }
    }

    let ctrf = serde_json::json!({
        "results": {
            "tool": {
                "name": "kite"
            },
            "summary": {
                "tests": tests.len(),
                "passed": passed,
                "failed": failed,
                "skipped": 0,
                "pending": 0,
                "other": 0,
                "start": now,
                "stop": now
            },
            "tests": tests
        }
    });

    println!("{}", serde_json::to_string_pretty(&ctrf).unwrap());
}

fn report_junit(report: &kite_core::CheckReport) {
    println!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let failures = report
        .violations
        .iter()
        .filter(|v| v.severity == kite_core::ViolationSeverity::Error)
        .count();
    let tests = report.violations.len().max(1);
    println!(
        "<testsuite name=\"kite\" tests=\"{}\" failures=\"{}\" errors=\"0\" skipped=\"0\" time=\"0\">",
        tests, failures
    );

    if report.violations.is_empty() {
        println!("  <testcase name=\"verification\" classname=\"kite.Verification\" time=\"0\"/>");
    } else {
        for violation in &report.violations {
            let classname = format!("kite.{}", violation.code);
            println!(
                "  <testcase name=\"{}\" classname=\"{}\" time=\"0\">",
                violation.code, classname
            );
            if violation.severity == kite_core::ViolationSeverity::Error {
                let escaped_message = violation.message.replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;");
                let escaped_hint = violation.hint.as_ref().map(|h| h.replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")).unwrap_or_default();
                println!(
                    "    <failure message=\"{}\">{}</failure>",
                    escaped_message, escaped_hint
                );
            }
            println!("  </testcase>");
        }
    }
    println!("</testsuite>");
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
