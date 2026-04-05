use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const CODEX_ROOT_INSTRUCTIONS: &str = include_str!("../../adapters/codex/root.instructions.md");
const CLAUDE_CONTRACT_TEMPLATE: &str = include_str!("../../adapters/claude/CLAUDE.md");
const CLAUDE_BASH_GUARD: &str = include_str!("../../adapters/claude/hooks/emberflow-bash-guard.sh");
const CLAUDE_WRITE_GUARD: &str =
    include_str!("../../adapters/claude/hooks/emberflow-write-guard.sh");

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("install") => {
            let target = parse_target(args.next())?;
            let mut scope = Scope::User;
            let mut project_root = None;

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--scope" => {
                        scope = parse_scope(
                            args.next()
                                .ok_or_else(|| "missing value after --scope".to_string())?,
                        )?;
                    }
                    "--project-root" => {
                        let value = args
                            .next()
                            .ok_or_else(|| "missing value after --project-root".to_string())?;
                        project_root = Some(PathBuf::from(value));
                    }
                    "-h" | "--help" => {
                        print_usage();
                        return Ok(());
                    }
                    other => return Err(format!("Unknown argument: {other}")),
                }
            }

            install(target, scope, project_root)
        }
        Some("version") | Some("--version") | Some("-V") => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("doctor") => doctor(),
        Some("help") | Some("-h") | Some("--help") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => Err(format!("Unknown command: {other}")),
    }
}

fn print_usage() {
    println!(
        "Usage: emberflow install claude|codex [--scope user|project] [--project-root PATH]\n\
         emberflow doctor\n\
         emberflow version\n\n\
         Install EmberFlow from the installed binary without depending on a repo checkout."
    );
}

#[derive(Copy, Clone)]
enum Target {
    Claude,
    Codex,
}

impl Target {
    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            other => Err(format!("Unknown install target: {other}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    fn target_dir(self) -> &'static str {
        match self {
            Self::Claude => ".claude",
            Self::Codex => ".codex",
        }
    }
}

#[derive(Copy, Clone)]
enum Scope {
    User,
    Project,
}

impl Scope {
    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "user" => Ok(Self::User),
            "project" => Ok(Self::Project),
            other => Err(format!("Invalid scope: {other}")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
        }
    }
}

fn parse_target(value: Option<String>) -> Result<Target, String> {
    Target::from_str(
        value
            .as_deref()
            .ok_or_else(|| usage_error("missing install target"))?,
    )
}

fn parse_scope(value: String) -> Result<Scope, String> {
    Scope::from_str(&value)
}

fn usage_error(message: &str) -> String {
    format!("{message}\n\n{}", usage())
}

fn usage() -> String {
    "Usage: emberflow install claude|codex [--scope user|project] [--project-root PATH]".to_string()
}

fn install(target: Target, scope: Scope, project_root: Option<PathBuf>) -> Result<(), String> {
    let target_root = resolve_target_root(target, scope, project_root.as_deref())?;
    fs::create_dir_all(target_root.join("bin")).map_err(|error| error.to_string())?;

    let mcp_binary = sibling_mcp_binary()?;
    let wrapper_path = target_root.join("bin/emberflow-mcp");
    write_executable_file(&wrapper_path, &mcp_wrapper_script(&mcp_binary))?;

    match target {
        Target::Codex => install_codex(&target_root, &wrapper_path)?,
        Target::Claude => {
            install_claude(&target_root, scope, project_root.as_deref(), &mcp_binary)?
        }
    }

    println!(
        "Installed EmberFlow {} adapter in {}",
        target.as_str(),
        target_root.display()
    );
    Ok(())
}

fn doctor() -> Result<(), String> {
    let current_exe = env::current_exe().map_err(|error| error.to_string())?;
    let mcp_binary = sibling_mcp_binary()?;

    println!("emberflow executable: {}", current_exe.display());
    println!("emberflow-mcp sibling: {}", mcp_binary.display());
    Ok(())
}

fn install_codex(target_root: &Path, wrapper_path: &Path) -> Result<(), String> {
    fs::write(
        target_root.join("root.instructions.md"),
        CODEX_ROOT_INSTRUCTIONS,
    )
    .map_err(|error| error.to_string())?;

    let config_path = target_root.join("config.toml");
    let existing = fs::read_to_string(&config_path).unwrap_or_default();
    let section = format!(
        "[mcp_servers.emberflow]\ncommand = \"{}\"\nargs = []\n",
        wrapper_path.display()
    );
    let updated = upsert_toml_section(&existing, "[mcp_servers.emberflow]", &section);
    fs::write(&config_path, updated).map_err(|error| error.to_string())?;

    Ok(())
}

