spec emberflow-execution-guard v0.1.0
title "EmberFlow execution guard"

description
  Defines task lease behaviors that give agents exclusive ownership of tasks,
  with enforcement on mutations and lazy lease expiration.

motivation
  Tasks currently have visibility fields (executor, execution, intent_summary)
  but these are purely informational. Any agent can mutate any task at any time.
  Execution guard adds opt-in exclusive leases so agents can claim ownership,
  preventing races and accidental overwrites in multi-agent workflows.

behavior claim-task-acquires-exclusive-lease [happy_path]
  "When an agent claims an unclaimed task, EmberFlow records a lease and returns the lease info"

  given
    A task exists without an active lease
    @task = Task {{ id: "task-001" }}

  when claim-task
    taskId = @task.id
    holder = "agent-a"
    durationSecs = 300

  then returns leaseInfo
    assert holder == "agent-a"
    assert acquired_at is_present
    assert expires_at is_present

  then side_effect
    assert A claim event is recorded for the task

behavior same-holder-reclaim-refreshes-lease [happy_path]
  "When the same holder re-claims a task it already holds, EmberFlow refreshes the lease expiry"

  given
    A task already has an active lease held by agent-a
    @task = Task {{ id: "task-001" }}
    @lease = Lease {{ holder: "agent-a" }}

  when claim-task
    taskId = @task.id
    holder = "agent-a"
    durationSecs = 600

  then returns leaseInfo
    assert holder == "agent-a"
    assert expires_at is_present

behavior different-holder-claim-rejected [error_case]
  "When a different holder tries to claim a task with an active lease, EmberFlow rejects the claim"

  given
    A task has an active lease held by agent-a
    @task = Task {{ id: "task-001" }}
    @lease = Lease {{ holder: "agent-a" }}

  when claim-task
    taskId = @task.id
    holder = "agent-b"

  then returns leaseConflictError
    assert reason contains "already held"

behavior release-clears-lease [happy_path]
  "When the lease holder releases a task, EmberFlow clears the lease fields"

  given
    A task has an active lease held by agent-a
    @task = Task {{ id: "task-001" }}

  when release-task
    taskId = @task.id
    holder = "agent-a"

  then returns success
    assert lease is_cleared

  then side_effect
    assert A release event is recorded for the task

behavior release-by-wrong-holder-fails [error_case]
  "When a non-holder tries to release a task lease, EmberFlow rejects the request"

  given
    A task has an active lease held by agent-a
    @task = Task {{ id: "task-001" }}

  when release-task
    taskId = @task.id
    holder = "agent-b"

  then returns authorizationError
    assert reason contains "not the lease holder"

behavior expired-lease-auto-cleared-on-access [edge_case]
  "When a lease has expired, EmberFlow auto-clears it on the next access and returns None"

  given
    A task has a lease that has already expired
    @task = Task {{ id: "task-001" }}
    @lease = Lease {{ holder: "agent-a", expires_at: "2020-01-01T00:00:00Z" }}

  when check-lease
    taskId = @task.id

  then returns noActiveLease
    assert lease == "none"

  then side_effect
    assert The expired lease fields are cleared from the task record

behavior event-record-gated-by-lease [error_case]
  "When a task has an active lease and a different executor tries to record an event, EmberFlow rejects it"

  given
    A task has an active lease held by agent-a
    @task = Task {{ id: "task-001" }}
    @lease = Lease {{ holder: "agent-a" }}

  when record-event
    taskId = @task.id
    executor = "agent-b"
    kind = "progress"

  then returns leaseConflictError
    assert reason contains "lease held by"

behavior task-event-requires-active-lease [error_case]
  "When a task has no active lease, any attempt to record an event targeting it is rejected"

  given
    A task exists without an active lease
    @task = Task {{ id: "task-001" }}

  when record-event
    taskId = @task.id
    executor = "agent-x"
    kind = "progress"

  then returns leaseRequiredError
    assert reason contains "no active lease"

behavior lease-state-in-transparency [happy_path]
  "When a task visibility resource is read, EmberFlow includes lease holder and expiry in the response"

  given
    A task has an active lease held by agent-a
    @task = Task {{ id: "task-001" }}
    @lease = Lease {{ holder: "agent-a", expires_at: "2099-01-01T00:00:00Z" }}

  when read-resource
    uri = "emberflow://tasks/task-001/visibility"

  then returns resource
    assert leaseHolder == "agent-a"
    assert leaseExpiresAt is_present

behavior expire-stale-leases-bulk-cleanup [happy_path]
  "EmberFlow can bulk-expire all stale leases in a single operation and return the count"

  given
    Multiple tasks have leases that have already expired
    @task_a = Task {{ id: "task-a", lease_expires_at: "2020-01-01T00:00:00Z" }}
    @task_b = Task {{ id: "task-b", lease_expires_at: "2020-01-01T00:00:00Z" }}

  when expire-stale-leases

  then returns expiredCount
    assert count == 2

  then side_effect
    assert All expired lease fields are cleared from the task records

behavior new-event-kinds-claim-release-lease-expired [happy_path]
  "EmberFlow accepts claim, release, and lease-expired as valid protocol event kinds"

  given
    A task and track exist

  when record-event
    kind = "claim"

  then returns eventRecord
    assert kind == "claim"

depends on emberflow-runtime-store >= 0.3.0
