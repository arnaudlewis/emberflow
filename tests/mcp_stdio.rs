use emberflow::mcp::server::{
    start_stdio_server, StdioTransportConfig, STDIO_TRANSPORT_AUTH, STDIO_TRANSPORT_ENDPOINT,
    STDIO_TRANSPORT_HOSTING, STDIO_TRANSPORT_MODE,
};
use emberflow::mcp::surface::{
    EmberFlowSurface, TaskInput, TrackBriefSectionInput, TrackMetadataInput, TrackPlanItemInput,
    TrackPlanPhaseInput,
};
use rusqlite::{params, Connection};
use serde_json::json;
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use tempfile::tempdir;

fn canonical_track_metadata_input(track_id: &str) -> TrackMetadataInput {
    TrackMetadataInput {
        track_id: track_id.to_string(),
        track_type: "feature".to_string(),
        status: "in-progress".to_string(),
        description: "Expose canonical track context through EmberFlow MCP".to_string(),
        branch: "feature/mcp-surface".to_string(),
        spec_ref: Some("emberflow/specs/emberflow-mcp-surface.spec".to_string()),
    }
}

fn brief_sections() -> Vec<TrackBriefSectionInput> {
    vec![
        TrackBriefSectionInput {
            section_key: "objective".to_string(),
            section_text: "Finish the canonical SQLite track model".to_string(),
            position: 0,
        },
        TrackBriefSectionInput {
            section_key: "context".to_string(),
            section_text: "The track resume context must survive workspace restarts".to_string(),
            position: 1,
        },
    ]
}

fn canonical_plan() -> Vec<TrackPlanPhaseInput> {
    vec![TrackPlanPhaseInput {
        phase_id: "phase-1".to_string(),
        title: "Canonical schema design".to_string(),
        position: 0,
        items: vec![TrackPlanItemInput {
            item_id: "phase-1/item-1".to_string(),
            title: "Define canonical metadata tables".to_string(),
            position: Some(0),
        }],
    }]
}

fn seed_workspace(path: &std::path::Path) {
    let surface = EmberFlowSurface::from_workspace_root(path).unwrap();
    surface
        .create_track("track-001", "Build EmberFlow V1", "in-progress")
        .unwrap();
    surface
        .upsert_track_metadata(canonical_track_metadata_input("track-001"))
        .unwrap();
    surface
        .replace_track_brief("track-001", brief_sections())
        .unwrap();
    surface
        .replace_track_plan("track-001", canonical_plan())
        .unwrap();
    surface
        .create_task(TaskInput {
            task_id: "task-001".to_string(),
            track_id: Some("track-001".to_string()),
            title: "Persist the first event".to_string(),
            status: "running".to_string(),
            phase: "implementing".to_string(),
            executor: Some("assistant".to_string()),
            agent_instance_id: None,
            execution: Some("interactive session".to_string()),
            intent_summary: Some("Persist the first event".to_string()),
        })
        .unwrap();
}

fn corrupt_legacy_metadata(path: &std::path::Path, track_id: &str) {
    let db_path = path.join(".emberflow/emberflow.db");
    let conn = Connection::open(db_path).unwrap();
    conn.execute(
        "UPDATE tracks SET track_type = NULL, description = NULL, branch = NULL, spec_ref = NULL WHERE id = ?",
        params![track_id],
    )
    .unwrap();
}

fn start_with_cwd(path: &std::path::Path) -> emberflow::mcp::server::StdioTransportSession {
    start_stdio_server(StdioTransportConfig {
        cwd: Some(path.to_path_buf()),
        workspace_root: None,
        state_path: None,
    })
    .unwrap()
}

struct McpClient {
    child: Child,
    reader: BufReader<std::process::ChildStdout>,
    stdin: std::process::ChildStdin,
    next_id: u64,
    init_response: serde_json::Value,
}

impl McpClient {
    fn new(cwd: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_emberflow-mcp"))
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn emberflow-mcp");

        let stdout = child.stdout.take().expect("capture stdout");
        let reader = BufReader::new(stdout);
        let stdin = child.stdin.take().expect("capture stdin");

        let mut client = Self {
            child,
            reader,
            stdin,
            next_id: 1,
            init_response: serde_json::Value::Null,
        };
        client.initialize();
        client
    }

    fn send_request(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let payload = serde_json::to_vec(&request).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n", payload.len()).unwrap();
        self.stdin.write_all(&payload).unwrap();
        self.stdin.flush().unwrap();
        self.read_response(id)
    }

