//! Tests for informative truncation markers.
//!
//! trunc's primary audience is AI agents reading command output. When output is
//! truncated, the agent needs to know *how much* was removed — both for lines
//! skipped between sections and characters removed from long lines. Without
//! this, the agent can't judge whether the truncated content might contain
//! something important, or how to ask for more targeted output.
//!
//! These tests verify that every truncation marker communicates what was lost.

use assert_cmd::Command;

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
// WITHIN-LINE TRUNCATION: CHARACTER COUNT
// =============================================================================
//
// An AI agent seeing a truncated line needs to know the scale of what's missing.
// "[... 500 chars ...]" tells it whether the hidden content is a few chars or
// thousands — which determines whether to re-run with -w 0.
//
// The marker format is: [... N chars ...]
// where N is the number of characters that were removed.
//
// Test cases:
// - A 700-char line with default width (100) removes 500 chars → "[... 500 chars ...]"
// - Verify the marker contains the exact count
// - Verify the overall structure: <first 100><marker><last 100>
// - Custom width: -w 20 on a 100-char line → removes 60 chars → "[... 60 chars ...]"
// - Unicode: char count not byte count in the marker number

mod line_truncation_char_count {
    use super::*;

    #[test]
    fn marker_shows_chars_removed() {
        // 700-char line: first 100 + last 100 = 200 kept, 500 removed
        // Marker should say "[... 500 chars ...]"
        let line = "x".repeat(700);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert!(
            output_line.contains("[... 500 chars ...]"),
            "Should show 500 chars removed. Got: {}",
            output_line
        );
    }

    #[test]
    fn marker_structure_preserved() {
        // Verify: <first 100 chars>[... N chars ...]<last 100 chars>
        let first_100 = "A".repeat(100);
        let middle = "M".repeat(500);
        let last_100 = "Z".repeat(100);
        let line = format!("{}{}{}", first_100, middle, last_100);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert!(
            output_line.starts_with(&first_100),
            "Should start with first 100 chars"
        );
        assert!(
            output_line.ends_with(&last_100),
            "Should end with last 100 chars"
        );
        assert!(
            output_line.contains("[... 500 chars ...]"),
            "Should show 500 chars removed in marker"
        );
    }

    #[test]
    fn custom_width_char_count() {
        // -w 20 on a 100-char line: keeps 20+20=40, removes 60
        let line = "x".repeat(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-w", "20"])
            .write_stdin(format!("{}\n", line))
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert!(
            output_line.contains("[... 60 chars ...]"),
            "Should show 60 chars removed with -w 20. Got: {}",
            output_line
        );
    }

    #[test]
    fn unicode_chars_not_bytes_in_count() {
        // Each emoji is 1 char but 4 bytes. 300 emoji = 300 chars.
        // With default width 100: keeps 100+100, removes 100.
        // Marker should say "[... 100 chars ...]" not a byte count.
        let line = "\u{1F389}".repeat(300); // 🎉 × 300

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert!(
            output_line.contains("[... 100 chars ...]"),
            "Should count chars not bytes. Got: {}",
            output_line
        );
    }
}

// =============================================================================
// WITHIN-LINE TRUNCATION: ONLY WHEN IT SAVES SPACE
// =============================================================================
//
// Truncation that makes the output longer is counterproductive — it wastes
// tokens and confuses the reader. The marker "[... N chars ...]" is at minimum
// ~17 chars long (for single-digit N). So a line must be long enough that
// removing the middle and inserting the marker actually makes it shorter.
//
// Rule: only truncate if len(first) + len(marker) + len(last) < len(original)
//
// Test cases:
// - A 201-char line with default width: marker would be "[... 1 chars ...]" (17 chars)
//   Result would be 100 + 17 + 100 = 217 > 201 → should NOT truncate
// - A 250-char line: marker "[... 50 chars ...]" (18 chars)
//   Result would be 100 + 18 + 100 = 218 < 250 → SHOULD truncate
// - Boundary: find the exact threshold and test at/around it

mod line_truncation_only_when_shorter {
    use super::*;

