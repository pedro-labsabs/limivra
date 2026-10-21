use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

const SERIAL: &str = "LIMIVRA-TEST-SERIAL-XYZ";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
static FIXTURE_LOCK: Mutex<()> = Mutex::new(());

struct Fixture {
    root: PathBuf,
    bin: PathBuf,
    home: PathBuf,
    log: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "limivra-cli-process-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let bin = root.join("bin");
        let home = root.join("home");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir(&home).unwrap();
        for path in [&root, &bin, &home] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let log = root.join("fake-tool-arguments.log");
        Self {
            root,
            bin,
            home,
            log,
        }
    }

    fn install(&self, name: &str, body: &str) {
        let executable = self.bin.join(name);
        std::fs::write(&executable, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_limivra-cli"))
            .env_clear()
            .env("PATH", &self.bin)
            .env("HOME", &self.home)
            .env("LC_ALL", "C")
            .env("TERM", "dumb")
            .env("LIMIVRA_FAKE_LOG", &self.log)
            .args(args)
            .output()
            .expect("run limivra-cli with isolated fake tools")
    }

    fn install_device_listing(&self) {
        self.install(
            "adb",
            &format!(
                "printf '%s\\n' \"$*\" >> \"$LIMIVRA_FAKE_LOG\"\n\
                 [ \"$*\" = 'devices -l' ] || exit 98\n\
                 printf '%s\\n' 'List of devices attached' '{SERIAL} device model:Synthetic product:test transport_id:7'"
            ),
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn output_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn devices_uses_only_fake_adb_and_redacts_serial_sentinel() {
    let _fixture_lock = FIXTURE_LOCK.lock().unwrap();
    let fixture = Fixture::new();
    fixture.install_device_listing();

    let output = fixture.run(&["devices"]);

    assert!(output.status.success());
    let stdout = output_text(&output.stdout);
    let stderr = output_text(&output.stderr);
    assert!(stdout.contains("[REDACTED]"), "stdout: {stdout}");
    assert!(!stdout.contains(SERIAL), "stdout leaked serial: {stdout}");
    assert!(!stderr.contains(SERIAL), "stderr leaked serial: {stderr}");
    assert_eq!(
        std::fs::read_to_string(&fixture.log).unwrap(),
        "devices -l\n"
    );
}

#[test]
fn doctor_handles_missing_and_nonzero_tools() {
    let _fixture_lock = FIXTURE_LOCK.lock().unwrap();
    let missing = Fixture::new();
    let output = missing.run(&["doctor"]);
    assert!(output.status.success());
    let stdout = output_text(&output.stdout);
    assert!(stdout.contains("[FALHA] adb: não encontrado no PATH"));
    assert!(stdout.contains("[FALHA] scrcpy:"));

    let nonzero = Fixture::new();
    nonzero.install("adb", "exit 7");
    nonzero.install("scrcpy", "exit 127");
    let output = nonzero.run(&["doctor"]);
    assert!(output.status.success());
    let stdout = output_text(&output.stdout);
    assert!(stdout.contains("[FALHA] adb: não encontrado no PATH"));
    assert!(stdout.contains("[FALHA] scrcpy:"));
}

#[test]
fn invalid_utf8_or_oversized_adb_output_does_not_panic() {
    let _fixture_lock = FIXTURE_LOCK.lock().unwrap();
    let invalid = Fixture::new();
    invalid.install("adb", "printf '\\377'");
    let output = invalid.run(&["devices"]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = output_text(&output.stderr);
    assert!(stderr.contains("not valid UTF-8"), "stderr: {stderr}");
    assert!(stderr.len() < 1024);
    assert!(!stderr.contains(SERIAL));

    let oversized = Fixture::new();
    oversized.install("adb", "exec /usr/bin/head -c 1048577 /dev/zero");
    let output = oversized.run(&["devices"]);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = output_text(&output.stderr);
    assert!(
        stderr.contains("output exceeded its limit"),
        "stderr: {stderr}"
    );
    assert!(stderr.len() < 1024);
    assert!(!stderr.contains(SERIAL));
}

#[test]
fn probe_failure_has_bounded_non_sensitive_output() {
    let _fixture_lock = FIXTURE_LOCK.lock().unwrap();
    let fixture = Fixture::new();
    let adb = format!(
        "printf '%s\\n' \"$*\" >> \"$LIMIVRA_FAKE_LOG\"\n\
         case \"$*\" in\n\
           'devices -l') printf '%s\\n' 'List of devices attached' '{SERIAL} device model:Synthetic product:test transport_id:7' ;;\n\
           '-s {SERIAL} shell getprop ro.product.manufacturer') printf 'Xiaomi\\n' ;;\n\
           '-s {SERIAL} shell getprop ro.product.model') printf 'Synthetic\\n' ;;\n\
           '-s {SERIAL} shell getprop ro.build.version.sdk') printf '35\\n' ;;\n\
           '-s {SERIAL} shell getprop ro.build.version.release') printf '15\\n' ;;\n\
           '-s {SERIAL} shell getprop ro.build.version.incremental') printf 'test-build\\n' ;;\n\
           '-s {SERIAL} shell dumpsys display') exec /usr/bin/head -c 1048577 /dev/zero ;;\n\
           *) exit 99 ;;\n\
         esac"
    );
    fixture.install("adb", &adb);
    fixture.install(
        "scrcpy",
        "case \"${1-}\" in\n\
           --version) printf '%s\\n' 'scrcpy 3.2.1 <synthetic>' ;;\n\
           --help) printf '%s\\n' '--new-display --start-app' ;;\n\
           *) exit 98 ;;\n\
         esac",
    );

    let output = fixture.run(&["probe", "--serial", SERIAL]);

    assert!(output.status.success());
    let stdout = output_text(&output.stdout);
    let stderr = output_text(&output.stderr);
    assert!(stdout.contains("[REDACTED]"), "stdout: {stdout}");
    assert!(!stdout.contains(SERIAL), "stdout leaked serial: {stdout}");
    assert!(!stderr.contains(SERIAL), "stderr leaked serial: {stderr}");
    assert!(stdout.len() + stderr.len() < 4096);
    let calls = std::fs::read_to_string(&fixture.log).unwrap();
    assert!(calls.contains("devices -l"));
    assert!(calls.contains(&format!("-s {SERIAL} shell dumpsys display")));
}
