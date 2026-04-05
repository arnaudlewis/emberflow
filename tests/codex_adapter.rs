use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn install_codex_adapter() -> (TempDir, TempDir) {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let script = repo_root().join("adapters/codex/install.sh");

    let output = Command::new("bash")
        .arg(script)
        .arg("--scope")
        .arg("project")
        .arg("--project-root")
        .arg(project.path())
        .env("HOME", home.path())
        .current_dir(repo_root())
        .output()
        .expect("install script should run");

    assert!(
        output.status.success(),
        "codex adapter install failed: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    (home, project)
}

fn root_instructions(project: &TempDir) -> String {
    fs::read_to_string(project.path().join(".codex/root.instructions.md"))
        .expect("root instructions should be installed")
}

fn config(project: &TempDir) -> String {
    fs::read_to_string(project.path().join(".codex/config.toml")).expect("config should exist")
}

// @minter:integration client-initializes-before-tracked-work
#[test]
fn client_initializes_before_tracked_work() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("initialize"));
    assert!(instructions.contains("tracked work"));
    assert!(instructions.contains("active workspace"));
}

// @minter:integration missing-workspace-root-fails-explicitly
#[test]
fn missing_workspace_root_fails_explicitly() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("fail explicitly"));
    assert!(instructions.contains("no silent fallback"));
    assert!(instructions.contains("workspace root"));
}

// @minter:integration canonical-state-remains-authoritative
#[test]
fn canonical_state_remains_authoritative() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("canonical state"));
    assert!(instructions.contains("derived views"));
    assert!(instructions.contains("source of truth"));
}

// @minter:integration client-loads-minimal-context-before-resume
#[test]
fn client_loads_minimal_context_before_resume() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("minimal context"));
    assert!(instructions.contains("track"));
    assert!(instructions.contains("track transparency"));
    assert!(instructions.contains("next"));
}

// @minter:integration client-blocks-planned-work-without-resolved-track
#[test]
fn planned_work_requires_resolved_track() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("planned work"));
    assert!(instructions.contains("durable track"));
    assert!(instructions.contains("does not proceed"));
}

// @minter:integration root-declares-durable-track-transitions advisory-updates-do-not-write-canonical-track-state
#[test]
fn root_layer_owns_semantic_tracked_state_transitions() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("root/orchestrator layer"));
    assert!(instructions
        .contains("Worker agents do not own canonical EmberFlow track, plan, or task writes"));
    assert!(instructions.contains("Hooks may enforce observable invariants"));
}

// @minter:integration durable-mutations-use-emberflow-surface
#[test]
fn durable_mutations_use_emberflow_surface() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);
    let config = config(&project);

    assert!(instructions.contains("durable mutations"));
    assert!(instructions.contains("EmberFlow MCP"));
    assert!(instructions.contains("projected files"));
    assert!(config.contains("[mcp_servers.emberflow]"));
}

// @minter:integration bootstrap-displays-required-transparency-fields
#[test]
fn bootstrap_displays_required_transparency_fields() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);
    let config = config(&project);

    assert!(instructions.contains("Source: EmberFlow"));
    assert!(instructions.contains("Track:"));
    assert!(instructions.contains("Track status:"));
    assert!(instructions.contains("Task status:"));
    assert!(instructions.contains("Phase:"));
    assert!(instructions.contains("Next:"));
    assert!(config.contains("developer_instructions"));
}

// @minter:integration missing-transparency-fields-are-marked-unavailable
#[test]
fn missing_transparency_fields_are_marked_unavailable() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("unavailable from EmberFlow"));
}

// @minter:integration post-mutation-transparency-reloads-canonical-state
#[test]
fn post_mutation_transparency_reloads_canonical_state() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("emberflow://tracks/{trackId}/transparency"));
}

// @minter:integration handoff-displays-current-emberflow-state
#[test]
fn handoff_displays_current_emberflow_state() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("handoff"));
    assert!(instructions.contains("current EmberFlow state"));
}

// @minter:integration emberflow-unavailable-never-produces-guessed-canonical-state
#[test]
fn emberflow_unavailable_never_produces_guessed_canonical_state() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("guessed canonical state"));
    assert!(instructions.contains("Source: unavailable"));
}

// @minter:integration contract-applies-uniformly-to-all-clients
#[test]
fn contract_applies_uniformly_to_all_clients() {
    let (_, project) = install_codex_adapter();
    let instructions = root_instructions(&project);

    assert!(instructions.contains("Claude"));
    assert!(instructions.contains("Codex"));
    assert!(instructions.contains("orchestration layer"));
    assert!(instructions.contains("same contract"));
}
