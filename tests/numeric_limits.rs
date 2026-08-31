use assert_cmd::Command;

#[test]
fn pathological_numeric_limits_are_rejected_before_reading_input() {
    let maximum_usize = usize::MAX.to_string();
    let cases: &[(&str, &[&str])] = &[
        ("--last", &[]),
        ("--context", &["match"]),
        ("--match-last", &["match"]),
        ("--width", &[]),
        ("--first", &["--last", "1"]),
    ];

    for (flag, trailing_arguments) in cases {
        let mut arguments = vec![*flag, maximum_usize.as_str()];
        arguments.extend_from_slice(trailing_arguments);

        let output = Command::cargo_bin("trunc")
            .expect("trunc binary should build")
            .args(arguments)
            .write_stdin("match\n")
            .output()
            .expect("trunc should reject the arguments");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert_eq!(
            output.status.code(),
            Some(2),
            "{flag} should be rejected as a usage error, stderr:\n{stderr}"
        );
        assert!(
            stdout.is_empty(),
            "{flag} should be rejected before stdin is processed, stdout:\n{stdout}"
        );
        assert!(
            stderr.contains(flag) && stderr.contains("maximum supported value is 1000000"),
            "{flag} should report the supported bound, stderr:\n{stderr}"
        );
        assert!(
            !stderr.contains("panicked"),
            "{flag} must not reach a panic, stderr:\n{stderr}"
        );
    }
}
