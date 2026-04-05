use crate::error::EmberFlowError;
use crate::mcp::surface::{
    EmberFlowSurface, EventRecord, InitializeResponse, LeaseInfo, ResourceReadResponse,
    ResourceViewDescriptor, TaskInput, TaskRecord, TrackBriefRecord, TrackBriefSectionInput,
    TrackBriefSectionRecord, TrackMetadataInput, TrackMetadataRecord, TrackPlanItemInput,
    TrackPlanItemRecord, TrackPlanPhaseInput, TrackPlanPhaseRecord, TrackPlanRecord, TrackRecord,
    WorkspaceDbInfo,
};
use crate::protocol::{
    EMBERFLOW_EVENT_RECORD_TOOL, EMBERFLOW_TASK_CLAIM_TOOL, EMBERFLOW_TASK_CREATE_TOOL,
    EMBERFLOW_TASK_RELEASE_TOOL, EMBERFLOW_TRACK_ARCHIVE_TOOL, EMBERFLOW_TRACK_BRIEF_REPLACE_TOOL,
    EMBERFLOW_TRACK_CREATE_TOOL, EMBERFLOW_TRACK_DELETE_TOOL, EMBERFLOW_TRACK_METADATA_UPSERT_TOOL,
    EMBERFLOW_TRACK_PLAN_REPLACE_TOOL,
};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

pub const STDIO_TRANSPORT_MODE: &str = "stdio";
pub const STDIO_TRANSPORT_HOSTING: &str = "local-process";
pub const STDIO_TRANSPORT_AUTH: &str = "none";
pub const STDIO_TRANSPORT_ENDPOINT: &str = "stdin-stdout";
const RESOURCE_WORKSPACE_OVERVIEW_URI: &str = "emberflow://workspace/overview";
const RESOURCE_CLIENT_CONTRACT_URI: &str = "emberflow://protocol/client-contract";
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const MCP_SERVER_NAME: &str = "emberflow";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StdioTransportConfig {
    pub cwd: Option<PathBuf>,
    pub workspace_root: Option<PathBuf>,
    pub state_path: Option<PathBuf>,
}

impl StdioTransportConfig {
    pub fn from_env() -> Self {
        let cwd = env::current_dir().ok();
        let workspace_root = env::var_os("EMBERFLOW_WORKSPACE_ROOT").map(PathBuf::from);
        let state_path = env::var_os("EMBERFLOW_STATE_PATH")
            .or_else(|| env::var_os("EMBERFLOW_DB_PATH"))
            .map(PathBuf::from);
        Self {
            cwd,
            workspace_root,
            state_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioTransportInfo {
    pub mode: &'static str,
    pub hosting: &'static str,
    pub auth: &'static str,
    pub endpoint: &'static str,
    pub workspace_root: String,
    pub workspace_db: WorkspaceDbInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportStartupError {
    pub source: &'static str,
    pub message: String,
}

impl TransportStartupError {
    fn workspace_resolution(message: impl Into<String>) -> Self {
        Self {
            source: "workspace-resolution",
            message: message.into(),
        }
    }
}

impl fmt::Display for TransportStartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.source, self.message)
    }
}

impl Error for TransportStartupError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportProtocolError {
    pub code: &'static str,
    pub message: String,
    pub field: Option<String>,
    pub reason: Option<String>,
}

impl TransportProtocolError {
    fn method_not_found(method: &str) -> Self {
        Self {
            code: "method_not_found",
            message: format!("unknown method: {method}"),
            field: None,
            reason: None,
        }
    }

    fn invalid_params(field: impl Into<String>, reason: impl Into<String>) -> Self {
        let field = field.into();
        Self {
            code: "invalid_params",
            message: format!("invalid parameters for {field}"),
            field: Some(field),
            reason: Some(reason.into()),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal_error",
            message: message.into(),
            field: None,
            reason: None,
        }
    }

    fn from_emberflow_error(error: EmberFlowError) -> Self {
        match error {
            EmberFlowError::UnsupportedValue { field, value } => Self {
                code: "invalid_params",
                message: format!("unsupported {field}: {value}"),
                field: Some(field.to_string()),
                reason: Some(format!("unsupported {field}: {value}")),
            },
            EmberFlowError::NotFound(value) => Self {
                code: "not_found",
                message: format!("record not found: {value}"),
                field: None,
                reason: Some(format!("record not found: {value}")),
            },
            EmberFlowError::Io(error) => Self::internal(format!("io error: {error}")),
            EmberFlowError::Sqlite(error) => Self::internal(format!("sqlite error: {error}")),
            EmberFlowError::Json(error) => Self::internal(format!("json error: {error}")),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "code": self.code,
            "message": self.message,
            "field": self.field,
            "reason": self.reason,
        })
    }
}

