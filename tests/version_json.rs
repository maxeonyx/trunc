use assert_cmd::Command;

#[test]
fn version_json_flag_prints_machine_readable_version() {
    let assert = Command::cargo_bin("trunc")
        .unwrap()
        .args(["--version", "--json"])
        .assert()
        .success();

    let value: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("stdout should be valid JSON");
    assert_eq!(value["package"], "trunc");
    assert_eq!(value["binary"], "trunc");
    assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
}
