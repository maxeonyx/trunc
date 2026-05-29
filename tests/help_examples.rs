//! Verifies help-text examples stay runnable.

use assert_cmd::Command;

fn trunc() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("trunc")
}

fn help_output() -> String {
    let assert = trunc().arg("--help").assert().success();
    String::from_utf8_lossy(&assert.get_output().stdout).into_owned()
}

fn example_commands(help: &str) -> Vec<Vec<String>> {
    help.lines()
        .filter_map(|line| line.trim().strip_prefix('$'))
        .filter(|line| line.contains("trunc"))
        .map(str::trim)
        .map(parse_example_command)
        .collect()
}

fn parse_example_command(example: &str) -> Vec<String> {
    let trunc_invocation = example.split('|').nth(1).map(str::trim).unwrap_or(example);

    let mut parts = trunc_invocation.split_whitespace();
    let binary = parts.next().expect("example should include trunc");
    assert_eq!(binary, "trunc", "example should invoke trunc: {example}");

    parts.map(str::to_string).collect()
}

#[test]
fn help_examples_exit_successfully() {
    let help = help_output();
    let examples = example_commands(&help);

    assert!(
        !examples.is_empty(),
        "--help should include at least one example command"
    );

    let sample_input = (1..=120)
        .map(|i| {
            if i == 60 {
                "warning at line 60".to_string()
            } else if i == 90 {
                format!("{}timeout{}", "x".repeat(120), "y".repeat(120))
            } else {
                format!("line {i}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    for args in examples {
        let printable = if args.is_empty() {
            "trunc".to_string()
        } else {
            format!("trunc {}", args.join(" "))
        };

        trunc()
            .args(&args)
            .write_stdin(format!("{sample_input}\n"))
            .assert()
            .success();

        assert!(!printable.is_empty());
    }
}
