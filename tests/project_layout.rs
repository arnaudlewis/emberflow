use emberflow::runtime::service::{EmberFlowRuntime, RuntimeStore};
use emberflow::EmberFlowSurface;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

fn init_git_repo(root: &Path) {
    let status = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .expect("git init to run");
    assert!(status.success(), "git init failed");
}

fn init_git_repo_with_commit(root: &Path) {
    init_git_repo(root);

    let git = |args: &[&str]| {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .expect("git command to run");
        assert!(status.success(), "git command failed: {:?}", args);
    };

    git(&["config", "user.email", "emberflow@example.com"]);
    git(&["config", "user.name", "EmberFlow Test"]);
    fs::write(root.join("README.md"), "# repo\n").unwrap();
    git(&["add", "README.md"]);
    git(&["commit", "-m", "init"]);
}

fn add_linked_worktree(repo_root: &Path, worktree: &Path) {
    let status = Command::new("git")
        .args(["worktree", "add", "--quiet"])
        .arg(worktree)
        .arg("HEAD")
        .current_dir(repo_root)
        .status()
        .expect("git worktree add to run");
    assert!(status.success(), "git worktree add failed");
}

// @minter:integration emberflow-resolves-project-root-from-git-common-dir
// @minter:integration canonical-track-resolves-project-state-root
// @minter:integration emberflow-v1-resolves-shared-project-db-path
#[test]
fn resolves_project_root_from_git_common_dir() {
    let repo = tempdir().unwrap();
    init_git_repo(repo.path());

    let workspace = repo.path().join("nested-worktree");
    fs::create_dir_all(&workspace).unwrap();
    let expected_root = fs::canonicalize(repo.path()).unwrap();

    let runtime = EmberFlowRuntime::from_workspace_root(&workspace).unwrap();
    let init = runtime.initialize().unwrap();

    assert_eq!(
        init.workspace_db.project_root,
        expected_root.display().to_string()
    );
    assert_eq!(
        init.workspace_db.state_root,
        expected_root.join(".emberflow").display().to_string()
    );
    assert_eq!(
        init.workspace_db.default_path,
        expected_root
            .join(".emberflow/emberflow.db")
            .display()
            .to_string()
    );
    assert_eq!(init.workspace_db.projection_mode, "canonical");
}

// @minter:integration emberflow-falls-back-to-local-root-without-git
// @minter:integration emberflow-v1-falls-back-to-local-root-without-git
#[test]
fn falls_back_to_local_root_without_git() {
    let workspace = tempdir().unwrap();

    let runtime = EmberFlowRuntime::from_workspace_root(workspace.path()).unwrap();
    let init = runtime.initialize().unwrap();

    assert_eq!(
        init.workspace_db.project_root,
        workspace.path().display().to_string()
    );
    assert_eq!(
        init.workspace_db.state_root,
        workspace.path().join(".emberflow").display().to_string()
    );
    assert_eq!(
        init.workspace_db.default_path,
        workspace
            .path()
            .join(".emberflow/emberflow.db")
            .display()
            .to_string()
    );
    assert_eq!(init.workspace_db.projection_mode, "canonical");
}

// @minter:integration emberflow-honors-config-root-override
// @minter:integration canonical-track-allows-root-override
#[test]
fn honors_config_root_override() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("emberflow.config.json"),
        r#"{"mode":"projected","root":"shared-project"}"#,
    )
    .unwrap();

    let runtime = EmberFlowRuntime::from_workspace_root(workspace.path()).unwrap();
    let init = runtime.initialize().unwrap();
    let expected_root = workspace.path().join("shared-project");

    assert_eq!(
        init.workspace_db.project_root,
        expected_root.display().to_string()
    );
    assert_eq!(
        init.workspace_db.state_root,
        expected_root.join(".emberflow").display().to_string()
    );
    assert_eq!(
        init.workspace_db.default_path,
        expected_root
            .join(".emberflow/emberflow.db")
            .display()
            .to_string()
    );
    assert_eq!(init.workspace_db.projection_mode, "projected");
}

// @minter:integration emberflow-honors-config-root-override
#[test]
fn loads_config_from_worktree_toplevel() {
    let repo = tempdir().unwrap();
    init_git_repo_with_commit(repo.path());

    let worktree = repo.path().join("linked-worktree");
    add_linked_worktree(repo.path(), &worktree);
    fs::write(
        worktree.join("emberflow.config.json"),
        r#"{"mode":"projected","root":"shared-project"}"#,
    )
    .unwrap();
    let nested = worktree.join("nested/child");
    fs::create_dir_all(&nested).unwrap();
    let expected_worktree = fs::canonicalize(&worktree).unwrap();

    let runtime = EmberFlowRuntime::from_workspace_root(&nested).unwrap();
    let init = runtime.initialize().unwrap();
    let expected_root = expected_worktree.join("shared-project");

    assert_eq!(
        init.workspace_db.project_root,
        expected_root.display().to_string()
    );
    assert_eq!(
        runtime.layout.config_path.as_ref().unwrap(),
        &expected_worktree.join("emberflow.config.json")
    );
    assert_eq!(init.workspace_db.projection_mode, "projected");
}

// @minter:integration emberflow-projects-filesystem-artifacts-under-emberflow
#[test]
fn projects_filesystem_artifacts_under_emberflow() {
    let workspace = tempdir().unwrap();
    fs::write(
        workspace.path().join("emberflow.config.json"),
        r#"{"mode":"projected"}"#,
    )
    .unwrap();

    let runtime = EmberFlowRuntime::from_workspace_root(workspace.path()).unwrap();
    let runtime_status = runtime.project_runtime_status("track-001").unwrap();

    assert_eq!(runtime_status.target_path, ".emberflow/context/status.md");
    assert_eq!(
        runtime.layout.runtime_status_path(),
        ".emberflow/context/status.md"
    );
    assert_eq!(
        runtime.layout.track_directory_prefix(),
        ".emberflow/tracks/"
    );
}

// @minter:integration emberflow-runtime-new-keeps-legacy-db-path
#[test]
fn runtime_new_keeps_legacy_db_path() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("legacy.db");
    let store = RuntimeStore::new(&db_path).unwrap();
    store
        .create_track("legacy-track", "Legacy DB", "planning")
        .unwrap();

    let runtime = EmberFlowRuntime::from_db_path(&db_path).unwrap();
    let init = runtime.initialize().unwrap();

    assert_eq!(
        init.workspace_db.default_path,
        db_path.display().to_string()
    );
    assert_eq!(
        runtime
            .read_resource("emberflow://tracks/legacy-track/record")
            .unwrap()
            .content
            .get("title")
            .and_then(|value| value.as_str()),
        Some("Legacy DB")
    );
}

// @minter:integration emberflow-surface-new-keeps-legacy-db-path
#[test]
fn surface_new_keeps_legacy_db_path() {
    let tmp = tempdir().unwrap();
    let db_path = tmp.path().join("legacy.db");
    let store = RuntimeStore::new(&db_path).unwrap();
    store
        .create_track("legacy-track", "Legacy DB", "planning")
        .unwrap();

    let surface = EmberFlowSurface::from_db_path(&db_path).unwrap();
    let init = surface.initialize().unwrap();

    assert_eq!(
        init.workspace_db.default_path,
        db_path.display().to_string()
    );
    assert_eq!(
        surface
            .read_resource("emberflow://tracks/legacy-track/record")
            .unwrap()
            .content
            .get("title")
            .and_then(|value| value.as_str()),
        Some("Legacy DB")
    );
}
