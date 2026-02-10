//! End-to-end black-box tests for trunc.
//!
//! These tests spawn the actual binary and verify its behavior.
//! They test observable behavior only - no internal knowledge.

use assert_cmd::Command;
use predicates::prelude::*;

/// Helper to create a Command for the trunc binary.
fn trunc() -> Command {
    Command::cargo_bin("trunc").unwrap()
}

/// Generate N lines of input: "line 1\nline 2\n..."
fn generate_lines(n: usize) -> String {
    (1..=n)
        .map(|i| format!("line {}", i))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Generate N lines with a specific pattern at certain positions.
fn generate_lines_with_matches(n: usize, match_at: &[usize], pattern: &str) -> String {
    (1..=n)
        .map(|i| {
            if match_at.contains(&i) {
                format!("line {} contains {}", i, pattern)
            } else {
                format!("line {}", i)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// =============================================================================
// BASIC TRUNCATION (NO PATTERN)
// =============================================================================

mod basic_truncation {
    use super::*;

    #[test]
    fn short_input_passes_through_unchanged() {
        // Input with 15 lines (less than 30 + 30) should pass through unchanged
        let input = generate_lines(15);

        trunc()
            .write_stdin(input.clone())
            .assert()
            .success()
            .stdout(format!("{}\n", input));
    }

    #[test]
    fn exactly_60_lines_passes_through_unchanged() {
        // Exactly 60 lines = 30 head + 30 tail with no overlap
        // Should pass through without truncation marker
        let input = generate_lines(60);

        trunc()
            .write_stdin(input.clone())
            .assert()
            .success()
            .stdout(format!("{}\n", input));
    }

    #[test]
    fn truncates_at_61_lines() {
        // 61 lines should show truncation marker
        let input = generate_lines(61);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(input).assert().success();

        // Should have first 30 lines
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(stdout.starts_with("line 1\n"), "Should start with line 1");
        assert!(stdout.contains("line 30\n"), "Should contain line 30");

        // Should have truncation marker with line count
        assert!(
            stdout.contains("[... 1 lines truncated ...]"),
            "Should contain truncation marker with line count"
        );

        // Should have last 30 lines
        assert!(stdout.contains("line 32\n"), "Should contain line 32");
        assert!(stdout.ends_with("line 61\n"), "Should end with line 61");
    }

    #[test]
    fn truncates_100_lines_default() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 30 head + 1 truncated marker + 30 tail = 61 lines
        assert_eq!(lines.len(), 61, "Should output exactly 61 lines");

        // First 30 lines
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[29], "line 30");

        // Truncation marker
        assert_eq!(lines[30], "[... 40 lines truncated ...]");

        // Last 30 lines
        assert_eq!(lines[31], "line 71");
        assert_eq!(lines[60], "line 100");
    }

    #[test]
    fn empty_input_produces_empty_output() {
        trunc().write_stdin("").assert().success().stdout("");
    }

    #[test]
    fn single_line_passes_through() {
        trunc()
            .write_stdin("hello world")
            .assert()
            .success()
            .stdout("hello world\n");
    }

    #[test]
    fn preserves_blank_lines() {
        let input = "line 1\n\nline 3\n\nline 5";

        trunc()
            .write_stdin(input)
            .assert()
            .success()
            .stdout("line 1\n\nline 3\n\nline 5\n");
    }

    #[test]
    fn handles_trailing_newline() {
        let input = "line 1\nline 2\nline 3\n";

        trunc()
            .write_stdin(input)
            .assert()
            .success()
            .stdout("line 1\nline 2\nline 3\n");
    }

    #[test]
    fn handles_no_trailing_newline() {
        let input = "line 1\nline 2\nline 3";

        trunc()
            .write_stdin(input)
            .assert()
            .success()
            .stdout("line 1\nline 2\nline 3\n");
    }
}

// =============================================================================
// CUSTOM LINE COUNTS
// =============================================================================

mod custom_line_counts {
    use super::*;

    #[test]
    fn custom_first_count() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd.args(["-f", "5"]).write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 5 first + 1 truncated + 30 last = 36 lines
        assert_eq!(lines.len(), 36);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[4], "line 5");
        assert_eq!(lines[5], "[... 65 lines truncated ...]");
        assert_eq!(lines[6], "line 71");
    }

    #[test]
    fn custom_last_count() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd.args(["-l", "5"]).write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 30 first + 1 truncated + 5 last = 36 lines
        assert_eq!(lines.len(), 36);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[29], "line 30");
        assert_eq!(lines[30], "[... 65 lines truncated ...]");
        assert_eq!(lines[31], "line 96");
        assert_eq!(lines[35], "line 100");
    }

    #[test]
    fn custom_first_and_last() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "3", "-l", "3"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 3 first + 1 truncated + 3 last = 7 lines
        assert_eq!(lines.len(), 7);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[2], "line 3");
        assert_eq!(lines[3], "[... 94 lines truncated ...]");
        assert_eq!(lines[4], "line 98");
        assert_eq!(lines[6], "line 100");
    }

    #[test]
    fn zero_first_lines() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd.args(["-f", "0"]).write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 0 first + 1 truncated + 30 last = 31 lines
        assert_eq!(lines.len(), 31);
        assert_eq!(lines[0], "[... 70 lines truncated ...]");
        assert_eq!(lines[1], "line 71");
        assert_eq!(lines[30], "line 100");
    }

    #[test]
    fn zero_last_lines() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd.args(["-l", "0"]).write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 30 first + 1 truncated + 0 last = 31 lines
        assert_eq!(lines.len(), 31);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[29], "line 30");
        assert_eq!(lines[30], "[... 70 lines truncated ...]");
    }

    #[test]
    fn long_form_first_last() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["--first", "5", "--last", "5"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 5 first + 1 truncated + 5 last = 11 lines
        assert_eq!(lines.len(), 11);
    }

    #[test]
    fn head_tail_aliases() {
        // --head and --tail should work as aliases for --first and --last
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["--head", "5", "--tail", "5"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 5 first + 1 truncated + 5 last = 11 lines
        assert_eq!(lines.len(), 11);
    }

    #[test]
    fn short_head_tail_aliases() {
        // -H and -T should work as aliases for -f and -l
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-H", "5", "-T", "5"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 5 first + 1 truncated + 5 last = 11 lines
        assert_eq!(lines.len(), 11);
    }
}