fn install_claude(
    target_root: &Path,
    scope: Scope,
    project_root: Option<&Path>,
    mcp_binary: &Path,
) -> Result<(), String> {
    let hooks_dir = target_root.join("hooks");
    fs::create_dir_all(&hooks_dir).map_err(|error| error.to_string())?;

    write_executable_file(
        &hooks_dir.join("emberflow-bash-guard.sh"),
        CLAUDE_BASH_GUARD,
    )?;
    write_executable_file(
        &hooks_dir.join("emberflow-write-guard.sh"),
        CLAUDE_WRITE_GUARD,
    )?;

    let settings_path = target_root.join("settings.json");
    let settings = load_json_object(&settings_path)?;
    let updated_settings = update_claude_settings(settings, &hooks_dir, scope, project_root)?;
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&updated_settings).map_err(|error| error.to_string())? + "\n",
    )
    .map_err(|error| error.to_string())?;

    let claude_path = target_root.join("CLAUDE.md");
    let existing = fs::read_to_string(&claude_path).unwrap_or_default();
    let merged = merge_claude_contract(&existing);
    fs::write(&claude_path, merged).map_err(|error| error.to_string())?;

    let claude_bin = resolve_claude_bin()?;
    let scope_flag = ["--scope", scope.as_str()];
    let mcp_binary_arg = mcp_binary.to_string_lossy().into_owned();

    let _ = Command::new(&claude_bin)
        .args(["mcp", "remove", "emberflow", "-s", scope.as_str()])
        .status();
    let status = Command::new(&claude_bin)
        .args([
            "mcp",
            "add",
            scope_flag[0],
            scope_flag[1],
            "emberflow",
            "--",
        ])
        .arg(mcp_binary_arg)
        .status()
        .map_err(|error| error.to_string())?;

    if !status.success() {
        return Err(format!("claude mcp add failed with status {status}"));
    }
    Ok(())
}

fn resolve_target_root(
    target: Target,
    scope: Scope,
    project_root: Option<&Path>,
) -> Result<PathBuf, String> {
    let base = match scope {
        Scope::User => home_dir()?,
        Scope::Project => project_root
            .map(Path::to_path_buf)
            .unwrap_or_else(current_dir),
    };

    Ok(base.join(target.target_dir()))
}

fn current_dir() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn home_dir() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn sibling_mcp_binary() -> Result<PathBuf, String> {
    let current_exe = env::current_exe().map_err(|error| error.to_string())?;
    let binary_dir = current_exe
        .parent()
        .ok_or_else(|| "could not resolve installed emberflow binary directory".to_string())?;
    Ok(binary_dir.join("emberflow-mcp"))
}

fn resolve_claude_bin() -> Result<PathBuf, String> {
    if let Some(value) = env::var_os("CLAUDE_BIN") {
        return Ok(PathBuf::from(value));
    }

    find_in_path("claude").ok_or_else(|| "claude CLI not found".to_string())
}

fn find_in_path(binary: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for entry in env::split_paths(&path) {
        let candidate = entry.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn mcp_wrapper_script(mcp_binary: &Path) -> String {
    format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n\nexec \"{}\" \"$@\"\n",
        mcp_binary.display()
    )
}

fn write_executable_file(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| error.to_string())?;

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .map_err(|error| error.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn load_json_object(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }

    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn ensure_object(value: &mut Value) -> Result<&mut Map<String, Value>, String> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }

    match value {
        Value::Object(map) => Ok(map),
        _ => Err("expected object".to_string()),
    }
}

fn ensure_array(value: &mut Value) -> Result<&mut Vec<Value>, String> {
    if !value.is_array() {
        *value = Value::Array(Vec::new());
    }

    match value {
        Value::Array(array) => Ok(array),
        _ => Err("expected array".to_string()),
    }
}

fn object_child<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    let entry = parent
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    ensure_object(entry)
}

fn array_child<'a>(
    parent: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Vec<Value>, String> {
    let entry = parent
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    ensure_array(entry)
}

fn push_unique_string(values: &mut Vec<Value>, value: &str) {
    if values.iter().any(|item| item.as_str() == Some(value)) {
        return;
    }

    values.push(Value::String(value.to_string()));
}

