use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::tempdir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("emberflow crate is nested inside the monorepo")
        .to_path_buf()
}

fn claude_install_script() -> PathBuf {
    repo_root().join("emberflow/adapters/claude/install.sh")
}

fn fake_claude_binary(dir: &Path, log: &Path) -> PathBuf {
    let path = dir.join("claude");
    let log_path = log.display().to_string().replace('\'', r#"'"'"'"#);
    let script = r#"#!/usr/bin/env bash
printf '%s\n' "$*" >> '__LOG_PATH__'
exit 0
"#;
    fs::write(&path, script.replace("__LOG_PATH__", &log_path)).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn run_install(
    home: &Path,
    project_root: Option<&Path>,
    claude_bin: &Path,
    scope: &str,
) -> std::process::Output {
    let mut command = Command::new("bash");
    command.arg(claude_install_script());
    command.arg("--scope").arg(scope);
    if let Some(project_root) = project_root {
        command.arg("--project-root").arg(project_root);
    }
    command.env("HOME", home);
    command.env("CLAUDE_BIN", claude_bin);
    command.current_dir(repo_root());
    command.output().expect("Claude installer should execute")
}

fn extract_emberflow_contract(contents: &str) -> String {
    let start = contents
        .find("<!-- EMBERFLOW CONTRACT START -->")
        .expect("emberflow contract start marker");
    let end = contents
        .find("<!-- EMBERFLOW CONTRACT END -->")
        .expect("emberflow contract end marker");
    contents[start..end].to_string()
}

fn run_hook(script: &Path, payload: serde_json::Value) -> (i32, String, String) {
    let mut child = Command::new("bash")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("hook should spawn");

    {
        let stdin = child.stdin.as_mut().expect("hook stdin");
        stdin
            .write_all(payload.to_string().as_bytes())
            .expect("write hook payload");
    }

    let output = child.wait_with_output().expect("hook output");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

// @minter:integration bootstrap-displays-required-transparency-fields missing-transparency-fields-are-marked-unavailable post-mutation-transparency-reloads-canonical-state handoff-displays-current-emberflow-state contract-applies-uniformly-to-all-clients
#[test]
fn claude_adapter_installs_additive_transparency_contract() {
    let tmp = tempdir().unwrap();
    let home = tmp.path().join("home");
    let project_root = tmp.path().join("project");
    let fake_bin = tmp.path().join("bin");
    let log = tmp.path().join("claude.log");
    let user_root = home.join(".claude");
    let project_claude_root = project_root.join(".claude");

    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&project_root).unwrap();
    fs::create_dir_all(&fake_bin).unwrap();
    fs::create_dir_all(&user_root).unwrap();
    fs::create_dir_all(&project_claude_root).unwrap();
    fs::create_dir_all(log.parent().unwrap()).unwrap();

    let claude_bin = fake_claude_binary(&fake_bin, &log);

    fs::write(
        user_root.join("CLAUDE.md"),
        "Existing instructions that must stay in place.\n",
    )
    .unwrap();
    fs::write(
        user_root.join("settings.json"),
        r#"{
  "permissions": {
    "allow": ["Read"],
    "additionalDirectories": ["~/custom"],
    "deny": ["Bash(sudo *)"]
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "/tmp/custom-hook.sh"
          }
        ]
      }
    ]
  }
}"#,
    )
    .unwrap();

    let output = run_install(&home, None, &claude_bin, "user");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let user_claude = fs::read_to_string(user_root.join("CLAUDE.md")).unwrap();
    assert!(user_claude.contains("Existing instructions that must stay in place."));
    assert!(user_claude.contains("<!-- EMBERFLOW CONTRACT START -->"));
    assert!(user_claude.contains("Source: EmberFlow"));
    assert!(user_claude.contains("Track:"));
    assert!(user_claude.contains("Track status:"));
    assert!(user_claude.contains("Task status:"));
    assert!(user_claude.contains("Phase:"));
    assert!(user_claude.contains("Next:"));
    assert!(user_claude.contains("unavailable from EmberFlow"));
    assert!(user_claude.contains("emberflow://tracks/{trackId}/transparency"));
    assert!(user_claude.contains("before the handoff proceeds"));
    assert!(user_claude.contains("durable EmberFlow track"));
    assert!(user_claude.contains("root/orchestrator layer"));
    assert!(user_claude
        .contains("Worker agents do not own canonical EmberFlow track, plan, or task writes"));
    assert!(user_claude.contains("Hooks may enforce observable invariants"));

    let user_contract = extract_emberflow_contract(&user_claude);

    let first_settings = fs::read_to_string(user_root.join("settings.json")).unwrap();
    assert!(first_settings.contains("\"mcp__emberflow__*\""));
    assert!(first_settings.contains("/tmp/custom-hook.sh"));
    assert!(first_settings.contains("emberflow-bash-guard.sh"));
    assert!(first_settings.contains("emberflow-write-guard.sh"));

    let output = run_install(&home, Some(&project_root), &claude_bin, "project");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let project_claude = fs::read_to_string(project_claude_root.join("CLAUDE.md")).unwrap();
    assert!(project_claude.contains("<!-- EMBERFLOW CONTRACT START -->"));
    assert!(project_claude.contains("Source: EmberFlow"));
    assert!(project_claude.contains("Track:"));
    assert!(project_claude.contains("Track status:"));
    assert!(project_claude.contains("Task status:"));
    assert!(project_claude.contains("Phase:"));
    assert!(project_claude.contains("Next:"));
    assert!(project_claude.contains("unavailable from EmberFlow"));
    assert!(project_claude.contains("emberflow://tracks/{trackId}/transparency"));
    assert!(project_claude.contains("before the handoff proceeds"));
    assert!(project_claude.contains("durable EmberFlow track"));
    assert!(project_claude.contains("root/orchestrator layer"));
    assert!(project_claude
        .contains("Worker agents do not own canonical EmberFlow track, plan, or task writes"));
    assert!(project_claude.contains("Hooks may enforce observable invariants"));

    let project_contract = extract_emberflow_contract(&project_claude);
    assert_eq!(user_contract, project_contract);

    let project_settings = fs::read_to_string(project_claude_root.join("settings.json")).unwrap();
    assert!(project_settings.contains("\"mcp__emberflow__*\""));
    assert!(project_settings.contains("emberflow-bash-guard.sh"));
    assert!(project_settings.contains("emberflow-write-guard.sh"));

    let log_text = fs::read_to_string(&log).unwrap();
    assert!(log_text.contains("mcp remove emberflow"));
    assert!(log_text.contains("mcp add --scope user emberflow"));
    assert!(log_text.contains("mcp add --scope project emberflow"));
}

