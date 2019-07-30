use cnd::config::{self, read_or_create_default, ConfigFile};
use rand::{
    rngs::{mock::StepRng, OsRng},
    Rng,
};
use spectral::prelude::*;

// Some of these tests rely on the fact that our config are different from the
// default ones This test makes sure this is always the case, even if the
// default config change If this test breaks, modify the `custom_settings`
// above.
#[test]
fn our_custom_settings_are_different_from_the_default_ones() {
    assert_ne!(custom_settings(rng()), ConfigFile::default(rng()))
}

#[test]
fn read_should_read_file_if_present() {
    let temp_config_file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
    let path = temp_config_file.path().to_path_buf();

    let expected_settings = custom_settings(OsRng).write_to(path.clone()).unwrap();

    let actual_settings = config::read_from(path);

    assert_that(&actual_settings).is_ok_containing(&expected_settings);
}

#[test]
fn read_or_create_default_should_read_file_if_present() {
    let fake_home_dir = tempfile::tempdir().unwrap().into_path();
    let expected_settings = custom_settings(OsRng)
        .write_to(ConfigFile::compute_default_path(fake_home_dir.as_path()))
        .unwrap();

    let actual_settings = read_or_create_default(Some(&fake_home_dir), rng());

    assert_that(&actual_settings).is_ok_containing(&expected_settings);
}

#[test]
fn read_or_create_default_should_create_file_if_not_present() {
    let fake_home_dir = tempfile::tempdir().unwrap().into_path();

    let actual_settings = read_or_create_default(Some(&fake_home_dir), rng());

    assert_that(&actual_settings).is_ok_containing(&ConfigFile::default(rng()));
}

#[test]
fn read_or_create_default_should_fail_if_no_homedir_is_given() {
    let actual_settings = read_or_create_default(None, rng());

    assert_that(&actual_settings).is_err();
}

/// Helper function that gives config different from the default ones with
/// hopefully as little maintenance-effort as possible. In these tests, we don't
/// care about the actual config, we just want to read, write and compare them
fn custom_settings<R: Rng>(rand: R) -> ConfigFile {
    let mut default_config = ConfigFile::default(rand);
    default_config.http_api.port = 1;

    default_config
}

fn rng() -> StepRng {
    StepRng::new(0, 0)
}