fn update_claude_settings(
    mut settings: Value,
    hooks_dir: &Path,
    scope: Scope,
    project_root: Option<&Path>,
) -> Result<Value, String> {
    let root = ensure_object(&mut settings)?;

    let permissions = object_child(root, "permissions")?;
    {
        let allow = array_child(permissions, "allow")?;
        for item in ["Bash", "Read", "Edit", "Write", "mcp__emberflow__*"] {
            push_unique_string(allow, item);
        }
    }

    {
        let additional_dirs = array_child(permissions, "additionalDirectories")?;
        if matches!(scope, Scope::Project) {
            if let Some(project_root) = project_root {
                push_unique_string(additional_dirs, &project_root.display().to_string());
            }
        }
    }

    let hooks = object_child(root, "hooks")?;
    let pre_tool_use = array_child(hooks, "PreToolUse")?;
    add_claude_hook(
        pre_tool_use,
        "Bash",
        hooks_dir.join("emberflow-bash-guard.sh"),
    )?;
    add_claude_hook(
        pre_tool_use,
        "Edit",
        hooks_dir.join("emberflow-write-guard.sh"),
    )?;
    add_claude_hook(
        pre_tool_use,
        "Write",
        hooks_dir.join("emberflow-write-guard.sh"),
    )?;
    Ok(settings)
}

fn add_claude_hook(hooks: &mut Vec<Value>, matcher: &str, command: PathBuf) -> Result<(), String> {
    let command_text = command.display().to_string();

    let already_present = hooks.iter().any(|entry| {
        entry
            .get("matcher")
            .and_then(Value::as_str)
            .map(|value| value == matcher)
            .unwrap_or(false)
            && entry
                .get("hooks")
                .and_then(Value::as_array)
                .is_some_and(|nested| {
                    nested.iter().any(|hook| {
                        hook.get("type")
                            .and_then(Value::as_str)
                            .map(|value| value == "command")
                            .unwrap_or(false)
                            && hook
                                .get("command")
                                .and_then(Value::as_str)
                                .map(|value| value == command_text)
                                .unwrap_or(false)
                    })
                })
    });

    if already_present {
        return Ok(());
    }

    hooks.push(json!({
        "matcher": matcher,
        "hooks": [
            {
                "type": "command",
                "command": command_text,
            }
        ]
    }));
    Ok(())
}

fn merge_claude_contract(existing: &str) -> String {
    let start_marker = "<!-- EMBERFLOW CONTRACT START -->";
    let end_marker = "<!-- EMBERFLOW CONTRACT END -->";

    if let (Some(start), Some(end)) = (existing.find(start_marker), existing.find(end_marker)) {
        let before = existing[..start].trim_end();
        let after = existing[end + end_marker.len()..].trim_start();
        let mut merged = String::new();

        if !before.is_empty() {
            merged.push_str(before);
            merged.push_str("\n\n");
        }

        merged.push_str(CLAUDE_CONTRACT_TEMPLATE.trim_end());

        if !after.is_empty() {
            merged.push_str("\n\n");
            merged.push_str(after);
        }

        merged
    } else {
        let mut merged = existing.trim_end().to_string();
        if !merged.is_empty() {
            merged.push_str("\n\n");
        }
        merged.push_str(CLAUDE_CONTRACT_TEMPLATE.trim_end());
        merged
    }
}

fn upsert_toml_section(existing: &str, header: &str, section: &str) -> String {
    let lines: Vec<&str> = existing.lines().collect();
    let start = lines.iter().position(|line| line.trim() == header);

    match start {
        Some(start_idx) => {
            let end = lines[start_idx + 1..]
                .iter()
                .position(|line| line.trim_start().starts_with('['))
                .map(|offset| start_idx + 1 + offset)
                .unwrap_or(lines.len());

            let before = lines[..start_idx].join("\n").trim_end().to_string();
            let after = lines[end..].join("\n").trim_start().to_string();
            let body = section.trim_end();

            let mut result = String::new();
            if !before.is_empty() {
                result.push_str(&before);
                result.push_str("\n\n");
            }
            result.push_str(body);
            if !after.is_empty() {
                result.push_str("\n\n");
                result.push_str(&after);
            }
            result.push('\n');
            result
        }
        None => {
            let mut result = existing.trim_end().to_string();
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(section.trim_end());
            result.push('\n');
            result
        }
    }
}
