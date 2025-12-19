#[test]
fn fails_with_no_arguments() {
    assert_cmd::Command::new(assert_cmd::cargo_bin!("espup"))
        .assert()
        .failure();
}

#[test]
fn verify_help() {
    assert_cmd::Command::new(assert_cmd::cargo_bin!("espup"))
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn verify_install_help() {
    assert_cmd::Command::new(assert_cmd::cargo_bin!("espup"))
        .args(["install", "--help"])
        .assert()
        .success();
}

#[test]
fn verify_update_help() {
    assert_cmd::Command::new(assert_cmd::cargo_bin!("espup"))
        .args(["update", "--help"])
        .assert()
        .success();
}

#[test]
fn verify_uninstall_help() {
    assert_cmd::Command::new(assert_cmd::cargo_bin!("espup"))
        .args(["uninstall", "--help"])
        .assert()
        .success();
}
