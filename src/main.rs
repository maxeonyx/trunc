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

/// Truncate a line if it's too long.
///
/// Produces: `<first W chars>[... N chars ...]<last W chars>`
/// where N is the number of characters removed.
/// Only truncates when the result is strictly shorter than the original.
fn truncate_line(line: &str, width: usize) -> String {
    if width == 0 {
        return line.to_string();
    }

    let char_count = line.chars().count();
    let max_len = width * 2;

    if char_count <= max_len {
        return line.to_string();
    }

    let removed = char_count - max_len;
    let marker = format!("[... {} chars ...]", removed);

    // Only truncate if the result is strictly shorter than the original
    let result_len = width + marker.len() + width;
    if result_len >= char_count {
        return line.to_string();
    }

    let first: String = line.chars().take(width).collect();
    let last: String = line.chars().skip(char_count - width).collect();
    format!("{}{}{}", first, marker, last)
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
    let mut matches_shown: usize = 0;
    let mut total_matches: usize = 0; // counts ALL matches including past cutoff
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
                if line_number > last_output_line {
                    let _ = writeln!(stdout, "{}", truncated);
                    let _ = stdout.flush();
                    record_output(&mut match_output_ranges, line_number);
                    last_output_line = line_number;
                }
                after_context_remaining -= 1;
            }

            // Check for match
            if re.is_match(&content) {
                total_matches += 1;

                // Only show if we haven't hit the display limit
                if matches_shown < max_matches {
                    matches_shown += 1;

                    // Calculate gap from last output to this match's context start
                    let context_start = line_number.saturating_sub(context_size);
                    let gap_start = last_output_line + 1;
                    let gap_end = context_start.max(gap_start);
                    let lines_truncated = gap_end.saturating_sub(gap_start);

                    // Emit marker before this match group
                    let match_annotation = if matches_shown == max_matches {
                        // This is the last match we'll show AND we hit the limit
                        format!("match {}/{}", matches_shown, max_matches)
                    } else {
                        format!("match {}", matches_shown)
                    };

                    if lines_truncated > 0 {
                        let _ = writeln!(
                            stdout,
                            "[... {} lines truncated, {} shown ...]",
                            lines_truncated, match_annotation
                        );
                        let _ = stdout.flush();
                    } else if matches_shown == 1 && last_output_line >= first_count {
                        // First match immediately after head — no gap but still need marker
                        // (context overlaps with head end)
                        let _ = writeln!(
                            stdout,
                            "[... 0 lines truncated, {} shown ...]",
                            match_annotation
                        );
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
        if matches_shown > 0 {
            // We showed matches — emit end marker with line gap and remaining match info
            let gap_start = last_output_line + 1;
            let gap_end = tail_start;
            let lines_truncated = gap_end.saturating_sub(gap_start);
            let remaining_matches = total_matches - matches_shown;

            if lines_truncated > 0 || remaining_matches > 0 {
                if remaining_matches > 0 {
                    let _ = writeln!(
                        stdout,
                        "[... {} lines and {} matches truncated ({} total) ...]",
                        lines_truncated, remaining_matches, total_matches
                    );
                } else {
                    let _ = writeln!(stdout, "[... {} lines truncated ...]", lines_truncated);
                }
            }
        } else if needs_truncation {
            // No matches found in middle
            let lines_truncated = total_lines - first_count - last_count;
            let _ = writeln!(
                stdout,
                "[... {} lines truncated, 0 matches found ...]",
                lines_truncated
            );
        }
    } else {
        // Default mode (no pattern)
        if needs_truncation {
            let lines_truncated = total_lines - first_count - last_count;
            let _ = writeln!(stdout, "[... {} lines truncated ...]", lines_truncated);
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
