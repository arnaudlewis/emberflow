use crate::error::{EmberFlowError, Result};
use serde_json::Value;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub const EMBERFLOW_CONFIG_FILE: &str = "emberflow.config.json";
pub const EMBERFLOW_STATE_DIR: &str = ".emberflow";
pub const EMBERFLOW_DB_FILE: &str = "emberflow.db";
pub const PROJECTED_RUNTIME_STATUS_PATH: &str = ".emberflow/context/status.md";
pub const PROJECTED_TRACK_DIRECTORY_PREFIX: &str = ".emberflow/tracks/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmberFlowMode {
    Canonical,
    Projected,
}

impl EmberFlowMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::Projected => "projected",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "canonical" => Some(Self::Canonical),
            "projected" => Some(Self::Projected),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmberFlowProjectLayout {
    pub workspace_root: PathBuf,
    pub project_root: PathBuf,
    pub state_root: PathBuf,
    pub db_path: PathBuf,
    pub mode: EmberFlowMode,
    pub config_path: Option<PathBuf>,
}

impl EmberFlowProjectLayout {
    pub fn discover<P: AsRef<Path>>(workspace_root: P) -> Result<Self> {
        let workspace_root = normalize_workspace_root(workspace_root.as_ref());
        let config_dir =
            git_worktree_root(&workspace_root).unwrap_or_else(|| workspace_root.clone());
        let default_state_root =
            git_common_root(&workspace_root).unwrap_or_else(|| workspace_root.clone());
        let config_path = config_dir.join(EMBERFLOW_CONFIG_FILE);
        let config = EmberFlowConfig::load(&config_path)?;
        let project_root = match config.root {
            Some(root) => resolve_root_override(&config_dir, &root),
            None => default_state_root,
        };
        let state_root = project_root.join(EMBERFLOW_STATE_DIR);
        Ok(Self {
            workspace_root,
            project_root,
            state_root: state_root.clone(),
            db_path: state_root.join(EMBERFLOW_DB_FILE),
            mode: config.mode.unwrap_or(EmberFlowMode::Canonical),
            config_path: config.present.then_some(config_path),
        })
    }

    pub fn from_db_path<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = normalize_path(db_path.as_ref());
        let state_root = db_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| db_path.clone());
        let project_root = match state_root.file_name().and_then(|name| name.to_str()) {
            Some(EMBERFLOW_STATE_DIR) => state_root
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| state_root.clone()),
            _ => state_root.clone(),
        };

        Ok(Self {
            workspace_root: project_root.clone(),
            project_root,
            state_root: state_root.clone(),
            db_path,
            mode: EmberFlowMode::Canonical,
            config_path: None,
        })
    }

    pub fn runtime_status_path(&self) -> &'static str {
        PROJECTED_RUNTIME_STATUS_PATH
    }

    pub fn track_directory_prefix(&self) -> &'static str {
        PROJECTED_TRACK_DIRECTORY_PREFIX
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct EmberFlowConfig {
    mode: Option<EmberFlowMode>,
    root: Option<String>,
    present: bool,
}

impl EmberFlowConfig {
    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)?;
        let value: Value = serde_json::from_str(&raw)?;
        let mode = match value.get("mode").and_then(Value::as_str) {
            Some(mode) => Some(EmberFlowMode::from_str(mode).ok_or_else(|| {
                EmberFlowError::UnsupportedValue {
                    field: "mode",
                    value: mode.to_string(),
                }
            })?),
            None => None,
        };
        let root = value
            .get("root")
            .and_then(Value::as_str)
            .map(ToString::to_string);

        Ok(Self {
            mode,
            root,
            present: true,
        })
    }
}

fn git_common_root(workspace_root: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(workspace_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let common_dir = stdout.trim();
    if common_dir.is_empty() {
        return None;
    }

    Path::new(common_dir).parent().map(Path::to_path_buf)
}

fn git_worktree_root(workspace_root: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--show-toplevel"])
        .current_dir(workspace_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let worktree_root = stdout.trim();
    if worktree_root.is_empty() {
        return None;
    }

    Some(PathBuf::from(worktree_root))
}

fn resolve_root_override(base_root: &Path, root_override: &str) -> PathBuf {
    let override_path = Path::new(root_override);
    if override_path.is_absolute() {
        normalize_path(override_path)
    } else {
        normalize_path(&base_root.join(override_path))
    }
}

fn normalize_workspace_root(path: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        normalize_path(&cwd.join(path))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}
