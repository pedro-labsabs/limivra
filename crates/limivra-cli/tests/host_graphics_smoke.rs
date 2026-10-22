use std::process::{Command, Output};

fn run_smoke(wayland_display: Option<&str>, display: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_limivra-cli"));
    command
        .arg("host-graphics-smoke")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("DISPLAY");

    if let Some(value) = wayland_display {
        command.env("WAYLAND_DISPLAY", value);
    }
    if let Some(value) = display {
        command.env("DISPLAY", value);
    }

    command.output().expect("run host graphics smoke command")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("smoke output is UTF-8")
}

#[test]
fn headless_smoke_exits_successfully_with_explicit_skip_diagnostic() {
    let output = run_smoke(None, None);

    assert!(
        output.status.success(),
        "headless is a supported skip state"
    );
    let diagnostic = stdout(&output);
    assert!(diagnostic.starts_with("SKIP: host headless;"));
    assert!(diagnostic.contains("WAYLAND_DISPLAY"));
    assert!(diagnostic.contains("DISPLAY"));
}

#[test]
fn empty_display_environment_values_are_treated_as_headless() {
    let output = run_smoke(Some(""), Some(""));

    assert!(output.status.success());
    assert!(stdout(&output).starts_with("SKIP: host headless;"));
}

#[test]
fn x11_environment_is_reported_without_exposing_the_display_value() {
    let output = run_smoke(None, Some(":limivra-ci-mock"));

    assert!(output.status.success());
    let diagnostic = stdout(&output);
    assert!(diagnostic.starts_with("DETECTED: X11 environment indicated;"));
    assert!(diagnostic.contains("server connection not verified"));
    assert!(!diagnostic.contains(":limivra-ci-mock"));
}

#[test]
fn wayland_environment_takes_precedence_over_x11_and_is_not_echoed() {
    let output = run_smoke(Some("limivra-ci-mock-wayland"), Some(":99"));

    assert!(output.status.success());
    let diagnostic = stdout(&output);
    assert!(diagnostic.starts_with("DETECTED: Wayland environment indicated;"));
    assert!(diagnostic.contains("server connection not verified"));
    assert!(!diagnostic.contains("limivra-ci-mock-wayland"));
    assert!(!diagnostic.contains(":99"));
}
