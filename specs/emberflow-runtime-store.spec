spec emberflow-runtime-store v0.3.0
title "EmberFlow runtime store"

description
  Defines the canonical runtime persistence behaviors for durable tracks,
  runtime tasks with visibility fields, append-only protocol events, and
  derived projections.

motivation
  EmberFlow needs a local runtime store that preserves durable work state,
  runtime execution state, user-visible task context, and protocol history
  without treating projections as the source of truth.

behavior create-track-with-durable-status [happy_path]
  "When a caller opens durable work in EmberFlow, the runtime persists a track with a valid track status"

  given
    A caller is creating durable work in EmberFlow

  when create-track
    trackId = "track-001"
    title = "Build EmberFlow V1"
    status = "planning"

  then returns trackRecord
    assert id == "track-001"
    assert title == "Build EmberFlow V1"
    assert status == "planning"
    assert createdAt is_present
    assert updatedAt is_present

behavior create-task-with-runtime-state [happy_path]
  "When a caller creates an execution unit, EmberFlow persists a task with runtime status and phase separately from the track"

  given
    A durable track already exists
    @track = Track {{ id: "track-001" }}

  when create-task
    taskId = "task-001"
    trackId = @track.id
    title = "Persist the first event"
    status = "queued"
    phase = "planning"

  then returns taskRecord
    assert id == "task-001"
    assert trackId == "track-001"
    assert title == "Persist the first event"
    assert status == "queued"
    assert phase == "planning"

behavior create-task-without-visibility-metadata [edge_case]
  "When a caller omits execution and intent_summary while creating a task, EmberFlow keeps the task valid and falls back to the generic executor label 'assistant'"

  given
    A durable track already exists
    @track = Track {{ id: "track-001" }}

  when create-task
    taskId = "task-001"
    trackId = @track.id
    title = "Persist the first event"
    status = "queued"
    phase = "planning"

  then returns taskRecord
    assert id == "task-001"
    assert trackId == "track-001"
    assert executor == "assistant"
    assert execution == "none"
    assert intent_summary == "none"

behavior create-task-with-visibility-context [happy_path]
  "When a caller creates a task with canonical visibility context, EmberFlow persists executor, execution, and intent_summary alongside runtime task state"

  given
    A durable track already exists
    @track = Track {{ id: "track-001" }}

  when create-task
    taskId = "task-visibility"
    trackId = @track.id
    title = "Investigate the transport contract"
    status = "running"
    phase = "planning"
    executor = "assistant"
    execution = "interactive session"
    intent_summary = "Investigate the transport contract end to end"

  then returns taskRecord
    assert id == "task-visibility"
    assert trackId == "track-001"
    assert executor == "assistant"
    assert execution == "interactive session"
    assert intent_summary == "Investigate the transport contract end to end"

behavior create-plan-review-task-with-need-input [happy_path]
  "When a caller creates a plan review task, EmberFlow persists need-input status with planning phase as canonical task execution state"

  given
    A durable track already exists
    @track = Track {{ id: "track-001" }}

  when create-task
    taskId = "task-plan-review"
    trackId = @track.id
    title = "Review and approve the execution plan"
    status = "need-input"
    phase = "planning"
    executor = "assistant"
    execution = "interactive session"
    intent_summary = "Review the plan before implementation"

  then returns taskRecord
    assert id == "task-plan-review"
    assert trackId == "track-001"
    assert status == "need-input"
    assert phase == "planning"
    assert intent_summary == "Review the plan before implementation"

behavior standalone-task-does-not-require-track [edge_case]
  "When a caller creates standalone runtime work, EmberFlow allows a task without durable track linkage"

  given
    A caller is creating a standalone runtime task

  when create-task
    taskId = "task-standalone"
    title = "Inspect recent events"
    status = "running"
    phase = "verifying"

  then returns taskRecord
    assert id == "task-standalone"
    assert title == "Inspect recent events"
    assert status == "running"
    assert phase == "verifying"

  then side_effect
    assert Track linkage remains optional for standalone runtime tasks