    #[test]
    fn does_not_truncate_when_result_would_be_longer() {
        // 201 chars: removing 1 char but adding "[... 1 chars ...]" (17 chars)
        // Result: 100 + 17 + 100 = 217 > 201 → don't truncate
        let line = "x".repeat(201);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert_eq!(
            output_line.len(),
            201,
            "201-char line should pass through unchanged. Got {} chars",
            output_line.len()
        );
        assert!(
            !output_line.contains("[..."),
            "Should not contain truncation marker"
        );
    }

    #[test]
    fn does_not_truncate_when_result_same_length() {
        // Find a length where truncated == original and verify no truncation.
        // With width=100, marker for N chars is "[... N chars ...]"
        // If removed = N, marker_len = 7 + digits(N) + 7 = 14 + digits(N) ... wait
        // Actually: "[... " = 5, " chars ...]" = 11, total overhead = 16 + digits(N)
        // Break-even: N = 16 + digits(N)
        // For N < 100: digits = 2, so N = 18 → line len = 200 + 18 = 218
        // marker = "[... 18 chars ...]" = 18 chars. 100 + 18 + 100 = 218 = 218 → same length
        let line = "x".repeat(218);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert_eq!(
            output_line.len(),
            218,
            "218-char line should pass through unchanged (truncation doesn't save space)"
        );
    }

    #[test]
    fn truncates_when_result_is_shorter() {
        // 300 chars: removes 100, marker "[... 100 chars ...]" = 19 chars
        // Result: 100 + 19 + 100 = 219 < 300 → should truncate
        let line = "x".repeat(300);

        let mut cmd = trunc();
        let assert = cmd.write_stdin(format!("{}\n", line)).assert().success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let output_line = stdout.lines().next().unwrap();

        assert!(
            output_line.len() < 300,
            "300-char line should be truncated. Got {} chars",
            output_line.len()
        );
        assert!(
            output_line.contains("chars ..."),
            "Should contain char count marker"
        );
    }

    #[test]
    fn zero_width_still_disables_truncation() {
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
}

// =============================================================================
// ACROSS-LINE TRUNCATION: LINE COUNTS
// =============================================================================
//
// When trunc hides 980 lines between head and tail, the agent needs to know the
// scale. "[... 980 lines truncated ...]" lets it decide whether to re-run with
// different flags or whether the truncated section is likely irrelevant.
//
// Test cases:
// - 100 lines with default 10+10: marker says "[... 80 lines truncated ...]"
// - 1000 lines: "[... 980 lines truncated ...]"
// - 21 lines (just 1 truncated): "[... 1 lines truncated ...]"
// - Custom -f/-l counts affect the number in the marker

mod line_truncation_line_count {
    use super::*;

    #[test]
    fn default_mode_shows_line_count() {
        // 100 lines, first 10 + last 10, so 80 lines truncated
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("[... 80 lines truncated ...]"),
            "Should show 80 lines truncated. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn single_line_truncated() {
        // 21 lines with -f 10 -l 10: exactly 1 line truncated
        let input = generate_lines(21);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("[... 1 lines truncated ...]"),
            "Should show 1 line truncated. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn large_input_line_count() {
        let input = generate_lines(1000);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("[... 980 lines truncated ...]"),
            "Should show 980 lines truncated. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn custom_first_last_affects_count() {
        // 100 lines, -f 5 -l 3: 92 lines truncated
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "5", "-l", "3"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("[... 92 lines truncated ...]"),
            "Should show 92 lines truncated. Got:\n{}",
            stdout
        );
    }
}

// =============================================================================
// PATTERN MODE: INFORMATIVE MATCH MARKERS
// =============================================================================
//
// When an agent searches for a pattern, it needs to understand the full picture:
// how many matches exist in the input, which ones it's seeing, and how many it
// missed. This is critical for deciding whether to adjust -m (show more matches)
// or narrow the pattern.
//
// The total match count only appears on the first marker to avoid repetition.
// After the last shown match, the remaining count tells the agent what it missed.
//
// Marker formats:
//   First:  [... 36 lines truncated, match 1 shown ...]
//   Next:   [... 23 lines truncated, match 2 shown ...]
//   At -m:  [... 31 lines truncated, match 5/5 shown ...]
//   End:    [... 48 lines and 208 matches truncated (213 total) ...]
//
// When all matches shown (didn't hit -m limit):
//   Each:   [... 24 lines truncated, match 1 shown ...]
//   After:  [... 48 lines truncated ...]
//
// Zero matches:
//   [... 980 lines truncated, 0 matches found ...]
//
// The "(N total)" only appears on the end marker — we stream matches as
// found, so the total isn't known until EOF. The "N/N" notation only
// appears when the -m limit is hit.
//
// Test cases:
// - Single match (didn't hit limit): "match 1 shown", end marker plain
// - 5 shown out of 11 (-m 5): "match 5/5 shown", end marker "(11 total)"
// - All matches shown, didn't hit limit: no "N/N", end marker plain
// - End marker with remaining: "N lines and M matches truncated (T total)"
// - End marker with 0 remaining: just "N lines truncated" (no matches mentioned)
// - Zero matches found: "0 matches found"
// - Total match count must include matches AFTER the cutoff (requires scanning all input)

mod pattern_informative_markers {
    use super::*;

