//! trunc - Smart truncation for pipe output
//!
//! Shows the first N and last M lines of stdin, with an optional
//! pattern-matching mode that extracts matches from the middle.

use clap::Parser;
use regex::Regex;
use std::io::{self, BufRead, Write};
use std::process;

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
        default_value = "10",
        visible_alias = "head",
        short_alias = 'H'
    )]
    first: usize,

    /// Number of lines to show from end
    #[arg(
        short = 'l',
        long = "last",
        default_value = "10",
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

/// A line with its original line number (1-indexed)
#[derive(Clone, Debug)]
struct Line {
    number: usize,
    content: String,
}

/// Truncate a line if it's too long
fn truncate_line(line: &str, width: usize) -> String {
    if width == 0 {
        return line.to_string();
    }

    let char_count = line.chars().count();
    let max_len = width * 2;

    if char_count <= max_len {
        return line.to_string();
    }

    let first: String = line.chars().take(width).collect();
    let last: String = line.chars().skip(char_count - width).collect();
    format!("{}[...]{}", first, last)
}

fn main() {
    let args = Args::parse();

    // Compile regex if provided
    let pattern = match &args.pattern {
        Some(p) => match Regex::new(p) {
            Ok(re) => Some(re),
            Err(e) => {
                eprintln!("Invalid regex pattern: {}", e);
                process::exit(1);
            }
        },
        None => None,
    };

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    // Collect all lines with their line numbers
    let mut all_lines: Vec<Line> = Vec::new();
    for (i, line_result) in stdin.lock().lines().enumerate() {
        let content = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                process::exit(1);
            }
        };
        all_lines.push(Line {
            number: i + 1,
            content,
        });
    }

    let total_lines = all_lines.len();

    // Handle empty input
    if total_lines == 0 {
        return;
    }

    // Determine which line numbers belong to which section
    let first_count = args.first;
    let last_count = args.last;

    // Calculate boundaries
    let head_end = first_count.min(total_lines);
    let tail_start = if total_lines > last_count {
        total_lines - last_count + 1
    } else {
        1
    };

    // Check if we need truncation at all
    let needs_truncation = total_lines > first_count + last_count;

    if !needs_truncation && pattern.is_none() {
        // Output all lines unchanged
        for line in &all_lines {
            let _ = writeln!(stdout, "{}", truncate_line(&line.content, args.width));
        }
        return;
    }

    // Pattern mode
    if let Some(ref re) = pattern {
        // Find matches in the middle section (not in head or tail)
        let mut match_line_numbers: Vec<usize> = Vec::new();

        for line in &all_lines {
            // Only look for matches in lines that are NOT in head or tail
            let in_head = line.number <= head_end;
            let in_tail = line.number >= tail_start;

            if !in_head && !in_tail && re.is_match(&line.content) {
                match_line_numbers.push(line.number);
                if match_line_numbers.len() >= args.matches {
                    break;
                }
            }
        }

        // Build regions to output: head, match contexts, tail
        // We track which lines are match context
        let mut is_match_context: Vec<bool> = vec![false; total_lines];

        // Mark match context lines
        for &match_num in &match_line_numbers {
            let start = match_num.saturating_sub(args.context);
            let end = (match_num + args.context).min(total_lines);
            for i in start..=end {
                if i >= 1 && i <= total_lines {
                    is_match_context[i - 1] = true;
                }
            }
        }

        // Now output in order
        // Structure: head, [... matches follow ...], matches with [...] between groups, [... matches end ...], tail

        // Output head
        for line in all_lines.iter().take(head_end) {
            let _ = writeln!(stdout, "{}", truncate_line(&line.content, args.width));
        }

        // Find match context regions (contiguous groups of match context lines)
        // that are NOT already in head or tail
        let mut match_regions: Vec<(usize, usize)> = Vec::new(); // (start, end) inclusive, 1-indexed
        let mut in_region = false;
        let mut region_start = 0;

        for (i, &is_match) in is_match_context.iter().enumerate() {
            let line_num = i + 1;
            let in_head = line_num <= head_end;
            let in_tail = line_num >= tail_start;
            let in_middle_match = is_match && !in_head && !in_tail;

            if in_middle_match && !in_region {
                in_region = true;
                region_start = line_num;
            } else if !in_middle_match && in_region {
                in_region = false;
                match_regions.push((region_start, line_num - 1));
            }
        }
        if in_region {
            match_regions.push((region_start, total_lines));
        }

        // If there are matches to show in the middle
        if !match_regions.is_empty() {
            let _ = writeln!(stdout, "[... matches follow ...]");

            for (region_idx, &(start, end)) in match_regions.iter().enumerate() {
                // Add [...] between non-contiguous match groups
                if region_idx > 0 {
                    let prev_end = match_regions[region_idx - 1].1;
                    if start > prev_end + 1 {
                        let _ = writeln!(stdout, "[...]");
                    }
                }

                for line_num in start..=end {
                    let line = &all_lines[line_num - 1];
                    let _ = writeln!(stdout, "{}", truncate_line(&line.content, args.width));
                }
            }

            // Only show "[... matches end ...]" if there's a gap before tail
            let last_region_end = match_regions.last().map(|r| r.1).unwrap_or(0);
            if last_region_end + 1 < tail_start {
                let _ = writeln!(stdout, "[... matches end ...]");
            }
        } else if needs_truncation {
            // No matches in middle - fall back to simple truncation marker
            let _ = writeln!(stdout, "[... truncated ...]");
        }

        // Output tail (only lines not already in head)
        for line in all_lines.iter().skip(tail_start - 1) {
            if line.number > head_end {
                let _ = writeln!(stdout, "{}", truncate_line(&line.content, args.width));
            }
        }
    } else {
        // Default mode (no pattern)
        if !needs_truncation {
            // Already handled above, but just in case
            for line in &all_lines {
                let _ = writeln!(stdout, "{}", truncate_line(&line.content, args.width));
            }
            return;
        }

        // Output head
        for line in all_lines.iter().take(head_end) {
            let _ = writeln!(stdout, "{}", truncate_line(&line.content, args.width));
        }

        let _ = writeln!(stdout, "[... truncated ...]");

        // Output tail
        for line in all_lines.iter().skip(tail_start - 1) {
            let _ = writeln!(stdout, "{}", truncate_line(&line.content, args.width));
        }
    }
}