    fn send_notification(&mut self, method: &str, params: serde_json::Value) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let payload = serde_json::to_vec(&notification).unwrap();
        write!(self.stdin, "Content-Length: {}\r\n\r\n", payload.len()).unwrap();
        self.stdin.write_all(&payload).unwrap();
        self.stdin.flush().unwrap();
    }

    fn read_response(&mut self, expected_id: u64) -> serde_json::Value {
        loop {
            // Read Content-Length header line
            let mut header = String::new();
            self.reader
                .read_line(&mut header)
                .expect("read Content-Length header");
            let trimmed = header.trim();
            if trimmed.is_empty() {
                continue;
            }
            let content_length: usize = if let Some(rest) = trimmed.strip_prefix("Content-Length:")
            {
                rest.trim().parse().expect("parse Content-Length value")
            } else {
                panic!("unexpected header line: {trimmed}");
            };
            // Read blank separator line
            let mut sep = String::new();
            self.reader.read_line(&mut sep).expect("read separator");
            // Read exactly content_length bytes
            let mut buf = vec![0u8; content_length];
            self.reader
                .read_exact(&mut buf)
                .expect("read framed payload");
            let response: serde_json::Value =
                serde_json::from_slice(&buf).expect("parse framed response");
            if response.get("id").is_some() {
                assert_eq!(response["id"].as_u64().unwrap(), expected_id);
                return response;
            }
        }
    }

    fn initialize(&mut self) {
        let response = self.send_request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "0.1.0"
                }
            }),
        );
        self.init_response = response["result"].clone();
        self.send_notification("notifications/initialized", json!({}));
    }

    fn list_resources(&mut self) -> serde_json::Value {
        self.send_request("resources/list", json!({}))["result"].clone()
    }

    fn read_resource(&mut self, uri: &str) -> serde_json::Value {
        self.send_request("resources/read", json!({"uri": uri}))["result"].clone()
    }

    fn list_tools(&mut self) -> serde_json::Value {
        self.send_request("tools/list", json!({}))["result"].clone()
    }

    fn call_tool(&mut self, name: &str, arguments: serde_json::Value) -> serde_json::Value {
        self.send_request(
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments,
            }),
        )["result"]
            .clone()
    }

    fn server_info(&self) -> serde_json::Value {
        self.init_response.clone()
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// @minter:integration stdio-transport-lists-tools-via-standard-mcp-method
#[test]
fn stdio_tools_list_includes_archive_and_delete() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let mut client = McpClient::new(tmp.path());

    let tools = client.list_tools();
    let names: BTreeSet<_> = tools["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|value| value.get("name").and_then(|value| value.as_str()))
        .collect();

    assert!(names.contains("emberflow-track-archive"));
    assert!(names.contains("emberflow-track-delete"));
}

// @minter:integration stdio-transport-calls-tools-via-standard-mcp-method
#[test]
fn stdio_active_resources_exclude_archived_tracks_but_direct_reads_work() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let mut client = McpClient::new(tmp.path());

    client.call_tool(
        "emberflow-track-metadata-upsert",
        json!({
            "trackId": "track-001",
            "trackType": "feature",
            "status": "review",
            "description": "Build EmberFlow V1",
            "branch": "feature/mcp-surface",
            "specRef": "emberflow/specs/emberflow-mcp-surface.spec"
        }),
    );

    client.call_tool(
        "emberflow-track-archive",
        json!({
            "trackId": "track-001"
        }),
    );

    let resources = client.list_resources();
    let uris: BTreeSet<_> = resources["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|value| value.get("uri").and_then(|value| value.as_str()))
        .collect();

    assert!(!uris.contains("emberflow://tracks/track-001/record"));

    let archived_record = client.read_resource("emberflow://tracks/track-001/record");
    assert_eq!(
        archived_record["resource"]["content"]["status"].as_str(),
        Some("archived")
    );
}

// @minter:integration stdio-transport-starts-as-local-process
#[test]
fn stdio_transport_starts_as_local_process() {
    let tmp = tempdir().unwrap();
    let session = start_with_cwd(tmp.path());

    let info = session.info();
    assert_eq!(info.mode, STDIO_TRANSPORT_MODE);
    assert_eq!(info.hosting, STDIO_TRANSPORT_HOSTING);
    assert_eq!(info.auth, STDIO_TRANSPORT_AUTH);
    assert_eq!(info.endpoint, STDIO_TRANSPORT_ENDPOINT);
    assert_eq!(info.workspace_root, tmp.path().display().to_string());
    assert!(info
        .workspace_db
        .default_path
        .contains(".emberflow/emberflow.db"));
}

// @minter:integration stdio-transport-initializes-over-stdio stdio-transport-advertises-stable-tool-surface stdio-transport-omits-resource-catalog-from-initialize stdio-transport-omits-emberflow-bootstrap-extension-fields-from-initialize stdio-transport-omits-runtime-bootstrap-fields-from-initialize
#[test]
fn stdio_transport_initializes_over_stdio() {
    let tmp = tempdir().unwrap();
    let session = start_with_cwd(tmp.path());

    let init = session.call("initialize", json!({})).unwrap();
    assert_eq!(
        init.get("protocolVersion").and_then(|value| value.as_str()),
        Some("2024-11-05")
    );
    assert_eq!(
        init.get("serverInfo")
            .and_then(|value| value.get("name"))
            .and_then(|value| value.as_str()),
        Some("emberflow")
    );
    assert!(init.get("capabilities").is_some());
    assert!(init
        .get("capabilities")
        .and_then(|value| value.get("resources"))
        .is_some());
    assert!(init
        .get("capabilities")
        .and_then(|value| value.get("tools"))
        .is_some());
    assert!(init.get("emberflowCapabilities").is_none());
    assert!(init.get("knowledgeViews").is_none());
    assert!(init.get("projectedFiles").is_none());
    assert!(init.get("sourceOfTruth").is_none());
    assert!(init.get("systemRole").is_none());
    assert!(init.get("trackBootstrap").is_none());
    assert!(init.get("workspaceDb").is_none());
    assert!(init.get("resources").is_none());
    assert!(init.get("resourceViews").is_none());
}

// @minter:integration stdio-transport-advertises-stable-tool-surface
#[test]
fn stdio_transport_advertises_stable_tool_surface() {
    let tmp = tempdir().unwrap();
    let session = start_with_cwd(tmp.path());

    let init = session.initialize().unwrap();
    let capabilities: BTreeSet<_> = init.capabilities.into_iter().collect();
    let expected: BTreeSet<_> = [
        "emberflow-track-create",
        "emberflow-track-metadata-upsert",
        "emberflow-track-brief-replace",
        "emberflow-track-plan-replace",
        "emberflow-track-archive",
        "emberflow-track-delete",
        "emberflow-task-create",
        "emberflow-event-record",
        "emberflow-task-claim",
        "emberflow-task-release",
    ]
    .into_iter()
    .collect();

    assert_eq!(capabilities, expected);
}

// @minter:integration stdio-transport-lists-readable-resource-catalog
#[test]
fn stdio_transport_lists_readable_resource_catalog() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    let catalog = session.call("resources/list", json!({})).unwrap();
    let resources = catalog
        .get("resources")
        .and_then(|value| value.as_array())
        .unwrap();

    assert_eq!(resources.len(), 12);
    assert!(resources.iter().any(|value| {
        value.get("uri").and_then(|uri| uri.as_str())
            == Some("emberflow://protocol/client-contract")
    }));
    assert!(resources
        .iter()
        .all(|value| value.get("uriTemplate").is_none()));
}

