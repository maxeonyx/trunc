//! trunc - Smart truncation for pipe output
//!
//! Shows the first N and last M lines of stdin, with an optional
//! pattern-matching mode that extracts matches from the middle.
//!
//! Streams output: first lines appear immediately, matches stream as found,
//! only the tail waits for EOF.

use clap::Parser;
use regex::Regex;
use std::collections::VecDeque;
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
    let pattern: Option<Regex> = match &args.pattern {
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

    let first_count = args.first;
    let last_count = args.last;
    let context_size = args.context;
    let max_matches = args.matches;
    let width = args.width;

    // State tracking
    let mut line_number: usize = 0;
    let mut head_output_count: usize = 0;
    let mut in_middle = false;
    let mut matches_found: usize = 0;
    let mut printed_matches_header = false;
    let mut last_output_line: usize = 0; // Track the last line number we output

    // Track contiguous ranges of lines output during match streaming,
    // so the tail loop can skip only lines that were actually output.
    let mut match_output_ranges: Vec<(usize, usize)> = Vec::new();

    // Ring buffer for tail
    let mut tail_buffer: VecDeque<(usize, String)> = VecDeque::with_capacity(last_count + 1);

    // Context buffer for pattern mode - holds recent lines for "before" context
    let mut context_buffer: VecDeque<(usize, String)> = VecDeque::with_capacity(context_size + 1);

    // Track pending "after" context
    let mut after_context_remaining: usize = 0;

    for line_result in stdin.lock().lines() {
        let content = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                process::exit(1);
            }
        };

        line_number += 1;
        let truncated = truncate_line(&content, width);

        // Phase 1: Output head lines immediately
        if head_output_count < first_count {
            let _ = writeln!(stdout, "{}", truncated);
            let _ = stdout.flush();
            head_output_count += 1;
            last_output_line = line_number;
            continue;
        }

        // We're now in the middle section
        if !in_middle {
            in_middle = true;
        }

        // Always maintain tail buffer
        tail_buffer.push_back((line_number, content.clone()));
        if tail_buffer.len() > last_count {
            tail_buffer.pop_front();
        }

        // Pattern mode: look for matches and stream them
        if let Some(ref re) = pattern {
            // Helper closure: record a line as output in match_output_ranges
            let record_output = |ranges: &mut Vec<(usize, usize)>, ln: usize| {
                if let Some(last) = ranges.last_mut() {
                    if ln == last.1 + 1 {
                        last.1 = ln; // extend current range
                        return;
                    }
                }
                ranges.push((ln, ln)); // start new range
            };

            // Are we still outputting "after" context from a previous match?
            if after_context_remaining > 0 {
                // Check if this line overlaps with tail - if so, skip (will be in tail)
                // We can't know tail boundaries yet, so just output it
                // But avoid duplicates - only output if we haven't output this line
                if line_number > last_output_line {
                    let _ = writeln!(stdout, "{}", truncated);
                    let _ = stdout.flush();
                    record_output(&mut match_output_ranges, line_number);
                    last_output_line = line_number;
                }
                after_context_remaining -= 1;
            }

            // Check for match (only if we haven't hit max matches)
            // Note: we check BEFORE adding to context buffer, so context_buffer
            // contains only lines *before* the current line
            if matches_found < max_matches && re.is_match(&content) {
                matches_found += 1;

                // Track if this is NOT the first match (for gap detection)
                let had_previous_match = printed_matches_header;

                // Print matches header if first match
                if !printed_matches_header {
                    let _ = writeln!(stdout, "[... matches follow ...]");
                    let _ = stdout.flush();
                    printed_matches_header = true;
                }

                // Check if we need [...] separator (gap between last output and this match context)
                // Only needed if we already printed a previous match - the "[... matches follow ...]"
                // header already serves as separator from head
                let context_start = line_number.saturating_sub(context_size);
                if had_previous_match && context_start > last_output_line + 1 {
                    let _ = writeln!(stdout, "[...]");
                    let _ = stdout.flush();
                }

                // Output "before" context (lines we haven't already output)
                for (ctx_line_num, ctx_content) in &context_buffer {
                    if *ctx_line_num > last_output_line && *ctx_line_num < line_number {
                        let _ = writeln!(stdout, "{}", truncate_line(ctx_content, width));
                        record_output(&mut match_output_ranges, *ctx_line_num);
                        last_output_line = *ctx_line_num;
                    }
                }

                // Output the match line itself (if not already output)
                if line_number > last_output_line {
                    let _ = writeln!(stdout, "{}", truncated);
                    let _ = stdout.flush();
                    record_output(&mut match_output_ranges, line_number);
                    last_output_line = line_number;
                }

                // Set up "after" context
                after_context_remaining = context_size;
            }

            // Maintain context buffer for "before" context (add AFTER checking for match)
            context_buffer.push_back((line_number, content.clone()));
            if context_buffer.len() > context_size {
                context_buffer.pop_front();
            }
        }
    }

    // EOF reached - now output tail

    let total_lines = line_number;

    // Handle empty input
    if total_lines == 0 {
        return;
    }

    // Calculate where tail starts
    let tail_start = if total_lines > last_count {
        total_lines - last_count + 1
    } else {
        1
    };

    // Determine if we need any separator before tail
    let needs_truncation = total_lines > first_count + last_count;

    if pattern.is_some() {
        // Pattern mode
        if printed_matches_header {
            // We printed matches - check if we need "[... matches end ...]"
            // Only if there's a gap between last output and tail
            if last_output_line + 1 < tail_start {
                let _ = writeln!(stdout, "[... matches end ...]");
            }
        } else if needs_truncation {
            // No matches found in middle - use simple truncation marker
            let _ = writeln!(stdout, "[... truncated ...]");
        }
    } else {
        // Default mode (no pattern)
        if needs_truncation {
            let _ = writeln!(stdout, "[... truncated ...]");
        }
    }

    // Output tail (only lines not already output)
    // Use match_output_ranges for precise duplicate detection instead of
    // last_output_line high-water mark (which incorrectly skips tail lines
    // that precede match context output).
    let was_output_in_match = |ln: usize| -> bool {
        match_output_ranges
            .iter()
            .any(|(start, end)| ln >= *start && ln <= *end)
    };
    for (tail_line_num, tail_content) in &tail_buffer {
        if *tail_line_num > first_count && !was_output_in_match(*tail_line_num) {
            let _ = writeln!(stdout, "{}", truncate_line(tail_content, width));
        }
    }
}
