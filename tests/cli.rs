use assert_cmd::Command;

#[test]
fn help_includes_core_options() {
    let mut cmd = Command::cargo_bin("system-monitor").expect("binary exists");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Usage"))
        .stdout(predicates::str::contains("--interval"))
        .stdout(predicates::str::contains("--sort"))
        .stdout(predicates::str::contains("--once"))
        .stdout(predicates::str::contains("--json"));
}

#[test]
fn version_matches_package_version() {
    let expected = format!("system-monitor {}", env!("CARGO_PKG_VERSION"));

    let mut cmd = Command::cargo_bin("system-monitor").expect("binary exists");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains(expected));
}
