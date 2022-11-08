#[test]
fn fails_with_no_arguments() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .assert()
        .failure();
}

#[test]
fn verify_install() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .arg("install")
        .assert()
        .success();
    let config_file = espup::config::Config::get_config_path().unwrap();
    assert!(config_file.exists());
}

#[test]
fn verify_update() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .arg("update")
        .assert()
        .success();
    let config_file = espup::config::Config::get_config_path().unwrap();
    assert!(config_file.exists());
}

#[test]
fn verify_uninstall() {
    assert_cmd::Command::cargo_bin("espup")
        .unwrap()
        .arg("uninstall")
        .assert()
        .success();
    let config_file = espup::config::Config::get_config_path().unwrap();
    assert!(!config_file.exists());
}
