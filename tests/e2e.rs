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
    (1..=n).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n")
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
        // Input with 15 lines (less than 10 + 10) should pass through unchanged
        let input = generate_lines(15);

        trunc()
            .write_stdin(input.clone())
            .assert()
            .success()
            .stdout(format!("{}\n", input));
    }

    #[test]
    fn exactly_20_lines_passes_through_unchanged() {
        // Exactly 20 lines = 10 head + 10 tail with no overlap
        // Should pass through without truncation marker
        let input = generate_lines(20);

        trunc()
            .write_stdin(input.clone())
            .assert()
            .success()
            .stdout(format!("{}\n", input));
    }

    #[test]
    fn truncates_at_21_lines() {
        // 21 lines should show truncation marker
        let input = generate_lines(21);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(input).assert().success();

        // Should have first 10 lines
        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(stdout.starts_with("line 1\n"), "Should start with line 1");
        assert!(stdout.contains("line 10\n"), "Should contain line 10");

        // Should have truncation marker
        assert!(
            stdout.contains("... truncated ..."),
            "Should contain truncation marker"
        );

        // Should have last 10 lines
        assert!(stdout.contains("line 12\n"), "Should contain line 12");
        assert!(stdout.ends_with("line 21\n"), "Should end with line 21");
    }

    #[test]
    fn truncates_100_lines_default() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 10 head + 1 truncated marker + 10 tail = 21 lines
        assert_eq!(lines.len(), 21, "Should output exactly 21 lines");

        // First 10 lines
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[9], "line 10");

        // Truncation marker
        assert_eq!(lines[10], "... truncated ...");

        // Last 10 lines
        assert_eq!(lines[11], "line 91");
        assert_eq!(lines[20], "line 100");
    }

    #[test]
    fn empty_input_produces_empty_output() {
        trunc()
            .write_stdin("")
            .assert()
            .success()
            .stdout("");
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

        // 5 first + 1 truncated + 10 last = 16 lines
        assert_eq!(lines.len(), 16);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[4], "line 5");
        assert_eq!(lines[5], "... truncated ...");
        assert_eq!(lines[6], "line 91");
    }

    #[test]
    fn custom_last_count() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd.args(["-l", "5"]).write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 10 first + 1 truncated + 5 last = 16 lines
        assert_eq!(lines.len(), 16);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[9], "line 10");
        assert_eq!(lines[10], "... truncated ...");
        assert_eq!(lines[11], "line 96");
        assert_eq!(lines[15], "line 100");
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
        assert_eq!(lines[3], "... truncated ...");
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

        // 0 first + 1 truncated + 10 last = 11 lines
        assert_eq!(lines.len(), 11);
        assert_eq!(lines[0], "... truncated ...");
        assert_eq!(lines[1], "line 91");
        assert_eq!(lines[10], "line 100");
    }

    #[test]
    fn zero_last_lines() {
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd.args(["-l", "0"]).write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // 10 first + 1 truncated + 0 last = 11 lines
        assert_eq!(lines.len(), 11);
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[9], "line 10");
        assert_eq!(lines[10], "... truncated ...");
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
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("... matches ..."),
            "Should contain matches marker instead of truncated marker"
        );
        assert!(
            !stdout.contains("... truncated ..."),
            "Should not contain truncated marker in pattern mode"
        );
    }

    #[test]
    fn pattern_mode_shows_matching_line() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

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
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Default context is 3 lines
        // Should show lines 47, 48, 49, 50 (match), 51, 52, 53
        assert!(stdout.contains("line 47"), "Should contain 3 lines before match");
        assert!(stdout.contains("line 48"), "Should contain context");
        assert!(stdout.contains("line 49"), "Should contain context");
        assert!(stdout.contains("line 50 contains ERROR"), "Should contain match");
        assert!(stdout.contains("line 51"), "Should contain context");
        assert!(stdout.contains("line 52"), "Should contain context");
        assert!(stdout.contains("line 53"), "Should contain 3 lines after match");
    }

    #[test]
    fn pattern_mode_limits_to_5_matches_by_default() {
        // Create input with 10 matches in the middle section
        let match_positions: Vec<usize> = (20..=70).step_by(5).collect(); // 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70 = 11 matches
        let input = generate_lines_with_matches(100, &match_positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Count how many match lines appear
        let match_count = stdout.matches("contains ERROR").count();
        assert_eq!(match_count, 5, "Should show exactly 5 matches by default");
    }

    #[test]
    fn pattern_mode_custom_match_limit() {
        let match_positions: Vec<usize> = (20..=70).step_by(5).collect();
        let input = generate_lines_with_matches(100, &match_positions, "ERROR");

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
            .args(["-C", "1", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // With context of 1: lines 49, 50 (match), 51
        assert!(stdout.contains("line 49"), "Should contain 1 line before match");
        assert!(stdout.contains("line 50 contains ERROR"), "Should contain match");
        assert!(stdout.contains("line 51"), "Should contain 1 line after match");

        // Should NOT contain lines further out
        assert!(!stdout.contains("line 47"), "Should not contain line 47 with context 1");
        assert!(!stdout.contains("line 53"), "Should not contain line 53 with context 1");
    }

    #[test]
    fn pattern_mode_zero_context() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-C", "0", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // With context of 0: only the matching line
        assert!(stdout.contains("line 50 contains ERROR"), "Should contain match");
        assert!(!stdout.contains("line 49"), "Should not contain context lines");
        assert!(!stdout.contains("line 51"), "Should not contain context lines");
    }

    #[test]
    fn pattern_mode_no_matches_in_middle() {
        // All matches are in the head or tail sections
        let input = generate_lines_with_matches(100, &[5, 95], "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Should still show the matches marker but with no matches
        // (the matches in head/tail are shown as part of those sections)
        assert!(stdout.contains("line 5 contains ERROR"), "Match in head should appear");
        assert!(stdout.contains("line 95 contains ERROR"), "Match in tail should appear");
    }

    #[test]
    fn pattern_mode_still_shows_head_and_tail() {
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Should have first 10 lines
        assert!(stdout.contains("line 1"), "Should contain head");
        assert!(stdout.contains("line 10"), "Should contain end of head");

        // Should have last 10 lines
        assert!(stdout.contains("line 91"), "Should contain start of tail");
        assert!(stdout.contains("line 100"), "Should contain tail");
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
        // 25 lines: head (1-10) and tail (16-25) don't overlap
        // But lines 11-15 are "middle" and should be truncated
        let input = generate_lines(25);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Each line should appear exactly once
        for i in 1..=10 {
            let count = lines.iter().filter(|&&l| l == format!("line {}", i)).count();
            assert_eq!(count, 1, "line {} should appear exactly once", i);
        }
        for i in 16..=25 {
            let count = lines.iter().filter(|&&l| l == format!("line {}", i)).count();
            assert_eq!(count, 1, "line {} should appear exactly once", i);
        }
    }

    #[test]
    fn no_duplicate_lines_when_match_overlaps_head() {
        // Match at line 8 with context 3 would show lines 5-11
        // But lines 1-10 are already in head
        let input = generate_lines_with_matches(100, &[8], "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Lines 1-10 should appear exactly once (in head)
        for i in 1..=10 {
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
        // But lines 91-100 are already in tail
        let input = generate_lines_with_matches(100, &[93], "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Lines 91-100 should appear exactly once (in tail)
        for i in 91..=100 {
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
    fn handles_very_long_lines() {
        let long_line = "x".repeat(100_000);
        let input = format!("{}\nshort\n{}", long_line, long_line);

        trunc()
            .write_stdin(input.clone())
            .assert()
            .success()
            .stdout(format!("{}\n", input));
    }

    #[test]
    fn handles_binary_looking_content() {
        // Content with null bytes and other binary-looking data
        let input = "line 1\nline \0 2\nline 3";

        trunc()
            .write_stdin(input)
            .assert()
            .success();
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
        let assert = cmd.arg(r"\[bracket\]").write_stdin(input).assert().success();

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
// OUTPUT SIZE GUARANTEES
// =============================================================================

mod output_size {
    use super::*;

    #[test]
    fn default_mode_max_21_lines() {
        // With any input > 20 lines, output should be exactly 21 lines
        // (10 head + 1 truncated + 10 tail)
        for size in [50, 100, 1000, 10000] {
            let input = generate_lines(size);

            let mut cmd = trunc();
            let assert = cmd.write_stdin(input).assert().success();

            let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
            let line_count = stdout.lines().count();
            assert_eq!(
                line_count, 21,
                "Output should be exactly 21 lines for input of {} lines",
                size
            );
        }
    }

    #[test]
    fn pattern_mode_max_56_lines() {
        // Maximum output in pattern mode:
        // 10 head + 1 matches + 35 (5 matches * 7 context) + 10 tail = 56
        // But with overlap deduplication, could be less

        // Create input with many matches spread out
        let match_positions: Vec<usize> = vec![30, 40, 50, 60, 70];
        let input = generate_lines_with_matches(100, &match_positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd.arg("ERROR").write_stdin(input).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let line_count = stdout.lines().count();

        // Should not exceed 56 lines
        assert!(
            line_count <= 56,
            "Pattern mode output ({} lines) should not exceed 56 lines",
            line_count
        );
    }
}
