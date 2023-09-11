mod get;

use clap::Parser;
use colored::*;

pub struct CliError {
    outer: String,
    inner: Option<String>,
}

impl CliError {
    fn new(outer: impl Into<String>, inner: impl Into<String>) -> Self {
        Self {
            outer: outer.into(),
            inner: Some(inner.into()),
        }
    }
}

impl<So: Into<String>> From<So> for CliError {
    fn from(outer: So) -> Self {
        CliError {
            outer: outer.into(),
            inner: None,
        }
    }
}

#[derive(Parser, Debug)]
enum Args {
    /// Read a full desktop file, a specific group, or a specific key
    Get(get::GetArgs),
}

fn main() {
    let result = match Args::parse() {
        Args::Get(args) => get::main(args),
    };

    match result {
        Ok(_) => {}
        Err(CliError { outer, inner }) => {
            eprintln!("{} {}", "error:".red(), outer);
            if let Some(inner) = inner {
                eprintln!("{}", inner);
            }
        }
    }
}
