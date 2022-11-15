#[test]
fn fails_with_no_arguments() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .assert()
        .failure();
}

#[test]
fn verify_help() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn verify_install_help() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .args(["install", "--help"])
        .assert()
        .success();
}

#[test]
fn verify_update_help() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .args(["update", "--help"])
        .assert()
        .success();
}

#[test]
fn verify_uninstall_help() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .args(["uninstall", "--help"])
        .assert()
        .success();
}
