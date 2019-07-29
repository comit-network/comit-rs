use cnd::{
    load_settings::{default_config_path, load_settings},
    settings::{CndSettings, HttpSocket},
};
use rand::{
    rngs::{mock::StepRng, OsRng},
    Rng,
};
use spectral::prelude::*;
use std::net::{IpAddr, Ipv4Addr};

/// Helper function that gives settings different from the default ones with
/// hopefully as little maintenance-effort as possible. In these tests, we don't
/// care about the actual settings, we just want to read, write and compare them
fn custom_settings<R: Rng>(rand: R) -> CndSettings {
    CndSettings {
        http_api: HttpSocket {
            address: IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
            port: 1,
        },
        ..CndSettings::default(rand)
    }
}

fn rng() -> StepRng {
    StepRng::new(0, 0)
}

// Some of these tests rely on the fact that our settings are different from the
// default ones This test makes sure this is always the case, even if the
// default settings change If this test breaks, modify the `custom_settings`
// above.
#[test]
fn our_custom_settings_are_different_from_the_default_ones() {
    assert_ne!(custom_settings(rng()), CndSettings::default(rng()))
}

#[test]
fn given_override_is_not_specified_should_read_from_default_location() {
    let fake_home_dir = tempfile::tempdir().unwrap().into_path();
    let expected_settings = custom_settings(OsRng)
        .write_to(default_config_path(fake_home_dir.as_path()))
        .unwrap();

    let actual_settings = load_settings(None, Some(&fake_home_dir), rng());

    assert_that(&actual_settings).is_ok_containing(&expected_settings);
}

#[test]
fn given_override_is_specified_should_read_from_given_location() {
    let temp_config_file = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
    let config_file_override = temp_config_file.path().to_path_buf();
    let expected_settings = custom_settings(OsRng)
        .write_to(config_file_override.to_path_buf())
        .unwrap();

    let actual_settings = load_settings(Some(&config_file_override), None, rng());

    assert_that(&actual_settings).is_ok_containing(&expected_settings);
}

#[test]
fn given_override_is_not_specified_and_file_not_found_should_write_default_settings() {
    let fake_home_dir = tempfile::tempdir().unwrap().into_path();

    let actual_settings = load_settings(None, Some(&fake_home_dir), rng());

    assert_that(&actual_settings).is_ok_containing(&CndSettings::default(rng()));
}

#[test]
fn given_override_is_not_specified_and_no_home_dir_is_given_should_fail() {
    let actual_settings = load_settings(None, None, rng());

    assert_that(&actual_settings).is_err();
}