    #[test]
    fn first_marker_says_match_1_shown() {
        // Single match at line 50 → "match 1 shown"
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("match 1 shown"),
            "First marker should say 'match 1 shown'. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn single_match_no_total_annotation() {
        // 1 match found, 1 shown → no "(N total)" anywhere
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            !stdout.contains("total"),
            "Should not mention 'total' when all matches shown. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn last_match_at_limit_shows_n_of_n() {
        // 11 matches, -m 5 → match 5 says "match 5/5 shown"
        // Matches spaced 10 apart so contexts (±3) don't overlap
        let match_positions: Vec<usize> = (20..=70).step_by(10).collect(); // 20,30,40,50,60,70 = 6 matches
                                                                           // Plus extras to make > 5: add more beyond
        let mut positions: Vec<usize> = (15..=85).step_by(10).collect(); // 15,25,35,45,55,65,75,85 = 8 matches
                                                                         // Actually, let's just use widely-spaced matches in range 20-80
        positions = vec![20, 30, 40, 50, 60, 70, 75, 80]; // 8 matches, first 5 shown
        let input = generate_lines_with_matches(100, &positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("match 5/5 shown"),
            "Last shown match at limit should say 'match 5/5 shown'. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn earlier_matches_no_denominator() {
        // With -m 5 and enough matches, match 1 and 2 should NOT say "1/5"
        let match_positions: Vec<usize> = (20..=70).step_by(10).collect(); // 6 matches
        let input = generate_lines_with_matches(100, &match_positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        // "match 1 shown" should appear, NOT "match 1/5 shown"
        assert!(
            stdout.contains("match 1 shown"),
            "Earlier matches should just say 'match N shown'. Got:\n{}",
            stdout
        );
        assert!(
            !stdout.contains("match 1/"),
            "Earlier matches should NOT have denominator. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn subsequent_markers_show_match_number() {
        let match_positions: Vec<usize> = (20..=70).step_by(10).collect(); // 20,30,40,50,60,70 = 6 matches
        let input = generate_lines_with_matches(100, &match_positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("match 2 shown"),
            "Should have match 2 marker. Got:\n{}",
            stdout
        );
        assert!(
            stdout.contains("match 3 shown"),
            "Should have match 3 marker. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn end_marker_shows_remaining_matches_and_total() {
        // 11 matches, showing 5 → 6 remaining, 11 total
        let match_positions: Vec<usize> = (20..=70).step_by(5).collect(); // 11 matches
        let input = generate_lines_with_matches(100, &match_positions, "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("6 matches truncated"),
            "End marker should show 6 remaining matches. Got:\n{}",
            stdout
        );
        assert!(
            stdout.contains("(11 total)"),
            "End marker should show 11 total. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn all_matches_shown_end_marker_no_match_count() {
        // 3 matches, -m 5 (showing all, didn't hit limit) → end marker just says lines
        let input = generate_lines_with_matches(100, &[30, 50, 70], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "-m", "5", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // The marker after the last match context should just be "[... N lines truncated ...]"
        let last_match_context = lines
            .iter()
            .rposition(|l| l.contains("line 7"))
            .expect("Should have context around match at 70");

        let end_marker = lines[last_match_context + 1..]
            .iter()
            .find(|l| l.contains("truncated"));
        if let Some(marker) = end_marker {
            assert!(
                !marker.contains("matches"),
                "When all matches shown, end marker should not mention matches. Got: {}",
                marker
            );
        }
    }

    #[test]
    fn all_shown_no_n_of_n_notation() {
        // 3 matches, -m 5 → didn't hit limit, so no "N/N" on any match
        let input = generate_lines_with_matches(100, &[30, 50, 70], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "-m", "5", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            !stdout.contains("/3"),
            "Should not have N/N notation when limit not hit. Got:\n{}",
            stdout
        );
        assert!(
            stdout.contains("match 3 shown"),
            "Last match should just say 'match 3 shown'. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn zero_matches_found() {
        // Pattern mode with no matches in middle → "0 matches found"
        let input = generate_lines(100); // no matches at all

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "NONEXISTENT"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("0 matches found"),
            "Should indicate 0 matches found. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn total_includes_matches_past_cutoff() {
        // Ensure the total count includes matches that occur AFTER we stop showing.
        // 20 matches spread across middle, showing only 5 → total should be 20.
        let match_positions: Vec<usize> = (15..=90).step_by(4).collect(); // many matches in middle
        let input = generate_lines_with_matches(100, &match_positions, "ERROR");
        let expected_total = match_positions
            .iter()
            .filter(|&&p| p > 10 && p <= 90)
            .count();

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let total_str = format!("({} total)", expected_total);
        assert!(
            stdout.contains(&total_str),
            "Should show correct total {}. Got:\n{}",
            total_str,
            stdout
        );
    }

    #[test]
    fn line_count_in_match_markers() {
        // Verify the line count in pattern markers is correct
        // Match at line 50 with -f 10: gap from line 10 to context start (~47)
        let input = generate_lines_with_matches(100, &[50], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "-C", "3", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

        // Between head (line 10) and first context line (line 47): 36 lines truncated
        assert!(
            stdout.contains("36 lines truncated"),
            "Should show correct line count before match. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn markers_between_non_contiguous_matches() {
        // Matches at 30 and 60 with context 3:
        // Match 1 context: 27-33, match 2 context: 57-63
        // Gap: lines 34-56 = 23 lines
        let input = generate_lines_with_matches(100, &[30, 60], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "-C", "3", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("23 lines truncated, match 2 shown"),
            "Should show line count between match groups. Got:\n{}",
            stdout
        );
    }

    #[test]
    fn adjacent_matches_no_marker() {
        // Matches at 50 and 52 with context 3: contexts overlap (47-55)
        // No gap → no marker between them
        let input = generate_lines_with_matches(100, &[50, 52], "ERROR");

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10", "ERROR"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        // Find match lines
        let first_match = lines
            .iter()
            .position(|l| l.contains("line 50 contains"))
            .unwrap();
        let second_match = lines
            .iter()
            .position(|l| l.contains("line 52 contains"))
            .unwrap();

        // Between them should be context lines only, no marker
        for line in &lines[first_match + 1..second_match] {
            assert!(
                !line.starts_with("[..."),
                "Should not have marker between adjacent matches. Got: {}",
                line
            );
        }
    }
}

// =============================================================================
// FRAMEWORK DEMONSTRATION TESTS
// =============================================================================

mod framework_demo {
    use super::*;

    #[test]
    fn passing_demo() {
        // Proves the test framework works: trunc exists and runs
        trunc()
            .write_stdin("hello")
            .assert()
            .success()
            .stdout("hello\n");
    }

    #[test]
    fn failing_demo() {
        // Proves tests can detect wrong output.
        // This test should FAIL until the new marker format is implemented.
        // It verifies the simplest case: default mode line count in marker.
        let input = generate_lines(100);

        let mut cmd = trunc();
        let assert = cmd
            .args(["-f", "10", "-l", "10"])
            .write_stdin(input)
            .assert()
            .success();

        let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
        assert!(
            stdout.contains("[... 80 lines truncated ...]"),
            "Marker should show line count. Got:\n{}",
            stdout
        );
    }
}
