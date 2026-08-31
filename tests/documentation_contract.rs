use assert_cmd::Command;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn repository_file(path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn cli_help() -> String {
    let output = Command::cargo_bin("trunc")
        .expect("trunc binary should build")
        .arg("--help")
        .output()
        .expect("trunc --help should run");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("help should be UTF-8")
}

fn option_default(help: &str, option: &str) -> usize {
    let option_start = help
        .find(option)
        .unwrap_or_else(|| panic!("help should document {option}"));
    let after_option = &help[option_start..];
    let default_start = after_option
        .find("[default: ")
        .unwrap_or_else(|| panic!("help should give a default for {option}"));
    let value = &after_option[default_start + "[default: ".len()..];
    let value = value
        .split(']')
        .next()
        .expect("default marker should close");
    value
        .parse()
        .unwrap_or_else(|error| panic!("default for {option} should be numeric: {error}"))
}

#[test]
fn published_defaults_follow_cli_help() {
    let help = cli_help();
    let first = option_default(&help, "--first");
    let last = option_default(&help, "--last");
    let matches = option_default(&help, "--matches");
    let context = option_default(&help, "--context");

    let readme = repository_file("README.md");
    let site = repository_file("docs/index.html");
    let skill = repository_file("docs/SKILL.md");

    assert!(
        site.contains(&format!("Show first {first} and last {last} lines")),
        "site defaults must match trunc --help"
    );
    assert!(
        site.contains(&format!("# First {first} + last {last} lines")),
        "the skill embedded in the site must match trunc --help"
    );
    assert!(
        skill.contains(&format!("# First {first} + last {last} lines")),
        "the downloadable skill must match trunc --help"
    );
    assert!(
        readme.contains(&format!("default: {matches} each")),
        "README match defaults must match trunc --help"
    );

    let selected_matches = matches * 2;
    let match_group_lines = context * 2 + 1;
    let pattern_max = first + last + 1 + selected_matches * (match_group_lines + 1);
    assert!(
        readme.contains(&format!(
            "| Pattern | ~{pattern_max} | {first} first + {selected_matches}×(1 marker + {match_group_lines}-line match group) + 1 end marker + {last} last |"
        )),
        "README pattern bound must be derived from the CLI defaults"
    );
}

#[test]
fn readme_release_assets_follow_workflow() {
    let workflow = repository_file(".github/workflows/ci.yml");
    let expected: BTreeSet<_> = workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("asset: "))
        .map(str::to_owned)
        .collect();

    let readme = repository_file("README.md");
    let section = readme
        .split_once("Available binaries:\n")
        .expect("README should list available binaries")
        .1
        .split_once("\nOn Unix")
        .expect("available-binaries section should end before Unix instructions")
        .0;
    let documented: BTreeSet<_> = section
        .lines()
        .filter_map(|line| line.split('`').nth(1))
        .map(str::to_owned)
        .collect();

    assert_eq!(
        documented, expected,
        "README release assets must exactly match the CI build matrix"
    );
}

#[test]
fn site_demo_uses_a_counted_marker() {
    let site = repository_file("docs/index.html");
    assert!(
        !site.contains("[... truncated ...]") && site.contains("lines truncated ...]"),
        "the site demo must use the current counted-marker format"
    );
}