// @minter:integration durable-mutations-use-emberflow-surface canonical-state-remains-authoritative
#[test]
fn claude_adapter_blocks_direct_emberflow_writes() {
    let hook_dir = repo_root().join("emberflow/adapters/claude/hooks");
    let bash_guard = hook_dir.join("emberflow-bash-guard.sh");
    let write_guard = hook_dir.join("emberflow-write-guard.sh");

    let (code, _stdout, stderr) = run_hook(
        &bash_guard,
        serde_json::json!({
            "tool_input": {
                "command": "rm -rf .emberflow"
            }
        }),
    );
    assert_eq!(code, 2, "unexpected stderr: {stderr}");
    assert!(stderr.contains("EmberFlow"));

    let (code, _stdout, stderr) = run_hook(
        &bash_guard,
        serde_json::json!({
            "tool_input": {
                "command": "sqlite3 .emberflow/emberflow.db '.schema'"
            }
        }),
    );
    assert_eq!(code, 2, "unexpected stderr: {stderr}");
    assert!(stderr.contains("canonical"));

    let (code, _stdout, stderr) = run_hook(
        &bash_guard,
        serde_json::json!({
            "tool_input": {
                "command": "echo hello > .emberflow/context/status.md"
            }
        }),
    );
    assert_eq!(code, 2, "unexpected stderr: {stderr}");
    assert!(stderr.contains("projections"));

    let (code, _stdout, stderr) = run_hook(
        &write_guard,
        serde_json::json!({
            "tool_input": {
                "file_path": ".emberflow/tracks/track-001/brief.md"
            }
        }),
    );
    assert_eq!(code, 2, "unexpected stderr: {stderr}");
    assert!(stderr.contains("derived views"));

    let (code, _stdout, _stderr) = run_hook(
        &write_guard,
        serde_json::json!({
            "tool_input": {
                "file_path": "docs/README.md"
            }
        }),
    );
    assert_eq!(code, 0);
}
