#![forbid(unsafe_code)]

pub mod error;
pub mod mcp;
pub mod protocol;
pub mod runtime;

pub use error::{EmberFlowError, Result};
pub use mcp::surface::EmberFlowSurface;
pub use runtime::layout::{EmberFlowMode, EmberFlowProjectLayout};
pub use runtime::projections::{
    ProjectionEngine, RuntimeProjection, TrackProjection, UserProjection,
};
pub use runtime::service::{
    EmberFlowRuntime, EventFeed, InitializeResponse, KnowledgeViewDescriptor, ResourceReadResponse,
    ResourceViewDescriptor, RuntimeStateResult, RuntimeStatusProjection, TaskVisibilityView,
    TrackBootstrapInfo, TrackBriefRecord, TrackBriefSectionInput, TrackBriefSectionRecord,
    TrackContextRecord, TrackMetadataInput, TrackMetadataRecord, TrackPlanItemInput,
    TrackPlanItemRecord, TrackPlanPhaseInput, TrackPlanPhaseRecord, TrackPlanRecord,
    TrackResumeView, WorkspaceDbInfo, WorkspaceOverview, WorkspaceOverviewTrack,
};
pub use runtime::store::{
    EventRecord, ProjectionRecord, RuntimeStore, TaskInput, TaskRecord, TrackRecord,
};
