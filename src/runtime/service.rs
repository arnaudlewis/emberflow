use super::layout::PROJECTED_RUNTIME_STATUS_PATH;
pub use super::layout::{EmberFlowMode, EmberFlowProjectLayout};
pub use super::projections::{
    ProjectionEngine, RuntimeProjection, TrackProjection, UserProjection,
};
pub use super::store::{
    EventRecord, LeaseInfo, ProjectionDirtyTargetInput, ProjectionDirtyTargetRecord,
    ProjectionRecord, RuntimeStore, TaskInput, TaskRecord, TrackBriefRecord,
    TrackBriefSectionInput, TrackBriefSectionRecord, TrackMetadataInput, TrackMetadataRecord,
    TrackPlanItemInput, TrackPlanItemRecord, TrackPlanPhaseInput, TrackPlanPhaseRecord,
    TrackPlanRecord, TrackRecord,
};
use crate::error::{EmberFlowError, Result};
use crate::protocol::validate_choice;
pub use crate::protocol::{
    EMBERFLOW_STANDARD_TOOLS, PHASES, RUNTIME_MESSAGES, TASK_STATUSES, TRACK_STATUSES,
};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const TRACK_BOOTSTRAP_BRIEF_ARTIFACT: &str = "brief.md";
const TRACK_BOOTSTRAP_REQUIRED_SECTIONS: &[&str] = &[
    "objective",
    "context",
    "decisions",
    "non_goals",
    "current_state",
    "workspace_branch_pr_context",
    "next_step",
];
const RESOURCE_WORKSPACE_OVERVIEW_URI: &str = "emberflow://workspace/overview";
const RESOURCE_TRACK_RECORD_URI_TEMPLATE: &str = "emberflow://tracks/{trackId}/record";
const RESOURCE_TRACK_RESUME_URI_TEMPLATE: &str = "emberflow://tracks/{trackId}/resume";
const RESOURCE_TRACK_TRANSPARENCY_URI_TEMPLATE: &str = "emberflow://tracks/{trackId}/transparency";
const RESOURCE_TRACK_CONTEXT_URI_TEMPLATE: &str = "emberflow://tracks/{trackId}/context";
const RESOURCE_TRACK_EVENTS_URI_TEMPLATE: &str = "emberflow://tracks/{trackId}/events";
const RESOURCE_TRACK_PLAN_URI_TEMPLATE: &str = "emberflow://tracks/{trackId}/plan";
const RESOURCE_TRACK_BRIEF_URI_TEMPLATE: &str = "emberflow://tracks/{trackId}/brief";
const RESOURCE_TRACK_RUNTIME_URI_TEMPLATE: &str = "emberflow://tracks/{trackId}/runtime";
const RESOURCE_TASK_VISIBILITY_URI_TEMPLATE: &str = "emberflow://tasks/{taskId}/visibility";
const RESOURCE_TASK_EVENTS_URI_TEMPLATE: &str = "emberflow://tasks/{taskId}/events";
const RESOURCE_CLIENT_CONTRACT_URI: &str = "emberflow://protocol/client-contract";
const WORKSPACE_DB_MODE: &str = "sqlite";
const WORKSPACE_DB_INITIALIZATION: &str = "ready";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackBootstrapInfo {
    pub brief_artifact: &'static str,
    pub required_sections: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDbInfo {
    pub project_root: String,
    pub state_root: String,
    pub default_path: String,
    pub projection_mode: &'static str,
    pub mode: &'static str,
    pub initialization: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializeResponse {
    pub capabilities: Vec<&'static str>,
    pub track_bootstrap: TrackBootstrapInfo,
    pub workspace_db: WorkspaceDbInfo,
    pub system_role: &'static str,
    pub source_of_truth: &'static str,
    pub projected_files: &'static str,
    pub preferred_client_sequence: Vec<&'static str>,
    pub knowledge_views: Vec<KnowledgeViewDescriptor>,
    pub resource_views: Vec<ResourceViewDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeViewDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub params: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceViewDescriptor {
    pub uri_template: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceReadResponse {
    pub uri: String,
    pub name: &'static str,
    pub description: &'static str,
    pub mime_type: &'static str,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStateResult {
    pub track_id: String,
    pub task_id: String,
    pub event_kind: String,
    pub store: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStatusProjection {
    pub track_id: String,
    pub task_id: Option<String>,
    pub target_path: String,
    pub status_line: String,
    pub status: Option<String>,
    pub phase: Option<String>,
    pub next: Option<String>,
    pub executor: Option<String>,
    pub execution: Option<String>,
    pub intent_summary: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskVisibilityView {
    pub source: String,
    pub task_id: String,
    pub track_id: Option<String>,
    pub title: String,
    pub status: String,
    pub phase: String,
    pub executor: Option<String>,
    pub execution: Option<String>,
    pub intent_summary: Option<String>,
    pub track_status: Option<String>,
    pub next: Option<String>,
    pub updated_at: String,
    pub lease_holder: Option<String>,
    pub lease_expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedFilesystemTargets {
    pub mode: String,
    pub runtime_status_path: String,
    pub track_list_path: String,
    pub track_directory_path: String,
    pub metadata_path: String,
    pub brief_path: String,
    pub plan_path: String,
    pub summary_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackContextRecord {
    pub track_id: String,
    pub metadata: TrackMetadataRecord,
    pub brief: TrackBriefRecord,
    pub plan: TrackPlanRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceOverviewTrack {
    pub track_id: String,
    pub title: String,
    pub track_type: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub updated_at: String,
    pub executor: Option<String>,
    pub execution: Option<String>,
    pub intent_summary: Option<String>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceOverview {
    pub source: String,
    pub projection_mode: String,
    pub tracks: Vec<WorkspaceOverviewTrack>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackResumeView {
    pub source: String,
    pub track_id: String,
    pub title: String,
    pub track_type: Option<String>,
    pub status: String,
    pub description: Option<String>,
    pub branch: Option<String>,
    pub spec_ref: Option<String>,
    pub summary_sections: Vec<TrackBriefSectionRecord>,
    pub plan: TrackPlanRecord,
    pub task_id: Option<String>,
    pub executor: Option<String>,
    pub execution: Option<String>,
    pub intent_summary: Option<String>,
    pub next: Option<String>,
    pub current_phase: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackTransparencyView {
    pub source: String,
    pub track_id: String,
    pub task_id: Option<String>,
    pub track_status: String,
    pub task_status: Option<String>,
    pub phase: Option<String>,
    pub executor: Option<String>,
    pub execution: Option<String>,
    pub intent_summary: Option<String>,
    pub next: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventFeed {
    pub track_id: Option<String>,
    pub task_id: Option<String>,
    pub items: Vec<EventRecord>,
}

#[derive(Debug, Clone)]
pub struct EmberFlowRuntime {
    pub store: RuntimeStore,
    pub engine: ProjectionEngine,
    pub layout: EmberFlowProjectLayout,
}

impl EmberFlowRuntime {
    pub const SUPPORTED_TOOLS: &'static [&'static str] = EMBERFLOW_STANDARD_TOOLS;

    pub fn from_workspace_root<P: AsRef<Path>>(workspace_root: P) -> Result<Self> {
        let layout = EmberFlowProjectLayout::discover(workspace_root)?;
        Ok(Self {
            store: RuntimeStore::new(&layout.db_path)?,
            engine: ProjectionEngine,
            layout,
        })
    }

    pub fn from_db_path<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let layout = EmberFlowProjectLayout::from_db_path(db_path)?;
        Ok(Self {
            store: RuntimeStore::new(&layout.db_path)?,
            engine: ProjectionEngine,
            layout,
        })
    }

    #[deprecated(
        note = "use EmberFlowRuntime::from_workspace_root for workspace discovery or EmberFlowRuntime::from_db_path for explicit db_path compatibility"
    )]
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        Self::from_db_path(db_path)
    }

    pub fn available_tools(&self) -> Vec<&'static str> {
        Self::SUPPORTED_TOOLS.to_vec()
    }

    pub fn available_resource_views(&self) -> Vec<ResourceViewDescriptor> {
        vec![
            ResourceViewDescriptor {
                uri_template: RESOURCE_WORKSPACE_OVERVIEW_URI,
                name: "workspace-overview",
                description: "Workspace-level canonical track overview from EmberFlow.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TRACK_RECORD_URI_TEMPLATE,
                name: "track-record",
                description: "Canonical track record from EmberFlow.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TRACK_RESUME_URI_TEMPLATE,
                name: "track-resume",
                description: "Resume view for one track, combining metadata, summary, plan, and runtime visibility.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TRACK_TRANSPARENCY_URI_TEMPLATE,
                name: "track-transparency",
                description: "Display-ready canonical transparency state for one track.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TRACK_CONTEXT_URI_TEMPLATE,
                name: "track-context",
                description: "Canonical track context combining metadata, brief, and plan.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TRACK_BRIEF_URI_TEMPLATE,
                name: "track-brief",
                description: "Canonical resume summary for one track.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TRACK_PLAN_URI_TEMPLATE,
                name: "track-plan",
                description: "Canonical execution plan for one track.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TRACK_RUNTIME_URI_TEMPLATE,
                name: "track-runtime",
                description: "Current runtime projection for one track.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TRACK_EVENTS_URI_TEMPLATE,
                name: "track-events",
                description: "Canonical event feed for one track.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TASK_VISIBILITY_URI_TEMPLATE,
                name: "task-visibility",
                description: "Task-level visibility fields plus current track status and next step when available.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_TASK_EVENTS_URI_TEMPLATE,
                name: "task-events",
                description: "Canonical event feed for one task.",
                mime_type: "application/json",
            },
            ResourceViewDescriptor {
                uri_template: RESOURCE_CLIENT_CONTRACT_URI,
                name: "client-contract",
                description: "Static client contract for EmberFlow bootstrap, mutation, and transparency.",
                mime_type: "application/json",
            },
        ]
    }

    pub fn initialize(&self) -> Result<InitializeResponse> {
        self.try_refresh_dirty_projection_targets();
        Ok(InitializeResponse {
            capabilities: self.available_tools(),
            track_bootstrap: TrackBootstrapInfo {
                brief_artifact: TRACK_BOOTSTRAP_BRIEF_ARTIFACT,
                required_sections: TRACK_BOOTSTRAP_REQUIRED_SECTIONS.to_vec(),
            },
            workspace_db: WorkspaceDbInfo {
                project_root: self.layout.project_root.display().to_string(),
                state_root: self.layout.state_root.display().to_string(),
                default_path: self.layout.db_path.display().to_string(),
                projection_mode: self.layout.mode.as_str(),
                mode: WORKSPACE_DB_MODE,
                initialization: WORKSPACE_DB_INITIALIZATION,
            },
            system_role: "canonical tracked runtime and visibility layer",
            source_of_truth: "emberflow-canonical-state",
            projected_files: "derived-only",
            preferred_client_sequence: vec![
                "initialize",
                "list_resources",
                "read_resource",
                "mutate_via_emberflow_mcp",
            ],
            knowledge_views: vec![
                KnowledgeViewDescriptor {
                    name: "workspace-overview",
                    description: "Read-only workspace overview resource exposed by EmberFlow.",
                    params: vec![],
                },
                KnowledgeViewDescriptor {
                    name: "track-record",
                    description: "Read-only canonical track record resource exposed by EmberFlow.",
                    params: vec!["trackId"],
                },
                KnowledgeViewDescriptor {
                    name: "track-resume",
                    description: "Read-only resume view for one track, combining metadata, summary, plan, and runtime visibility.",
                    params: vec!["trackId"],
                },
                KnowledgeViewDescriptor {
                    name: "track-transparency",
                    description: "Read-only display-ready canonical transparency state for one track.",
                    params: vec!["trackId"],
                },
                KnowledgeViewDescriptor {
                    name: "track-context",
                    description: "Read-only canonical track context resource exposed by EmberFlow.",
                    params: vec!["trackId"],
                },
                KnowledgeViewDescriptor {
                    name: "track-brief",
                    description: "Read-only canonical brief resource exposed by EmberFlow.",
                    params: vec!["trackId"],
                },
                KnowledgeViewDescriptor {
                    name: "track-plan",
                    description: "Read-only canonical plan resource exposed by EmberFlow.",
                    params: vec!["trackId"],
                },
                KnowledgeViewDescriptor {
                    name: "track-runtime",
                    description: "Read-only runtime projection resource exposed by EmberFlow.",
                    params: vec!["trackId"],
                },
                KnowledgeViewDescriptor {
                    name: "track-events",
                    description: "Read-only track event feed resource exposed by EmberFlow.",
                    params: vec!["trackId"],
                },
                KnowledgeViewDescriptor {
                    name: "task-visibility",
                    description: "Read-only task visibility resource exposed by EmberFlow.",
                    params: vec!["taskId"],
                },
                KnowledgeViewDescriptor {
                    name: "task-events",
                    description: "Read-only task event feed resource exposed by EmberFlow.",
                    params: vec!["taskId"],
                },
                KnowledgeViewDescriptor {
                    name: "client-contract",
                    description: "Read-only bootstrap contract resource exposed by EmberFlow.",
                    params: vec![],
                },
            ],
            resource_views: self.available_resource_views(),
        })
    }

    pub fn list_resource_views(&self) -> Vec<ResourceViewDescriptor> {
        self.available_resource_views()
    }

    pub fn list_tracks(&self) -> Result<Vec<TrackRecord>> {
        self.store.list_tracks()
    }

    pub fn list_active_tracks(&self) -> Result<Vec<TrackRecord>> {
        self.store.list_active_tracks()
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskRecord>> {
        self.store.list_tasks()
    }

    pub fn read_resource(&self, uri: &str) -> Result<ResourceReadResponse> {
        if uri == RESOURCE_WORKSPACE_OVERVIEW_URI {
            let overview = self.read_workspace_overview()?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "workspace-overview",
                description: "Workspace-level canonical track overview from EmberFlow.",
                mime_type: "application/json",
                content: self.workspace_overview_content(&overview),
            });
        }

        if let Some(track_id) = uri
            .strip_prefix("emberflow://tracks/")
            .and_then(|value| value.strip_suffix("/record"))
        {
            let track = self.read_track(track_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "track-record",
                description: "Canonical track record from EmberFlow.",
                mime_type: "application/json",
                content: self.track_record_content(&track),
            });
        }

        if uri == RESOURCE_CLIENT_CONTRACT_URI {
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "client-contract",
                description:
                    "Static client contract for EmberFlow bootstrap, mutation, and transparency.",
                mime_type: "application/json",
                content: self.client_contract_content(),
            });
        }

        if let Some(track_id) = uri
            .strip_prefix("emberflow://tracks/")
            .and_then(|value| value.strip_suffix("/resume"))
        {
            let track_resume = self.read_track_resume(track_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "track-resume",
                description: "Resume view for one track, combining metadata, summary, plan, and runtime visibility.",
                mime_type: "application/json",
                content: self.track_resume_content(&track_resume),
            });
        }

        if let Some(track_id) = uri
            .strip_prefix("emberflow://tracks/")
            .and_then(|value| value.strip_suffix("/transparency"))
        {
            let transparency = self.read_track_transparency(track_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "track-transparency",
                description: "Display-ready canonical transparency state for one track.",
                mime_type: "application/json",
                content: self.track_transparency_content(&transparency),
            });
        }

        if let Some(track_id) = uri
            .strip_prefix("emberflow://tracks/")
            .and_then(|value| value.strip_suffix("/context"))
        {
            let context = self.load_track_context(track_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "track-context",
                description: "Canonical track context combining metadata, brief, and plan.",
                mime_type: "application/json",
                content: self.track_context_content(&context),
            });
        }

        if let Some(track_id) = uri
            .strip_prefix("emberflow://tracks/")
            .and_then(|value| value.strip_suffix("/plan"))
        {
            let track_plan = self.list_track_plan(track_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "track-plan",
                description: "Canonical execution plan for one track.",
                mime_type: "application/json",
                content: self.track_plan_content(&track_plan),
            });
        }

        if let Some(track_id) = uri
            .strip_prefix("emberflow://tracks/")
            .and_then(|value| value.strip_suffix("/brief"))
        {
            let track_brief = self.read_track_brief(track_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "track-brief",
                description: "Canonical resume summary for one track.",
                mime_type: "application/json",
                content: self.track_brief_content(&track_brief),
            });
        }

        if let Some(track_id) = uri
            .strip_prefix("emberflow://tracks/")
            .and_then(|value| value.strip_suffix("/runtime"))
        {
            let runtime = self.get_runtime_status(track_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "track-runtime",
                description: "Current runtime projection for one track.",
                mime_type: "application/json",
                content: self.track_runtime_content(&runtime),
            });
        }

        if let Some(track_id) = uri
            .strip_prefix("emberflow://tracks/")
            .and_then(|value| value.strip_suffix("/events"))
        {
            let feed = self.track_events(track_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "track-events",
                description: "Canonical event feed for one track.",
                mime_type: "application/json",
                content: self.event_feed_content(&feed),
            });
        }

        if let Some(task_id) = uri
            .strip_prefix("emberflow://tasks/")
            .and_then(|value| value.strip_suffix("/visibility"))
        {
            let visibility = self.read_task_visibility(task_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "task-visibility",
                description: "Task-level visibility fields plus current track status and next step when available.",
                mime_type: "application/json",
                content: self.task_visibility_content(&visibility),
            });
        }

        if let Some(task_id) = uri
            .strip_prefix("emberflow://tasks/")
            .and_then(|value| value.strip_suffix("/events"))
        {
            let feed = self.task_events(task_id)?;
            return Ok(ResourceReadResponse {
                uri: uri.to_string(),
                name: "task-events",
                description: "Canonical event feed for one task.",
                mime_type: "application/json",
                content: self.event_feed_content(&feed),
            });
        }

        Err(EmberFlowError::NotFound(uri.to_string()))
    }

    pub fn projected_track_filesystem_targets(
        &self,
        track_id: &str,
    ) -> Result<ProjectedFilesystemTargets> {
        Ok(ProjectedFilesystemTargets {
            mode: self.layout.mode.as_str().to_string(),
            runtime_status_path: PROJECTED_RUNTIME_STATUS_PATH.to_string(),
            track_list_path: ".emberflow/tracks/tracks.md".to_string(),
            track_directory_path: self.track_directory_path(track_id),
            metadata_path: self.track_target_path(track_id, "metadata.json"),
            brief_path: self.track_target_path(track_id, "brief.md"),
            plan_path: self.track_target_path(track_id, "plan.md"),
            summary_path: self.track_target_path(track_id, "index.md"),
        })
    }

    pub fn dirty_projection_targets(
        &self,
        track_id: Option<&str>,
    ) -> Result<Vec<ProjectionDirtyTargetRecord>> {
        self.store.list_dirty_projection_targets(track_id)
    }

    pub fn refresh_dirty_projection_targets(&self) -> Result<()> {
        if self.layout.mode == EmberFlowMode::Canonical {
            return Ok(());
        }

        let dirty_targets = self.store.list_dirty_projection_targets(None)?;
        if dirty_targets.is_empty() {
            return Ok(());
        }

        let mut failures = Vec::new();
        for target in dirty_targets {
            match self.refresh_dirty_projection_target(&target) {
                Ok(()) => {
                    self.store
                        .clear_dirty_projection_target(&target.target_path)?;
                }
                Err(error) => {
                    failures.push(format!("{}: {error}", target.target_path));
                }
            }
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(EmberFlowError::UnsupportedValue {
                field: "projection_refresh",
                value: format!("filesystem materialization failed: {}", failures.join("; ")),
            })
        }
    }

    fn refresh_dirty_projection_target(&self, target: &ProjectionDirtyTargetRecord) -> Result<()> {
        let contents = self.render_projection_target(target)?;
        self.write_projected_file(&target.target_path, &contents)
    }

    fn render_projection_target(&self, target: &ProjectionDirtyTargetRecord) -> Result<String> {
        match target.projection_kind.as_str() {
            "runtime-status" => self.render_runtime_status_target(target),
            "track-list" => self.render_track_list_target(),
            "track-metadata" => self.render_track_metadata_target(target),
            "track-brief" => self.render_track_brief_target(target),
            "track-plan" => self.render_track_plan_target(target),
            "track-summary" => self.render_track_summary_target(target),
            projection_kind => Err(EmberFlowError::UnsupportedValue {
                field: "projection_kind",
                value: projection_kind.to_string(),
            }),
        }
    }

    fn render_runtime_status_target(&self, target: &ProjectionDirtyTargetRecord) -> Result<String> {
        if let Some(track_id) = target.track_id.as_deref() {
            return self.runtime_status_line_without_refresh(track_id);
        }

        Ok(String::new())
    }

    fn render_track_list_target(&self) -> Result<String> {
        let mut output = String::from("# Project Tracks\n\n");
        output.push_str("| ID | Type | Status | Description | Created |\n");
        output.push_str("|----|------|--------|-------------|---------|\n");

        for track in self.store.list_active_tracks()? {
            let metadata = self.store.get_track_metadata_optional(&track.id)?;
            let track_type = metadata
                .as_ref()
                .and_then(|record| {
                    if record.track_type.trim().is_empty() {
                        None
                    } else {
                        Some(record.track_type.as_str())
                    }
                })
                .unwrap_or("-");
            let description = metadata
                .as_ref()
                .and_then(|record| {
                    if record.description.trim().is_empty() {
                        None
                    } else {
                        Some(record.description.as_str())
                    }
                })
                .unwrap_or(track.title.as_str());
            let created_at = metadata
                .as_ref()
                .map(|record| record.created_at.as_str())
                .unwrap_or(track.created_at.as_str());

            output.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                self.escape_markdown_cell(&track.id),
                self.escape_markdown_cell(track_type),
                self.escape_markdown_cell(&track.status),
                self.escape_markdown_cell(description),
                self.escape_markdown_cell(created_at),
            ));
        }

        Ok(output)
    }

    fn render_track_metadata_target(&self, target: &ProjectionDirtyTargetRecord) -> Result<String> {
        let track_id =
            target
                .track_id
                .as_deref()
                .ok_or_else(|| EmberFlowError::UnsupportedValue {
                    field: "track_id",
                    value: "dirty metadata target missing track id".to_string(),
                })?;
        let metadata = self.store.get_track_metadata(track_id)?;
        Ok(format!(
            "{}\n",
            serde_json::to_string(&json!({
                "track_id": metadata.track_id,
                "track_type": metadata.track_type,
                "status": metadata.status,
                "description": metadata.description,
                "branch": metadata.branch,
                "spec_ref": metadata.spec_ref,
                "created_at": metadata.created_at,
                "updated_at": metadata.updated_at,
            }))?
        ))
    }

    fn render_track_brief_target(&self, target: &ProjectionDirtyTargetRecord) -> Result<String> {
        let track_id =
            target
                .track_id
                .as_deref()
                .ok_or_else(|| EmberFlowError::UnsupportedValue {
                    field: "track_id",
                    value: "dirty brief target missing track id".to_string(),
                })?;
        let metadata = self.store.get_track_metadata_optional(track_id)?;
        let track = self.store.get_track(track_id)?;
        let brief = self.store.get_track_brief(track_id)?;
        let title = metadata
            .as_ref()
            .and_then(|record| {
                if record.description.trim().is_empty() {
                    None
                } else {
                    Some(record.description.as_str())
                }
            })
            .unwrap_or(track.title.as_str());
        let branch = metadata
            .as_ref()
            .map(|record| record.branch.as_str())
            .unwrap_or("-");

        let mut output = String::new();
        output.push_str(&format!("# Track Brief: {title}\n\n"));
        output
            .push_str("This brief is the durable human-readable resume context for the track.\n\n");
        output.push_str("## Objective\n");
        for section in &brief.sections {
            if section.section_key == "objective" {
                output.push_str(&format!("- {}\n", section.section_text));
            }
        }
        output.push_str("\n## Context\n");
        for section in &brief.sections {
            if section.section_key == "context" {
                output.push_str(&format!("- {}\n", section.section_text));
            }
        }
        output.push_str("\n## Decisions already made\n- <recorded decisions that should not be re-litigated>\n\n");
        output.push_str("## Non-goals\n- <what is intentionally out of scope>\n\n");
        output.push_str("## Current state\n- <what is done and what remains>\n\n");
        output.push_str("## Workspace / branch / PR context\n");
        output.push_str(&format!(
            "- Workspace: {}\n- Branch: {branch}\n- PRs: none\n\n",
            self.layout.project_root.display()
        ));
        output.push_str("## Next step\n");
        output.push_str("- <the very next action a fresh workspace should take>\n");
        Ok(output)
    }

    fn render_track_plan_target(&self, target: &ProjectionDirtyTargetRecord) -> Result<String> {
        let track_id =
            target
                .track_id
                .as_deref()
                .ok_or_else(|| EmberFlowError::UnsupportedValue {
                    field: "track_id",
                    value: "dirty plan target missing track id".to_string(),
                })?;
        let metadata = self.store.get_track_metadata_optional(track_id)?;
        let track = self.store.get_track(track_id)?;
        let plan = self.store.get_track_plan(track_id)?;
        let title = metadata
            .as_ref()
            .and_then(|record| {
                if record.description.trim().is_empty() {
                    None
                } else {
                    Some(record.description.as_str())
                }
            })
            .unwrap_or(track.title.as_str());

        let mut output = String::new();
        output.push_str(&format!("# Implementation Plan: {title}\n\n"));
        output.push_str(
            "<!-- Task status markers:\n     [ ]  = pending\n     [~]  = in-progress\n     [x]  = completed\n\n     Completed tasks include the commit SHA:\n       - [x] Task: description (SHA: abc1234)\n\n     Phase headers get a checkpoint SHA once all tasks are verified:\n       ## Phase 1: Name [checkpoint: abc1234]\n\n     Phase completion tasks trigger the verification protocol:\n       run tests, manual check, create checkpoint commit.\n-->\n\n",
        );

        for (index, phase) in plan.phases.iter().enumerate() {
            output.push_str(&format!("## Phase {}: {}\n\n", index + 1, phase.title));
            for item in &phase.items {
                output.push_str(&format!("- [ ] Task: {}\n", item.title));
            }
            output.push('\n');
            output.push_str(&format!(
                "- [ ] Task: Phase Completion -- Verify '{}'\n\n",
                phase.title
            ));
        }

        Ok(output)
    }

    fn render_track_summary_target(&self, target: &ProjectionDirtyTargetRecord) -> Result<String> {
        let track_id =
            target
                .track_id
                .as_deref()
                .ok_or_else(|| EmberFlowError::UnsupportedValue {
                    field: "track_id",
                    value: "dirty summary target missing track id".to_string(),
                })?;
        let track = self.store.get_track(track_id)?;
        let metadata = self.store.get_track_metadata_optional(track_id)?;
        let brief = self.store.get_track_brief(track_id)?;
        let plan = self.store.get_track_plan(track_id)?;
        let description = metadata
            .as_ref()
            .and_then(|record| {
                if record.description.trim().is_empty() {
                    None
                } else {
                    Some(record.description.as_str())
                }
            })
            .unwrap_or(track.title.as_str());
        let track_type = metadata
            .as_ref()
            .and_then(|record| {
                if record.track_type.trim().is_empty() {
                    None
                } else {
                    Some(record.track_type.as_str())
                }
            })
            .unwrap_or("-");
        let branch = metadata
            .as_ref()
            .map(|record| record.branch.as_str())
            .unwrap_or("-");

        let mut output = String::new();
        output.push_str(&format!("# Track: {track_id}\n\n"));
        output.push_str("- [Brief](./brief.md) — durable resume context\n");
        output.push_str("- [Plan](./plan.md) — implementation plan\n");
        output.push_str("- [Metadata](./metadata.json) — canonical track metadata\n\n");
        output.push_str("## Summary\n");
        output.push_str(&format!("- Type: {track_type}\n"));
        output.push_str(&format!("- Status: {}\n", track.status));
        output.push_str(&format!("- Description: {description}\n"));
        output.push_str(&format!("- Branch: {branch}\n"));
        output.push_str(&format!("- Brief sections: {}\n", brief.sections.len()));
        output.push_str(&format!("- Plan phases: {}\n", plan.phases.len()));
        Ok(output)
    }

    fn write_projected_file(&self, relative_path: &str, contents: &str) -> Result<()> {
        let target_path = self.layout.project_root.join(relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = self.projected_temp_path(&target_path);
        let write_result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp_path)?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp_path, &target_path)?;
            Ok(())
        })();

        if write_result.is_err() {
            let _ = fs::remove_file(&temp_path);
        }

        write_result
    }

    fn projected_temp_path(&self, target_path: &Path) -> PathBuf {
        let parent = target_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.layout.project_root.clone());
        let file_name = target_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("projection");
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        parent.join(format!("{file_name}.tmp-{stamp}"))
    }

    fn escape_markdown_cell(&self, value: &str) -> String {
        value.replace('|', "\\|").replace('\n', " ")
    }

    pub fn create_track(&self, track_id: &str, title: &str, status: &str) -> Result<TrackRecord> {
        let track = self.store.create_track(track_id, title, status)?;
        self.mark_track_metadata_dirty(track_id, "track-created", None)?;
        self.mark_track_summary_dirty(track_id, "track-created", None)?;
        self.try_refresh_dirty_projection_targets();
        Ok(track)
    }

    pub fn archive_track(&self, track_id: &str) -> Result<TrackRecord> {
        let track = self.store.get_track(track_id)?;
        match track.status.as_str() {
            "review" | "done" | "archived" => {
                let track = self.store.update_track_status(track_id, "archived")?;
                self.mark_track_metadata_dirty(track_id, "track-archived", None)?;
                self.mark_track_summary_dirty(track_id, "track-archived", None)?;
                self.try_refresh_dirty_projection_targets();
                Ok(track)
            }
            other => Err(EmberFlowError::UnsupportedValue {
                field: "status",
                value: format!("manual archive requires review or done status (found {other})"),
            }),
        }
    }

    pub fn delete_track(&self, track_id: &str) -> Result<TrackRecord> {
        let track = self.store.delete_track(track_id)?;
        self.remove_projected_track_artifacts(track_id)?;
        self.try_refresh_dirty_projection_targets();
        Ok(track)
    }

    pub fn read_track(&self, track_id: &str) -> Result<TrackRecord> {
        self.try_refresh_dirty_projection_targets();
        self.store.get_track(track_id)
    }

    pub fn create_task(&self, input: TaskInput) -> Result<TaskRecord> {
        let mut input = input;
        if input.executor.is_none() && (input.execution.is_some() || input.intent_summary.is_some())
        {
            input.executor = Some("assistant".to_string());
        }
        self.store.create_task(input)
    }

    pub fn record_event(
        &self,
        event_id: &str,
        track_id: Option<&str>,
        task_id: Option<&str>,
        kind: &str,
        payload: serde_json::Value,
    ) -> Result<EventRecord> {
        // Lease guard: task-targeting events require an active lease.
        // No lease → rejected. Wrong holder → rejected.
        if let Some(tid) = task_id {
            match self.store.check_lease(tid)? {
                None => {
                    return Err(EmberFlowError::UnsupportedValue {
                        field: "taskId",
                        value: format!("task {tid} has no active lease, claim it first"),
                    });
                }
                Some(lease) => {
                    let executor = payload
                        .get("executor")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !executor.is_empty() && executor != lease.holder {
                        return Err(EmberFlowError::UnsupportedValue {
                            field: "executor",
                            value: format!(
                                "task {tid} has a lease held by {} — executor does not hold the lease",
                                lease.holder
                            ),
                        });
                    }
                }
            }
        }
        let event = self
            .store
            .record_event(event_id, track_id, task_id, kind, payload)?;
        self.persist_projections(&event)?;
        self.project_runtime_state(&event)?;
        self.mark_runtime_projection_dirty(event.track_id.as_deref(), Some(&event.id))?;
        self.try_refresh_dirty_projection_targets();
        Ok(event)
    }

    pub fn claim_task(
        &self,
        task_id: &str,
        holder: &str,
        duration_secs: Option<u64>,
    ) -> Result<LeaseInfo> {
        let lease = self.store.claim_task(task_id, holder, duration_secs)?;
        let event_id = format!("{task_id}:claim:{holder}");
        let _ = self.store.record_event(
            &event_id,
            None,
            Some(task_id),
            "claim",
            json!({ "holder": holder }),
        );
        Ok(lease)
    }

    pub fn release_task(&self, task_id: &str, holder: &str) -> Result<()> {
        self.store.release_task(task_id, holder)?;
        let event_id = format!("{task_id}:release:{holder}");
        let _ = self.store.record_event(
            &event_id,
            None,
            Some(task_id),
            "release",
            json!({ "holder": holder }),
        );
        Ok(())
    }

    pub fn record_runtime_state(
        &self,
        track_id: &str,
        task_id: &str,
        event_kind: &str,
        payload: serde_json::Value,
    ) -> Result<RuntimeStateResult> {
        validate_choice(event_kind, crate::protocol::RUNTIME_MESSAGES, "kind")?;
        self.store.ensure_track(track_id)?;
        self.store.ensure_task(task_id, track_id)?;
        let event = self.record_event(
            &format!("{track_id}:{task_id}:{event_kind}"),
            Some(track_id),
            Some(task_id),
            event_kind,
            payload,
        )?;
        Ok(RuntimeStateResult {
            track_id: track_id.to_string(),
            task_id: task_id.to_string(),
            event_kind: event.kind,
            store: "canonical".to_string(),
        })
    }

    pub fn list_events(&self, track_id: Option<&str>, task_id: Option<&str>) -> Result<EventFeed> {
        let items = self.store.list_events(track_id, task_id, None)?;
        Ok(EventFeed {
            track_id: track_id.map(ToString::to_string),
            task_id: task_id.map(ToString::to_string),
            items,
        })
    }

    pub fn project_runtime_status(&self, track_id: &str) -> Result<RuntimeStatusProjection> {
        self.try_refresh_dirty_projection_targets();
        self.project_runtime_status_without_refresh(track_id)
    }

    fn project_runtime_status_without_refresh(
        &self,
        track_id: &str,
    ) -> Result<RuntimeStatusProjection> {
        let latest_task = self.store.get_latest_task_for_track(track_id)?;
        let latest_event = self.store.get_latest_event_for_track(track_id)?;

        if let Some(task) = latest_task.as_ref() {
            let details = self.runtime_details_for_state(latest_event.as_ref(), Some(task));
            let next = match latest_event.as_ref() {
                Some(event) => self.next_step_from_event(event, track_id)?,
                None => self.track_next_step(track_id)?,
            };
            return Ok(RuntimeStatusProjection {
                track_id: track_id.to_string(),
                task_id: Some(task.id.clone()),
                target_path: PROJECTED_RUNTIME_STATUS_PATH.to_string(),
                status_line: format!(
                    "phase: {} | status: {} | details: {}",
                    task.phase, task.status, details
                ),
                status: Some(task.status.clone()),
                phase: Some(task.phase.clone()),
                next,
                executor: task.executor.clone(),
                execution: task.execution.clone(),
                intent_summary: task.intent_summary.clone(),
                source: "canonical-event-store".to_string(),
            });
        }

        let event = match latest_event {
            Some(event) => event,
            None => {
                let status = latest_task
                    .as_ref()
                    .map(|task| task.status.clone())
                    .or_else(|| {
                        self.store
                            .get_track(track_id)
                            .ok()
                            .map(|track| track.status)
                    });
                let phase = latest_task
                    .as_ref()
                    .map(|task| task.phase.clone())
                    .unwrap_or_else(|| "planning".to_string());
                let details = latest_task
                    .as_ref()
                    .and_then(|task| task.intent_summary.clone())
                    .or_else(|| latest_task.as_ref().map(|task| task.title.clone()))
                    .unwrap_or_else(|| "update".to_string());
                return Ok(RuntimeStatusProjection {
                    track_id: track_id.to_string(),
                    task_id: latest_task.as_ref().map(|task| task.id.clone()),
                    target_path: PROJECTED_RUNTIME_STATUS_PATH.to_string(),
                    status_line: format!(
                        "phase: {phase} | status: {} | details: {details}",
                        status.clone().unwrap_or_else(|| "running".to_string())
                    ),
                    status,
                    phase: Some(phase),
                    next: self.track_next_step(track_id)?,
                    executor: latest_task.as_ref().and_then(|task| task.executor.clone()),
                    execution: latest_task.as_ref().and_then(|task| task.execution.clone()),
                    intent_summary: latest_task
                        .as_ref()
                        .and_then(|task| task.intent_summary.clone()),
                    source: "canonical-event-store".to_string(),
                });
            }
        };
        let task = self.task_for_event(&event)?;
        let projection = self.engine.project_runtime_view(&event, task.as_ref());
        Ok(RuntimeStatusProjection {
            track_id: track_id.to_string(),
            task_id: event.task_id.clone(),
            target_path: projection.target_path,
            status_line: projection.line,
            status: Some(projection.status),
            phase: Some(projection.phase),
            next: self.next_step_from_event(&event, track_id)?,
            executor: task.as_ref().and_then(|task| task.executor.clone()),
            execution: task.as_ref().and_then(|task| task.execution.clone()),
            intent_summary: self.intent_summary_for_event(&event, task.as_ref()),
            source: "canonical-event-store".to_string(),
        })
    }

    pub fn get_runtime_status(&self, track_id: &str) -> Result<RuntimeStatusProjection> {
        self.try_refresh_dirty_projection_targets();
        let target_path = self
            .store
            .get_latest_projection_for_track(track_id, "runtime")?
            .and_then(|projection| projection.target_path)
            .unwrap_or_else(|| PROJECTED_RUNTIME_STATUS_PATH.to_string());

        let mut derived = self.project_runtime_status_without_refresh(track_id)?;
        derived.target_path = target_path;
        derived.source = "derived-projection".to_string();
        Ok(derived)
    }

    pub fn read_task_visibility(&self, task_id: &str) -> Result<TaskVisibilityView> {
        self.try_refresh_dirty_projection_targets();
        let task = self.store.get_task(task_id)?;
        let latest_track_status = task
            .track_id
            .as_deref()
            .and_then(|track_id| self.store.get_track(track_id).ok())
            .map(|track| track.status);
        let next = match task.track_id.as_deref() {
            Some(track_id) => self.track_next_step(track_id)?,
            None => None,
        };

        Ok(TaskVisibilityView {
            source: "emberflow-canonical-state".to_string(),
            task_id: task.id,
            track_id: task.track_id,
            title: task.title,
            status: task.status,
            phase: task.phase,
            executor: task.executor,
            execution: task.execution,
            intent_summary: task.intent_summary,
            track_status: latest_track_status,
            next,
            updated_at: task.updated_at,
            lease_holder: task.lease_holder,
            lease_expires_at: task.lease_expires_at,
        })
    }

    fn workspace_overview_content(&self, overview: &WorkspaceOverview) -> serde_json::Value {
        serde_json::json!({
            "source": overview.source,
            "projectionMode": overview.projection_mode,
            "tracks": overview.tracks.iter().map(|track| {
                serde_json::json!({
                    "trackId": track.track_id,
                    "title": track.title,
                    "trackType": track.track_type,
                    "status": track.status,
                    "description": track.description,
                    "updatedAt": track.updated_at,
                    "executor": track.executor,
                    "execution": track.execution,
                    "intentSummary": track.intent_summary,
                    "next": track.next,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn track_resume_content(&self, view: &TrackResumeView) -> serde_json::Value {
        serde_json::json!({
            "source": view.source,
            "trackId": view.track_id,
            "title": view.title,
            "trackType": view.track_type,
            "status": view.status,
            "description": view.description,
            "branch": view.branch,
            "specRef": view.spec_ref,
            "summarySections": view.summary_sections.iter().map(|section| {
                serde_json::json!({
                    "trackId": section.track_id,
                    "sectionKey": section.section_key,
                    "sectionText": section.section_text,
                    "position": section.position,
                    "createdAt": section.created_at,
                    "updatedAt": section.updated_at,
                })
            }).collect::<Vec<_>>(),
            "plan": self.track_plan_content(&view.plan),
            "taskId": view.task_id,
            "executor": view.executor,
            "execution": view.execution,
            "intentSummary": view.intent_summary,
            "next": view.next,
            "currentPhase": view.current_phase,
        })
    }

    fn track_plan_content(&self, plan: &TrackPlanRecord) -> serde_json::Value {
        serde_json::json!({
            "trackId": plan.track_id,
            "phases": plan.phases.iter().map(|phase| {
                serde_json::json!({
                    "phaseId": phase.phase_id,
                    "trackId": phase.track_id,
                    "title": phase.title,
                    "position": phase.position,
                    "items": phase.items.iter().map(|item| {
                        serde_json::json!({
                            "itemId": item.item_id,
                            "trackId": item.track_id,
                            "phaseId": item.phase_id,
                            "title": item.title,
                            "position": item.position,
                            "createdAt": item.created_at,
                            "updatedAt": item.updated_at,
                        })
                    }).collect::<Vec<_>>(),
                    "createdAt": phase.created_at,
                    "updatedAt": phase.updated_at,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn track_brief_content(&self, brief: &TrackBriefRecord) -> serde_json::Value {
        serde_json::json!({
            "trackId": brief.track_id,
            "sections": brief.sections.iter().map(|section| {
                serde_json::json!({
                    "trackId": section.track_id,
                    "sectionKey": section.section_key,
                    "sectionText": section.section_text,
                    "position": section.position,
                    "createdAt": section.created_at,
                    "updatedAt": section.updated_at,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn track_runtime_content(&self, runtime: &RuntimeStatusProjection) -> serde_json::Value {
        serde_json::json!({
            "trackId": runtime.track_id,
            "taskId": runtime.task_id,
            "targetPath": runtime.target_path,
            "statusLine": runtime.status_line,
            "status": runtime.status,
            "phase": runtime.phase,
            "next": runtime.next,
            "executor": runtime.executor,
            "execution": runtime.execution,
            "intentSummary": runtime.intent_summary,
            "source": runtime.source,
        })
    }

    fn track_transparency_content(&self, view: &TrackTransparencyView) -> serde_json::Value {
        serde_json::json!({
            "source": view.source,
            "trackId": view.track_id,
            "taskId": view.task_id,
            "trackStatus": view.track_status,
            "taskStatus": view.task_status,
            "phase": view.phase,
            "executor": view.executor,
            "execution": view.execution,
            "intentSummary": view.intent_summary,
            "next": view.next,
            "updatedAt": view.updated_at,
        })
    }

    fn task_visibility_content(&self, visibility: &TaskVisibilityView) -> serde_json::Value {
        serde_json::json!({
            "source": visibility.source,
            "taskId": visibility.task_id,
            "trackId": visibility.track_id,
            "title": visibility.title,
            "status": visibility.status,
            "phase": visibility.phase,
            "executor": visibility.executor,
            "execution": visibility.execution,
            "intentSummary": visibility.intent_summary,
            "trackStatus": visibility.track_status,
            "next": visibility.next,
            "updatedAt": visibility.updated_at,
            "leaseHolder": visibility.lease_holder,
            "leaseExpiresAt": visibility.lease_expires_at,
        })
    }

    fn track_record_content(&self, track: &TrackRecord) -> serde_json::Value {
        serde_json::json!({
            "trackId": track.id,
            "title": track.title,
            "status": track.status,
            "createdAt": track.created_at,
            "updatedAt": track.updated_at,
        })
    }

    fn track_context_content(&self, context: &TrackContextRecord) -> serde_json::Value {
        serde_json::json!({
            "trackId": context.track_id,
            "metadata": {
                "trackId": context.metadata.track_id,
                "trackType": context.metadata.track_type,
                "status": context.metadata.status,
                "description": context.metadata.description,
                "branch": context.metadata.branch,
                "specRef": context.metadata.spec_ref,
                "createdAt": context.metadata.created_at,
                "updatedAt": context.metadata.updated_at,
            },
            "brief": self.track_brief_content(&context.brief),
            "plan": self.track_plan_content(&context.plan),
        })
    }

    fn event_feed_content(&self, feed: &EventFeed) -> serde_json::Value {
        serde_json::json!({
            "source": "emberflow-canonical-state",
            "trackId": feed.track_id,
            "taskId": feed.task_id,
            "items": feed.items.iter().map(|event| {
                serde_json::json!({
                    "id": event.id,
                    "trackId": event.track_id,
                    "taskId": event.task_id,
                    "kind": event.kind,
                    "payload": event.payload,
                    "createdAt": event.created_at,
                })
            }).collect::<Vec<_>>(),
        })
    }

    fn client_contract_content(&self) -> serde_json::Value {
        serde_json::json!({
            "source": "emberflow-canonical-state",
            "role": "canonical tracked runtime and visibility layer",
            "initializeFirst": true,
            "projectedFiles": "derived-only",
            "mutations": "through-emberflow-only",
            "readSequence": [
                "list_resources",
                "read_resource"
            ],
            "resources": self
                .available_resource_views()
                .into_iter()
                .map(|view| view.uri_template)
                .collect::<Vec<_>>(),
            "transparency": {
                "resource": RESOURCE_TRACK_TRANSPARENCY_URI_TEMPLATE,
                "requiredFields": [
                    "source",
                    "trackId",
                    "trackStatus",
                    "taskStatus",
                    "phase",
                    "next"
                ],
                "recommendedFields": [
                    "taskId",
                    "executor",
                    "execution",
                    "intentSummary",
                    "updatedAt"
                ],
                "reloadAfterMutation": true,
                "displayUnavailableAs": "unavailable from EmberFlow"
            }
        })
    }

    fn track_events(&self, track_id: &str) -> Result<EventFeed> {
        self.try_refresh_dirty_projection_targets();
        Ok(EventFeed {
            track_id: Some(track_id.to_string()),
            task_id: None,
            items: self.store.list_events(Some(track_id), None, None)?,
        })
    }

    fn task_events(&self, task_id: &str) -> Result<EventFeed> {
        self.try_refresh_dirty_projection_targets();
        let task = self.store.get_task(task_id)?;
        Ok(EventFeed {
            track_id: task.track_id,
            task_id: Some(task_id.to_string()),
            items: self.store.list_events(None, Some(task_id), None)?,
        })
    }

    pub fn load_track_context(&self, track_id: &str) -> Result<TrackContextRecord> {
        self.try_refresh_dirty_projection_targets();
        Ok(TrackContextRecord {
            track_id: track_id.to_string(),
            metadata: self.store.get_track_metadata(track_id)?,
            brief: self.store.get_track_brief(track_id)?,
            plan: self.store.get_track_plan(track_id)?,
        })
    }

    pub fn read_workspace_overview(&self) -> Result<WorkspaceOverview> {
        self.try_refresh_dirty_projection_targets();
        let mut tracks = Vec::new();

        for track in self.store.list_active_tracks()? {
            let metadata = self.store.get_track_metadata_optional(&track.id)?;
            let runtime = self.get_runtime_status(&track.id).ok();
            let latest_task = self.store.get_latest_task_for_track(&track.id)?;

            tracks.push(WorkspaceOverviewTrack {
                track_id: track.id.clone(),
                title: track.title.clone(),
                track_type: metadata.as_ref().map(|value| value.track_type.clone()),
                status: metadata
                    .as_ref()
                    .map(|value| value.status.clone())
                    .unwrap_or_else(|| track.status.clone()),
                description: metadata.as_ref().map(|value| value.description.clone()),
                updated_at: metadata
                    .as_ref()
                    .map(|value| value.updated_at.clone())
                    .unwrap_or_else(|| track.updated_at.clone()),
                executor: runtime
                    .as_ref()
                    .and_then(|value| value.executor.clone())
                    .or_else(|| {
                        latest_task
                            .as_ref()
                            .and_then(|value| value.executor.clone())
                    }),
                execution: runtime
                    .as_ref()
                    .and_then(|value| value.execution.clone())
                    .or_else(|| {
                        latest_task
                            .as_ref()
                            .and_then(|value| value.execution.clone())
                    }),
                intent_summary: runtime
                    .as_ref()
                    .and_then(|value| value.intent_summary.clone())
                    .or_else(|| {
                        latest_task
                            .as_ref()
                            .and_then(|value| value.intent_summary.clone())
                    }),
                next: runtime.as_ref().and_then(|value| value.next.clone()),
            });
        }

        Ok(WorkspaceOverview {
            source: "emberflow-canonical-state".to_string(),
            projection_mode: self.layout.mode.as_str().to_string(),
            tracks,
        })
    }

    pub fn read_track_resume(&self, track_id: &str) -> Result<TrackResumeView> {
        self.try_refresh_dirty_projection_targets();
        let track = self.store.get_track(track_id)?;
        let metadata = self.store.get_track_metadata_optional(track_id)?;
        let brief = self.store.get_track_brief(track_id)?;
        let plan = self.store.get_track_plan(track_id)?;
        let runtime = self.get_runtime_status(track_id)?;
        let latest_task = self.store.get_latest_task_for_track(track_id)?;

        Ok(TrackResumeView {
            source: "emberflow-canonical-state".to_string(),
            track_id: track.id.clone(),
            title: track.title,
            track_type: metadata.as_ref().map(|value| value.track_type.clone()),
            status: metadata
                .as_ref()
                .map(|value| value.status.clone())
                .unwrap_or(track.status),
            description: metadata.as_ref().map(|value| value.description.clone()),
            branch: metadata.as_ref().map(|value| value.branch.clone()),
            spec_ref: metadata.as_ref().and_then(|value| value.spec_ref.clone()),
            summary_sections: brief.sections,
            current_phase: runtime.phase.clone(),
            plan,
            task_id: runtime
                .task_id
                .or_else(|| latest_task.as_ref().map(|value| value.id.clone())),
            executor: runtime.executor.or_else(|| {
                latest_task
                    .as_ref()
                    .and_then(|value| value.executor.clone())
            }),
            execution: runtime.execution.or_else(|| {
                latest_task
                    .as_ref()
                    .and_then(|value| value.execution.clone())
            }),
            intent_summary: runtime.intent_summary.or_else(|| {
                latest_task
                    .as_ref()
                    .and_then(|value| value.intent_summary.clone())
            }),
            next: runtime.next,
        })
    }

    pub fn read_track_transparency(&self, track_id: &str) -> Result<TrackTransparencyView> {
        self.try_refresh_dirty_projection_targets();
        let track = self.store.get_track(track_id)?;
        let metadata = self.store.get_track_metadata_optional(track_id)?;
        let runtime = self.get_runtime_status(track_id)?;
        let current_task = match runtime.task_id.as_deref() {
            Some(task_id) => Some(self.store.get_task(task_id)?),
            None => self.store.get_latest_task_for_track(track_id)?,
        };
        let track_status = metadata
            .as_ref()
            .map(|value| value.status.clone())
            .unwrap_or_else(|| track.status.clone());
        let updated_at = current_task
            .as_ref()
            .map(|task| task.updated_at.clone())
            .or_else(|| metadata.as_ref().map(|value| value.updated_at.clone()))
            .unwrap_or(track.updated_at.clone());

        Ok(TrackTransparencyView {
            source: "emberflow-canonical-state".to_string(),
            track_id: track.id,
            task_id: runtime
                .task_id
                .clone()
                .or_else(|| current_task.as_ref().map(|task| task.id.clone())),
            track_status,
            task_status: runtime
                .status
                .clone()
                .or_else(|| current_task.as_ref().map(|task| task.status.clone())),
            phase: runtime
                .phase
                .clone()
                .or_else(|| current_task.as_ref().map(|task| task.phase.clone())),
            executor: runtime
                .executor
                .clone()
                .or_else(|| current_task.as_ref().and_then(|task| task.executor.clone())),
            execution: runtime.execution.clone().or_else(|| {
                current_task
                    .as_ref()
                    .and_then(|task| task.execution.clone())
            }),
            intent_summary: runtime.intent_summary.clone().or_else(|| {
                current_task
                    .as_ref()
                    .and_then(|task| task.intent_summary.clone())
            }),
            next: runtime.next.clone(),
            updated_at,
        })
    }

    pub fn read_track_brief(&self, track_id: &str) -> Result<TrackBriefRecord> {
        self.try_refresh_dirty_projection_targets();
        self.store.get_track_brief(track_id)
    }

    pub fn list_track_plan(&self, track_id: &str) -> Result<TrackPlanRecord> {
        self.try_refresh_dirty_projection_targets();
        self.store.get_track_plan(track_id)
    }

    pub fn upsert_track_metadata(&self, input: TrackMetadataInput) -> Result<TrackMetadataRecord> {
        let track_id = input.track_id.clone();
        let metadata = self.store.upsert_track_metadata(input)?;
        self.mark_track_metadata_dirty(&track_id, "track-metadata", None)?;
        self.mark_track_summary_dirty(&track_id, "track-metadata", None)?;
        self.try_refresh_dirty_projection_targets();
        Ok(metadata)
    }

    pub fn replace_track_brief(
        &self,
        track_id: &str,
        sections: Vec<TrackBriefSectionInput>,
    ) -> Result<TrackBriefRecord> {
        let brief = self.store.replace_track_brief(track_id, sections)?;
        self.mark_track_brief_dirty(track_id, "track-brief", None)?;
        self.try_refresh_dirty_projection_targets();
        Ok(brief)
    }

    pub fn replace_track_plan(
        &self,
        track_id: &str,
        phases: Vec<TrackPlanPhaseInput>,
    ) -> Result<TrackPlanRecord> {
        let plan = self.store.replace_track_plan(track_id, phases)?;
        self.mark_track_plan_dirty(track_id, "track-plan", None)?;
        self.try_refresh_dirty_projection_targets();
        Ok(plan)
    }

    fn try_refresh_dirty_projection_targets(&self) {
        let _ = self.refresh_dirty_projection_targets();
    }

    fn mark_track_metadata_dirty(
        &self,
        track_id: &str,
        reason: &str,
        source_event_id: Option<&str>,
    ) -> Result<()> {
        if self.layout.mode == EmberFlowMode::Canonical {
            return Ok(());
        }
        for target in self.track_metadata_dirty_targets(track_id, reason, source_event_id) {
            self.store.record_dirty_projection_target(target)?;
        }
        Ok(())
    }

    fn mark_track_brief_dirty(
        &self,
        track_id: &str,
        reason: &str,
        source_event_id: Option<&str>,
    ) -> Result<()> {
        if self.layout.mode == EmberFlowMode::Canonical {
            return Ok(());
        }
        for target in self.track_brief_dirty_targets(track_id, reason, source_event_id) {
            self.store.record_dirty_projection_target(target)?;
        }
        Ok(())
    }

    fn mark_track_plan_dirty(
        &self,
        track_id: &str,
        reason: &str,
        source_event_id: Option<&str>,
    ) -> Result<()> {
        if self.layout.mode == EmberFlowMode::Canonical {
            return Ok(());
        }
        for target in self.track_plan_dirty_targets(track_id, reason, source_event_id) {
            self.store.record_dirty_projection_target(target)?;
        }
        Ok(())
    }

    fn mark_runtime_projection_dirty(
        &self,
        track_id: Option<&str>,
        source_event_id: Option<&str>,
    ) -> Result<()> {
        if self.layout.mode == EmberFlowMode::Canonical {
            return Ok(());
        }

        self.store
            .record_dirty_projection_target(ProjectionDirtyTargetInput {
                track_id: track_id.map(ToString::to_string),
                projection_kind: "runtime-status".to_string(),
                target_path: PROJECTED_RUNTIME_STATUS_PATH.to_string(),
                reason: "runtime-state".to_string(),
                source_event_id: source_event_id.map(ToString::to_string),
            })?;
        if let Some(track_id) = track_id {
            self.mark_track_summary_dirty(track_id, "runtime-state", source_event_id)?;
        }

        Ok(())
    }

    fn mark_track_summary_dirty(
        &self,
        track_id: &str,
        reason: &str,
        source_event_id: Option<&str>,
    ) -> Result<()> {
        if self.layout.mode == EmberFlowMode::Canonical {
            return Ok(());
        }
        for target in self.track_summary_dirty_targets(track_id, reason, source_event_id) {
            self.store.record_dirty_projection_target(target)?;
        }
        Ok(())
    }

    fn track_summary_dirty_targets(
        &self,
        track_id: &str,
        reason: &str,
        source_event_id: Option<&str>,
    ) -> Vec<ProjectionDirtyTargetInput> {
        vec![
            ProjectionDirtyTargetInput {
                track_id: Some(track_id.to_string()),
                projection_kind: "track-list".to_string(),
                target_path: ".emberflow/tracks/tracks.md".to_string(),
                reason: reason.to_string(),
                source_event_id: source_event_id.map(ToString::to_string),
            },
            ProjectionDirtyTargetInput {
                track_id: Some(track_id.to_string()),
                projection_kind: "track-summary".to_string(),
                target_path: self.track_target_path(track_id, "index.md"),
                reason: reason.to_string(),
                source_event_id: source_event_id.map(ToString::to_string),
            },
        ]
    }

    fn track_metadata_dirty_targets(
        &self,
        track_id: &str,
        reason: &str,
        source_event_id: Option<&str>,
    ) -> Vec<ProjectionDirtyTargetInput> {
        vec![ProjectionDirtyTargetInput {
            track_id: Some(track_id.to_string()),
            projection_kind: "track-metadata".to_string(),
            target_path: self.track_target_path(track_id, "metadata.json"),
            reason: reason.to_string(),
            source_event_id: source_event_id.map(ToString::to_string),
        }]
    }

    fn track_brief_dirty_targets(
        &self,
        track_id: &str,
        reason: &str,
        source_event_id: Option<&str>,
    ) -> Vec<ProjectionDirtyTargetInput> {
        let mut targets = self.track_summary_dirty_targets(track_id, reason, source_event_id);
        targets.push(ProjectionDirtyTargetInput {
            track_id: Some(track_id.to_string()),
            projection_kind: "track-brief".to_string(),
            target_path: self.track_target_path(track_id, "brief.md"),
            reason: reason.to_string(),
            source_event_id: source_event_id.map(ToString::to_string),
        });
        targets
    }

    fn track_plan_dirty_targets(
        &self,
        track_id: &str,
        reason: &str,
        source_event_id: Option<&str>,
    ) -> Vec<ProjectionDirtyTargetInput> {
        let mut targets = self.track_summary_dirty_targets(track_id, reason, source_event_id);
        targets.push(ProjectionDirtyTargetInput {
            track_id: Some(track_id.to_string()),
            projection_kind: "track-plan".to_string(),
            target_path: self.track_target_path(track_id, "plan.md"),
            reason: reason.to_string(),
            source_event_id: source_event_id.map(ToString::to_string),
        });
        targets
    }

    fn track_directory_path(&self, track_id: &str) -> String {
        format!("{}{track_id}/", self.layout.track_directory_prefix())
    }

    fn track_target_path(&self, track_id: &str, file_name: &str) -> String {
        format!(
            "{}{track_id}/{file_name}",
            self.layout.track_directory_prefix()
        )
    }

    fn remove_projected_track_artifacts(&self, track_id: &str) -> Result<()> {
        if self.layout.mode == EmberFlowMode::Canonical {
            return Ok(());
        }

        let track_directory = self
            .layout
            .project_root
            .join(self.track_directory_path(track_id));
        if track_directory.exists() {
            fs::remove_dir_all(track_directory)?;
        }

        let track_list_contents = self.render_track_list_target()?;
        self.write_projected_file(".emberflow/tracks/tracks.md", &track_list_contents)?;
        Ok(())
    }

    fn persist_projections(&self, event: &EventRecord) -> Result<()> {
        let user_projection = self.engine.project_user_view(event);
        self.store.record_projection(
            &event.id,
            "user",
            None,
            json!({
                "event_id": user_projection.event_id,
                "kind": user_projection.kind,
                "format": user_projection.format,
                "summary": user_projection.summary,
                "line": user_projection.line,
            }),
        )?;

        let task = self.task_for_event(event)?;
        let runtime_projection = self.engine.project_runtime_view(event, task.as_ref());
        let track_id = event.track_id.as_deref().unwrap_or_default();
        self.store.record_projection(
            &event.id,
            "runtime",
            Some(PROJECTED_RUNTIME_STATUS_PATH),
            json!({
                "event_id": runtime_projection.event_id,
                "kind": runtime_projection.kind,
                "task_id": event.task_id,
                "target_path": runtime_projection.target_path,
                "line_format": runtime_projection.line_format,
                "line": runtime_projection.line,
                "status": runtime_projection.status,
                "phase": runtime_projection.phase,
                "details": runtime_projection.details,
                "executor": task.as_ref().and_then(|task| task.executor.clone()),
                "execution": task.as_ref().and_then(|task| task.execution.clone()),
                "intent_summary": self.intent_summary_for_event(event, task.as_ref()),
                "next": if track_id.is_empty() { None } else { self.next_step_from_event(event, track_id)? },
            }),
        )?;

        if let Some(track) = self.track_for_event(event)? {
            let track_projection = self.engine.project_track_view(event, &track);
            if track_projection.durable_change != "none" {
                self.store.record_projection(
                    &event.id,
                    "track",
                    None,
                    json!({
                        "event_id": track_projection.event_id,
                        "track_id": track_projection.track_id,
                        "kind": track_projection.kind,
                        "summary": track_projection.summary,
                        "durable_change": track_projection.durable_change,
                        "status": track_projection.status,
                    }),
                )?;
            }
        }

        Ok(())
    }

    fn project_runtime_state(&self, event: &EventRecord) -> Result<()> {
        if let Some(track) = self.track_for_event(event)? {
            let projection = self.engine.project_track_view(event, &track);
            if let Some(status) = projection.status.as_deref() {
                if status != track.status {
                    self.store.update_track_status(&track.id, status)?;
                }
            }
        }

        if let Some(task) = self.task_for_event(event)? {
            let (status, phase) = self.task_state_for_event(&event.kind, &event.payload, &task);
            self.store.update_task_state(
                &task.id,
                crate::runtime::store::TaskStateUpdate {
                    status: Some(&status),
                    phase: Some(&phase),
                    track_id: event.track_id.as_deref().or(task.track_id.as_deref()),
                    executor: self
                        .executor_from_payload(&event.payload)
                        .or(task.executor.as_deref()),
                    agent_instance_id: event
                        .payload
                        .get("agent_instance_id")
                        .and_then(serde_json::Value::as_str),
                    execution: event
                        .payload
                        .get("execution")
                        .and_then(serde_json::Value::as_str)
                        .or(task.execution.as_deref()),
                    intent_summary: event
                        .payload
                        .get("intent_summary")
                        .and_then(serde_json::Value::as_str)
                        .or(task.intent_summary.as_deref()),
                },
            )?;
        }

        Ok(())
    }

    fn track_for_event(&self, event: &EventRecord) -> Result<Option<TrackRecord>> {
        match event.track_id.as_deref() {
            Some(track_id) => Ok(Some(self.store.get_track(track_id)?)),
            None => Ok(None),
        }
    }

    fn task_for_event(&self, event: &EventRecord) -> Result<Option<TaskRecord>> {
        match event.task_id.as_deref() {
            Some(task_id) => Ok(Some(self.store.get_task(task_id)?)),
            None => Ok(None),
        }
    }

    fn task_state_for_event(
        &self,
        kind: &str,
        payload: &serde_json::Value,
        current_task: &TaskRecord,
    ) -> (String, String) {
        match kind {
            "assign" => (
                "queued".to_string(),
                payload
                    .get("phase")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(&current_task.phase)
                    .to_string(),
            ),
            "ack" => ("running".to_string(), "planning".to_string()),
            "progress" => (
                payload
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(&current_task.status)
                    .to_string(),
                payload
                    .get("phase")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(current_task.phase.as_str())
                    .to_string(),
            ),
            "blocker" => (
                "blocked".to_string(),
                payload
                    .get("phase")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(current_task.phase.as_str())
                    .to_string(),
            ),
            "handoff" => (
                "awaiting-review".to_string(),
                payload
                    .get("phase")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("verifying")
                    .to_string(),
            ),
            "close" => (
                "done".to_string(),
                payload
                    .get("phase")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("verifying")
                    .to_string(),
            ),
            _ => (current_task.status.clone(), current_task.phase.clone()),
        }
    }

    fn executor_from_payload<'a>(&self, payload: &'a serde_json::Value) -> Option<&'a str> {
        payload
            .get("executor")
            .and_then(serde_json::Value::as_str)
            .or_else(|| payload.get("agent").and_then(serde_json::Value::as_str))
    }

    fn intent_summary_for_event(
        &self,
        event: &EventRecord,
        task: Option<&TaskRecord>,
    ) -> Option<String> {
        event
            .payload
            .get("intent_summary")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
            .or_else(|| task.and_then(|task| task.intent_summary.clone()))
    }

    fn runtime_details_for_state(
        &self,
        event: Option<&EventRecord>,
        task: Option<&TaskRecord>,
    ) -> String {
        if let Some(event) = event {
            [
                "summary",
                "current_action",
                "understanding",
                "goal",
                "recommendation",
                "details",
            ]
            .iter()
            .find_map(|key| {
                event
                    .payload
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
            .unwrap_or_else(|| {
                task.and_then(|task| task.intent_summary.clone())
                    .or_else(|| task.map(|task| task.title.clone()))
                    .unwrap_or_else(|| "update".to_string())
            })
        } else {
            task.and_then(|task| task.intent_summary.clone())
                .or_else(|| task.map(|task| task.title.clone()))
                .unwrap_or_else(|| "update".to_string())
        }
    }

    fn runtime_status_line_without_refresh(&self, track_id: &str) -> Result<String> {
        let latest_task = self.store.get_latest_task_for_track(track_id)?;
        let latest_event = self.store.get_latest_event_for_track(track_id)?;

        if let Some(task) = latest_task.as_ref() {
            return Ok(format!(
                "phase: {} | status: {} | details: {}",
                task.phase,
                task.status,
                self.runtime_details_for_state(latest_event.as_ref(), Some(task))
            ));
        }

        if let Some(event) = latest_event.as_ref() {
            let task = self.task_for_event(event)?;
            return Ok(self.engine.project_runtime_view(event, task.as_ref()).line);
        }

        let status = latest_task
            .as_ref()
            .map(|task| task.status.clone())
            .or_else(|| {
                self.store
                    .get_track(track_id)
                    .ok()
                    .map(|track| track.status)
            });
        let phase = latest_task
            .as_ref()
            .map(|task| task.phase.clone())
            .unwrap_or_else(|| "planning".to_string());
        let details = latest_task
            .as_ref()
            .and_then(|task| task.intent_summary.clone())
            .or_else(|| latest_task.as_ref().map(|task| task.title.clone()))
            .unwrap_or_else(|| "update".to_string());

        Ok(format!(
            "phase: {phase} | status: {} | details: {details}",
            status.unwrap_or_else(|| "running".to_string())
        ))
    }

    fn next_step_from_event(&self, event: &EventRecord, track_id: &str) -> Result<Option<String>> {
        Ok(event
            .payload
            .get("recommended_next_step")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
            .or_else(|| {
                event
                    .payload
                    .get("next")
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
            .or_else(|| {
                event
                    .payload
                    .get("next_step")
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
            .or(self.track_next_step(track_id)?))
    }

    fn track_next_step(&self, track_id: &str) -> Result<Option<String>> {
        match self.store.get_track_brief(track_id) {
            Ok(brief) => Ok(brief
                .sections
                .into_iter()
                .find(|section| section.section_key == "next_step")
                .map(|section| section.section_text)),
            Err(EmberFlowError::NotFound(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
