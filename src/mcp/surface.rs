use crate::error::Result;
pub use crate::runtime::service::{
    EmberFlowMode, EmberFlowProjectLayout, EmberFlowRuntime, EventFeed, InitializeResponse,
    KnowledgeViewDescriptor, LeaseInfo, ResourceReadResponse, ResourceViewDescriptor,
    RuntimeStateResult, RuntimeStatusProjection, TaskVisibilityView, TrackBootstrapInfo,
    TrackBriefRecord, TrackBriefSectionInput, TrackBriefSectionRecord, TrackContextRecord,
    TrackMetadataInput, TrackMetadataRecord, TrackPlanItemInput, TrackPlanItemRecord,
    TrackPlanPhaseInput, TrackPlanPhaseRecord, TrackPlanRecord, TrackResumeView,
    TrackTransparencyView, WorkspaceDbInfo, WorkspaceOverview, WorkspaceOverviewTrack,
};
pub use crate::runtime::store::{EventRecord, TaskInput, TaskRecord, TrackRecord};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct EmberFlowSurface {
    runtime: EmberFlowRuntime,
}

impl EmberFlowSurface {
    pub fn from_workspace_root<P: AsRef<Path>>(workspace_root: P) -> Result<Self> {
        Ok(Self {
            runtime: EmberFlowRuntime::from_workspace_root(workspace_root)?,
        })
    }

    pub fn from_db_path<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        Ok(Self {
            runtime: EmberFlowRuntime::from_db_path(db_path)?,
        })
    }

    #[deprecated(
        note = "use EmberFlowSurface::from_workspace_root for workspace discovery or EmberFlowSurface::from_db_path for explicit db_path compatibility"
    )]
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        Self::from_db_path(db_path)
    }

    pub fn available_tools(&self) -> Vec<&'static str> {
        self.runtime.available_tools()
    }

    pub fn initialize(&self) -> Result<InitializeResponse> {
        self.runtime.initialize()
    }

    pub fn list_resources(&self) -> Vec<ResourceViewDescriptor> {
        self.runtime.list_resource_views()
    }

    pub fn list_tracks(&self) -> Result<Vec<TrackRecord>> {
        self.runtime.list_tracks()
    }

    pub fn list_active_tracks(&self) -> Result<Vec<TrackRecord>> {
        self.runtime.list_active_tracks()
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskRecord>> {
        self.runtime.list_tasks()
    }

    pub fn read_resource(&self, uri: &str) -> Result<ResourceReadResponse> {
        self.runtime.read_resource(uri)
    }

    pub fn create_track(&self, track_id: &str, title: &str, status: &str) -> Result<TrackRecord> {
        self.runtime.create_track(track_id, title, status)
    }

    pub fn archive_track(&self, track_id: &str) -> Result<TrackRecord> {
        self.runtime.archive_track(track_id)
    }

    pub fn delete_track(&self, track_id: &str) -> Result<TrackRecord> {
        self.runtime.delete_track(track_id)
    }

    pub fn create_task(&self, input: TaskInput) -> Result<TaskRecord> {
        self.runtime.create_task(input)
    }

    pub fn record_event(
        &self,
        event_id: &str,
        track_id: Option<&str>,
        task_id: Option<&str>,
        kind: &str,
        payload: serde_json::Value,
    ) -> Result<EventRecord> {
        self.runtime
            .record_event(event_id, track_id, task_id, kind, payload)
    }

    pub fn record_runtime_state(
        &self,
        track_id: &str,
        task_id: &str,
        event_kind: &str,
        payload: serde_json::Value,
    ) -> Result<RuntimeStateResult> {
        self.runtime
            .record_runtime_state(track_id, task_id, event_kind, payload)
    }

    pub fn upsert_track_metadata(&self, input: TrackMetadataInput) -> Result<TrackMetadataRecord> {
        self.runtime.upsert_track_metadata(input)
    }

    pub fn replace_track_brief(
        &self,
        track_id: &str,
        sections: Vec<TrackBriefSectionInput>,
    ) -> Result<TrackBriefRecord> {
        self.runtime.replace_track_brief(track_id, sections)
    }

    pub fn replace_track_plan(
        &self,
        track_id: &str,
        phases: Vec<TrackPlanPhaseInput>,
    ) -> Result<TrackPlanRecord> {
        self.runtime.replace_track_plan(track_id, phases)
    }

    pub fn claim_task(
        &self,
        task_id: &str,
        holder: &str,
        duration_secs: Option<u64>,
    ) -> Result<LeaseInfo> {
        self.runtime.claim_task(task_id, holder, duration_secs)
    }

    pub fn release_task(&self, task_id: &str, holder: &str) -> Result<()> {
        self.runtime.release_task(task_id, holder)
    }
}