impl fmt::Display for TransportProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl Error for TransportProtocolError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportDiagnostic {
    pub channel: &'static str,
    pub level: &'static str,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct StdioTransportSession {
    surface: EmberFlowSurface,
    info: StdioTransportInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransportRequest {
    id: Option<Value>,
    method: String,
    params: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TransportResourceDescriptor {
    uri: String,
    name: &'static str,
    description: &'static str,
    mime_type: &'static str,
}

impl TransportResourceDescriptor {
    fn new(
        uri: impl Into<String>,
        name: &'static str,
        description: &'static str,
        mime_type: &'static str,
    ) -> Self {
        Self {
            uri: uri.into(),
            name,
            description,
            mime_type,
        }
    }
}

pub fn start_stdio_server(
    config: StdioTransportConfig,
) -> std::result::Result<StdioTransportSession, TransportStartupError> {
    let surface = build_surface(&config)?;
    let init = surface
        .initialize()
        .map_err(|error| TransportStartupError::workspace_resolution(error.to_string()))?;

    let info = StdioTransportInfo {
        mode: STDIO_TRANSPORT_MODE,
        hosting: STDIO_TRANSPORT_HOSTING,
        auth: STDIO_TRANSPORT_AUTH,
        endpoint: STDIO_TRANSPORT_ENDPOINT,
        workspace_root: init.workspace_db.project_root.clone(),
        workspace_db: init.workspace_db,
    };

    Ok(StdioTransportSession { surface, info })
}

pub fn start_stdio_server_from_env(
    env_config: StdioTransportConfig,
) -> std::result::Result<StdioTransportSession, TransportStartupError> {
    start_stdio_server(env_config)
}

fn build_surface(
    config: &StdioTransportConfig,
) -> std::result::Result<EmberFlowSurface, TransportStartupError> {
    if config.workspace_root.is_some() && config.state_path.is_some() {
        return Err(TransportStartupError::workspace_resolution(
            "EmberFlow stdio transport accepts either workspaceRoot or statePath, not both",
        ));
    }

    if let Some(state_path) = &config.state_path {
        return EmberFlowSurface::from_db_path(state_path)
            .map_err(|error| TransportStartupError::workspace_resolution(error.to_string()));
    }

    let workspace_root = if let Some(workspace_root) = &config.workspace_root {
        workspace_root.clone()
    } else {
        config
            .cwd
            .clone()
            .or_else(|| env::current_dir().ok())
            .ok_or_else(|| {
                TransportStartupError::workspace_resolution(
                    "EmberFlow workspace root could not be resolved from the current directory",
                )
            })?
    };

    if !workspace_root.exists() {
        return Err(TransportStartupError::workspace_resolution(format!(
            "EmberFlow workspace root could not be resolved: {}",
            workspace_root.display()
        )));
    }

    EmberFlowSurface::from_workspace_root(&workspace_root)
        .map_err(|error| TransportStartupError::workspace_resolution(error.to_string()))
}

impl StdioTransportSession {
    pub fn info(&self) -> &StdioTransportInfo {
        &self.info
    }

    pub fn initialize(&self) -> std::result::Result<InitializeResponse, TransportProtocolError> {
        self.surface
            .initialize()
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    fn list_resources(
        &self,
    ) -> std::result::Result<Vec<TransportResourceDescriptor>, TransportProtocolError> {
        let mut resources = vec![TransportResourceDescriptor::new(
            RESOURCE_WORKSPACE_OVERVIEW_URI,
            "workspace-overview",
            "Workspace-level canonical track overview from EmberFlow.",
            "application/json",
        )];

        let tracks = self
            .surface
            .list_active_tracks()
            .map_err(TransportProtocolError::from_emberflow_error)?;
        let active_track_ids: std::collections::BTreeSet<_> =
            tracks.iter().map(|track| track.id.clone()).collect();
        for track in tracks {
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tracks/{}/record", track.id),
                "track-record",
                "Canonical track record from EmberFlow.",
                "application/json",
            ));
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tracks/{}/resume", track.id),
                "track-resume",
                "Resume view for one track, combining metadata, summary, plan, and runtime visibility.",
                "application/json",
            ));
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tracks/{}/transparency", track.id),
                "track-transparency",
                "Display-ready canonical transparency state for one track.",
                "application/json",
            ));
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tracks/{}/context", track.id),
                "track-context",
                "Canonical track context combining metadata, brief, and plan.",
                "application/json",
            ));
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tracks/{}/brief", track.id),
                "track-brief",
                "Canonical resume summary for one track.",
                "application/json",
            ));
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tracks/{}/plan", track.id),
                "track-plan",
                "Canonical execution plan for one track.",
                "application/json",
            ));
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tracks/{}/runtime", track.id),
                "track-runtime",
                "Current runtime projection for one track.",
                "application/json",
            ));
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tracks/{}/events", track.id),
                "track-events",
                "Canonical event feed for one track.",
                "application/json",
            ));
        }

        let tasks = self
            .surface
            .list_tasks()
            .map_err(TransportProtocolError::from_emberflow_error)?;
        for task in tasks {
            if let Some(track_id) = task.track_id.as_deref() {
                if !active_track_ids.contains(track_id) {
                    continue;
                }
            }
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tasks/{}/visibility", task.id),
                "task-visibility",
                "Task-level visibility fields plus current track status and next step when available.",
                "application/json",
            ));
            resources.push(TransportResourceDescriptor::new(
                format!("emberflow://tasks/{}/events", task.id),
                "task-events",
                "Canonical event feed for one task.",
                "application/json",
            ));
        }

        resources.push(TransportResourceDescriptor::new(
            RESOURCE_CLIENT_CONTRACT_URI,
            "client-contract",
            "Static client contract for EmberFlow bootstrap, mutation, and transparency.",
            "application/json",
        ));

        Ok(resources)
    }

    fn list_resource_templates(
        &self,
    ) -> std::result::Result<Vec<ResourceViewDescriptor>, TransportProtocolError> {
        Ok(self.surface.list_resources())
    }

    pub fn read_resource(
        &self,
        uri: &str,
    ) -> std::result::Result<ResourceReadResponse, TransportProtocolError> {
        self.surface
            .read_resource(uri)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn emit_diagnostic(&self, level: &str, message: &str) -> TransportDiagnostic {
        let level = match level {
            "debug" => "debug",
            "info" => "info",
            "warn" => "warn",
            "error" => "error",
            _ => "info",
        };
        TransportDiagnostic {
            channel: "stderr",
            level,
            message: message.to_string(),
        }
    }

    pub fn create_track(
        &self,
        track_id: &str,
        title: &str,
        status: &str,
    ) -> std::result::Result<TrackRecord, TransportProtocolError> {
        self.surface
            .create_track(track_id, title, status)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn archive_track(
        &self,
        track_id: &str,
    ) -> std::result::Result<TrackRecord, TransportProtocolError> {
        self.surface
            .archive_track(track_id)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn delete_track(
        &self,
        track_id: &str,
    ) -> std::result::Result<TrackRecord, TransportProtocolError> {
        self.surface
            .delete_track(track_id)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn upsert_track_metadata(
        &self,
        input: TrackMetadataInput,
    ) -> std::result::Result<TrackMetadataRecord, TransportProtocolError> {
        self.surface
            .upsert_track_metadata(input)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn replace_track_brief(
        &self,
        track_id: &str,
        sections: Vec<TrackBriefSectionInput>,
    ) -> std::result::Result<TrackBriefRecord, TransportProtocolError> {
        self.surface
            .replace_track_brief(track_id, sections)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn replace_track_plan(
        &self,
        track_id: &str,
        phases: Vec<TrackPlanPhaseInput>,
    ) -> std::result::Result<TrackPlanRecord, TransportProtocolError> {
        self.surface
            .replace_track_plan(track_id, phases)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn create_task(
        &self,
        input: TaskInput,
    ) -> std::result::Result<TaskRecord, TransportProtocolError> {
        self.surface
            .create_task(input)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn record_event(
        &self,
        event_id: &str,
        track_id: Option<&str>,
        task_id: Option<&str>,
        kind: &str,
        payload: Value,
    ) -> std::result::Result<EventRecord, TransportProtocolError> {
        self.surface
            .record_event(event_id, track_id, task_id, kind, payload)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn claim_task(
        &self,
        task_id: &str,
        holder: &str,
        duration_secs: Option<u64>,
    ) -> std::result::Result<LeaseInfo, TransportProtocolError> {
        self.surface
            .claim_task(task_id, holder, duration_secs)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn release_task(
        &self,
        task_id: &str,
        holder: &str,
    ) -> std::result::Result<(), TransportProtocolError> {
        self.surface
            .release_task(task_id, holder)
            .map_err(TransportProtocolError::from_emberflow_error)
    }

    pub fn call(
        &self,
        method: &str,
        params: Value,
    ) -> std::result::Result<Value, TransportProtocolError> {
        match method {
            "initialize" => Ok(initialize_response_json(&self.initialize()?, &params)),
            "notifications/initialized" => Ok(json!({})),
            "resources/list" => Ok(resources_json(&self.list_resources()?)),
            "resources/templates/list" => {
                Ok(resource_templates_json(&self.list_resource_templates()?))
            }
            "resources/read" => {
                let uri = required_string(&params, "uri")?;
                Ok(resource_read_json(&self.read_resource(&uri)?))
            }
            "tools/list" => Ok(tools_json()),
            "tools/call" => {
                let name = required_string(&params, "name")?;
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                self.call_standard_tool(&name, arguments)
            }
            "list-resources" | "list_resources" => Ok(resources_json(&self.list_resources()?)),
            "read-resource" | "read_resource" => {
                let uri = required_string(&params, "uri")?;
                Ok(resource_json(&self.read_resource(&uri)?))
            }
            EMBERFLOW_TRACK_CREATE_TOOL => {
                let track_id = required_string(&params, "trackId")?;
                let title = required_string(&params, "title")?;
                let status = required_string(&params, "status")?;
                Ok(track_json(&self.create_track(&track_id, &title, &status)?))
            }
            EMBERFLOW_TRACK_METADATA_UPSERT_TOOL => {
                let input = TrackMetadataInput {
                    track_id: required_string(&params, "trackId")?,
                    track_type: required_string(&params, "trackType")?,
                    status: required_string(&params, "status")?,
                    description: required_string(&params, "description")?,
                    branch: required_string(&params, "branch")?,
                    spec_ref: optional_string(&params, "specRef"),
                };
                Ok(track_metadata_json(&self.upsert_track_metadata(input)?))
            }
            EMBERFLOW_TRACK_BRIEF_REPLACE_TOOL => {
                let track_id = required_string(&params, "trackId")?;
                let sections = track_brief_sections_from_params(&params)?;
                Ok(track_brief_json(
                    &self.replace_track_brief(&track_id, sections)?,
                ))
            }
            EMBERFLOW_TRACK_PLAN_REPLACE_TOOL => {
                let track_id = required_string(&params, "trackId")?;
                let phases = track_plan_from_params(&params)?;
                Ok(track_plan_json(
                    &self.replace_track_plan(&track_id, phases)?,
                ))
            }
            EMBERFLOW_TRACK_ARCHIVE_TOOL => {
                let track_id = required_string(&params, "trackId")?;
                Ok(track_json(&self.archive_track(&track_id)?))
            }
            EMBERFLOW_TRACK_DELETE_TOOL => {
                let track_id = required_string(&params, "trackId")?;
                Ok(track_json(&self.delete_track(&track_id)?))
            }
            EMBERFLOW_TASK_CREATE_TOOL => {
                let input = TaskInput {
                    task_id: required_string(&params, "taskId")?,
                    track_id: optional_string(&params, "trackId"),
                    title: required_string(&params, "title")?,
                    status: required_string(&params, "status")?,
                    phase: required_string(&params, "phase")?,
                    executor: optional_string(&params, "executor")
                        .or_else(|| optional_string(&params, "agentType")),
                    agent_instance_id: optional_string(&params, "agentInstanceId"),
                    execution: optional_string(&params, "execution"),
                    intent_summary: optional_string(&params, "intentSummary"),
                };
                Ok(task_json(&self.create_task(input)?))
            }
            EMBERFLOW_EVENT_RECORD_TOOL => {
                let payload = params.get("payload").cloned().unwrap_or_else(|| json!({}));
                Ok(event_json(&self.record_event(
                    &required_string(&params, "eventId")?,
                    optional_string_ref(&params, "trackId"),
                    optional_string_ref(&params, "taskId"),
                    &required_string(&params, "kind")?,
                    payload,
                )?))
            }
            EMBERFLOW_TASK_CLAIM_TOOL => {
                let task_id = required_string(&params, "taskId")?;
                let holder = required_string(&params, "holder")?;
                let duration_secs = params.get("durationSecs").and_then(Value::as_u64);
                Ok(lease_json(&self.claim_task(
                    &task_id,
                    &holder,
                    duration_secs,
                )?))
            }
            EMBERFLOW_TASK_RELEASE_TOOL => {
                let task_id = required_string(&params, "taskId")?;
                let holder = required_string(&params, "holder")?;
                self.release_task(&task_id, &holder)?;
                Ok(json!({ "released": true, "taskId": task_id }))
            }
            other => Err(TransportProtocolError::method_not_found(other)),
        }
    }

    fn call_standard_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> std::result::Result<Value, TransportProtocolError> {
        let result = match name {
            EMBERFLOW_TRACK_CREATE_TOOL => self.call(EMBERFLOW_TRACK_CREATE_TOOL, arguments)?,
            EMBERFLOW_TRACK_METADATA_UPSERT_TOOL => {
                self.call(EMBERFLOW_TRACK_METADATA_UPSERT_TOOL, arguments)?
            }
            EMBERFLOW_TRACK_BRIEF_REPLACE_TOOL => {
                self.call(EMBERFLOW_TRACK_BRIEF_REPLACE_TOOL, arguments)?
            }
            EMBERFLOW_TRACK_PLAN_REPLACE_TOOL => {
                self.call(EMBERFLOW_TRACK_PLAN_REPLACE_TOOL, arguments)?
            }
            EMBERFLOW_TRACK_ARCHIVE_TOOL => self.call(EMBERFLOW_TRACK_ARCHIVE_TOOL, arguments)?,
            EMBERFLOW_TRACK_DELETE_TOOL => self.call(EMBERFLOW_TRACK_DELETE_TOOL, arguments)?,
            EMBERFLOW_TASK_CREATE_TOOL => self.call(EMBERFLOW_TASK_CREATE_TOOL, arguments)?,
            EMBERFLOW_EVENT_RECORD_TOOL => self.call(EMBERFLOW_EVENT_RECORD_TOOL, arguments)?,
            EMBERFLOW_TASK_CLAIM_TOOL => self.call(EMBERFLOW_TASK_CLAIM_TOOL, arguments)?,
            EMBERFLOW_TASK_RELEASE_TOOL => self.call(EMBERFLOW_TASK_RELEASE_TOOL, arguments)?,
            other => return Err(TransportProtocolError::method_not_found(other)),
        };

        Ok(tool_result_json(result))
    }

    pub fn serve_stdio<R, W, E>(
        &self,
        input: R,
        stdout: &mut W,
        stderr: &mut E,
    ) -> std::result::Result<(), TransportProtocolError>
    where
        R: BufRead,
        W: Write,
        E: Write,
    {
        let mut input = input;
        while let Some((request, framing)) = read_stdio_request(&mut input, stderr)? {
            let expects_response = request.id.is_some();

            match self.call(&request.method, request.params) {
                Ok(result) => {
                    if expects_response {
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": request.id.clone().unwrap(),
                            "result": result,
                        });
                        write_json(stdout, &response, framing)
                            .map_err(|error| TransportProtocolError::internal(error.to_string()))?;
                    }
                }
                Err(error) => {
                    if expects_response {
                        let response = json!({
                            "jsonrpc": "2.0",
                            "id": request.id.clone().unwrap(),
                            "error": error.to_json(),
                        });
                        write_json(stdout, &response, framing)
                            .map_err(|error| TransportProtocolError::internal(error.to_string()))?;
                    } else {
                        self.write_diagnostic(stderr, "warn", &error.to_string())
                            .map_err(|error| TransportProtocolError::internal(error.to_string()))?;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn write_protocol_response<W: Write>(
        &self,
        writer: &mut W,
        id: Value,
        result: Value,
    ) -> io::Result<()> {
        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        });
        write_framed_json(writer, &response)
    }

    pub fn write_diagnostic<W: Write>(
        &self,
        writer: &mut W,
        level: &str,
        message: &str,
    ) -> io::Result<()> {
        let diagnostic = self.emit_diagnostic(level, message);
        writeln!(writer, "{}: {}", diagnostic.level, diagnostic.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StdioFramingMode {
    ContentLength,
    JsonLines,
}

fn read_stdio_request<R: BufRead, E: Write>(
    input: &mut R,
    stderr: &mut E,
) -> std::result::Result<Option<(TransportRequest, StdioFramingMode)>, TransportProtocolError> {
    loop {
        let mut line = String::new();
        let bytes = input
            .read_line(&mut line)
            .map_err(|error| TransportProtocolError::internal(error.to_string()))?;
        if bytes == 0 {
            return Ok(None);
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }

        // Content-Length framing (LSP/MCP wire protocol)
        if let Some(content_length_header) = trimmed.strip_prefix("Content-Length:") {
            const MAX_CONTENT_LENGTH: usize = 10 * 1024 * 1024; // 10 MiB
            let length_str = content_length_header.trim();
            let content_length: usize = match length_str.parse() {
                Ok(n) if n <= MAX_CONTENT_LENGTH => n,
                Ok(n) => {
                    let diagnostic =
                        format!("Content-Length {n} exceeds maximum {MAX_CONTENT_LENGTH}");
                    let _ = writeln!(stderr, "warn: {diagnostic}");
                    continue;
                }
                Err(error) => {
                    let diagnostic = format!("invalid Content-Length value: {error}");
                    let _ = writeln!(stderr, "warn: {diagnostic}");
                    continue;
                }
            };
            // Read until blank line separator (may be additional headers or just \r\n / \n)
            loop {
                let mut sep = String::new();
                let sep_bytes = input
                    .read_line(&mut sep)
                    .map_err(|error| TransportProtocolError::internal(error.to_string()))?;
                if sep_bytes == 0 || sep.trim().is_empty() {
                    break;
                }
            }
            // Read exactly content_length bytes
            let mut buf = vec![0u8; content_length];
            input
                .read_exact(&mut buf)
                .map_err(|error| TransportProtocolError::internal(error.to_string()))?;
            let request: Value = match serde_json::from_slice(&buf) {
                Ok(value) => value,
                Err(error) => {
                    let diagnostic =
                        format!("failed to decode Content-Length framed request: {error}");
                    let _ = writeln!(stderr, "warn: {diagnostic}");
                    continue;
                }
            };
            return parse_stdio_request(request)
                .map(|opt| opt.map(|req| (req, StdioFramingMode::ContentLength)));
        }

        // JSON lines fallback
        let request: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(error) => {
                let diagnostic = format!("failed to decode request: {error}");
                let _ = writeln!(stderr, "warn: {diagnostic}");
                continue;
            }
        };
        return parse_stdio_request(request)
            .map(|opt| opt.map(|req| (req, StdioFramingMode::JsonLines)));
    }
}

fn parse_stdio_request(
    request: Value,
) -> std::result::Result<Option<TransportRequest>, TransportProtocolError> {
    let method = match request.get("method").and_then(Value::as_str) {
        Some(method) => method.to_string(),
        None => {
            return Err(TransportProtocolError::invalid_params(
                "method",
                "method must be present",
            ))
        }
    };

    Ok(Some(TransportRequest {
        id: request.get("id").cloned(),
        method,
        params: request.get("params").cloned().unwrap_or_else(|| json!({})),
    }))
}

fn write_framed_json<W: Write>(writer: &mut W, value: &Value) -> io::Result<()> {
    let payload = serde_json::to_vec(value).map_err(|error| io::Error::other(error.to_string()))?;
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
    writer.write_all(&payload)?;
    writer.flush()
}

fn write_line_delimited_json<W: Write>(writer: &mut W, value: &Value) -> io::Result<()> {
    let payload =
        serde_json::to_string(value).map_err(|error| io::Error::other(error.to_string()))?;
    writeln!(writer, "{payload}")?;
    writer.flush()
}

fn write_json<W: Write>(writer: &mut W, value: &Value, mode: StdioFramingMode) -> io::Result<()> {
    match mode {
        StdioFramingMode::ContentLength => write_framed_json(writer, value),
        StdioFramingMode::JsonLines => write_line_delimited_json(writer, value),
    }
}

fn required_string(
    params: &Value,
    field: &'static str,
) -> std::result::Result<String, TransportProtocolError> {
    let value = params
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            TransportProtocolError::invalid_params(field, format!("{field} must be present"))
        })?;
    Ok(value.to_string())
}

fn optional_string(params: &Value, field: &'static str) -> Option<String> {
    params
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn optional_string_ref<'a>(params: &'a Value, field: &'static str) -> Option<&'a str> {
    params
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn initialize_response_json(_response: &InitializeResponse, params: &Value) -> Value {
    let protocol_version =
        optional_string_ref(params, "protocolVersion").unwrap_or(MCP_PROTOCOL_VERSION);
    json!({
        "protocolVersion": protocol_version,
        "serverInfo": {
            "name": MCP_SERVER_NAME,
            "version": env!("CARGO_PKG_VERSION"),
        },
        "capabilities": {
            "resources": {
                "subscribe": false,
                "listChanged": false,
            },
            "tools": {
                "listChanged": false,
            }
        }
    })
}

fn resource_view_json(view: &ResourceViewDescriptor) -> Value {
    json!({
        "uriTemplate": view.uri_template,
        "name": view.name,
        "description": view.description,
        "mimeType": view.mime_type,
    })
}

fn transport_resource_json(resource: &TransportResourceDescriptor) -> Value {
    json!({
        "uri": resource.uri,
        "name": resource.name,
        "description": resource.description,
        "mimeType": resource.mime_type,
    })
}

fn resources_json(resources: &[TransportResourceDescriptor]) -> Value {
    json!({
        "resources": resources.iter().map(transport_resource_json).collect::<Vec<_>>(),
    })
}

fn resource_json(resource: &ResourceReadResponse) -> Value {
    json!({
        "resource": {
            "uri": resource.uri,
            "name": resource.name,
            "description": resource.description,
            "mimeType": resource.mime_type,
            "content": resource.content,
        }
    })
}

fn resource_templates_json(resources: &[ResourceViewDescriptor]) -> Value {
    json!({
        "resourceTemplates": resources.iter().map(resource_view_json).collect::<Vec<_>>(),
    })
}

fn resource_read_json(resource: &ResourceReadResponse) -> Value {
    json!({
        "contents": [{
            "uri": resource.uri,
            "mimeType": resource.mime_type,
            "text": resource.content.to_string(),
        }],
        "resource": {
            "uri": resource.uri,
            "name": resource.name,
            "description": resource.description,
            "mimeType": resource.mime_type,
            "content": resource.content,
        }
    })
}

fn tools_json() -> Value {
    json!({
        "tools": [
            tool_descriptor(
                EMBERFLOW_TRACK_CREATE_TOOL,
                "Create a canonical track record in EmberFlow.",
                json!({
                    "type": "object",
                    "properties": {
                        "trackId": {"type": "string"},
                        "title": {"type": "string"},
                        "status": {"type": "string"}
                    },
                    "required": ["trackId", "title", "status"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_TRACK_METADATA_UPSERT_TOOL,
                "Create or update canonical track metadata in EmberFlow.",
                json!({
                    "type": "object",
                    "properties": {
                        "trackId": {"type": "string"},
                        "trackType": {"type": "string"},
                        "status": {"type": "string"},
                        "description": {"type": "string"},
                        "branch": {"type": "string"},
                        "specRef": {"type": "string"}
                    },
                    "required": ["trackId", "trackType", "status", "description", "branch"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_TRACK_BRIEF_REPLACE_TOOL,
                "Replace the canonical brief sections for a track.",
                json!({
                    "type": "object",
                    "properties": {
                        "trackId": {"type": "string"},
                        "sections": {"type": "array"}
                    },
                    "required": ["trackId", "sections"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_TRACK_PLAN_REPLACE_TOOL,
                "Replace the canonical execution plan for a track.",
                json!({
                    "type": "object",
                    "properties": {
                        "trackId": {"type": "string"},
                        "phases": {"type": "array"}
                    },
                    "required": ["trackId", "phases"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_TRACK_ARCHIVE_TOOL,
                "Archive a canonical track in EmberFlow without deleting its direct resources.",
                json!({
                    "type": "object",
                    "properties": {
                        "trackId": {"type": "string"}
                    },
                    "required": ["trackId"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_TRACK_DELETE_TOOL,
                "Delete a canonical track in EmberFlow.",
                json!({
                    "type": "object",
                    "properties": {
                        "trackId": {"type": "string"}
                    },
                    "required": ["trackId"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_TASK_CREATE_TOOL,
                "Create a canonical task with visibility metadata in EmberFlow.",
                json!({
                    "type": "object",
                    "properties": {
                        "taskId": {"type": "string"},
                        "trackId": {"type": "string"},
                        "title": {"type": "string"},
                        "status": {"type": "string"},
                        "phase": {"type": "string"},
                        "executor": {"type": "string"},
                        "agentInstanceId": {"type": "string"},
                        "execution": {"type": "string"},
                        "intentSummary": {"type": "string"}
                    },
                    "required": ["taskId", "title", "status", "phase"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_EVENT_RECORD_TOOL,
                "Record a canonical event in EmberFlow.",
                json!({
                    "type": "object",
                    "properties": {
                        "eventId": {"type": "string"},
                        "trackId": {"type": "string"},
                        "taskId": {"type": "string"},
                        "kind": {"type": "string"},
                        "payload": {}
                    },
                    "required": ["eventId", "kind"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_TASK_CLAIM_TOOL,
                "Claim an exclusive execution lease on a task in EmberFlow.",
                json!({
                    "type": "object",
                    "properties": {
                        "taskId": {"type": "string"},
                        "holder": {"type": "string"},
                        "durationSecs": {"type": "integer"}
                    },
                    "required": ["taskId", "holder"],
                    "additionalProperties": false
                }),
            ),
            tool_descriptor(
                EMBERFLOW_TASK_RELEASE_TOOL,
                "Release an execution lease on a task in EmberFlow.",
                json!({
                    "type": "object",
                    "properties": {
                        "taskId": {"type": "string"},
                        "holder": {"type": "string"}
                    },
                    "required": ["taskId", "holder"],
                    "additionalProperties": false
                }),
            )
        ]
    })
}

fn tool_descriptor(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

fn tool_result_json(result: Value) -> Value {
    json!({
        "structuredContent": result,
        "content": [{
            "type": "text",
            "text": result.to_string(),
        }],
        "isError": false,
    })
}

fn track_metadata_json(metadata: &TrackMetadataRecord) -> Value {
    json!({
        "trackId": metadata.track_id,
        "trackType": metadata.track_type,
        "status": metadata.status,
        "description": metadata.description,
        "branch": metadata.branch,
        "specRef": metadata.spec_ref,
        "createdAt": metadata.created_at,
        "updatedAt": metadata.updated_at,
    })
}

fn track_brief_json(brief: &TrackBriefRecord) -> Value {
    json!({
        "trackId": brief.track_id,
        "sections": brief
            .sections
            .iter()
            .map(track_brief_section_json)
            .collect::<Vec<_>>(),
    })
}

fn track_brief_section_json(section: &TrackBriefSectionRecord) -> Value {
    json!({
        "trackId": section.track_id,
        "sectionKey": section.section_key,
        "sectionText": section.section_text,
        "position": section.position,
        "createdAt": section.created_at,
        "updatedAt": section.updated_at,
    })
}

fn track_plan_json(plan: &TrackPlanRecord) -> Value {
    json!({
        "trackId": plan.track_id,
        "phases": plan.phases.iter().map(track_plan_phase_json).collect::<Vec<_>>(),
    })
}

fn track_plan_phase_json(phase: &TrackPlanPhaseRecord) -> Value {
    json!({
        "phaseId": phase.phase_id,
        "trackId": phase.track_id,
        "title": phase.title,
        "position": phase.position,
        "items": phase.items.iter().map(track_plan_item_json).collect::<Vec<_>>(),
        "createdAt": phase.created_at,
        "updatedAt": phase.updated_at,
    })
}

fn track_plan_item_json(item: &TrackPlanItemRecord) -> Value {
    json!({
        "itemId": item.item_id,
        "trackId": item.track_id,
        "phaseId": item.phase_id,
        "title": item.title,
        "position": item.position,
        "createdAt": item.created_at,
        "updatedAt": item.updated_at,
    })
}

fn track_json(track: &TrackRecord) -> Value {
    json!({
        "id": track.id,
        "title": track.title,
        "status": track.status,
        "createdAt": track.created_at,
        "updatedAt": track.updated_at,
    })
}

fn task_json(task: &TaskRecord) -> Value {
    json!({
        "id": task.id,
        "trackId": task.track_id,
        "planItemId": task.plan_item_id,
        "title": task.title,
        "status": task.status,
        "phase": task.phase,
        "executor": task.executor,
        "agentType": task.executor,
        "agentInstanceId": task.agent_instance_id,
        "execution": task.execution,
        "intentSummary": task.intent_summary,
        "createdAt": task.created_at,
        "updatedAt": task.updated_at,
    })
}

fn event_json(event: &EventRecord) -> Value {
    json!({
        "id": event.id,
        "trackId": event.track_id,
        "taskId": event.task_id,
        "kind": event.kind,
        "payload": event.payload,
        "createdAt": event.created_at,
    })
}

fn lease_json(lease: &LeaseInfo) -> Value {
    json!({
        "holder": lease.holder,
        "acquiredAt": lease.acquired_at,
        "expiresAt": lease.expires_at,
    })
}

fn track_brief_sections_from_params(
    params: &Value,
) -> std::result::Result<Vec<TrackBriefSectionInput>, TransportProtocolError> {
    let sections = params
        .get("sections")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            TransportProtocolError::invalid_params("sections", "sections must be an array")
        })?;

    let mut output = Vec::new();
    for section in sections {
        let section_key = section
            .get("sectionKey")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                TransportProtocolError::invalid_params("sectionKey", "sectionKey must be present")
            })?;
        let section_text = section
            .get("sectionText")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TransportProtocolError::invalid_params("sectionText", "sectionText must be present")
            })?;
        let position = section
            .get("position")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                TransportProtocolError::invalid_params("position", "position must be present")
            })?;
        output.push(TrackBriefSectionInput {
            section_key: section_key.to_string(),
            section_text: section_text.to_string(),
            position,
        });
    }
    Ok(output)
}

fn track_plan_from_params(
    params: &Value,
) -> std::result::Result<Vec<TrackPlanPhaseInput>, TransportProtocolError> {
    let phases = params
        .get("phases")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            TransportProtocolError::invalid_params("phases", "phases must be an array")
        })?;

    let mut output = Vec::new();
    for phase in phases {
        let phase_id = phase
            .get("phaseId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                TransportProtocolError::invalid_params("phaseId", "phaseId must be present")
            })?;
        let title = phase.get("title").and_then(Value::as_str).ok_or_else(|| {
            TransportProtocolError::invalid_params("title", "title must be present")
        })?;
        let position = phase
            .get("position")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                TransportProtocolError::invalid_params("position", "position must be present")
            })?;
        let items = phase
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                TransportProtocolError::invalid_params("items", "items must be an array")
            })?;
        let mut phase_items = Vec::new();
        for item in items {
            let item_id = item
                .get("itemId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    TransportProtocolError::invalid_params("itemId", "itemId must be present")
                })?;
            let title = item.get("title").and_then(Value::as_str).ok_or_else(|| {
                TransportProtocolError::invalid_params("title", "title must be present")
            })?;
            let position = item
                .get("position")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    TransportProtocolError::invalid_params("position", "position must be present")
                })?;
            phase_items.push(TrackPlanItemInput {
                item_id: item_id.to_string(),
                title: title.to_string(),
                position: Some(position),
            });
        }
        output.push(TrackPlanPhaseInput {
            phase_id: phase_id.to_string(),
            title: title.to_string(),
            position,
            items: phase_items,
        });
    }
    Ok(output)
}
