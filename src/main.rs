//! trunc - Smart truncation for pipe output
//!
//! Shows the first N and last M lines of stdin, with an optional
//! pattern-matching mode that extracts matches from the middle.
//!
//! Streams output: first lines appear immediately, matches stream as found,
//! only the tail waits for EOF.

use clap::Parser;
use std::io;
use std::process;
use trunc::{run, Config, RunError};

/// Smart truncation for pipe output - like head+tail combined.
///
/// Shows the first N and last M lines, with optional grep-style pattern matching
/// to extract relevant lines from the middle.
#[derive(Parser, Debug)]
#[command(name = "trunc", version, about)]
struct Args {
    /// Number of lines to show from start
    #[arg(
        short = 'f',
        long = "first",
        default_value = "30",
        visible_alias = "head",
        short_alias = 'H'
    )]
    first: usize,

    /// Number of lines to show from end
    #[arg(
        short = 'l',
        long = "last",
        default_value = "30",
        visible_alias = "tail",
        short_alias = 'T'
    )]
    last: usize,

    /// Max matches to show in pattern mode
    #[arg(short = 'm', long = "matches", default_value = "5")]
    matches: usize,

    /// Lines of context around each match
    #[arg(short = 'C', long = "context", default_value = "3")]
    context: usize,

    /// Chars to show at start/end of long lines (0 = no limit)
    #[arg(short = 'w', long = "width", default_value = "100")]
    width: usize,

    /// Regex pattern to search for in the middle section
    pattern: Option<String>,
}

impl From<Args> for Config {
    fn from(args: Args) -> Self {
        Self {
            first: args.first,
            last: args.last,
            matches: args.matches,
            context: args.context,
            width: args.width,
            pattern: args.pattern,
        }
    }
}

fn main() {
    let args = Args::parse();
    let stdin = io::stdin();
    let stdout = io::stdout();

    match run(stdin.lock(), stdout.lock(), args.into()) {
        Ok(_) => {}
        Err(RunError::InvalidPattern(error)) => {
            eprintln!("Invalid regex pattern: {error}");
            process::exit(1);
        }
        Err(RunError::Read(error)) => {
            eprintln!("Error reading input: {error}");
            process::exit(1);
        }
        Err(RunError::Write(error)) => {
            eprintln!("Error writing output: {error}");
            process::exit(1);
        }
    }
}
