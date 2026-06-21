//! trunc - Smart truncation for pipe output
//!
//! Shows the first N and last M lines of stdin, with an optional
//! pattern-matching mode that extracts matches from the middle.
//!
//! Streams output: first lines and early matches appear immediately, while
//! recent matches and the final tail are emitted during finalization.

use clap::Parser;
use std::env;
use std::io;
use std::process;
use trunc::{run, Config, RunError, RunOutcome};

const AFTER_HELP: &str = "Examples:\n  $ seq 1 100 | trunc\n  $ seq 1 100 | trunc --first 10 --last 20\n  $ printf '%s\\n' ok WARNING done | trunc WARNING\n  $ python3 -c \"print('x'*240); print('timeout after 30s'); print('y'*240)\" | trunc --width 20 timeout";

/// Smart truncation for pipe output - like head+tail combined.
///
/// Shows the first N and last M lines, with optional grep-style pattern matching
/// to extract relevant lines from the middle.
#[derive(Parser, Debug)]
#[command(name = "trunc", version, about, after_help = AFTER_HELP)]
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

    /// Shorthand: set both --match-first and --match-last
    #[arg(short = 'm', long = "matches", default_value = "3")]
    matches: usize,

    /// Number of earliest matches to show in pattern mode (overrides --matches for the first side)
    #[arg(long = "match-first")]
    match_first: Option<usize>,

    /// Number of latest matches to show in pattern mode (overrides --matches for the last side)
    #[arg(long = "match-last")]
    match_last: Option<usize>,

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
            match_first: args.match_first.unwrap_or(args.matches),
            match_last: args.match_last.unwrap_or(args.matches),
            context: args.context,
            width: args.width,
            pattern: args.pattern,
        }
    }
}

fn main() {
    if try_handle_version_request() {
        return;
    }

    let args = Args::parse();
    let stdin = io::stdin();
    let stdout = io::stdout();

    match run(stdin.lock(), stdout.lock(), args.into()) {
        Ok(RunOutcome::Completed | RunOutcome::BrokenPipe) => {}
        Ok(RunOutcome::Interrupted(signal)) => {
            process::exit(signal.exit_code());
        }
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

fn try_handle_version_request() -> bool {
    let args: Vec<String> = env::args().skip(1).collect();

    if is_version_json_request(&args) {
        println!(
            "{{\"package\":\"broken-trunc\",\"binary\":\"trunc\",\"version\":\"{}\"}}",
            env!("CARGO_PKG_VERSION")
        );
        return true;
    }

    if is_version_request(&args) {
        println!("trunc {}", env!("CARGO_PKG_VERSION"));
        return true;
    }

    false
}

fn is_version_request(args: &[String]) -> bool {
    args.len() == 1 && matches!(args[0].as_str(), "--version" | "-V")
}

fn is_version_json_request(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "--version" | "-V"))
        && args.iter().any(|arg| arg == "--json")
        && args
            .iter()
            .all(|arg| matches!(arg.as_str(), "--version" | "-V" | "--json"))
}