// @minter:integration stdio-transport-omits-resource-catalog-from-initialize stdio-transport-lists-resource-templates-via-standard-mcp-method stdio-transport-reads-resource-via-standard-mcp-method
#[test]
fn stdio_transport_supports_standard_mcp_resource_methods() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    let init = session.call("initialize", json!({})).unwrap();
    assert!(init.get("resources").is_none());
    assert!(init.get("resourceViews").is_none());

    let standard_resource_list = session.call("resources/list", json!({})).unwrap();
    let standard_resources = standard_resource_list
        .get("resources")
        .and_then(|value| value.as_array())
        .expect("standard resource listing should be available");
    assert_eq!(standard_resources.len(), 12);
    assert!(standard_resources.iter().any(|value| {
        value.get("uri").and_then(|uri| uri.as_str()) == Some("emberflow://workspace/overview")
    }));
    assert!(standard_resources
        .iter()
        .all(|value| value.get("uriTemplate").is_none()));

    let standard_catalog = session.call("resources/templates/list", json!({})).unwrap();
    let templates = standard_catalog
        .get("resourceTemplates")
        .and_then(|value| value.as_array())
        .expect("standard resource templates should be listed");
    assert_eq!(templates.len(), 12);
    assert!(templates.iter().any(|value| {
        value.get("uriTemplate").and_then(|uri| uri.as_str())
            == Some("emberflow://protocol/client-contract")
    }));
    assert!(templates.iter().all(|value| value.get("uri").is_none()));

    let standard_read = session
        .call(
            "resources/read",
            json!({"uri": "emberflow://workspace/overview"}),
        )
        .unwrap();
    let contents = standard_read
        .get("contents")
        .and_then(|value| value.as_array())
        .expect("standard resource read should return contents");
    assert_eq!(contents.len(), 1);
    assert_eq!(
        contents[0].get("uri").and_then(|value| value.as_str()),
        Some("emberflow://workspace/overview")
    );
    assert_eq!(
        contents[0].get("mimeType").and_then(|value| value.as_str()),
        Some("application/json")
    );
    assert!(contents[0]
        .get("text")
        .and_then(|value| value.as_str())
        .map(|value| value.contains("emberflow-canonical-state"))
        .unwrap_or(false));

    let legacy_catalog = session.call("list-resources", json!({})).unwrap();
    let legacy_resources = legacy_catalog
        .get("resources")
        .and_then(|value| value.as_array())
        .expect("legacy resource catalog should still work");
    assert_eq!(legacy_resources.len(), 12);

    let legacy_read = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://workspace/overview"}),
        )
        .unwrap();
    assert_eq!(
        legacy_read
            .get("resource")
            .and_then(|value| value.get("uri"))
            .and_then(|value| value.as_str()),
        Some("emberflow://workspace/overview")
    );
}