// =============================================================================
// PATTERN MODE
// =============================================================================

mod pattern_mode {
    use super::*;

    #[test]
    fn pattern_mode_shows_matches_marker() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("lines truncated, match 1 shown"),
            "Should contain match marker with line count. Got:\n{}",
            stdout
        );
        assert!(
            !stdout.contains("[... truncated ...]"),
            "Should not contain plain truncated marker in pattern mode"
        );
    }

    #[test]
    fn pattern_mode_shows_matching_line() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("line 50 contains ERROR"),
            "Should contain the matching line"
        );
    }

    #[test]
    fn pattern_mode_shows_context_around_match() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Default context is 3 lines
        // Should show lines 47, 48, 49, 50 (match), 51, 52, 53
        assert!(
            stdout.contains("line 47"),
            "Should contain 3 lines before match"
        );
        assert!(stdout.contains("line 48"), "Should contain context");
        assert!(stdout.contains("line 49"), "Should contain context");
        assert!(
            stdout.contains("line 50 contains ERROR"),
            "Should contain match"
        );
        assert!(stdout.contains("line 51"), "Should contain context");
        assert!(stdout.contains("line 52"), "Should contain context");
        assert!(
            stdout.contains("line 53"),
            "Should contain 3 lines after match"
        );
    }

    #[test]
    fn pattern_mode_limits_to_5_matches_by_default() {
        // Create input with 10 matches in the middle section
        let match_positions: Vec<usize> = (40..=90).step_by(5).collect(); // 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90 = 11 matches in middle
        let input = generate_lines_with_matches(200, &match_positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Count how many match lines appear
        let match_count = stdout.matches("contains ERROR").count();
        assert_eq!(match_count, 5, "Should show exactly 5 matches by default");
    }

    #[test]
    fn pattern_mode_custom_match_limit() {
        let match_positions: Vec<usize> = (40..=90).step_by(5).collect();
        let input = generate_lines_with_matches(200, &match_positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-m", "3", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let match_count = stdout.matches("contains ERROR").count();
        assert_eq!(match_count, 3, "Should show exactly 3 matches with -m 3");
    }

    #[test]
    fn pattern_mode_custom_context() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "-C", "1", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // With context of 1: lines 49, 50 (match), 51
        assert!(
            stdout.contains("line 49"),
            "Should contain 1 line before match"
        );
        assert!(
            stdout.contains("line 50 contains ERROR"),
            "Should contain match"
        );
        assert!(
            stdout.contains("line 51"),
            "Should contain 1 line after match"
        );

        // Should NOT contain lines further out
        assert!(
            !stdout.contains("line 47"),
            "Should not contain line 47 with context 1"
        );
        assert!(
            !stdout.contains("line 53"),
            "Should not contain line 53 with context 1"
        );
    }

    #[test]
    fn pattern_mode_zero_context() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "-C", "0", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // With context of 0: only the matching line
        assert!(
            stdout.contains("line 50 contains ERROR"),
            "Should contain match"
        );
        assert!(
            !stdout.contains("line 49"),
            "Should not contain context lines"
        );
        assert!(
            !stdout.contains("line 51"),
            "Should not contain context lines"
        );
    }

    #[test]
    fn pattern_mode_no_matches_in_middle() {
        // All matches are in the head or tail sections
        let input = generate_lines_with_matches(100, &[5, 95], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Should still show the matches marker but with no matches
        // (the matches in head/tail are shown as part of those sections)
        assert!(
            stdout.contains("line 5 contains ERROR"),
            "Match in head should appear"
        );
        assert!(
            stdout.contains("line 95 contains ERROR"),
            "Match in tail should appear"
        );
    }

    #[test]
    fn pattern_mode_still_shows_head_and_tail() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Should have first 10 lines
        assert!(stdout.contains("line 1"), "Should contain head");
        assert!(stdout.contains("line 10"), "Should contain end of head");

        // Should have last 10 lines
        assert!(stdout.contains("line 91"), "Should contain start of tail");
        assert!(stdout.contains("line 100"), "Should contain tail");
    }

    #[test]
    fn pattern_mode_shows_ellipsis_between_matches() {
        // Matches at 50 and 80 - far enough apart that their contexts don't overlap
        // With context 3: match 50 shows 47-53, match 80 shows 77-83
        // There's a gap between 53 and 77, so we need a marker between them
        let input = generate_lines_with_matches(200, &[50, 80], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Should have an informative marker between the two match groups
        assert!(
            stdout.contains("lines truncated, match 2 shown"),
            "Should have match 2 marker between non-contiguous matches. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn pattern_mode_no_ellipsis_between_adjacent_matches() {
        // Matches at 50 and 52 - close enough that contexts overlap
        // With context 3: match 50 shows 47-53, match 52 shows 49-55
        // They overlap, so no marker needed between them
        let input = generate_lines_with_matches(200, &[50, 52], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Find the match lines
        let first_match_idx = lines
            .iter()
            .position(|l| l.contains("line 50 contains"))
            .expect("Output should contain match at line 50");
        let second_match_idx = lines
            .iter()
            .position(|l| l.contains("line 52 contains"))
            .expect("Output should contain match at line 52");

        // In the matches section, there should be no marker between them
        let between = &lines[first_match_idx + 1..second_match_idx];
        assert!(
            !between.iter().any(|l| l.starts_with("[...")),
            "Should not have marker between adjacent matches. Between: {:?}",
            between
        );
    }

    #[test]
    fn pattern_mode_ellipsis_between_head_and_matches() {
        // Match at 50, head is 1-10, so there's a gap
        let input = generate_lines_with_matches(200, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Should have an informative match marker between head and match context
        let match_marker = lines
            .iter()
            .find(|l| l.contains("match 1 shown"))
            .expect("Should have match 1 marker");

        // The marker should appear right after the head (line 10)
        let line_10_idx = lines
            .iter()
            .position(|l| *l == "line 10")
            .expect("Output should contain 'line 10'");
        assert_eq!(
            lines[line_10_idx + 1],
            *match_marker,
            "Match marker should come right after end of head"
        );
    }

    #[test]
    fn pattern_mode_ellipsis_between_matches_and_tail() {
        // Match at 50 with context 3 shows lines 47-53
        // Tail starts at 191, so there's a gap
        let input = generate_lines_with_matches(200, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // After the match context (line 53), before tail (line 191), should have a line-count marker
        let line_53_idx = lines
            .iter()
            .position(|l| *l == "line 53")
            .expect("Output should contain 'line 53' (end of match context)");
        let line_191_idx = lines
            .iter()
            .position(|l| *l == "line 191")
            .expect("Output should contain 'line 191' (start of tail)");

        // There should be exactly one line between them — an informative marker
        assert_eq!(
            line_191_idx - line_53_idx,
            2,
            "Should have exactly one line between match context and tail"
        );
        assert!(
            lines[line_53_idx + 1].contains("lines truncated"),
            "Line between match context and tail should show lines truncated. Got: '{}'",
            lines[line_53_idx + 1]
        );
    }

    #[test]
    fn pattern_mode_regex_support() {
        let input = "error: something\nERROR: something\nwarning: something\nError: something";

        let mut cmd = trunc();
        let assert = cmd
            .arg("(?i)error") // Case-insensitive regex
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        assert!(stdout.contains("error: something"));
        assert!(stdout.contains("ERROR: something"));
        assert!(stdout.contains("Error: something"));
    }

    #[test]
    fn pattern_mode_long_form_args() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["--matches", "3", "--context", "2", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(stdout.contains("line 50 contains ERROR"));
    }
}

// =============================================================================
// OVERLAPPING REGIONS
// =============================================================================

mod overlapping_regions {
    use super::*;

    #[test]
    fn no_duplicate_lines_when_head_tail_overlap() {
        // 65 lines: head (1-30) and tail (36-65) don't overlap
        // But lines 31-35 are "middle" and should be truncated
        let input = generate_lines(65);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Each line should appear exactly once
        for i in 1..=30 {
            let count = lines
                .iter()
                .filter(|&&l| l == format!("line {}", i))
                .count();
            assert_eq!(count, 1, "line {} should appear exactly once", i);
        }
        for i in 36..=65 {
            let count = lines
                .iter()
                .filter(|&&l| l == format!("line {}", i))
                .count();
            assert_eq!(count, 1, "line {} should appear exactly once", i);
        }
    }

    #[test]
    fn no_duplicate_lines_when_match_overlaps_head() {
        // Match at line 8 with context 3 would show lines 5-11
        // But lines 1-30 are already in head
        let input = generate_lines_with_matches(100, &[8], "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Lines 1-30 should appear exactly once (in head)
        for i in 1..=30 {
            let expected = if i == 8 {
                format!("line {} contains ERROR", i)
            } else {
                format!("line {}", i)
            };
            let count = lines.iter().filter(|&&l| l == expected).count();
            assert_eq!(count, 1, "line {} should appear exactly once", i);
        }
    }

    #[test]
    fn no_duplicate_lines_when_match_overlaps_tail() {
        // Match at line 93 with context 3 would show lines 90-96
        // But lines 71-100 are already in tail
        let input = generate_lines_with_matches(100, &[93], "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Lines 71-100 should appear exactly once (in tail)
        for i in 71..=100 {
            let expected = if i == 93 {
                format!("line {} contains ERROR", i)
            } else {
                format!("line {}", i)
            };
            let count = lines.iter().filter(|&&l| l == expected).count();
            assert_eq!(count, 1, "line {} should appear exactly once", i);
        }
    }

    #[test]
    fn no_duplicate_lines_when_matches_overlap_each_other() {
        // Matches at lines 50 and 52 with context 3
        // Line 50: context 47-53
        // Line 52: context 49-55
        // Lines 49-53 overlap
        let input = generate_lines_with_matches(100, &[50, 52], "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Check that overlapping context lines appear only once
        for i in 47..=55 {
            let expected = if i == 50 || i == 52 {
                format!("line {} contains ERROR", i)
            } else {
                format!("line {}", i)
            };
            let count = lines.iter().filter(|&&l| l == expected).count();
            assert_eq!(count, 1, "line {} should appear exactly once", i);
        }
    }
}

// =============================================================================
// EDGE CASES
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn long_lines_are_truncated() {
        // Lines over 200 chars (100 + 100) should be truncated (if result is shorter)
        let long_line = "x".repeat(1000);
        let input = format!("{}\nshort\n{}", long_line, long_line);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // First line should be truncated with char count marker
        assert!(
            lines[0].contains("[... 800 chars ...]"),
            "Long line should contain char count marker. Got: {}",
            lines[0]
        );
        assert!(
            lines[0].len() < 500,
            "Truncated line should be much shorter than 1000 chars"
        );

        // Short line should pass through unchanged
        assert_eq!(lines[1], "short");
    }

    #[test]
    fn handles_binary_looking_content() {
        // Content with null bytes and other binary-looking data
        let input = "line 1\nline \0 2\nline 3";

        trunc().write_stdin(input).assert().success();
    }

    #[test]
    fn handles_unicode() {
        let input = "héllo wörld\n日本語\nемайл\n🎉🎊🎈";

        trunc()
            .write_stdin(input)
            .assert()
            .success()
            .stdout(format!("{}\n", input));
    }

    #[test]
    fn pattern_with_special_regex_chars() {
        let input = "test [bracket]\ntest (paren)\ntest .dot\ntest *star";

        // Literal brackets should work
        let mut cmd = trunc();
        let assert = cmd
            .arg(r"\[bracket\]")
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(stdout.contains("[bracket]"));
    }

    #[test]
    fn invalid_regex_returns_error() {
        let input = "some input";

        trunc()
            .arg("[invalid")
            .write_stdin(input)
            .assert()
            .failure()
            .stderr(predicate::str::contains("regex").or(predicate::str::contains("pattern")));
    }
}

// =============================================================================
// HELP AND VERSION
// =============================================================================

mod cli_basics {
    use super::*;

    #[test]
    fn help_flag() {
        trunc()
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("trunc"))
            .stdout(predicate::str::contains("truncat")); // truncate or truncation
    }

    #[test]
    fn short_help_flag() {
        // -h is reserved for help, --head uses -H
        trunc()
            .arg("-h")
            .assert()
            .success()
            .stdout(predicate::str::contains("trunc"));
    }

    #[test]
    fn version_flag() {
        trunc()
            .arg("--version")
            .assert()
            .success()
            .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
    }
}

// =============================================================================
// LINE TRUNCATION
// =============================================================================

mod line_truncation {
    use super::*;

    #[test]
    fn short_lines_pass_through_unchanged() {
        let input = "short line\nanother short line\n";

        trunc().write_stdin(input).assert().success().stdout(input);
    }

    #[test]
    fn line_at_200_chars_passes_through() {
        // Exactly 200 chars (100 + 100) should not be truncated
        let line = "x".repeat(200);
        let input = format!("{}\n", line);

        trunc()
            .write_stdin(input.clone())
            .assert()
            .success()
            .stdout(input);
    }

    #[test]
    fn line_at_201_chars_is_not_truncated() {
        // 201 chars: truncation would produce 100 + "[... 1 chars ...]" (17) + 100 = 217 > 201
        // So truncation should NOT happen (result wouldn't be shorter)
        let line = format!("{}y{}", "a".repeat(100), "b".repeat(100));
        assert_eq!(line.len(), 201);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert_eq!(
            output_line.len(),
            201,
            "201-char line should pass through unchanged"
        );
        assert!(
            !output_line.contains("[..."),
            "Should not contain truncation marker"
        );
    }

    #[test]
    fn truncated_line_shows_first_and_last_100_chars() {
        let first_100 = "A".repeat(100);
        let middle = "M".repeat(500);
        let last_100 = "Z".repeat(100);
        let line = format!("{}{}{}", first_100, middle, last_100);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        // Should be: first_100 + "[... 500 chars ...]" (19) + last_100 = 219 chars
        assert_eq!(
            output_line.len(),
            219,
            "Truncated line should be exactly 219 chars"
        );
        assert!(
            output_line.starts_with(&first_100),
            "Should start with first 100 chars"
        );
        assert!(
            output_line.contains("[... 500 chars ...]"),
            "Should contain char count marker"
        );
        assert!(
            output_line.ends_with(&last_100),
            "Should end with last 100 chars"
        );
    }

    #[test]
    fn custom_line_width() {
        let line = "x".repeat(100);

        // With -w 20, lines over 40 chars should be truncated
        let mut cmd = trunc();
        let assert = cmd
            .args(["-w", "20"])
            .write_stdin(format!("{}\n", line))
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        // Should be: 20 + "[... 60 chars ...]" (18) + 20 = 58 chars
        assert_eq!(
            output_line.len(),
            58,
            "Truncated line with -w 20 should be 58 chars"
        );
    }

    #[test]
    fn long_form_width_arg() {
        let line = "x".repeat(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["--width", "20"])
            .write_stdin(format!("{}\n", line))
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        // 20 + "[... 60 chars ...]" (18) + 20 = 58 chars
        assert_eq!(
            output_line.len(),
            58,
            "Truncated line with --width 20 should be 58 chars"
        );
    }

    #[test]
    fn zero_width_disables_line_truncation() {
        let line = "x".repeat(1000);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-w", "0"])
            .write_stdin(format!("{}\n", line))
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert_eq!(
            output_line.len(),
            1000,
            "With -w 0, lines should not be truncated"
        );
    }

    #[test]
    fn unicode_line_truncation_counts_chars_not_bytes() {
        // Each emoji is 1 char but 4 bytes
        let first = "🎉".repeat(100); // 100 chars, 400 bytes
        let middle = "x".repeat(500);
        let last = "🎊".repeat(100); // 100 chars, 400 bytes
        let line = format!("{}{}{}", first, middle, last);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        // Should be: 100 emoji + "[... 500 chars ...]" (19) + 100 emoji = 219 chars
        assert_eq!(
            output_line.chars().count(),
            219,
            "Should count chars, not bytes"
        );
        assert!(
            output_line.starts_with(&first),
            "Should preserve first 100 emoji"
        );
        assert!(
            output_line.ends_with(&last),
            "Should preserve last 100 emoji"
        );
    }
}

// =============================================================================
// OUTPUT SIZE GUARANTEES
// =============================================================================

mod output_size {
    use super::*;

    // Default worst case calculation:
    // - Lines: 61 max (30 first + 1 truncated + 30 last)
    // - Chars per line: 220 max (100 + "[... 9800 chars ...]" (20) + 100) for 10k-char input
    // - Total: 61 * 220 + 60 newlines = 13460 chars
    const DEFAULT_MAX_CHARS: usize = 13460;

    // Pattern mode worst case:
    // - Lines: 101 max (30 first + 1 "[... matches follow ...]" + 35 match lines + 4 "[...]" + 1 "[... matches end ...]" + 30 last)
    // - Chars per line: 220 max
    // - Total: 101 * 220 + 100 newlines = 22320 chars
    const PATTERN_MAX_CHARS: usize = 22320;

    #[test]
    fn default_mode_max_chars() {
        // Generate input with very long lines
        let long_line = "x".repeat(10_000);
        let input = (0..100)
            .map(|_| long_line.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let mut cmd = trunc();
        let assert = cmd.write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        assert!(
            stdout.len() <= DEFAULT_MAX_CHARS,
            "Default mode output ({} chars) should not exceed {} chars",
            stdout.len(),
            DEFAULT_MAX_CHARS
        );
    }

    #[test]
    fn pattern_mode_max_chars() {
        // Generate input with very long lines and matches spread out
        let long_line = "x".repeat(10_000);
        let match_line = format!("{}ERROR{}", "y".repeat(5000), "z".repeat(5000));

        let mut lines: Vec<String> = Vec::new();
        for i in 1..=200 {
            if [50, 70, 90, 110, 130].contains(&i) {
                lines.push(match_line.clone());
            } else {
                lines.push(long_line.clone());
            }
        }
        let input = lines.join("\n");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        assert!(
            stdout.len() <= PATTERN_MAX_CHARS,
            "Pattern mode output ({} chars) should not exceed {} chars",
            stdout.len(),
            PATTERN_MAX_CHARS
        );
    }

    #[test]
    fn default_mode_max_61_lines() {
        // With any input > 60 lines, output should be exactly 61 lines
        // (30 first + 1 truncated + 30 last)
        for size in [100, 500, 1000] {
            let input = generate_lines(size);

            let mut cmd = trunc();
            let assert = cmd.write_stdin(input).assert().success();

            let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
            let line_count = stdout.lines().count();
            assert_eq!(
                line_count, 61,
                "Output should be exactly 61 lines for input of {} lines",
                size
            );
        }
    }

    #[test]
    fn pattern_mode_max_lines() {
        // Maximum lines in pattern mode with ellipsis separators:
        // 30 first + 1 "[... matches follow ...]" + 35 (5 matches * 7 context) + 4 "[...]" + 1 "[... matches end ...]" + 30 last = 101

        let match_positions: Vec<usize> = vec![50, 60, 70, 80, 90];
        let input = generate_lines_with_matches(200, &match_positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let line_count = stdout.lines().count();

        assert!(
            line_count <= 101,
            "Pattern mode output ({} lines) should not exceed 101 lines",
            line_count
        );
    }
}

// =============================================================================
// STREAMING BEHAVIOR
// =============================================================================

mod streaming {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::Duration;

    /// Get path to the trunc binary
    fn trunc_bin() -> std::path::PathBuf {
        assert_cmd::cargo::cargo_bin("trunc")
    }

    #[test]
    fn first_lines_stream_immediately() {
        // Spawn trunc and feed it lines slowly
        // The first 30 lines should appear on stdout BEFORE we send more input
        let mut child = Command::new(trunc_bin())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn trunc");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");

        // Start a reader thread that sends lines to a channel as they arrive
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let _ = tx.send(l);
                }
            }
        });

        // Send first 30 lines
        for i in 1..=30 {
            writeln!(stdin, "line {}", i).unwrap();
        }
        stdin.flush().unwrap();

        // Wait briefly for trunc to process
        std::thread::sleep(Duration::from_millis(100));

        // Check what we've received SO FAR (stdin is still open!)
        let mut received_before_more_input = Vec::new();
        while let Ok(line) = rx.try_recv() {
            received_before_more_input.push(line);
        }

        // We should have received the first 30 lines already
        assert!(
            received_before_more_input.len() >= 30,
            "First 30 lines should stream immediately before more input. \
             Got {} lines while stdin still open: {:?}",
            received_before_more_input.len(),
            received_before_more_input
        );

        // Now send the rest and close
        for i in 31..=100 {
            writeln!(stdin, "line {}", i).unwrap();
        }
        drop(stdin);

        let _ = child.wait();
    }

    #[test]
    fn matches_stream_as_they_arrive() {
        // In pattern mode, matches should stream as they're found
        // We verify by checking output arrives BEFORE stdin is closed
        let mut child = Command::new(trunc_bin())
            .arg("ERROR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn trunc");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");

        // Start reader thread
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let _ = tx.send(l);
                }
            }
        });

        // Send first 30 lines (head section)
        for i in 1..=30 {
            writeln!(stdin, "line {}", i).unwrap();
        }
        stdin.flush().unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // Check head lines arrived
        let mut received = Vec::new();
        while let Ok(line) = rx.try_recv() {
            received.push(line);
        }
        assert!(
            received.len() >= 30,
            "Head lines should stream immediately. Got {} lines: {:?}",
            received.len(),
            received
        );

        // Now send middle lines with a match at line 45
        for i in 31..=44 {
            writeln!(stdin, "line {}", i).unwrap();
        }
        writeln!(stdin, "line 45 contains ERROR").unwrap();
        // Send context after the match
        for i in 46..=48 {
            writeln!(stdin, "line {}", i).unwrap();
        }
        stdin.flush().unwrap();
        std::thread::sleep(Duration::from_millis(100));

        // Check that match has streamed (stdin still open!)
        while let Ok(line) = rx.try_recv() {
            received.push(line);
        }

        let has_match_marker = received.iter().any(|l| l.contains("match 1 shown"));
        let has_error_line = received.iter().any(|l| l.contains("ERROR"));

        assert!(
            has_match_marker,
            "Match marker should stream before stdin closes. Got: {:?}",
            received
        );
        assert!(
            has_error_line,
            "Match line should stream before stdin closes. Got: {:?}",
            received
        );

        // Close stdin and wait
        drop(stdin);
        let _ = child.wait();
    }
}
