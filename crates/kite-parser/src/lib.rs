pub mod grammar;

use anyhow::{anyhow, Result};
use rust_sitter::Language;
use std::path::Path;

pub use grammar::Program;

pub fn parse(input: &str) -> Result<Program> {
    grammar::Program::parse(input)
        .into_result()
        .map_err(|errors| anyhow!("failed to parse .kite file: {errors:?}"))
}

pub fn parse_file(path: &Path) -> Result<Program> {
    let source = std::fs::read_to_string(path)?;
    parse(&source)
}