// @minter:integration stdio-transport-uses-content-length-framed-json-rpc stdio-transport-initializes-over-stdio stdio-transport-advertises-stable-tool-surface stdio-transport-omits-resource-catalog-from-initialize stdio-transport-omits-emberflow-bootstrap-extension-fields-from-initialize stdio-transport-omits-runtime-bootstrap-fields-from-initialize stdio-transport-accepts-standard-initialized-notification stdio-transport-lists-resource-templates-via-standard-mcp-method stdio-transport-reads-resource-via-standard-mcp-method stdio-transport-lists-tools-via-standard-mcp-method stdio-transport-calls-tools-via-standard-mcp-method
#[test]
fn stdio_transport_supports_standard_mcp_client_lifecycle() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let mut client = McpClient::new(tmp.path());

    let info = client.server_info();
    assert_eq!(
        info.get("protocolVersion").and_then(|value| value.as_str()),
        Some("2024-11-05")
    );
    assert_eq!(
        info.get("serverInfo")
            .and_then(|value| value.get("name"))
            .and_then(|value| value.as_str()),
        Some("emberflow")
    );
    assert!(info
        .get("capabilities")
        .and_then(|value| value.get("resources"))
        .is_some());
    assert!(info
        .get("capabilities")
        .and_then(|value| value.get("tools"))
        .is_some());
    assert!(info.get("emberflowCapabilities").is_none());
    assert!(info.get("knowledgeViews").is_none());
    assert!(info.get("projectedFiles").is_none());
    assert!(info.get("sourceOfTruth").is_none());
    assert!(info.get("systemRole").is_none());
    assert!(info.get("trackBootstrap").is_none());
    assert!(info.get("workspaceDb").is_none());
    assert!(info.get("resources").is_none());
    assert!(info.get("resourceViews").is_none());

    let resources = client
        .list_resources()
        .get("resources")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap();
    assert_eq!(resources.len(), 12);
    assert!(resources
        .iter()
        .all(|value| value.get("uriTemplate").is_none()));
    assert!(resources.iter().any(|value| {
        value.get("uri").and_then(|value| value.as_str()) == Some("emberflow://workspace/overview")
    }));

    let templates = client
        .send_request("resources/templates/list", json!({}))
        .get("result")
        .and_then(|value| value.get("resourceTemplates"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap();
    assert_eq!(templates.len(), 12);
    assert!(templates.iter().all(|value| value.get("uri").is_none()));
    assert!(templates.iter().any(|value| {
        value.get("uriTemplate").and_then(|value| value.as_str())
            == Some("emberflow://protocol/client-contract")
    }));

    let read = client.read_resource("emberflow://workspace/overview");
    assert!(read
        .get("contents")
        .and_then(|value| value.as_array())
        .is_some());

    let tools = client
        .list_tools()
        .get("tools")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap();
    let tool_names: Vec<_> = tools
        .iter()
        .map(|tool| tool.get("name").and_then(|value| value.as_str()).unwrap())
        .collect();
    assert_eq!(
        tool_names,
        vec![
            "emberflow-track-create",
            "emberflow-track-metadata-upsert",
            "emberflow-track-brief-replace",
            "emberflow-track-plan-replace",
            "emberflow-track-archive",
            "emberflow-track-delete",
            "emberflow-task-create",
            "emberflow-event-record",
            "emberflow-task-claim",
            "emberflow-task-release",
        ]
    );
    assert!(tools.iter().all(|tool| tool.get("inputSchema").is_some()));

    let result = client.call_tool(
        "emberflow-track-create",
        json!({
            "trackId": "track-002",
            "title": "Verify MCP lifecycle",
            "status": "planning"
        }),
    );
    assert_eq!(
        result.get("isError").and_then(|value| value.as_bool()),
        Some(false)
    );
    assert!(result
        .get("content")
        .and_then(|value| value.as_array())
        .is_some());
}

// @minter:integration stdio-transport-rejects-unprefixed-tool-names
#[test]
fn stdio_transport_rejects_unprefixed_tool_names() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let mut client = McpClient::new(tmp.path());

    for alias in [
        "create_track",
        "create-track",
        "upsert_track_metadata",
        "upsert-track-metadata",
        "replace_track_brief",
        "replace-track-brief",
        "replace_track_plan",
        "replace-track-plan",
        "create_task",
        "create-task",
        "record_event",
        "record-event",
    ] {
        let response = client.send_request(
            "tools/call",
            json!({
                "name": alias,
                "arguments": {
                    "trackId": "track-legacy",
                    "title": "Verify MCP legacy alias",
                    "status": "planning"
                }
            }),
        );
        let error = response
            .get("error")
            .expect("legacy aliases should be rejected");
        assert_eq!(
            error.get("code").and_then(|value| value.as_str()),
            Some("method_not_found")
        );
        assert!(error
            .get("message")
            .and_then(|value| value.as_str())
            .expect("error message should be present")
            .contains("unknown method"));
    }
}

