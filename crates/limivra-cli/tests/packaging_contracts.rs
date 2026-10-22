#![cfg(target_os = "linux")]

use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the Unix epoch")
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "limivra-packaging-{}-{nonce}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create isolated packaging test directory");
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_success(output: &Output, operation: &str) {
    assert!(
        output.status.success(),
        "{operation} failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn standalone_tarball_checksum_install_and_uninstall_are_complete() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("CLI package is nested two levels below the workspace root");
    let package_script = project_root.join("scripts/package-linux.sh");
    assert!(
        package_script.is_file(),
        "Linux packaging script must exist at {}",
        package_script.display()
    );

    let temp = TempDir::new();
    let dist_dir = temp.path().join("dist");
    let target_dir = temp.path().join("cargo-target");
    let package = Command::new(&package_script)
        .current_dir(project_root)
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", &target_dir)
        .env("LIMIVRA_DIST_DIR", &dist_dir)
        .output()
        .expect("run the standalone Linux packager");
    assert_success(&package, "package-linux.sh");

    let archive_name = "limivra-linux-x86_64.tar.gz";
    let archive = dist_dir.join(archive_name);
    assert!(
        archive.is_file(),
        "packager must create {}",
        archive.display()
    );
    let checksum = Command::new("sha256sum")
        .args(["--check", "SHA256SUMS"])
        .current_dir(&dist_dir)
        .output()
        .expect("verify package SHA-256 manifest");
    assert_success(&checksum, "sha256sum --check");

    let listing = Command::new("tar")
        .args(["-tzf"])
        .arg(&archive)
        .output()
        .expect("list package archive");
    assert_success(&listing, "tar -tzf");
    let entries = String::from_utf8(listing.stdout).expect("tar listing is UTF-8");
    let required = [
        "limivra",
        "install.sh",
        "uninstall.sh",
        "LICENSE",
        "NOTICES.md",
        "README.md",
    ];
    let actual: Vec<_> = entries.lines().collect();
    assert_eq!(
        actual.len(),
        required.len(),
        "unexpected archive entries: {entries}"
    );
    for entry in required {
        assert!(actual.contains(&entry), "archive omits {entry}: {entries}");
    }

    let extracted = temp.path().join("extracted");
    fs::create_dir(&extracted).expect("create extraction directory");
    let extraction = Command::new("tar")
        .args(["-xzf"])
        .arg(&archive)
        .args(["-C"])
        .arg(&extracted)
        .output()
        .expect("extract standalone package");
    assert_success(&extraction, "tar -xzf");

    let prefix = temp.path().join("prefix with spaces");
    fs::create_dir(&prefix).expect("create isolated install prefix");
    let install = Command::new(extracted.join("install.sh"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("PREFIX", &prefix)
        .output()
        .expect("run package installer");
    assert_success(&install, "install.sh");

    let installed_binary = prefix.join("bin/limivra");
    let metadata = fs::metadata(&installed_binary).expect("installed binary exists");
    assert_eq!(metadata.permissions().mode() & 0o777, 0o755);
    let version = Command::new(&installed_binary)
        .env_clear()
        .arg("--version")
        .output()
        .expect("run installed CLI");
    assert_success(&version, "limivra --version");
    let version_text = String::from_utf8(version.stdout).expect("version output is UTF-8");
    assert!(
        version_text.contains("limivra"),
        "unexpected version: {version_text}"
    );

    let uninstall = Command::new(extracted.join("uninstall.sh"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("PREFIX", &prefix)
        .output()
        .expect("run package uninstaller");
    assert_success(&uninstall, "uninstall.sh");
    assert!(
        !installed_binary.exists(),
        "uninstaller left the binary behind"
    );
    assert!(
        fs::read_dir(&prefix)
            .expect("inspect isolated install prefix")
            .next()
            .is_none(),
        "uninstaller left files or directories under the isolated prefix"
    );
}
