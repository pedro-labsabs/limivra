use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("relógio deve estar após epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("limivra-cli-test-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("diretório temporário deve ser criado");
        Self(path)
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

fn cli_binary() -> PathBuf {
    std::env::current_exe()
        .expect("teste deve ter caminho")
        .parent()
        .and_then(Path::parent)
        .expect("binário CLI deve estar no diretório de perfil do Cargo")
        .join("limivra-cli")
}

fn run_cli(args: &[&str]) -> (Output, TempDir, PathBuf) {
    let temp_dir = TempDir::new();
    let fake_tools = temp_dir.path().join("bin");
    fs::create_dir_all(&fake_tools).expect("diretório de ferramentas deve ser criado");
    let marker = temp_dir.path().join("android-tool-was-invoked");

    for tool in ["adb", "scrcpy"] {
        let path = fake_tools.join(tool);
        fs::write(
            &path,
            "#!/bin/sh\nprintf '%s\\n' called > \"$LIMIVRA_TOOL_MARKER\"\n",
        )
        .expect("fake de ferramenta deve ser escrito");
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .expect("fake de ferramenta deve ser executável");
    }

    let output = Command::new(cli_binary())
        .args(args)
        .env("PATH", fake_tools)
        .env("LIMIVRA_TOOL_MARKER", &marker)
        .output()
        .expect("CLI deve ser iniciada");
    (output, temp_dir, marker)
}

fn combined_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn help_succeeds_and_is_readable_without_starting_android_tools() {
    let (output, _temp_dir, marker) = run_cli(&["--help"]);
    let help = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(help.contains("Uso:"), "ajuda deve conter Uso: {help}");
    assert!(help.contains("--help"), "ajuda deve listar --help: {help}");
    assert!(
        help.contains("--version"),
        "ajuda deve listar --version: {help}"
    );
    assert!(!marker.exists(), "ajuda não pode executar adb ou scrcpy");
}

#[test]
fn version_succeeds_without_starting_android_tools() {
    let (output, _temp_dir, marker) = run_cli(&["--version"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("limivra "));
    assert!(!marker.exists(), "versão não pode executar adb ou scrcpy");
}

#[test]
fn unsupported_start_does_not_fabricate_active_state_or_leak_identifiers() {
    let identifier = "SYNTHETIC-DEVICE-ID-NOT-REAL-42";
    let (output, _temp_dir, marker) = run_cli(&["start", "--serial", identifier]);
    let text = combined_output(&output);

    assert!(
        !text.contains("Active"),
        "CLI não pode expor estado ativo: {text}"
    );
    assert!(
        !text.contains(identifier),
        "CLI não pode ecoar identificadores: {text}"
    );
    assert!(!marker.exists(), "CLI não pode executar adb ou scrcpy");
    assert_eq!(Some(2), output.status.code());
}