// @minter:integration stdio-transport-resolves-workspace-root-from-current-directory
#[test]
fn stdio_transport_resolves_workspace_root_from_current_directory() {
    let tmp = tempdir().unwrap();
    let session = start_with_cwd(tmp.path());

    assert_eq!(
        session.info().workspace_root,
        tmp.path().display().to_string()
    );
    assert!(session
        .info()
        .workspace_db
        .default_path
        .ends_with(".emberflow/emberflow.db"));
}

// @minter:integration stdio-transport-honors-explicit-workspace-root
#[test]
fn stdio_transport_honors_explicit_workspace_root() {
    let tmp = tempdir().unwrap();
    let session = start_stdio_server(StdioTransportConfig {
        cwd: None,
        workspace_root: Some(tmp.path().to_path_buf()),
        state_path: None,
    })
    .unwrap();

    assert_eq!(
        session.info().workspace_root,
        tmp.path().display().to_string()
    );
    assert_eq!(
        session.info().workspace_db.state_root,
        tmp.path().join(".emberflow").display().to_string()
    );
}

// @minter:integration stdio-transport-honors-explicit-state-path-compatibility
#[test]
fn stdio_transport_honors_explicit_state_path_compatibility() {
    let tmp = tempdir().unwrap();
    let state_path = tmp.path().join(".emberflow/emberflow.db");
    let session = start_stdio_server(StdioTransportConfig {
        cwd: None,
        workspace_root: None,
        state_path: Some(state_path.clone()),
    })
    .unwrap();

    assert_eq!(
        session.info().workspace_db.default_path,
        state_path.display().to_string()
    );
}

// @minter:integration stdio-transport-dispatches-canonical-track-reads
#[test]
fn stdio_transport_dispatches_canonical_track_reads() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    let context = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/context"}),
        )
        .unwrap();
    assert_eq!(
        context
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("trackId"))
            .and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert!(context
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("metadata"))
        .is_some());
    assert!(context
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("brief"))
        .is_some());
    assert!(context
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("plan"))
        .is_some());
}

// @minter:integration stdio-transport-reads-resource-views
#[test]
fn stdio_transport_reads_resource_views() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    let overview = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://workspace/overview"}),
        )
        .unwrap();
    let overview_resource = overview.get("resource").unwrap();
    assert_eq!(
        overview_resource
            .get("uri")
            .and_then(|value| value.as_str()),
        Some("emberflow://workspace/overview")
    );
    assert_eq!(
        overview_resource
            .get("name")
            .and_then(|value| value.as_str()),
        Some("workspace-overview")
    );

    let resume = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/resume"}),
        )
        .unwrap();
    assert_eq!(
        resume
            .get("resource")
            .and_then(|value| value.get("uri"))
            .and_then(|value| value.as_str()),
        Some("emberflow://tracks/track-001/resume")
    );
    assert_eq!(
        resume
            .get("resource")
            .and_then(|value| value.get("name"))
            .and_then(|value| value.as_str()),
        Some("track-resume")
    );

    let transparency = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/transparency"}),
        )
        .unwrap();
    assert_eq!(
        transparency
            .get("resource")
            .and_then(|value| value.get("uri"))
            .and_then(|value| value.as_str()),
        Some("emberflow://tracks/track-001/transparency")
    );
    assert_eq!(
        transparency
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("phase"))
            .and_then(|value| value.as_str()),
        Some("implementing")
    );

    let plan = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/plan"}),
        )
        .unwrap();
    assert_eq!(
        plan.get("resource")
            .and_then(|value| value.get("uri"))
            .and_then(|value| value.as_str()),
        Some("emberflow://tracks/track-001/plan")
    );

    let brief = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/brief"}),
        )
        .unwrap();
    assert_eq!(
        brief
            .get("resource")
            .and_then(|value| value.get("uri"))
            .and_then(|value| value.as_str()),
        Some("emberflow://tracks/track-001/brief")
    );

    let runtime = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/runtime"}),
        )
        .unwrap();
    assert_eq!(
        runtime
            .get("resource")
            .and_then(|value| value.get("uri"))
            .and_then(|value| value.as_str()),
        Some("emberflow://tracks/track-001/runtime")
    );

    let visibility = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tasks/task-001/visibility"}),
        )
        .unwrap();
    assert_eq!(
        visibility
            .get("resource")
            .and_then(|value| value.get("uri"))
            .and_then(|value| value.as_str()),
        Some("emberflow://tasks/task-001/visibility")
    );
    assert_eq!(
        visibility
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("executor"))
            .and_then(|value| value.as_str()),
        Some("assistant")
    );

    let contract = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://protocol/client-contract"}),
        )
        .unwrap();
    assert_eq!(
        contract
            .get("resource")
            .and_then(|value| value.get("uri"))
            .and_then(|value| value.as_str()),
        Some("emberflow://protocol/client-contract")
    );
    assert!(contract
        .get("resource")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.get("resources"))
        .and_then(|value| value.as_array())
        .is_some());
}