behavior create-reviewing-task [happy_path]
  "When a caller creates a task in the reviewing phase, EmberFlow persists it as a valid canonical execution state for cross-agent review workflows"

  given
    A caller is creating a review task on an existing track
    @track = Track {{ id: "track-review-001" }}

  when create-task
    taskId = "task-review-001"
    trackId = @track.id
    title = "Cross-review auth middleware"
    status = "running"
    phase = "reviewing"
    executor = "codex"

  then returns taskRecord
    assert id == "task-review-001"
    assert status == "running"
    assert phase == "reviewing"
    assert executor == "codex"

behavior record-event-with-track-and-task-context [happy_path]
  "When a caller records a canonical protocol event, EmberFlow persists the event with its task and track context"

  given
    Durable and runtime state already exist
    @track = Track {{ id: "track-001" }}
    @task = Task {{ id: "task-001" }}

  when record-event
    eventId = "event-001"
    trackId = @track.id
    taskId = @task.id
    kind = "progress"

  then returns eventRecord
    assert id == "event-001"
    assert trackId == "track-001"
    assert taskId == "task-001"
    assert kind == "progress"
    assert payload is_present

behavior update-task-runtime-state [happy_path]
  "When a caller updates canonical task execution state, EmberFlow persists the new task status and phase without rewriting the durable track status"

  given
    Durable and runtime state already exist
    @track = Track {{ id: "track-001", status: "planning" }}
    @task = Task {{ id: "task-001", trackId: @track.id, status: "queued", phase: "planning" }}

  when update-task
    taskId = @task.id
    status = "need-input"
    phase = "planning"
    intent_summary = "Review the plan before implementation"

  then returns taskRecord
    assert id == "task-001"
    assert status == "need-input"
    assert phase == "planning"
    assert intent_summary == "Review the plan before implementation"

  then side_effect
    assert The durable track remains planning because task execution state is stored separately from track lifecycle state

behavior store-multiple-projections-for-one-event [happy_path]
  "When one canonical event projects into multiple views, EmberFlow persists one projection row per projection kind"

  given
    A canonical event has already been recorded
    @event = Event {{ id: "event-001" }}

  when project-event
    eventId = @event.id
    projectionKind = "user"

  then returns projectionRecord
    assert eventId == "event-001"
    assert projectionKind == "user"
    assert payload is_present

  then side_effect
    assert The same event may also persist a runtime or track projection without overwriting the user projection

behavior invalid-status-is-rejected [error_case]
  "When a caller provides an unsupported status, EmberFlow rejects the write instead of storing an invalid record"

  given
    A caller submits a track status outside the supported contract

  when create-track
    trackId = "track-invalid"
    title = "Broken track"
    status = "paused"

  then returns validationError
    assert field == "status"
    assert reason contains "unsupported"

behavior invalid-task-status-is-rejected [error_case]
  "When a caller provides an unsupported task status, EmberFlow rejects the task write instead of storing invalid execution state"

  given
    A durable track already exists
    @track = Track {{ id: "track-001" }}

  when create-task
    taskId = "task-invalid"
    trackId = @track.id
    title = "Broken task"
    status = "planning"
    phase = "planning"

  then returns validationError
    assert field == "status"
    assert reason contains "unsupported"

behavior delete-track-removes-associated-runtime-state [happy_path]
  "When a caller deletes a durable track, EmberFlow removes the track and its related runtime tasks and protocol events instead of leaving orphaned state behind"

  given
    A durable track already exists with runtime tasks and protocol events
    @track = Track {{ id: "track-001" }}
    @task = Task {{ id: "task-001" }}
    @event = Event {{ id: "event-001" }}

  when delete-track
    trackId = @track.id

  then returns deletedTrackRecord
    assert id == "track-001"

  then side_effect
    assert Related task rows and event rows for the deleted track are removed from the runtime store
