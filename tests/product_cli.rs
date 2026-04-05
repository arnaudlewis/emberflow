use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn built_emberflow_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_emberflow"))
}

fn make_executable(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn install_binary(prefix: &Path) -> PathBuf {
    let bin_dir = prefix.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let installed_emberflow = bin_dir.join("emberflow");
    fs::copy(built_emberflow_binary(), &installed_emberflow).unwrap();
    let mut permissions = fs::metadata(&installed_emberflow).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&installed_emberflow, permissions).unwrap();

    make_executable(
        &bin_dir.join("emberflow-mcp"),
        "#!/usr/bin/env bash\nexec printf '%s\\n' \"emberflow-mcp stub\"\n",
    );

    installed_emberflow
}

fn run_installed_emberflow(binary: &Path, args: &[&str], cwd: &Path, home: &Path) -> String {
    let output = Command::new(binary)
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .output()
        .expect("installed emberflow to execute");

    assert!(
        output.status.success(),
        "emberflow failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).to_string()
}

// @minter:integration durable-mutations-use-emberflow-surface
#[test]
fn installed_emberflow_configures_codex_without_repo_checkout() {
    let tmp = tempdir().unwrap();
    let prefix = tmp.path().join("prefix");
    let home = tmp.path().join("home");
    let project = tmp.path().join("project");

    fs::create_dir_all(&prefix).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(project.join(".codex")).unwrap();

    let binary = install_binary(&prefix);

    fs::write(
        project.join(".codex/config.toml"),
        "model = \"gpt-5.4\"\n\n[mcp_servers.custom]\ncommand = \"/tmp/custom\"\nargs = []\n",
    )
    .unwrap();

    let cwd = tmp.path().join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    let project_root_arg = project.display().to_string();
    run_installed_emberflow(
        &binary,
        &[
            "install",
            "codex",
            "--scope",
            "project",
            "--project-root",
            &project_root_arg,
        ],
        &cwd,
        &home,
    );

    let target_root = project.join(".codex");
    let wrapper = fs::read_to_string(target_root.join("bin/emberflow-mcp")).unwrap();
    let instructions = fs::read_to_string(target_root.join("root.instructions.md")).unwrap();
    let config = fs::read_to_string(target_root.join("config.toml")).unwrap();

    let binary_dir = prefix.join("bin");
    assert!(wrapper.contains(&binary_dir.join("emberflow-mcp").display().to_string()));
    assert!(wrapper.contains("#!/usr/bin/env bash"));
    assert!(instructions.contains("EmberFlow Codex Root Instructions"));
    assert!(instructions.contains("root/orchestrator layer"));
    assert!(instructions.contains("current EmberFlow state"));
    assert!(config.contains("[mcp_servers.custom]"));
    assert!(config.contains("[mcp_servers.emberflow]"));
    assert!(config.contains("emberflow-mcp"));
}

// @minter:integration bootstrap-displays-required-transparency-fields missing-transparency-fields-are-marked-unavailable handoff-displays-current-emberflow-state contract-applies-uniformly-to-all-clients
#[test]
fn installed_emberflow_configures_claude_without_repo_checkout() {
    let tmp = tempdir().unwrap();
    let prefix = tmp.path().join("prefix");
    let home = tmp.path().join("home");
    let project = tmp.path().join("project");
    let fake_bin = tmp.path().join("bin");
    let log = tmp.path().join("claude.log");

    fs::create_dir_all(&prefix).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(&fake_bin).unwrap();
    fs::create_dir_all(home.join(".claude")).unwrap();

    let binary = install_binary(&prefix);

    make_executable(
        &fake_bin.join("claude"),
        &format!(
            "#!/usr/bin/env bash\nprintf '%s\\n' \"$*\" >> '{}'\nexit 0\n",
            log.display()
        ),
    );

    fs::write(
        home.join(".claude/settings.json"),
        r#"{
  "permissions": {
    "allow": ["Read"],
    "additionalDirectories": ["~/custom"]
  }
}"#,
    )
    .unwrap();

    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join(".claude/CLAUDE.md"),
        "Existing instructions that must stay in place.\n",
    )
    .unwrap();

    let cwd = tmp.path().join("cwd");
    fs::create_dir_all(&cwd).unwrap();
    let project_root_arg = project.display().to_string();
    let output = Command::new(&binary)
        .args(["install", "claude", "--scope", "project", "--project-root"])
        .arg(&project_root_arg)
        .current_dir(&cwd)
        .env("HOME", &home)
        .env(
            "PATH",
            format!("{}:{}", fake_bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("CLAUDE_BIN", fake_bin.join("claude"))
        .output()
        .expect("installed emberflow to execute");

    assert!(
        output.status.success(),
        "emberflow failed with status {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let target_root = project.join(".claude");
    let wrapper = fs::read_to_string(target_root.join("bin/emberflow-mcp")).unwrap();
    let claude_md = fs::read_to_string(target_root.join("CLAUDE.md")).unwrap();
    let settings = fs::read_to_string(target_root.join("settings.json")).unwrap();
    let log_text = fs::read_to_string(&log).unwrap();

    let binary_dir = prefix.join("bin");
    assert!(wrapper.contains(&binary_dir.join("emberflow-mcp").display().to_string()));
    assert!(claude_md.contains("Existing instructions that must stay in place."));
    assert!(claude_md.contains("<!-- EMBERFLOW CONTRACT START -->"));
    assert!(claude_md.contains("Source: EmberFlow"));
    assert!(claude_md.contains("root/orchestrator layer"));
    assert!(claude_md
        .contains("Worker agents do not own canonical EmberFlow track, plan, or task writes"));
    assert!(settings.contains("\"mcp__emberflow__*\""));
    assert!(settings.contains("emberflow-bash-guard.sh"));
    assert!(settings.contains("emberflow-write-guard.sh"));
    assert!(log_text.contains("mcp remove emberflow"));
    assert!(log_text.contains("mcp add --scope project emberflow"));
    assert!(log_text.contains(&binary_dir.join("emberflow-mcp").display().to_string()));
}

#[test]
fn installed_emberflow_reports_sibling_mcp_path() {
    let tmp = tempdir().unwrap();
    let prefix = tmp.path().join("prefix");
    let home = tmp.path().join("home");
    let project = tmp.path().join("project");

    fs::create_dir_all(&prefix).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project).unwrap();

    let binary = install_binary(&prefix);
    let cwd = tmp.path().join("cwd");
    fs::create_dir_all(&cwd).unwrap();

    let output = run_installed_emberflow(&binary, &["doctor"], &cwd, &home);
    let expected_mcp = prefix.join("bin/emberflow-mcp");

    assert!(output.contains("emberflow-mcp"));
    assert!(output.contains(&expected_mcp.display().to_string()));
}