// @minter:integration stdio-transport-returns-workspace-overview-view
#[test]
fn stdio_transport_returns_workspace_overview_view() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    let overview = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://workspace/overview"}),
        )
        .unwrap();

    let overview = overview.get("resource").unwrap().get("content").unwrap();
    assert_eq!(
        overview.get("source").and_then(|value| value.as_str()),
        Some("emberflow-canonical-state")
    );
    assert_eq!(
        overview
            .get("projectionMode")
            .and_then(|value| value.as_str()),
        Some("canonical")
    );
    let tracks = overview
        .get("tracks")
        .and_then(|value| value.as_array())
        .unwrap();
    assert_eq!(tracks.len(), 1);
    let track = &tracks[0];
    assert_eq!(
        track.get("trackId").and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert_eq!(
        track.get("executor").and_then(|value| value.as_str()),
        Some("assistant")
    );
}

// @minter:integration stdio-transport-reads-workspace-overview-with-legacy-null-metadata
#[test]
fn stdio_transport_reads_workspace_overview_with_legacy_null_metadata() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    corrupt_legacy_metadata(tmp.path(), "track-001");
    let session = start_with_cwd(tmp.path());

    let overview = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://workspace/overview"}),
        )
        .unwrap();

    let overview = overview.get("resource").unwrap().get("content").unwrap();
    assert_eq!(
        overview.get("source").and_then(|value| value.as_str()),
        Some("emberflow-canonical-state")
    );
    let tracks = overview
        .get("tracks")
        .and_then(|value| value.as_array())
        .unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(
        tracks[0].get("trackId").and_then(|value| value.as_str()),
        Some("track-001")
    );
}

// @minter:integration stdio-transport-returns-track-resume-view
#[test]
fn stdio_transport_returns_track_resume_view() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    let resume = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/resume"}),
        )
        .unwrap();

    let resume = resume.get("resource").unwrap().get("content").unwrap();
    assert_eq!(
        resume.get("source").and_then(|value| value.as_str()),
        Some("emberflow-canonical-state")
    );
    assert_eq!(
        resume.get("trackId").and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert_eq!(
        resume.get("intentSummary").and_then(|value| value.as_str()),
        Some("Persist the first event")
    );
    assert_eq!(
        resume.get("currentPhase").and_then(|value| value.as_str()),
        Some("implementing")
    );
    assert!(resume.get("summarySections").is_some());
    assert!(resume.get("plan").is_some());
}

// @minter:integration mcp-creates-task-with-generic-executor-fallback
#[test]
fn stdio_transport_returns_visibility_fields_from_create_task() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    let task = session
        .call(
            "emberflow-task-create",
            json!({
                "taskId": "task-visibility",
                "trackId": "track-001",
                "title": "Inspect runtime visibility",
                "status": "running",
                "phase": "planning",
                "execution": "interactive session",
                "intentSummary": "Inspect runtime visibility for the user"
            }),
        )
        .unwrap();

    assert_eq!(
        task.get("id").and_then(|value| value.as_str()),
        Some("task-visibility")
    );
    assert_eq!(
        task.get("trackId").and_then(|value| value.as_str()),
        Some("track-001")
    );
    assert_eq!(
        task.get("executor").and_then(|value| value.as_str()),
        Some("assistant")
    );
    assert_eq!(
        task.get("execution").and_then(|value| value.as_str()),
        Some("interactive session")
    );
    assert_eq!(
        task.get("intentSummary").and_then(|value| value.as_str()),
        Some("Inspect runtime visibility for the user")
    );
}

// @minter:integration stdio-transport-dispatches-canonical-runtime-writes
#[test]
fn stdio_transport_dispatches_canonical_runtime_writes() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    session
        .call(
            "emberflow-task-claim",
            json!({
                "taskId": "task-001",
                "holder": "assistant",
                "durationSecs": 300
            }),
        )
        .unwrap();

    let event = session
        .call(
            "emberflow-event-record",
            json!({
                "eventId": "event-001",
                "trackId": "track-001",
                "taskId": "task-001",
                "kind": "progress",
                "payload": {
                    "summary": "Writing the first event",
                    "recommended_next_step": "Share the current EmberFlow state"
                }
            }),
        )
        .unwrap();

    assert_eq!(
        event.get("id").and_then(|value| value.as_str()),
        Some("event-001")
    );
    assert_eq!(
        event.get("kind").and_then(|value| value.as_str()),
        Some("progress")
    );
    assert!(event.get("payload").is_some());

    let runtime_status = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/runtime"}),
        )
        .unwrap();
    let runtime_status = runtime_status
        .get("resource")
        .unwrap()
        .get("content")
        .unwrap();
    assert_eq!(
        runtime_status
            .get("executor")
            .and_then(|value| value.as_str()),
        Some("assistant")
    );
    assert_eq!(
        runtime_status
            .get("execution")
            .and_then(|value| value.as_str()),
        Some("interactive session")
    );
    assert_eq!(
        runtime_status
            .get("intentSummary")
            .and_then(|value| value.as_str()),
        Some("Persist the first event")
    );
    assert_eq!(
        runtime_status.get("next").and_then(|value| value.as_str()),
        Some("Share the current EmberFlow state")
    );
}

