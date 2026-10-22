#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::{
    collections::BTreeSet,
    env, fs,
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
            "limivra-rc-gate-{}-{nonce}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create isolated release-candidate gate directory");
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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("CLI package is nested two levels below the workspace root")
        .to_path_buf()
}

#[test]
fn candidate_docs_manifest_and_release_binary_match_workspace_version() {
    let root = workspace_root();
    let version = env!("CARGO_PKG_VERSION");
    let notes_path = root.join("docs/releases/v0.1.0-rc1.md");
    let checklist_path = root.join("docs/testing/issue-34-release-candidate-gate.md");

    assert!(
        notes_path.is_file(),
        "release notes must exist at {}",
        notes_path.display()
    );
    let notes = fs::read_to_string(&notes_path).expect("release notes are UTF-8");
    assert!(
        notes.contains(&format!("v{version}-rc1")),
        "candidate notes must align with workspace package version {version}"
    );
    for limitation in ["HyperOS", "Wayland", "headless", "sem root"] {
        assert!(
            notes.to_lowercase().contains(&limitation.to_lowercase()),
            "release notes must disclose {limitation} limits"
        );
    }
    let plain_notes = notes.replace('`', "");
    for security_claim in [
        "sem root",
        "sem APK companheiro",
        "sem bypass de segurança",
        "respeita integralmente as restrições de segurança e permissões do Android/ROM",
        "não contorna DRM, FLAG_SECURE nem controles de privilégio",
        "UHID",
    ] {
        assert!(
            plain_notes
                .to_lowercase()
                .contains(&security_claim.to_lowercase()),
            "release notes must explicitly state the security posture: {security_claim}"
        );
    }

    assert!(
        checklist_path.is_file(),
        "release-candidate checklist must exist at {}",
        checklist_path.display()
    );
    let checklist = fs::read_to_string(&checklist_path).expect("release checklist is UTF-8");
    for milestone in ["M0", "M1", "M2", "M3", "M4"] {
        assert!(
            checklist.contains(milestone),
            "release checklist must record readiness for {milestone}"
        );
    }
    assert!(
        checklist.contains("HOLD / NEEDS_EVIDENCE") && checklist.contains("Issue #20"),
        "release checklist must keep the physical USB gate open until its issue is reconciled"
    );

    let manifest_relative = "docs/releases/v0.1.0-rc1-docs.sha256";
    let manifest_path = root.join(manifest_relative);
    assert!(
        manifest_path.is_file(),
        "documentation checksum manifest must exist at {}",
        manifest_path.display()
    );
    let manifest = fs::read_to_string(&manifest_path).expect("documentation manifest is UTF-8");
    let mut declared_paths = BTreeSet::new();
    for line in manifest.lines() {
        let (digest, path) = line
            .split_once("  ")
            .expect("manifest uses sha256sum two-space format");
        assert_eq!(digest.len(), 64, "manifest digest must be SHA-256");
        assert!(
            digest.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "manifest digest must contain hexadecimal characters"
        );
        assert!(
            declared_paths.insert(path),
            "duplicate manifest path {path}"
        );
    }
    let expected_paths = BTreeSet::from([
        "docs/releases/v0.1.0-rc1.md",
        "docs/testing/issue-34-release-candidate-gate.md",
    ]);
    assert_eq!(
        declared_paths, expected_paths,
        "manifest must cover exactly the release notes and signed gate checklist"
    );
    let manifest_check = Command::new("sha256sum")
        .args(["--check", manifest_relative])
        .current_dir(&root)
        .output()
        .expect("verify release documentation checksums");
    assert_success(&manifest_check, "sha256sum --check documentation manifest");

    let temp = TempDir::new();
    let target_dir = temp.path().join("cargo-target");
    let dist_dir = temp.path().join("dist");
    let package_script = root.join("scripts/package-linux.sh");
    let package = Command::new(&package_script)
        .current_dir(&root)
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", &target_dir)
        .env("LIMIVRA_DIST_DIR", &dist_dir)
        .env("TMPDIR", temp.path())
        .output()
        .expect("build the release binary through the distribution packager");
    assert_success(&package, "package-linux.sh release build");

    let package_checksum = Command::new("sha256sum")
        .args(["--check", "SHA256SUMS"])
        .current_dir(&dist_dir)
        .output()
        .expect("verify standalone release package checksum");
    assert_success(&package_checksum, "sha256sum --check package manifest");

    let extracted = temp.path().join("extracted");
    fs::create_dir(&extracted).expect("create release archive extraction directory");
    let extraction = Command::new("tar")
        .args(["-xzf"])
        .arg(dist_dir.join("limivra-linux-x86_64.tar.gz"))
        .arg("-C")
        .arg(&extracted)
        .output()
        .expect("extract standalone release package");
    assert_success(&extraction, "tar -xzf release candidate");

    let release_binary = extracted.join("limivra");
    assert!(
        release_binary.is_file(),
        "release binary must exist at {}",
        release_binary.display()
    );
    let version_output = Command::new(&release_binary)
        .arg("--version")
        .output()
        .expect("execute release binary version command");
    assert_success(&version_output, "release binary --version");
    let version_text = String::from_utf8(version_output.stdout).expect("version output is UTF-8");
    assert!(
        version_text.contains(version),
        "release binary version {version_text:?} must match workspace version {version}"
    );
}