// @minter:integration progress-runtime-inherits-canonical-task-state
#[test]
fn stdio_transport_keeps_runtime_aligned_with_canonical_task_for_progress_without_overrides() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    session
        .call(
            "emberflow-task-create",
            json!({
                "taskId": "task-plan-review",
                "trackId": "track-001",
                "title": "Review and approve the execution plan",
                "status": "need-input",
                "phase": "planning",
                "execution": "interactive session",
                "intentSummary": "Review the plan before implementation"
            }),
        )
        .unwrap();

    session
        .call(
            "emberflow-task-claim",
            json!({
                "taskId": "task-plan-review",
                "holder": "assistant",
                "durationSecs": 300
            }),
        )
        .unwrap();

    session
        .call(
            "emberflow-event-record",
            json!({
                "eventId": "event-plan-review",
                "trackId": "track-001",
                "taskId": "task-plan-review",
                "kind": "progress",
                "payload": {
                    "summary": "Waiting for plan approval"
                }
            }),
        )
        .unwrap();

    let runtime_status = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tracks/track-001/runtime"}),
        )
        .unwrap();
    let runtime_status = runtime_status
        .get("resource")
        .unwrap()
        .get("content")
        .unwrap();

    assert_eq!(
        runtime_status
            .get("status")
            .and_then(|value| value.as_str()),
        Some("need-input")
    );
    assert_eq!(
        runtime_status.get("phase").and_then(|value| value.as_str()),
        Some("planning")
    );
    assert_eq!(
        runtime_status
            .get("statusLine")
            .and_then(|value| value.as_str()),
        Some("phase: planning | status: need-input | details: Waiting for plan approval")
    );

    let visibility = session
        .call(
            "read-resource",
            json!({"uri": "emberflow://tasks/task-plan-review/visibility"}),
        )
        .unwrap();
    let visibility = visibility.get("resource").unwrap().get("content").unwrap();

    assert_eq!(
        visibility.get("status").and_then(|value| value.as_str()),
        Some("need-input")
    );
    assert_eq!(
        visibility.get("phase").and_then(|value| value.as_str()),
        Some("planning")
    );
}

// @minter:integration stdio-transport-rejects-unknown-method
#[test]
fn stdio_transport_rejects_unknown_method() {
    let tmp = tempdir().unwrap();
    let session = start_with_cwd(tmp.path());

    let error = session.call("delete-track", json!({})).unwrap_err();
    assert_eq!(error.code, "method_not_found");
    assert!(error.message.contains("unknown"));
}

// @minter:integration stdio-transport-calls-tools-via-standard-mcp-method stdio-transport-deletes-track-state-through-prefixed-tools
#[test]
fn stdio_transport_archives_and_deletes_tracks_via_prefixed_tools() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let mut client = McpClient::new(tmp.path());

    client.call_tool(
        "emberflow-track-metadata-upsert",
        json!({
            "trackId": "track-001",
            "trackType": "feature",
            "status": "review",
            "description": "Build EmberFlow V1",
            "branch": "feature/mcp-surface",
            "specRef": "emberflow/specs/emberflow-mcp-surface.spec"
        }),
    );

    let archived = client.call_tool(
        "emberflow-track-archive",
        json!({
            "trackId": "track-001"
        }),
    );
    assert_eq!(
        archived
            .get("structuredContent")
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("archived")
    );

    let resources = client.list_resources();
    let uris: Vec<_> = resources
        .get("resources")
        .and_then(|value| value.as_array())
        .unwrap()
        .iter()
        .filter_map(|value| value.get("uri").and_then(|uri| uri.as_str()))
        .collect();
    assert!(!uris.contains(&"emberflow://tracks/track-001/record"));

    let archived_record = client.read_resource("emberflow://tracks/track-001/record");
    assert_eq!(
        archived_record
            .get("resource")
            .and_then(|value| value.get("content"))
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str()),
        Some("archived")
    );

    client.call_tool(
        "emberflow-track-delete",
        json!({
            "trackId": "track-001"
        }),
    );

    let deleted_task_visibility = client.send_request(
        "resources/read",
        json!({
            "uri": "emberflow://tasks/task-001/visibility"
        }),
    );
    assert_eq!(
        deleted_task_visibility
            .get("error")
            .and_then(|value| value.get("code"))
            .and_then(|value| value.as_str()),
        Some("not_found")
    );

    let deleted = client.send_request(
        "resources/read",
        json!({
            "uri": "emberflow://tracks/track-001/record"
        }),
    );
    assert_eq!(
        deleted
            .get("error")
            .and_then(|value| value.get("code"))
            .and_then(|value| value.as_str()),
        Some("not_found")
    );
}

// @minter:integration stdio-transport-rejects-invalid-parameters
#[test]
fn stdio_transport_rejects_invalid_parameters() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());
    let session = start_with_cwd(tmp.path());

    let error = session
        .call(
            "emberflow-event-record",
            json!({
                "eventId": "",
                "kind": "retry"
            }),
        )
        .unwrap_err();

    assert_eq!(error.code, "invalid_params");
    assert!(error.field.is_some());
    assert!(error.reason.is_some());
}

// @minter:integration stdio-transport-reports-missing-project-state
#[test]
fn stdio_transport_reports_missing_project_state() {
    let broken = tempdir().unwrap();
    let missing = broken.path().join("missing-repo");
    let error = start_stdio_server(StdioTransportConfig {
        cwd: Some(missing),
        workspace_root: None,
        state_path: None,
    })
    .unwrap_err();

    assert_eq!(error.source, "workspace-resolution");
    assert!(error.message.contains("EmberFlow"));
}

// @minter:integration stdio-transport-keeps-protocol-output-separate-from-diagnostics
#[test]
fn stdio_transport_keeps_protocol_output_separate_from_diagnostics() {
    let tmp = tempdir().unwrap();
    let session = start_with_cwd(tmp.path());
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    session
        .write_protocol_response(&mut stdout, json!(1), json!({"ok": true}))
        .unwrap();
    session
        .write_diagnostic(&mut stderr, "warn", "runtime diagnostic")
        .unwrap();

    let stdout_text = String::from_utf8(stdout).unwrap();
    let stderr_text = String::from_utf8(stderr).unwrap();
    assert!(stdout_text.contains("\"jsonrpc\":\"2.0\""));
    assert!(stdout_text.contains("\"ok\":true"));
    assert_eq!(stderr_text.trim(), "warn: runtime diagnostic");
}

// @minter:integration stdio-transport-mirrors-client-framing
#[test]
fn stdio_transport_responds_in_json_lines_when_client_uses_json_lines() {
    let tmp = tempdir().unwrap();
    let session = start_with_cwd(tmp.path());

    // Build a JSON lines request (no Content-Length header)
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "json-lines-client",
                "version": "0.1"
            }
        }
    });
    let input = format!("{}\n", serde_json::to_string(&request).unwrap());

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    session
        .serve_stdio(
            std::io::BufReader::new(input.as_bytes()),
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

    let stdout_text = String::from_utf8(stdout).unwrap();

    // Must NOT start with Content-Length framing
    assert!(
        !stdout_text.starts_with("Content-Length:"),
        "expected JSON lines response, got Content-Length framing: {stdout_text}"
    );

    // First line must be valid JSON
    let first_line = stdout_text
        .lines()
        .next()
        .expect("at least one response line");
    let response: serde_json::Value =
        serde_json::from_str(first_line).expect("first line must be valid JSON");

    assert_eq!(response["id"], json!(1));
    assert_eq!(response["jsonrpc"], json!("2.0"));
    assert!(
        response.get("result").is_some(),
        "expected result, got: {response}"
    );
    assert_eq!(response["result"]["protocolVersion"], json!("2024-11-05"));
    assert_eq!(response["result"]["serverInfo"]["name"], json!("emberflow"));
}

// @minter:unit claim-task-acquires-exclusive-lease
#[test]
fn stdio_task_claim_roundtrip() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());

    let mut client = McpClient::new(tmp.path());

    let response = client.send_request(
        "tools/call",
        json!({
            "name": "emberflow-task-claim",
            "arguments": {
                "taskId": "task-001",
                "holder": "agent-a",
                "durationSecs": 300
            }
        }),
    );

    assert!(
        response.get("result").is_some(),
        "expected result, got: {response}"
    );
    let content = &response["result"]["structuredContent"];
    assert_eq!(content["holder"], json!("agent-a"));
    assert!(content["acquiredAt"].is_string());
}

// @minter:unit release-clears-lease
#[test]
fn stdio_task_release_roundtrip() {
    let tmp = tempdir().unwrap();
    seed_workspace(tmp.path());

    let mut client = McpClient::new(tmp.path());

    // Claim first
    client.send_request(
        "tools/call",
        json!({
            "name": "emberflow-task-claim",
            "arguments": {
                "taskId": "task-001",
                "holder": "agent-a",
                "durationSecs": 300
            }
        }),
    );

    // Then release
    let response = client.send_request(
        "tools/call",
        json!({
            "name": "emberflow-task-release",
            "arguments": {
                "taskId": "task-001",
                "holder": "agent-a"
            }
        }),
    );

    assert!(
        response.get("result").is_some(),
        "expected result, got: {response}"
    );
    let content = &response["result"]["structuredContent"];
    assert_eq!(content["released"], json!(true));
}
